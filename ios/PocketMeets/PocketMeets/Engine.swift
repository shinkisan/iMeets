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
    var onFrame: (([UInt32], Bool) -> Void)?
    var onNotice: ((String) -> Void)?
    var onStopped: ((String) -> Void)?
    /// Replay progress while catching up: (seconds done, seconds total).
    var onCatchup: ((Double, Double) -> Void)?
    private var wasCatchingUp = false
    var onRoute: ((Int32) -> Void)?
    var onCall: ((Bool) -> Void)?
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
            return Prefs.load(json)
        }
    }

    func write(_ prefs: Prefs) -> String {
        queue.sync {
            guard let data = try? JSONEncoder().encode(prefs), let text = String(data: data, encoding: .utf8) else {
                return "invalid JSON"
            }
            if let session { pm_set_fast_keys(session, prefs.fast_keys ? 1 : 0) }
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
            audio.stop()
        }
    }

    func setShelfOpen(_ open: Bool) {
        queue.sync { if let session { pm_set_shelf_open(session, open ? 1 : 0) } }
    }

    func key(_ index: Int, down: Bool) {
        queue.sync {
            guard let session else { return }
            _ = pm_key(session, Int32(index), down ? 1 : 0)
        }
    }

    func releaseKeys() {
        queue.sync { if let session { pm_release_keys(session) } }
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
            if let session {
                pm_care_pause(session, paused ? 1 : 0, Date().timeIntervalSince1970)
            }
            if paused {
                UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ["2"])
                UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: ["2"])
            }
        }
    }

    func setEnhanced(_ on: Bool) -> Prefs {
        queue.sync {
            let json = root.path.withCString { CString.take(pm_set_background_care($0, on ? 1 : 0)) }
            return Prefs.load(json)
        }
    }

    func foregroundCare() -> String {
        queue.sync {
            let now = Int64(Date().timeIntervalSince1970 * 1000)
            _ = root.path.withCString { pm_care_watch($0, now) }
            return localizedUnlocked(root.path.withCString { CString.take(pm_care_foreground($0)) })
        }
    }

    func noteHeartbeat() {
        queue.sync {
            let now = Int64(Date().timeIntervalSince1970 * 1000)
            _ = root.path.withCString { pm_note_heartbeat($0, now) }
        }
    }

    func watchCare() -> Bool {
        queue.sync {
            let now = Int64(Date().timeIntervalSince1970 * 1000)
            return root.path.withCString { pm_care_watch($0, now) != 0 }
        }
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

    func displayHint(visible: Bool, active: Bool, catchupDone: Bool, paused: Bool, follow: Bool, interactive: Bool) -> Bool {
        queue.sync {
            pm_display_hint(visible ? 1 : 0, active ? 1 : 0, catchupDone ? 1 : 0, paused ? 1 : 0, follow ? 1 : 0, interactive ? 1 : 0) != 0
        }
    }

    private func armTimer(ms: UInt64) {
        timer?.cancel()
        let source = DispatchSource.makeTimerSource(queue: queue)
        source.schedule(deadline: .now() + .milliseconds(Int(ms)), repeating: .milliseconds(Int(max(ms, 1))))
        source.setEventHandler { [weak self] in self?.tick() }
        source.resume()
        timer = source
    }

    private func tick() {
        guard let session else { return }
        let now = CFAbsoluteTimeGetCurrent()
        let dt = now - lastTick
        lastTick = now
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
        var output = PmPumpOut()
        var pcm = [Int16](repeating: 0, count: 44_100 / 5)
        let cap = Int32(pcm.count)
        _ = pcm.withUnsafeMutableBufferPointer { raw in
            pm_pump(session, &input, &output, raw.baseAddress, cap)
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
            let count = min(Int(output.pcm_count), pcm.count)
            audio.write(Array(pcm.prefix(count)), volume: output.volume)
        }
        // TODO(call-jingle): SPEC §14.15. Only firmware PCM is played. No 22050 Hz UI jingle is mixed in.
        if output.take_call != 0 {
            handleCall(prefs: prefs, visible: pump.visible)
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
        let next = output.next_ms == 0 ? 100 : output.next_ms
        if next != pump.intervalMs {
            pump.intervalMs = next
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

    func postResidence(lang: Int32) {
        let content = UNMutableNotificationContent()
        content.title = text("residence_title", lang: lang)
        content.body = text("residence_body", lang: lang)
        content.threadIdentifier = "pet_running"
        content.sound = nil
        content.interruptionLevel = .passive
        content.categoryIdentifier = "residence"
        // Foreground-service id is 1. Call id is 2, failure id is 3, self-test id is 4.
        let request = UNNotificationRequest(identifier: "1", content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request)
    }

    func postCareFailure(lang: Int32) {
        let content = UNMutableNotificationContent()
        content.title = text("care_attention", lang: lang)
        content.body = text("care_fallback", lang: lang)
        content.threadIdentifier = "pet_running"
        let request = UNNotificationRequest(identifier: "3", content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request)
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
        Prefs.load(settingsURL.path.withCString { CString.take(pm_settings_json($0)) })
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

    init() {
        format = AVAudioFormat(commonFormat: .pcmFormatInt16, sampleRate: 44100, channels: 1, interleaved: true)
        guard let format else { return }
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        try? AVAudioSession.sharedInstance().setCategory(.ambient, options: [.mixWithOthers])
        try? AVAudioSession.sharedInstance().setActive(true)
        try? engine.start()
        player.play()
    }

    func write(_ samples: [Int16], volume: Float) {
        guard let format, !samples.isEmpty else { return }
        engine.mainMixerNode.outputVolume = volume
        guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(samples.count)) else { return }
        buffer.frameLength = AVAudioFrameCount(samples.count)
        samples.withUnsafeBufferPointer { source in
            guard let base = source.baseAddress, let dest = buffer.int16ChannelData?[0] else { return }
            dest.update(from: base, count: samples.count)
        }
        player.scheduleBuffer(buffer, completionHandler: nil)
        if !player.isPlaying { player.play() }
    }

    func stop() {
        player.stop()
    }
}
