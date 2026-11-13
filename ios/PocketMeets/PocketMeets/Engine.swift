import AVFoundation
import Foundation
import UIKit
import UserNotifications

/// One serial queue owns the native session. UI work is posted back to main.
/// `pm_care_adopt` and `pm_advance_clock` stay in the C ABI and are not called.
final class Engine {
    let root: URL
    let queue = DispatchQueue(label: "app.pocketmeets.session")
    private var session: UnsafeMutablePointer<PmSession>?
    private var timer: DispatchSourceTimer?
    private var lastTick: CFAbsoluteTime = CFAbsoluteTimeGetCurrent()
    private var pump = PumpClock()
    private let audio = PcmPlayer()
    private var notBeforeMs: Int64 = 0
    /// Settings are read once and kept here; the pump used to parse the file
    /// 125 times a second, which is most of the idle heat.
    private var cachedPrefs: Prefs?
    /// Reused for every pump so the tick allocates nothing.
    private var pcmBuffer = [Int16](repeating: 0, count: 44_100 / 5)
    /// True while any shell key is held; keeps the tick short for response.
    private var holdingKeys = false
    private var lastLanSample: CFAbsoluteTime = 0
    private var lastPottySeconds: Int32 = -1
    private let pottyNotifications = PottyAlerts()
    private var pottyEpisode: UInt64 = 0
    var onFrame: (([UInt32], Bool) -> Void)?
    var onNotice: ((String) -> Void)?
    var onStopped: ((String) -> Void)?
    /// Replay progress while catching up: (seconds done, seconds total).
    var onCatchup: ((Double, Double) -> Void)?
    private var wasCatchingUp = false
    var onRoute: ((Int32) -> Void)?
    var onCall: ((Bool) -> Void)?
    var onOpenPotty: (() -> Void)?
    var onPottyNotice: ((String?) -> Void)?
    var onPrefs: ((Prefs) -> Void)?

    init() {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        root = base.appendingPathComponent("PocketMeets", isDirectory: true)
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var mutable = root
        try? mutable.setResourceValues(values)
        queue.sync {
            _ = root.path.withCString { pm_container_ensure($0) }
        }
        pottyNotifications.isCurrent = { [weak self] episode in
            guard let self else { return false }
            return self.queue.sync { self.lastPottySeconds > 0 && self.pottyEpisode == episode }
        }
        pottyNotifications.canPlaySound = { [weak self] episode in
            guard let self else { return false }
            return self.queue.sync {
                self.lastPottySeconds > 0 && self.pottyEpisode == episode && !self.reloadUnlocked().muted
            }
        }
        pottyNotifications.onInApp = { [weak self] text in self?.onPottyNotice?(text) }
        pottyNotifications.onOpen = { [weak self] in
            DispatchQueue.main.async { self?.onOpenPotty?() }
        }
        UNUserNotificationCenter.current().delegate = pottyNotifications
    }

    var settingsURL: URL { root.appendingPathComponent("settings") }
    var planURL: URL { root.appendingPathComponent("care_plan") }
    var firmwareURL: URL { root.appendingPathComponent("firmware-v1.bin") }
    var savesURL: URL { root.appendingPathComponent("Saves", isDirectory: true) }
    var cacheURL: URL { root.appendingPathComponent("cache", isDirectory: true) }

    func text(_ key: String, lang: Int32) -> String {
        queue.sync {
            key.withCString { keyPointer in
                CString.take(pm_text(lang, keyPointer))
            }
        }
    }

    func reloadPrefs() -> Prefs {
        queue.sync {
            let json = settingsURL.path.withCString { CString.take(pm_settings_json($0)) }
            let prefs = Prefs.load(json)
            cachedPrefs = prefs
            return prefs
        }
    }

    func write(_ prefs: Prefs) -> String {
        if prefs.muted {
            DispatchQueue.main.async { [weak self] in self?.pottyNotifications.mute() }
        }
        return queue.sync {
            guard let data = try? JSONEncoder().encode(prefs), let text = String(data: data, encoding: .utf8) else {
                return "invalid JSON"
            }
            if let session { pm_set_fast_keys(session, prefs.fast_keys ? 1 : 0) }
            cachedPrefs = prefs
            return settingsURL.path.withCString { path in
                text.withCString { body in
                    let reply = CString.take(pm_settings_write(path, body))
                    let parsed = CString.ok(reply)
                    return parsed.0 ? "" : parsed.1
                }
            }
        }
    }

    func hasCurrent() -> Bool {
        queue.sync { root.path.withCString { pm_has_current($0) != 0 } }
    }

    func library() -> [String] {
        queue.sync {
            let json = root.path.withCString { CString.take(pm_library_json($0)) }
            guard let data = json.data(using: .utf8),
                  let names = try? JSONDecoder().decode([String].self, from: data) else {
                return []
            }
            return names
        }
    }

    func importRom(from copiedPath: String) -> (ok: Bool, error: String, current: String?) {
        queue.sync {
            let reply = root.path.withCString { rootPointer in
                copiedPath.withCString { path in
                    CString.take(pm_import_rom(rootPointer, path))
                }
            }
            let parsed = CString.ok(reply)
            var current: String?
            if let result = parsed.2 as? [String: Any] {
                current = result["current"] as? String
            }
            return (parsed.0, localizedUnlocked(parsed.1), current)
        }
    }

    func openIfNeeded(paused: Bool) -> String {
        queue.sync {
            if session != nil { return "" }
            guard hasCurrentUnlocked() else { return "" }
            let opened = firmwareURL.path.withCString { rom in
                savesURL.path.withCString { saves in
                    pm_open_care(rom, saves, paused ? 1 : 0)
                }
            }
            guard let opened else {
                return localizedUnlocked(CString.take(pm_last_error()))
            }
            session = opened
            pm_set_fast_keys(opened, self.reloadUnlocked().fast_keys ? 1 : 0)
            root.path.withCString { pm_set_root(opened, $0) }
            pm_link_enable(opened, 1)
            if pm_link_recovered(opened) != 0 {
                let notice = localizedUnlocked(CString.take(pm_notice(opened)))
                DispatchQueue.main.async { self.onNotice?(notice) }
            }
            armTimer(ms: 8)
            return ""
        }
    }

    func closeSession() {
        queue.sync {
            timer?.cancel()
            timer = nil
            if let session {
                _ = pm_save(session)
                pm_close(session)
            }
            session = nil
            lastPottySeconds = -1
            clearPottyNotification()
            audio.stop()
        }
    }

    func setShelfOpen(_ open: Bool) {
        queue.sync { if let session { pm_set_shelf_open(session, open ? 1 : 0) } }
    }

    func key(_ index: Int, down: Bool) {
        let trace = LanDiagnostics.shared.active
        let requested = trace ? ProcessInfo.processInfo.systemUptime : 0
        queue.sync {
            guard let session else { return }
            let result = pm_key(session, Int32(index), down ? 1 : 0)
            if trace {
                LanDiagnostics.shared.record("key", ["index": index, "down": down, "result": result,
                    "sim_s": pm_seconds(session), "requested_uptime_s": requested,
                    "queue_wait_ms": (ProcessInfo.processInfo.systemUptime - requested) * 1000])
            }
            holdingKeys = down
            if down { armTimer(ms: 1) }
        }
    }

    func releaseKeys() {
        queue.sync {
            if let session {
                pm_release_keys(session)
                LanDiagnostics.shared.record("release_keys", ["sim_s": pm_seconds(session)])
            }
            holdingKeys = false
        }
    }

    func saveNow() -> String {
        queue.sync {
            guard let session else { return textUnlocked("import_rom_first", lang: 0) }
            if pm_save(session) == 0 {
                return textUnlocked("progress_saved", lang: currentLang())
            }
            return textUnlocked("save_failed_prefix", lang: currentLang()) + localizedUnlocked(CString.take(pm_error(session)))
        }
    }

    func exportSave(to path: String) -> (ok: Bool, message: String) {
        queue.sync {
            guard let session else { return (false, textUnlocked("import_rom_first", lang: currentLang())) }
            let reply = path.withCString { CString.take(pm_export_save(session, $0)) }
            let parsed = CString.ok(reply)
            return (parsed.0, parsed.0 ? textUnlocked("save_exported", lang: currentLang()) : localizedUnlocked(parsed.1))
        }
    }

    func importSave(from path: String) -> String {
        queue.sync {
            guard let session else { return textUnlocked("import_rom_first", lang: currentLang()) }
            let reply = path.withCString { CString.take(pm_import_save(session, $0)) }
            let parsed = CString.ok(reply)
            return parsed.0 ? textUnlocked("save_imported", lang: currentLang()) : localizedUnlocked(parsed.1)
        }
    }

    func deleteCurrent() -> String {
        queue.sync {
            guard let session else { return textUnlocked("import_rom_first", lang: currentLang()) }
            let code = pm_delete_current_save(session)
            if code == 0 { return textUnlocked("delete_ok", lang: currentLang()) }
            return textUnlocked("delete_failed_prefix", lang: currentLang()) + localizedUnlocked(CString.take(pm_error(session)))
        }
    }

    func switchRom(_ path: String) -> String {
        queue.sync {
            guard let session else { return textUnlocked("import_rom_first", lang: currentLang()) }
            let reply = path.withCString { CString.take(pm_switch_image(session, $0)) }
            let parsed = CString.ok(reply)
            if parsed.0 {
                if let result = parsed.2 as? [String: Any], let message = result["message"] as? String {
                    return localizedUnlocked(message)
                }
                return textUnlocked("loaded_prefix", lang: currentLang())
            }
            return localizedUnlocked(parsed.1)
        }
    }

    func setPaused(_ paused: Bool) {
        queue.sync {
            _ = root.path.withCString { pm_set_care_paused($0, paused ? 1 : 0) }
            cachedPrefs = nil
            if let session {
                pm_care_pause(session, paused ? 1 : 0, Date().timeIntervalSince1970)
            }
            if paused {
                lastPottySeconds = -1
                clearPottyNotification()
                UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ["2"])
                UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: ["2"])
            }
        }
    }

    /// Settings and the forecast are refreshed behind any catch-up replay, then
    /// the caller runs on the main thread.
    func foregroundTasks(_ done: @escaping () -> Void) {
        queue.async {
            self.cachedPrefs = nil
            DispatchQueue.main.async { done() }
        }
    }

    /// Re-arm audio after returning to the foreground.
    func resumeAudio() {
        audio.resume()
    }

    /// Debug helper: write the 1 MiB SRAM to a file.
    func dumpSRAM(to path: String) {
        queue.sync {
            guard let session else { return }
            _ = path.withCString { pm_dump_sram(session, $0) }
        }
    }

    func dumpWindow(start: UInt32, length: UInt32, to path: String) {
        queue.sync {
            guard let session else { return }
            _ = path.withCString { pm_dump_window(session, start, length, $0) }
        }
    }

    func usesSweetsFont() -> Bool {
        queue.sync {
            guard let session else { return false }
            return pm_current_is_sweets(session) != 0
        }
    }

    /// Read on the existing serial queue; never advances or changes firmware.
    func careInfo() -> CareInfoSnapshot? {
        queue.sync { CareInfoSnapshot.decode(CString.take(pm_care_info(session))) }
    }

    /// Player name stored in the game state.
    func playerName() -> String {
        queue.sync {
            guard let session else { return "" }
            return CString.take(pm_player_name(session))
        }
    }

    /// Current on-screen message decoded from the SRAM text buffer.
    func messageText() -> String {
        queue.sync {
            guard let session else { return "" }
            return CString.take(pm_message_text(session))
        }
    }

    func shelfList(tab: String, language: String) -> (ok: Bool, error: String, items: [ShelfItem]) {
        queue.sync {
            let reply = root.path.withCString { rootPointer in
                tab.withCString { tabPointer in
                    language.withCString { langPointer in
                        CString.take(pm_shelf_list(rootPointer, tabPointer, langPointer))
                    }
                }
            }
            let parsed = CString.ok(reply)
            var items: [ShelfItem] = []
            if let rows = parsed.2 as? [[String: Any]] {
                items = rows.map {
                    ShelfItem(
                        name: $0["name"] as? String ?? "",
                        kind: $0["kind"] as? String ?? tab,
                        path: $0["path"] as? String ?? "",
                        giftId: ($0["id"] as? NSNumber)?.intValue ?? 0,
                        price: ($0["price"] as? NSNumber)?.intValue ?? 0
                    )
                }
            }
            return (parsed.0, localizedUnlocked(parsed.1), items)
        }
    }

    func shelfInstall(_ path: String) -> String {
        queue.sync {
            let reply = root.path.withCString { rootPointer in
                path.withCString { zip in
                    settingsURL.path.withCString { settings in
                        CString.take(pm_shelf_install(rootPointer, zip, settings))
                    }
                }
            }
            let parsed = CString.ok(reply)
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func shelfSend(_ path: String) -> String {
        queue.sync {
            guard let session else { return localizedUnlocked("null session or request") }
            let reply = path.withCString { CString.take(pm_shelf_send(session, $0)) }
            let parsed = CString.ok(reply)
            if parsed.0, let result = parsed.2 as? [String: Any], let message = result["message"] as? String {
                return localizedUnlocked(message)
            }
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func shelfBalance(add: Bool, amount: Int) -> String {
        queue.sync {
            guard let session else { return localizedUnlocked("null session or request") }
            let reply = CString.take(pm_shelf_balance(session, add ? 1 : 0, Int32(amount)))
            let parsed = CString.ok(reply)
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func shelfUndo() -> String {
        queue.sync {
            guard let session else { return localizedUnlocked("null session or request") }
            let parsed = CString.ok(CString.take(pm_shelf_undo(session)))
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func shelfMarry(_ json: String) -> String {
        queue.sync {
            guard let session else { return localizedUnlocked("null session or request") }
            let parsed = CString.ok(json.withCString { CString.take(pm_shelf_marry(session, $0)) })
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func customImport(_ path: String, kind: Int32) -> String {
        queue.sync {
            let reply = root.path.withCString { rootPointer in
                path.withCString { file in
                    CString.take(pm_custom_import(rootPointer, file, kind))
                }
            }
            let parsed = CString.ok(reply)
            if parsed.0, let result = parsed.2 as? [String: Any], let message = result["message"] as? String {
                return localizedUnlocked(message)
            }
            return parsed.0 ? "" : localizedUnlocked(parsed.1)
        }
    }

    func linkState() -> Int32 {
        queue.sync { session.map { pm_link_state($0) } ?? 0 }
    }

    func linkCancel() {
        queue.sync { if let session { pm_link_cancel(session) } }
    }

    func linkConnect() {
        queue.sync { if let session { pm_link_connect(session) } }
    }

    func linkWrite(_ bytes: [UInt8]) -> Bool {
        queue.sync {
            guard let session, !bytes.isEmpty else { return false }
            return bytes.withUnsafeBufferPointer { raw in
                guard let base = raw.baseAddress else { return false }
                return pm_link_write(session, base, Int32(bytes.count)) == 0
            }
        }
    }

    func linkRead() -> [UInt8] {
        queue.sync {
            guard let session else { return [] }
            var buffer = [UInt8](repeating: 0, count: 20)
            let n = buffer.withUnsafeMutableBufferPointer { raw -> Int32 in
                guard let base = raw.baseAddress else { return 0 }
                return pm_link_read(session, base, 20)
            }
            if n <= 0 { return [] }
            return Array(buffer.prefix(Int(n)))
        }
    }

    /// Feed bytes received from the app-side infrared transport.
    func irWrite(_ bytes: [UInt8]) {
        let trace = LanDiagnostics.shared.active
        let requested = trace ? ProcessInfo.processInfo.systemUptime : 0
        queue.sync {
            guard let session, !bytes.isEmpty else { return }
            let wait = trace ? (ProcessInfo.processInfo.systemUptime - requested) * 1000 : 0
            var result: Int32 = -1
            bytes.withUnsafeBufferPointer { raw in
                if let base = raw.baseAddress {
                    result = pm_ir_write(session, base, Int32(bytes.count))
                }
            }
            if trace { traceIr("uart_rx", bytes: bytes, session: session, waitMs: wait, result: result) }
        }
    }

    /// Drain firmware infrared output for the app-side transport.
    func irRead() -> [UInt8] {
        let trace = LanDiagnostics.shared.active
        let requested = trace ? ProcessInfo.processInfo.systemUptime : 0
        return queue.sync {
            guard let session else { return [] }
            let wait = trace ? (ProcessInfo.processInfo.systemUptime - requested) * 1000 : 0
            var buffer = [UInt8](repeating: 0, count: 512)
            let n = buffer.withUnsafeMutableBufferPointer { raw -> Int32 in
                guard let base = raw.baseAddress else { return 0 }
                return pm_ir_read(session, base, 512)
            }
            if n <= 0 { return [] }
            let bytes = Array(buffer.prefix(Int(n)))
            if trace { traceIr("uart_tx", bytes: bytes, session: session, waitMs: wait, result: n) }
            return bytes
        }
    }

    private func traceIr(_ event: String, bytes: [UInt8], session: UnsafeMutablePointer<PmSession>, waitMs: Double, result: Int32) {
        guard LanDiagnostics.shared.active else { return }
        // Keep normal UART frames intact; cap malformed/oversized transport frames.
        LanDiagnostics.shared.record(event, ["sim_s": pm_seconds(session), "count": bytes.count,
            "hex": bytes.prefix(8192).map { String(format: "%02x", $0) }.joined(),
            "truncated": bytes.count > 8192, "queue_wait_ms": waitMs, "result": result])
    }

    func traceLanState(_ event: String) {
        guard LanDiagnostics.shared.active else { return }
        queue.sync { traceLanStateUnlocked(event) }
    }

    private func traceLanStateUnlocked(_ event: String, fields: [String: Any] = [:]) {
        guard LanDiagnostics.shared.active else { return }
        var fields = fields
        if let session {
            let text = CString.take(pm_ir_diagnostics(session))
            if let data = text.data(using: .utf8),
               let state = try? JSONSerialization.jsonObject(with: data) {
                fields["firmware"] = state
            }
            fields["screen"] = pullFrame().withUnsafeBufferPointer { pixels in
                CString.take(pm_menu_text(pixels.baseAddress, pixels.count))
            }
        } else {
            fields["session_missing"] = true
        }
        LanDiagnostics.shared.record(event, fields)
    }

    func frame() -> [UInt32] {
        queue.sync { pullFrame() }
    }

    func pngName(millis: UInt32, hex: String) -> String {
        let parts = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute, .second], from: Date())
        return queue.sync {
            hex.withCString {
                CString.take(pm_png_name(
                    Int32(parts.year ?? 1970),
                    UInt32(parts.month ?? 1),
                    UInt32(parts.day ?? 1),
                    UInt32(parts.hour ?? 0),
                    UInt32(parts.minute ?? 0),
                    UInt32(parts.second ?? 0),
                    millis,
                    $0
                ))
            }
        }
    }

    func exportName() -> String {
        let parts = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute, .second], from: Date())
        return queue.sync {
            CString.take(pm_export_name(
                Int32(parts.year ?? 1970),
                UInt32(parts.month ?? 1),
                UInt32(parts.day ?? 1),
                UInt32(parts.hour ?? 0),
                UInt32(parts.minute ?? 0),
                UInt32(parts.second ?? 0)
            ))
        }
    }

    func mixingVersion() -> String { queue.sync { CString.take(pm_mixing_version()) } }
    func releaseText() -> String { queue.sync { CString.take(pm_release()) } }
    func buildNumber() -> Int32 { queue.sync { pm_build() } }
    func mixingURL() -> String { queue.sync { CString.take(pm_mixing_url()) } }
    func supportURL() -> String { queue.sync { CString.take(pm_support_url()) } }
    func loginForm() -> String { queue.sync { CString.take(pm_login_form()) } }
    func fakeDeviceJSON() -> String { queue.sync { CString.take(pm_fake_device_json()) } }
    func notifyHex(_ bytes: [UInt8]) -> String {
        queue.sync {
            bytes.withUnsafeBufferPointer { raw in
                CString.take(pm_notify_hex(raw.baseAddress, Int32(bytes.count)))
            }
        }
    }
    func alarmSentence(exact: Bool) -> String {
        queue.sync { localizedUnlocked(CString.take(pm_alarm_exact(exact ? 1 : 0))) }
    }

    func gattAccept(owner: Bool, subscribed: Bool, prepared: Bool, offset: Int, length: Int, queued: Int) -> Bool {
        queue.sync {
            pm_gatt_rx(owner ? 1 : 0, subscribed ? 1 : 0, prepared ? 1 : 0, Int32(offset), Int32(length), Int32(queued)) == 0
        }
    }

    func zone(x: Float, y: Float, w: Float, h: Float, screenshots: Bool) -> Int32 {
        queue.sync { pm_zone(x, y, w, h, screenshots ? 1 : 0) }
    }

    func dotStrength(_ points: Float) -> Float { pm_dot_strength(points) }

    func guideVisible(appearance: Int32, ready: Bool, focused: Bool, shelf: Bool, guide: Int32, shots: Bool, seen: Bool) -> Bool {
        pm_guide_visible(appearance, ready ? 1 : 0, focused ? 1 : 0, shelf ? 1 : 0, guide, shots ? 1 : 0, seen ? 1 : 0) != 0
    }

    func exitAllowed(ready: Bool, shelf: Bool, guide: Bool) -> Bool {
        queue.sync { pm_exit_allowed(ready ? 1 : 0, shelf ? 1 : 0, guide ? 1 : 0) != 0 }
    }

    func displayHint(visible: Bool, active: Bool, paused: Bool, follow: Bool, interactive: Bool, present: Bool) -> Bool {
        queue.sync {
            pm_display_hint(visible ? 1 : 0, active ? 1 : 0, paused ? 1 : 0, follow ? 1 : 0, interactive ? 1 : 0, present ? 1 : 0) != 0
        }
    }

    private func armTimer(ms: UInt64) {
        pump.intervalMs = ms
        let delay = DispatchTime.now() + .milliseconds(Int(ms))
        let period = DispatchTimeInterval.milliseconds(Int(max(ms, 1)))
        if let timer {
            timer.schedule(deadline: delay, repeating: period)
            return
        }
        let source = DispatchSource.makeTimerSource(queue: queue)
        source.schedule(deadline: delay, repeating: period)
        source.setEventHandler { [weak self] in self?.tick() }
        source.resume()
        timer = source
    }

    private func tick() {
        guard let session else { return }
        let now = CFAbsoluteTimeGetCurrent()
        let dt = now - lastTick
        lastTick = now
        let trace = LanDiagnostics.shared.active
        let simBefore = trace ? pm_seconds(session) : 0
        let pumpStart = trace ? ProcessInfo.processInfo.systemUptime : 0
        let prefs = reloadUnlocked()
        var input = PmPumpIn()
        input.now_unix = Date().timeIntervalSince1970
        input.dt = dt
        input.visible = pump.visible ? 1 : 0
        input.monotonic_ms = pump.monotonic
        input.last_frame_ns = pump.lastFrameNs
        input.last_save_ms = pump.lastSaveMs
        input.last_catchup_save_ms = pump.lastCatchupSaveMs
        input.backlog = pump.backlog
        input.phone_interactive = pump.interactive ? 1 : 0
        input.follow_system_sleep = prefs.follow_system_sleep ? 1 : 0
        input.muted = prefs.muted ? 1 : 0
        input.frame_ns = pump.frameNs
        input.present = pump.pipFrames ? 1 : 0
        var output = PmPumpOut()
        let cap = Int32(pcmBuffer.count)
        pcmBuffer.withUnsafeMutableBufferPointer { raw in
            pm_pump(session, &input, &output, raw.baseAddress, cap)
        }
        if trace && (now - lastLanSample >= 1 || dt > 0.1 || output.step_code != 0 || output.stopped != 0) {
            lastLanSample = now
            traceLanStateUnlocked("pump", fields: ["dt_ms": dt * 1000,
                "pump_ms": (ProcessInfo.processInfo.systemUptime - pumpStart) * 1000,
                "sim_before_s": simBefore, "sim_after_s": pm_seconds(session),
                "visible": pump.visible, "interactive": pump.interactive,
                "step_code": output.step_code, "route": output.route,
                "backlog_s": output.backlog, "catching_up": output.catching_up != 0,
                "timer_ms": pump.intervalMs])
        }
        if output.catching_up != 0 {
            let total = output.catchup_total
            let done = max(0, total - output.remain)
            DispatchQueue.main.async { self.onCatchup?(done, total) }
            wasCatchingUp = true
        } else if wasCatchingUp {
            wasCatchingUp = false
            DispatchQueue.main.async { self.onCatchup?(0, 0) }
        }
        pump.backlog = output.backlog
        pump.lastFrameNs = output.last_frame_ns
        pump.lastSaveMs = output.last_save_ms
        pump.lastCatchupSaveMs = output.last_catchup_save_ms
        if output.play_audio != 0, output.pcm_count > 0 {
            let count = min(Int(output.pcm_count), pcmBuffer.count)
            audio.write(pcmBuffer, count: count, volume: output.volume)
        }
        // Calls use firmware PCM; the one-shot potty chime is handled separately.
        if output.take_call != 0 {
            handleCall(prefs: prefs, visible: pump.visible)
        }
        let pottySeconds: Int32 = output.catching_up == 0 && output.stopped == 0 && !prefs.care_paused
            ? pm_potty_seconds(session) : -1
        if pottySeconds != lastPottySeconds {
            let started = pottySeconds > 0 && lastPottySeconds <= 0
            lastPottySeconds = pottySeconds
            if started {
                pottyEpisode &+= 1
                let episode = pottyEpisode
                let title = textUnlocked("potty_title", lang: prefs.langCode)
                let body = textUnlocked("potty_urge", lang: prefs.langCode)
                DispatchQueue.main.async { [weak self] in
                    self?.pottyNotifications.begin(episode: episode, title: title, body: body)
                }
            } else if pottySeconds <= 0 {
                clearPottyNotification()
            }
        }
        if output.drew_frame != 0 {
            let pixels = pullFrame()
            let asleep = pm_asleep(session) != 0
            DispatchQueue.main.async { self.onFrame?(pixels, asleep) }
        }
        if output.step_code == 1 {
            let notice = localizedUnlocked(CString.take(pm_notice(session)))
            DispatchQueue.main.async { self.onNotice?(notice) }
        } else if output.step_code == 2 {
            let route = output.route
            DispatchQueue.main.async { self.onRoute?(route) }
        }
        if output.stopped != 0 {
            let detail = localizedUnlocked(CString.take(pm_error(session)))
            let error = textUnlocked("emulator_stopped", lang: prefs.langCode) + " " + detail
            audio.stop()
            DispatchQueue.main.async { self.onStopped?(error) }
        }
        var next = output.next_ms == 0 ? 100 : output.next_ms
        // A still screen only needs checking ~30 times a second; key presses
        // wake the timer immediately, so this costs no responsiveness.
        if pump.visible, output.drew_frame == 0, output.catching_up == 0, !holdingKeys {
            next = max(next, 33)
        }
        if next != pump.intervalMs {
            armTimer(ms: next)
        }
        pump.monotonic &+= next
        pump.frameNs &+= next * 1_000_000
    }

    func setVisible(_ visible: Bool, interactive: Bool) {
        queue.async {
            self.pump.visible = visible
            self.pump.interactive = interactive
        }
    }

    /// Picture in Picture still needs LCD frames after the toy page is hidden.
    /// Foreground audio stays tied to `setVisible`.
    func setPipFrames(_ on: Bool) {
        queue.async { self.pump.pipFrames = on }
    }

    private func handleCall(prefs: Prefs, visible: Bool) {
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        let json = CString.take(pm_call_decision(
            1, visible ? 1 : 0, prefs.muted ? 1 : 0, now, prefs.last_call_notification, notBeforeMs
        ))
        let object = CString.object(json)
        let banner = object["banner"] as? Bool ?? false
        let channel = object["channel"] as? String ?? ""
        if let next = CString.int64(object["not_before_ms"]) { notBeforeMs = next }
        let last = CString.int64(object["last_ms"]) ?? prefs.last_call_notification
        if last != prefs.last_call_notification {
            var updated = prefs
            updated.last_call_notification = last
            _ = writeUnlocked(updated)
        }
        DispatchQueue.main.async { self.onCall?(true) }
        guard banner else { return }
        let lang = prefs.langCode
        let content = UNMutableNotificationContent()
        content.title = textUnlocked("call_title", lang: lang)
        content.body = textUnlocked("call_body", lang: lang)
        content.threadIdentifier = channel
        content.sound = channel == "pet_calls_silent" ? nil : .default
        let request = UNNotificationRequest(identifier: "2", content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request)
    }

    private func clearPottyNotification() {
        pottyEpisode &+= 1
        DispatchQueue.main.async { [weak self] in self?.pottyNotifications.clear() }
    }

    func scheduleForecastAlarm(eventMs: Int64, missedGap: Bool, visible: Bool, muted: Bool, lang: Int32) {
        queue.async {
            let now = Int64(Date().timeIntervalSince1970 * 1000)
            guard pm_alarm_post(eventMs, now, visible ? 1 : 0, missedGap ? 1 : 0) != 0 else { return }
            let content = UNMutableNotificationContent()
            content.title = self.textUnlocked("call_title", lang: lang)
            content.body = self.textUnlocked("call_body", lang: lang)
            content.threadIdentifier = muted ? "pet_calls_silent" : "pet_calls"
            content.sound = muted ? nil : .default
            let interval = max(1, Double(eventMs - now) / 1000)
            let trigger = UNTimeIntervalNotificationTrigger(timeInterval: interval, repeats: false)
            let request = UNNotificationRequest(identifier: "2", content: content, trigger: trigger)
            UNUserNotificationCenter.current().add(request)
        }
    }

    /// Offline forecast. Budget 200, horizon handling stays in the core's decide().
    func planForecast(visible: Bool, missedGap: Bool) {
        queue.async {
            guard let session = self.session else { return }
            let prefs = self.reloadUnlocked()
            if prefs.care_paused { return }
            let now = Date().timeIntervalSince1970
            let seed = pm_forecast_seed(now, prefs.last_call_notification)
            guard let forecast = pm_forecast_create(session, seed) else { return }
            let started = Date()
            let code = pm_forecast_run(forecast, 200)
            let elapsed = Date().timeIntervalSince(started)
            let event = pm_forecast_event(forecast)
            let at = pm_forecast_at(forecast)
            let epoch = Int64(now)
            let file = self.root.appendingPathComponent("Care").appendingPathComponent("\(epoch)-forecast.t18s")
            _ = file.path.withCString { pm_forecast_save(forecast, $0) }
            pm_forecast_close(forecast)
            let care = self.root.appendingPathComponent("Care").path
            let keep = "[\"\(epoch)-forecast.t18s\"]"
            _ = care.withCString { dir in keep.withCString { pm_forecast_retain(dir, epoch, $0) } }
            let decidedCode: Int32 = at >= now + 86400 ? 2 : code
            let json = CString.take(pm_forecast_decide(decidedCode, event, visible ? 1 : 0))
            let object = CString.object(json)
            if elapsed > 4 { /* wall budget is the worker's limit; the core run already returned */ }
            if let error = object["error"] as? String, !error.isEmpty {
                let text = self.localizedUnlocked(error)
                DispatchQueue.main.async { self.onNotice?(text) }
                return
            }
            if let eventMs = CString.int64(object["event_ms"]), eventMs > 0 {
                self.scheduleForecastAlarm(eventMs: eventMs, missedGap: missedGap, visible: visible, muted: prefs.muted, lang: prefs.langCode)
            }
        }
    }

    private func pullFrame() -> [UInt32] {
        guard let session else { return Array(repeating: 0xFF29_2B2A, count: 16384) }
        var pixels = [UInt32](repeating: 0, count: 16384)
        pm_frame(session, &pixels)
        return pixels
    }

    private func hasCurrentUnlocked() -> Bool {
        root.path.withCString { pm_has_current($0) != 0 }
    }

    private func reloadUnlocked() -> Prefs {
        if let cachedPrefs { return cachedPrefs }
        let prefs = Prefs.load(settingsURL.path.withCString { CString.take(pm_settings_json($0)) })
        cachedPrefs = prefs
        return prefs
    }

    private func currentLang() -> Int32 { reloadUnlocked().langCode }

    /// Shell messages are written in English deep in the crate. Translate them
    /// once at the boundary using the language stored in settings.
    private func localizedUnlocked(_ text: String) -> String {
        guard !text.isEmpty else { return text }
        return text.withCString { CString.take(pm_localize($0, currentLang())) }
    }

    private func textUnlocked(_ key: String, lang: Int32) -> String {
        key.withCString { CString.take(pm_text(lang, $0)) }
    }

    private func writeUnlocked(_ prefs: Prefs) -> String {
        guard let data = try? JSONEncoder().encode(prefs), let text = String(data: data, encoding: .utf8) else {
            return "invalid JSON"
        }
        return settingsURL.path.withCString { path in
            text.withCString { body in
                let parsed = CString.ok(CString.take(pm_settings_write(path, body)))
                return parsed.0 ? "" : parsed.1
            }
        }
    }

}

private struct PumpClock {
    var visible = true
    var interactive = true
    var pipFrames = false
    var monotonic: UInt64 = 0
    var lastFrameNs: UInt64 = 0
    var lastSaveMs: UInt64 = 0
    var lastCatchupSaveMs: UInt64 = 0
    var backlog: Double = 0
    var frameNs: UInt64 = 0
    var intervalMs: UInt64 = 8
}

/// Firmware PCM only. Volume 0 is mute. The player is not a background audio session.
private final class PcmPlayer {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private let format: AVAudioFormat?
    private var observers: [NSObjectProtocol] = []

    init() {
        format = AVAudioFormat(commonFormat: .pcmFormatInt16, sampleRate: 44100, channels: 1, interleaved: true)
        guard let format else { return }
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        try? AVAudioSession.sharedInstance().setCategory(.ambient, options: [.mixWithOthers])
        try? AVAudioSession.sharedInstance().setActive(true)
        try? engine.start()
        player.play()
        listen()
    }

    deinit {
        observers.forEach(NotificationCenter.default.removeObserver)
    }

    /// Route changes, interruptions and a media-services reset all stop the
    /// engine, and a stopped engine swallows everything scheduled into it, so
    /// the toy went silent until the app was restarted. Watch for them and
    /// re-arm the graph.
    private func listen() {
        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: .AVAudioEngineConfigurationChange, object: engine, queue: .main) { [weak self] _ in
            self?.resume()
        })
        observers.append(center.addObserver(forName: AVAudioSession.interruptionNotification, object: nil, queue: .main) { [weak self] note in
            let raw = (note.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt) ?? 0
            if AVAudioSession.InterruptionType(rawValue: raw) == .ended { self?.resume() }
        })
        observers.append(center.addObserver(forName: AVAudioSession.mediaServicesWereResetNotification, object: nil, queue: .main) { [weak self] _ in
            self?.resume()
        })
    }

    func resume() {
        guard let format else { return }
        try? AVAudioSession.sharedInstance().setActive(true)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        if !engine.isRunning { try? engine.start() }
        if !player.isPlaying { player.play() }
    }

    func write(_ samples: [Int16], count: Int, volume: Float) {
        guard let format, count > 0 else { return }
        if !engine.isRunning || !player.isPlaying { resume() }
        engine.mainMixerNode.outputVolume = volume
        guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(count)) else { return }
        buffer.frameLength = AVAudioFrameCount(count)
        samples.withUnsafeBufferPointer { source in
            guard let base = source.baseAddress, let dest = buffer.int16ChannelData?[0] else { return }
            dest.update(from: base, count: count)
        }
        player.scheduleBuffer(buffer, completionHandler: nil)
        if !player.isPlaying { player.play() }
    }

    func stop() {
        player.stop()
    }
}
