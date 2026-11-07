import BackgroundTasks
import Combine
import SwiftUI
import UserNotifications

enum AppPage: Equatable {
    case setup
    case notes
    case toy
    case settings
    case library
    case help
    case about
    case web
    case mixing
    case shelf
}

@MainActor
final class AppModel: ObservableObject {
    let engine = Engine()
    let lan = LanLink()
    let pip = PiPController()
    @Published var page: AppPage = .setup
    @Published var prefs = Prefs()
    @Published var notice = "" {
        didSet {
            noticeTask?.cancel()
            guard !notice.isEmpty else { return }
            noticeTask = Task { @MainActor [weak self] in
                try? await Task.sleep(nanoseconds: 4_000_000_000)
                guard !Task.isCancelled else { return }
                self?.notice = ""
            }
        }
    }
    @Published var pixels: [UInt32] = Array(repeating: 0xFF29_2B2A, count: 16384)
    /// Non-nil while the launch replay is running: seconds done so far.
    @Published var catchupDone: Double?
    @Published var catchupTotal: Double = 0
    /// Cached "a ROM is loaded" flag; never call the engine from a view body.
    @Published private(set) var hasSession = false
    @Published var asleep = true
    @Published var library: [String] = []
    @Published var shelfOpen = false
    @Published var focused = true
    @Published var flashToken = 0
    @Published var endWeb = 0
    @Published var confirmMixingBack = false
    @Published var showNotes = false
    @Published var showHelp = false
    @Published var showLanguage = false
    @Published var showLan = false
    private var heart: Timer?
    private var noticeTask: Task<Void, Never>?
    private var lanCancel: AnyCancellable?
    private var lanAutoStarted = false
    private var cameFromBackground = false
    private var backgroundedAt: Date?
    private var catchupProvisional = false

    init() {
        lan.engine = engine
        pip.frameProvider = { [weak self] in
            guard let self else { return ([], true) }
            return (self.pixels, self.asleep)
        }
        if UserDefaults.standard.bool(forKey: "pm.pip") { pip.startFeeding() }
        lanCancel = lan.objectWillChange.sink { [weak self] _ in
            Task { @MainActor in self?.objectWillChange.send() }
        }
        engine.onFrame = { [weak self] pixels, asleep in
            guard let self else { return }
            self.pixels = pixels
            self.asleep = asleep
            self.pip.noteFrame()
        }
        engine.onNotice = { [weak self] text in
            self?.notice = text
        }
        engine.onStopped = { [weak self] text in
            self?.notice = text
        }
        engine.onCatchup = { [weak self] done, total in
            guard let self else { return }
            if total <= 0 {
                self.catchupDone = nil
                self.catchupTotal = 0
            } else {
                self.catchupDone = done
                self.catchupTotal = total
            }
            self.catchupProvisional = false
        }
        engine.onCall = { [weak self] _ in
            self?.notice = self?.t("call_body") ?? ""
        }
        engine.onRoute = { [weak self] route in
            guard let self else { return }
            // A hosted link is opened only while the toy is in front.
            // Otherwise cancel so the firmware is not left inside the link.
            guard self.focused, self.page == .toy else {
                self.engine.linkCancel()
                return
            }
            if route == 2 {
                self.page = .web
            } else if route == 3 {
                self.page = .mixing
            } else {
                self.notice = self.t("unsupported_link")
            }
        }
        registerBackground()
        reload()
        enter()
    }

    func t(_ key: String) -> String { engine.text(key, lang: prefs.langCode) }

    func reload() {
        prefs = engine.reloadPrefs()
        KeyHaptics.enabled = prefs.key_haptics
        library = engine.library()
    }

    private func startLanFromLaunchArgs() {
        guard !lanAutoStarted else { return }
        let args = ProcessInfo.processInfo.arguments
        if args.contains("-pmLanHost") {
            lanAutoStarted = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
                self?.lan.host()
                self?.showLan = true
            }
        } else if let i = args.firstIndex(of: "-pmLanJoin"), i + 1 < args.count {
            lanAutoStarted = true
            let payload = args[i + 1]
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
                self?.showLan = true
                self?.lan.join(payload: payload)
            }
        }
    }

    func enter() {
        reload()
        let ready = engine.hasCurrent()
        hasSession = ready
        if ready {
            let error = engine.openIfNeeded(paused: prefs.care_paused)
            if !error.isEmpty { notice = error }
            if page == .setup || page == .notes {
                page = .toy
            }
            engine.setVisible(page == .toy || page == .mixing, interactive: focused)
            startLanFromLaunchArgs()
            askNotificationsIfNeeded()
        } else if page != .shelf {
            page = .setup
            engine.setVisible(false, interactive: focused)
        }
        showNotes = false
    }

    func markNotesShown() {
        prefs.care_intro_version = Int(engine.buildNumber())
        _ = engine.write(prefs)
        reload()
        showNotes = false
    }

    /// About is reachable from the settings page and from the first-run screen.
    func closeAbout() {
        page = hasSession ? .settings : .setup
    }

    func closeHelp() {
        showHelp = false
        if page == .help {
            page = engine.hasCurrent() ? .toy : .setup
        }
    }

    func scene(_ phase: ScenePhase) {
        switch phase {
        case .active:
            pip.foregrounded()
            engine.setPipFrames(false)
            focused = true
            let away = backgroundedAt.map { Date().timeIntervalSince($0) } ?? 0
            backgroundedAt = nil
            if away > 30, !prefs.care_paused {
                catchupProvisional = true
                catchupDone = 0
                catchupTotal = away
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
                    guard let self, self.catchupProvisional else { return }
                    self.catchupProvisional = false
                    self.catchupDone = nil
                    self.catchupTotal = 0
                }
            }
            let missedGap = cameFromBackground
            cameFromBackground = false
            engine.setVisible(page == .toy || page == .mixing, interactive: true)
            heart?.invalidate()
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ["1"])
            engine.foregroundTasks { [weak self] sentence in
                guard let self else { return }
                self.reload()
                if !sentence.isEmpty { self.notice = sentence }
                self.engine.planForecast(visible: true, missedGap: missedGap)
            }
        case .background:
            focused = false
            cameFromBackground = true
            if backgroundedAt == nil { backgroundedAt = Date() }
            // Keep copying the LCD. A hidden tick otherwise stops publishing
            // frames, and the floating window stays on the first one.
            engine.setPipFrames(pipWanted)
            engine.setVisible(false, interactive: false)
            if pipWanted { pip.backgrounded() }
            _ = engine.saveNow()
            engine.noteHeartbeat()
            if prefs.background_care && !prefs.care_paused {
                engine.postResidence(lang: prefs.langCode)
                heart = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
                    Task { @MainActor in self?.engine.noteHeartbeat() }
                }
            }
            scheduleRefresh()
        case .inactive:
            focused = false
            if pipWanted { engine.setPipFrames(true) }
            engine.setVisible(false, interactive: false)
            // Give the PiP layer a live frame before UIKit moves the app to
            // the background; starting PiP from the background is too late.
            if pipWanted { pip.backgrounding() }
        default:
            focused = false
            engine.setPipFrames(false)
            engine.setVisible(false, interactive: false)
        }
    }

    /// The LCD floats in Picture in Picture only when the person enabled it
    /// and a ROM is actually running.
    private var pipWanted: Bool {
        UserDefaults.standard.bool(forKey: "pm.pip") && hasSession
    }

    func setPiP(_ on: Bool) {
        if on {
            pip.startFeeding()
        } else {
            pip.shutdown()
            engine.setPipFrames(false)
        }
    }

    func setLanguage(_ code: String) {
        prefs.language = code
        let error = engine.write(prefs)
        reload()
        if !error.isEmpty { notice = error }
    }

    func setEnhanced(_ on: Bool) {
        prefs = engine.setEnhanced(on)
        if on && !prefs.care_paused { engine.postResidence(lang: prefs.langCode) }
    }

    /// Matches `settings::finish_guide`: version 2, and the capture flag when screenshots are on.
    func finishGuide() {
        var next = prefs
        next.small_screen_guide_version = 2
        if next.screenshots { next.small_screen_capture_guide_seen = true }
        _ = engine.write(next)
        reload()
    }

    func captureScreen(bypassGuide: Bool) {
        let guide = engine.guideVisible(
            appearance: Int32(prefs.appearance),
            ready: engine.hasCurrent(),
            focused: focused,
            shelf: shelfOpen,
            guide: Int32(prefs.small_screen_guide_version),
            shots: prefs.screenshots,
            seen: prefs.small_screen_capture_guide_seen
        )
        guard engine.hasCurrent(), !shelfOpen, prefs.screenshots, bypassGuide || !guide else { return }
        let side = LcdMetrics.side(appearance: prefs.appearance, scalePercent: prefs.small_screen_scale)
        let strength = engine.dotStrength(Float(side / 128))
        let image = LcdBitmap.image(
            pixels: pixels,
            asleep: asleep,
            dotMatrix: prefs.dot_matrix && !asleep,
            strength: strength,
            points: CGSize(width: side, height: side)
        )
        flashToken += 1
        let token = flashToken
        Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: 800_000_000)
            guard let self, self.flashToken == token else { return }
            // Clear the token so re-entering the toy does not replay the sweep.
            self.flashToken = 0
        }
        let millis = UInt32((Date().timeIntervalSince1970 * 1000).truncatingRemainder(dividingBy: 1000))
        let hex = String(format: "%08x", UInt32.random(in: 0...UInt32.max))
        let name = engine.pngName(millis: millis, hex: String(hex.prefix(8)))
        let url = engine.cacheURL.appendingPathComponent(name)
        guard let data = image.pngData() else {
            notice = t("screenshot_failed_prefix") + "png"
            return
        }
        do {
            try data.write(to: url)
        } catch {
            notice = t("screenshot_failed_prefix") + error.localizedDescription
            return
        }
        let appearance = prefs.appearance
        let screenshots = prefs.screenshots
        PhotoStore.savePNG(url) { [weak self] result in
            Task { @MainActor in
                guard let self else { return }
                switch result {
                case .success:
                    self.notice = self.t("screenshot_ok")
                    if screenshots {
                        var next = self.prefs
                        next.small_screen_capture_guide_seen = true
                        if appearance == 2 && next.small_screen_guide_version < 2 {
                            next.small_screen_guide_version = 2
                        }
                        _ = self.engine.write(next)
                        self.reload()
                    }
                case .failure(.permission):
                    self.notice = self.t("photo_permission")
                case .failure:
                    self.notice = self.t("screenshot_failed_prefix") + "photo"
                }
            }
        }
    }

    /// System back closes the shelf before any other page.
    func edgeBack() {
        if page == .shelf || shelfOpen {
            shelfOpen = false
            engine.setShelfOpen(false)
            page = engine.hasCurrent() ? .toy : .setup
            return
        }
        switch page {
        case .mixing:
            let state = engine.linkState()
            if state == 0 || state == 5 || state > 5 {
                engine.linkCancel()
                page = .toy
            } else {
                confirmMixingBack = true
            }
        case .web:
            endWeb += 1
        case .library, .about:
            page = .settings
        case .help, .settings:
            page = engine.hasCurrent() ? .toy : .setup
        default:
            break
        }
    }

    func setPaused(_ paused: Bool) {
        engine.setPaused(paused)
        reload()
        if paused {
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ["2", "1"])
        }
    }

    func copyFile(from url: URL, name: String) -> String? {
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        let dest = engine.cacheURL.appendingPathComponent(name)
        try? FileManager.default.removeItem(at: dest)
        do {
            try FileManager.default.copyItem(at: url, to: dest)
            return dest.path
        } catch {
            return nil
        }
    }

    private func askNotificationsIfNeeded() {
        guard !prefs.notifications_requested else { return }
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in
            Task { @MainActor in
                self.prefs.notifications_requested = true
                _ = self.engine.write(self.prefs)
                self.reload()
            }
        }
        let pause = UNNotificationAction(identifier: "pause", title: t("pause"), options: [])
        let category = UNNotificationCategory(identifier: "residence", actions: [pause], intentIdentifiers: [], options: [])
        UNUserNotificationCenter.current().setNotificationCategories([category])
    }

    private func registerBackground() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: "app.pocketmeets.ios.refresh", using: nil) { task in
            task.expirationHandler = { task.setTaskCompleted(success: false) }
            Task { @MainActor in
                self.engine.noteHeartbeat()
                _ = self.engine.watchCare()
                self.scheduleRefresh()
                task.setTaskCompleted(success: true)
            }
        }
        BGTaskScheduler.shared.register(forTaskWithIdentifier: "app.pocketmeets.ios.care", using: nil) { task in
            task.expirationHandler = { task.setTaskCompleted(success: false) }
            Task { @MainActor in
                self.engine.noteHeartbeat()
                _ = self.engine.saveNow()
                self.scheduleRefresh()
                task.setTaskCompleted(success: true)
            }
        }
    }

    private func scheduleRefresh() {
        let request = BGAppRefreshTaskRequest(identifier: "app.pocketmeets.ios.refresh")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 30)
        try? BGTaskScheduler.shared.submit(request)
        let processing = BGProcessingTaskRequest(identifier: "app.pocketmeets.ios.care")
        processing.requiresNetworkConnectivity = false
        processing.earliestBeginDate = Date(timeIntervalSinceNow: 60)
        try? BGTaskScheduler.shared.submit(processing)
    }
}
