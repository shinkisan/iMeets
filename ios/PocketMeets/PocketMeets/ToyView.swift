import SwiftUI
import UIKit

struct ToyView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var exitStart: Date?
    @State private var downKeys: Set<Int> = []
    @State private var flashActive = false
    @AppStorage("pm.liveTranslation") private var liveTranslation = false
    @State private var ocrText = ""
    @State private var glyphSource = false
    @State private var fontMatchable: Bool?
    @AppStorage("pm.playerName") private var playerName = ""

    var body: some View {
        let guide = showingGuide
        ZStack {
            backgroundColor.ignoresSafeArea()
            if look == 2 {
                smallStage(guide: guide)
            } else if look == 1 {
                minimalStage
            } else {
                deviceStage
            }
            VStack {
                toolbar
                Spacer()
            }
            if liveTranslation {
                VStack {
                    Spacer(minLength: 0)
                    liveCaption
                        .padding(.horizontal, 20)
                        .padding(.bottom, model.notice.isEmpty ? 36 : 100)
                }
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .animation(.easeOut(duration: 0.2), value: liveTranslation)
            }
            if let done = model.catchupDone {
                catchupCard(done: done, total: model.catchupTotal)
            }
            if guide {
                VStack(spacing: 14) {
                    Text(model.t("guide_body"))
                        .font(.subheadline)
                        .multilineTextAlignment(.center)
                    Button(model.t("ok")) { model.finishGuide() }
                        .pmProminent()
                }
                .padding(22)
                .frame(maxWidth: 320)
                .pmGlassCard(28, fallback: .thickMaterial)
                .padding(24)
            }
        }
        .navigationBarHidden(true)
        .background(KeyCatcher { name, down in
            guard let index = keyIndex(name) else { return }
            if down {
                if downKeys.contains(index) { return }
                downKeys.insert(index)
                model.engine.key(index, down: true)
            } else {
                downKeys.remove(index)
                model.engine.key(index, down: false)
            }
        })
        .onChange(of: model.flashToken) { token in
            guard token > 0 else { flashActive = false; return }
            flashActive = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.9) {
                guard model.flashToken == token else { return }
                flashActive = false
            }
        }
        .task(id: liveTranslation) {
            guard liveTranslation else {
                ocrText = ""
                glyphSource = false
                return
            }
            let forceMenu = CommandLine.arguments.contains("--pm-force-menu")
            while !Task.isCancelled {
                if model.catchupDone != nil {
                    try? await Task.sleep(nanoseconds: 200_000_000)
                    continue
                }
                if model.asleep || !model.hasSession {
                    if !ocrText.isEmpty {
                        ocrText = ""
                        glyphSource = false
                    }
                } else {
                    let sweets = model.engine.usesSweetsFont() || forceMenu
                    if fontMatchable != sweets { fontMatchable = sweets }
                    let menu = MenuGlyphs.read(model.pixels, sweets: sweets)
                    let memory = model.engine.messageText()
                    // The game state is the authority for the player's name;
                    // the naming screen is only a fallback while it is unset.
                    let stored = model.engine.playerName()
                    if !stored.isEmpty, stored != playerName {
                        playerName = stored
                    }
                    let raw = menu ?? queuedPrompt(memory)
                    capturePlayerName(raw)
                    // The clock/date setup screen only shows a time value; it
                    // is not message text and needs no translation.
                    let text = isClockValue(raw) ? "" : raw
                    // Only publish actual changes so SwiftUI and the
                    // translation task stay idle on unchanged screens.
                    if text != ocrText || (menu != nil) != glyphSource {
                        ocrText = text
                        glyphSource = menu != nil
                    }
                }
                try? await Task.sleep(nanoseconds: 250_000_000)
            }
        }
        .task {
            if CommandLine.arguments.contains("--pm-fast") {
                Task { @MainActor in
                    var f = 0
                    while !Task.isCancelled {
                        try? await Task.sleep(nanoseconds: 50_000_000)
                        f += 1
                        let cache = model.engine.cacheURL
                        model.engine.dumpWindow(start: 0xF800D000, length: 0x2000, to: cache.appendingPathComponent("fastd2-\(f).bin").path)
                        model.engine.dumpWindow(start: 0xF801C000, length: 0x4000, to: cache.appendingPathComponent("fastc2-\(f).bin").path)
                    }
                }
            }
            if let spec = CommandLine.arguments.first(where: { $0.hasPrefix("--pm-keys=") }) {
                let keys = spec.dropFirst("--pm-keys=".count).split(separator: ",").compactMap { Int($0) }
                for (i, key) in keys.enumerated() {
                    try? await Task.sleep(nanoseconds: UInt64(1_500_000_000 + i * 900_000_000))
                    model.engine.key(key, down: true)
                    try? await Task.sleep(nanoseconds: 150_000_000)
                    model.engine.key(key, down: false)
                }
            }
            guard CommandLine.arguments.contains("--pm-capture") else { return }
            var n = 0
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                n += 1
                let image = LcdBitmap.image(
                    pixels: model.pixels,
                    asleep: false,
                    dotMatrix: false,
                    strength: 0,
                    points: CGSize(width: 512, height: 512)
                )
                if let data = image.pngData() {
                    try? data.write(to: model.engine.cacheURL.appendingPathComponent("lcd-\(n).png"))
                }
                model.engine.dumpSRAM(to: model.engine.cacheURL.appendingPathComponent("sram-\(n).bin").path)
            }
        }
        .onAppear { model.engine.setVisible(true, interactive: true) }
        .onDisappear {
            model.engine.releaseKeys()
            model.engine.setVisible(false, interactive: model.focused)
        }
    }

    @ViewBuilder
    private var backgroundColor: some View {
        switch look {
        case 1:
            LinearGradient(
                colors: [Color(rgb: 0xFFF8FA), Color(rgb: 0xFBE7EE)],
                startPoint: .top,
                endPoint: .bottom
            )
        case 2:
            Color.black
        case 3:
            NightSky()
        default:
            LinearGradient(
                colors: [Color(rgb: 0xFFFDFB), Color(rgb: 0xFAEDF1)],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }

    private var showingGuide: Bool {
        model.engine.guideVisible(
            appearance: Int32(look),
            ready: model.hasSession,
            focused: model.focused,
            shelf: model.shelfOpen,
            guide: Int32(model.prefs.small_screen_guide_version),
            shots: model.prefs.screenshots,
            seen: model.prefs.small_screen_capture_guide_seen
        )
    }

    @ViewBuilder
    private var liveCaption: some View {
        if #available(iOS 18.0, *) {
            LiveCaptionPanel(
                source: captionText,
                target: translationTarget,
                localized: callName ?
                    (model.prefs.language == "en" ? "(Calling your name)" : "（正在喊你名字）")
                    : (fontMatchable == true ? LocalizedText.lookup(captionText, strict: glyphSource) : nil),
                hideSource: callName,
                placeholder: captionPlaceholder
            )
        }
    }

    /// True while the pet's line is just the player's name, optionally with a
    /// decoration such as a heart that the matcher reads as punctuation.
    private var callName: Bool {
        let name = playerName.filter { !$0.isWhitespace }
        let text = ocrText.filter { !$0.isWhitespace }
        guard !name.isEmpty else { return false }
        return text == name
    }

    /// The name-entry screen reads as "あなたのなまえは？ <name>". Remember the
    /// typed name so a later bubble that repeats it can be labelled.
    private func capturePlayerName(_ text: String) {
        guard text.hasPrefix("あなたのなまえは") else { return }
        let rest = text.dropFirst("あなたのなまえは".count)
        // Keep spaces inside the name: the game allows them, and the matcher
        // now reports an empty name box as one.
        let name = rest.trimmingCharacters(in: CharacterSet(charactersIn: "？?.… 　"))
        if !name.isEmpty && name.count <= 8 {
            playerName = name
        }
    }

    /// Some prompts are written into the message buffer before they are drawn
    /// (the name prompt is queued while the egg/birthday scene is shown). They
    /// are only displayed when the glyph matcher reads them from the screen.
    private func queuedPrompt(_ memory: String) -> String {
        memory.hasPrefix("あなたのなまえは") ? "" : memory
    }

    /// What the panel shows. Prices are formatted at runtime and often carry
    /// only a placeholder in the buffer, so price tokens are removed unless the
    /// whole line already has a curated translation (for example a phrase that
    /// legitimately contains "10G").
    private var captionText: String {
        let text = ocrText
        guard !text.isEmpty else { return "" }
        if LocalizedText.hasExact(text) { return text }
        return stripPrices(text)
    }

    private func stripPrices(_ text: String) -> String {
        let stripped = text.replacingOccurrences(
            of: #"[0-9０-９,，]+ ?[GgＧｇ]"#,
            with: "",
            options: .regularExpression
        )
        return stripped
            .replacingOccurrences(of: #"\s{2,}"#, with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// True for the date/time setup value, including the blinking field where
    /// digits are replaced by blanks ("/1 AM 12:").
    private func isClockValue(_ text: String) -> Bool {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty else { return false }
        if t.range(of: #"^[0-9/ ]*(AM|PM)[0-9: ]*$"#, options: .regularExpression) != nil {
            return true
        }
        return t.range(of: #"^[0-9]{1,2}/[0-9]{1,2}$"#, options: .regularExpression) != nil
    }

    /// Shown while the panel is up but nothing has been recognised yet, so the
    /// panel never disappears between lines.
    private var captionPlaceholder: String {
        let english = model.prefs.language == "en"
        if model.prefs.care_paused {
            // "Sleeping, press B" reads like the toy is simply asleep, which
            // hides the reason nothing is moving.
            return model.asleep
                ? (english ? "Care paused · sleeping — B wakes" : "已暂停养育 · 已休眠，按 B 唤醒")
                : (english ? "Care is paused" : "已暂停养育")
        }
        if model.asleep {
            return english ? "Sleeping — press B to wake" : "已休眠，按 B 唤醒"
        }
        if fontMatchable == false {
            return english ? "No font table for this ROM yet" : "当前 ROM 暂无字模支持"
        }
        return "……"
    }

    private func catchupCard(done: Double, total: Double) -> some View {
        let english = model.prefs.language == "en"
        let shown = min(max(done, 0), max(total, 1))
        return VStack(spacing: 12) {
            Text(english ? "Catching up, please wait…" : "拓麻歌子正在追赶时间，请稍候~")
                .font(.subheadline.weight(.medium))
                .multilineTextAlignment(.center)
            ProgressView(value: shown, total: max(total, 1))
                .progressViewStyle(.linear)
                .tint(Color.pmAccent)
            Text((english ? "Caught up " : "已补齐 ") + catchupText(shown)
                + (english ? " of " : " / 共 ") + catchupText(total))
                .font(.caption)
                .foregroundStyle(.secondary)
                .monospacedDigit()
        }
        .padding(20)
        .frame(maxWidth: 320)
        .pmGlassCard(24, fallback: .thickMaterial)
        .padding(24)
    }

    private func catchupText(_ seconds: Double) -> String {
        let total = Int(seconds.rounded())
        let hours = total / 3600
        let minutes = (total % 3600) / 60
        let english = model.prefs.language == "en"
        if hours > 0 {
            return english ? "\(hours)h \(minutes)m" : "\(hours) 小时 \(minutes) 分"
        }
        if minutes > 0 {
            return english ? "\(minutes)m \(total % 60)s" : "\(minutes) 分 \(total % 60) 秒"
        }
        return english ? "\(total)s" : "\(total) 秒"
    }

    private var translationTarget: Locale.Language {
        model.prefs.language == "en"
            ? Locale.Language(identifier: "en")
            : Locale.Language(identifier: "zh-Hans")
    }

    private var toolbar: some View {
        HStack {
            Spacer()
            if look == 1 || look == 2, model.prefs.screenshots, shutterAllowed(guide: showingGuide) {
                chromeButton("camera.fill", model.t("screenshots_label")) { model.captureScreen(bypassGuide: false) }
            }
            if look == 0 || look == 3 {
                chromeButton(
                    look == 3 ? "moon.fill" : "sun.max.fill",
                    model.t(look == 3 ? "appearance_night" : "appearance_shell")
                ) { toggleNight() }
            }
            chromeButton(
                model.prefs.care_paused ? "play.circle" : "pause.circle",
                model.t(model.prefs.care_paused ? "resume_care" : "pause_care"),
                active: model.prefs.care_paused
            ) { model.setPaused(!model.prefs.care_paused) }
            if #available(iOS 18.0, *) {
                Button {
                    liveTranslation.toggle()
                } label: {
                    Image(systemName: liveTranslation ? "character.bubble.fill" : "character.bubble")
                        .font(.system(size: 17, weight: .semibold))
                        .foregroundStyle(chromeColor)
                        .frame(width: 44, height: 44)
                        .pmGlass(Circle(), interactive: true)
                        .contentShape(Circle())
                }
                .accessibilityLabel(ocrTitle)
            }
            chromeButton("ellipsis", model.t("setup_settings")) { model.page = .settings }
        }
        .padding(.horizontal, 20)
        .padding(.top, 4)
    }

    /// The look that is actually drawn. "Follow system" only has two
    /// outcomes: the shell look in light mode, night in dark mode.
    private var look: Int {
        let style = model.prefs.appearance
        guard model.prefs.appearance_auto, style == 0 || style == 3 else { return style }
        return colorScheme == .dark ? 3 : 0
    }

    private func toggleNight() {
        var next = model.prefs
        next.appearance = look == 3 ? 0 : 3
        // An explicit tap wins over the system; following resumes from the
        // appearance sheet if the user wants it back.
        next.appearance_auto = false
        _ = model.engine.write(next)
        model.reload()
    }

    private var chromeColor: Color {
        look == 3 ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83)
    }

    private var ocrTitle: String {
        model.prefs.language == "en" ? "Live translation" : "实时翻译"
    }

    private func chromeButton(_ symbol: String, _ label: String, active: Bool = false, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(active ? Color.pmAccent : chromeColor)
                .frame(width: 44, height: 44)
                .pmGlass(Circle(), interactive: true)
                .contentShape(Circle())
        }
        .accessibilityLabel(label)
    }

    private var deviceStage: some View {
        GeometryReader { geo in
            let maxWidth = min(geo.size.width - 28, 420)
            let maxHeight = max(220, geo.size.height - 72)
            let width = min(maxWidth, maxHeight * 6 / 7)
            let height = EggMetrics.bodyHeight(width)
            let screen = EggMetrics.screen(bodyWidth: width)
            let originY = max(56, (geo.size.height - height) * 0.46)
            deviceBody(width: width, screen: screen)
                .frame(width: width, height: height)
                .position(x: geo.size.width / 2, y: originY + height / 2)
        }
    }

    private func deviceBody(width: CGFloat, screen: CGRect) -> some View {
        let key = EggMetrics.keySide(bodyWidth: width)
        return ZStack {
            EggShell(screen: screen, night: look == 3)
            lcd(side: screen.width)
                .frame(width: screen.width, height: screen.height)
                .position(x: screen.midX, y: screen.midY)
            shellKey("A", 0, "A", side: key, minimal: false)
                .position(EggMetrics.keyCenter(index: 0, bodyWidth: width, screen: screen))
            shellKey("B", 1, model.t("confirm_wake"), side: key, minimal: false)
                .position(EggMetrics.keyCenter(index: 1, bodyWidth: width, screen: screen))
            shellKey("C", 2, "C", side: key, minimal: false)
                .position(EggMetrics.keyCenter(index: 2, bodyWidth: width, screen: screen))
            if model.prefs.screenshots {
                let shutter = EggMetrics.shutter(bodyWidth: width)
                shutterButton
                    .frame(width: shutter.width, height: shutter.height)
                    .position(x: shutter.midX, y: shutter.midY)
            }
        }
    }

    private var minimalStage: some View {
        GeometryReader { geo in
            let side = min(geo.size.width - 48, geo.size.height - 188, 420)
            let key = min(64, max(44, geo.size.width / 7))
            VStack(spacing: 28) {
                Spacer(minLength: 48)
                lcd(side: side)
                    .frame(width: side, height: side)
                    .background(
                        RoundedRectangle(cornerRadius: 18)
                            .fill(Color(rgb: 0xF0D6E1))
                            .padding(-13)
                    )
                HStack(spacing: max(18, key * 0.45)) {
                    shellKey("A", 0, "A", side: key, minimal: true)
                    shellKey("B", 1, model.t("confirm_wake"), side: key, minimal: true)
                    shellKey("C", 2, "C", side: key, minimal: true)
                }
                Spacer(minLength: 24)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func smallStage(guide: Bool) -> some View {
        let side = LcdMetrics.side(appearance: 2, scalePercent: model.prefs.small_screen_scale)
        return VStack(spacing: 8) {
            Spacer(minLength: 0)
            lcd(side: side)
                .overlay(smallScreenPads(side: side, guide: guide))
            Text(model.t("small_a11y"))
                .font(.caption2)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 16)
            Spacer(minLength: 0)
        }
    }

    private func lcd(side: CGFloat) -> some View {
        LcdView(
            pixels: model.pixels,
            asleep: model.asleep,
            dotMatrix: model.prefs.dot_matrix,
            side: side,
            strength: model.engine.dotStrength(Float(side / 128))
        )
        .overlay {
            if flashActive {
                FlashOverlay(token: model.flashToken).allowsHitTesting(false)
            }
        }
        .onTapGesture(count: 2) {
            guard look != 2 else { return }
            model.captureScreen(bypassGuide: false)
        }
    }

    private var shutterButton: some View {
        Button(action: { model.captureScreen(bypassGuide: false) }) {
            Image(systemName: "camera.fill")
                .font(.body.weight(.semibold))
                .foregroundStyle(look == 3 ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .pmGlass(Capsule(style: .continuous), interactive: true)
        }
        .disabled(!shutterAllowed(guide: showingGuide))
        .accessibilityLabel(model.t("screenshots_label"))
    }

    private func shellKey(_ title: String, _ index: Int, _ label: String, side: CGFloat, minimal: Bool) -> some View {
        ShellKey(
            title: title,
            side: side,
            minimal: minimal,
            night: look == 3,
            accessibility: label
        ) { down in
            model.engine.key(index, down: down)
        }
    }

    @ViewBuilder
    private func smallScreenPads(side: CGFloat, guide: Bool) -> some View {
        if look == 2 {
            MultiPad(side: side) { x, y in
                model.engine.zone(
                    x: Float(x), y: Float(y),
                    w: Float(side), h: Float(side),
                    screenshots: model.prefs.screenshots
                )
            } onChange: { held, ended in
                applyZones(held: held, ended: ended, guide: guide)
            }
            .frame(width: side, height: side)
            .onAppear { KeyHaptics.prepare() }
            // Two-finger pinch resizes the small-screen look. There is no
            // slider for this any more; the gesture is the only way in.
            .simultaneousGesture(MagnificationGesture().onEnded { scale in
                model.engine.releaseKeys()
                var next = model.prefs
                let percent = Int((CGFloat(next.small_screen_scale) * scale).rounded())
                next.small_screen_scale = max(50, min(150, percent))
                next.small_screen_size_configured = true
                _ = model.engine.write(next)
                model.reload()
            })
        }
    }

    private func applyZones(held: [Int32], ended: [Int32], guide: Bool) {
        // TODO(exit-slop): SPEC §7.1. No pixel slop was specified. Leaving the zone cancels the hold.
        let keys = Set(held.compactMap { zone -> Int? in
            switch zone {
            case 1: return 0
            case 2: return 1
            case 3: return 2
            default: return nil
            }
        })
        for index in 0...2 {
            model.engine.key(index, down: keys.contains(index))
        }
        if ended.contains(4) { model.captureScreen(bypassGuide: false) }
        if held.contains(5) && model.engine.exitAllowed(ready: model.engine.hasCurrent(), shelf: model.shelfOpen, guide: guide) {
            if exitStart == nil { exitStart = Date() }
            if let exitStart, Date().timeIntervalSince(exitStart) >= 1 {
                var next = model.prefs
                next.appearance = 1
                next.appearance_auto = false
                _ = model.engine.write(next)
                model.reload()
                self.exitStart = nil
                UIImpactFeedbackGenerator(style: .light).impactOccurred()
            }
        } else {
            exitStart = nil
        }
    }

    private func shutterAllowed(guide: Bool) -> Bool {
        model.engine.hasCurrent() && !model.shelfOpen && model.prefs.screenshots && !guide
    }

}

/// Two fingers can hold A and C together. Each touch is its own zone.
struct MultiPad: UIViewRepresentable {
    var side: CGFloat
    var zoneAt: (CGFloat, CGFloat) -> Int32
    var onChange: ([Int32], [Int32]) -> Void

    func makeUIView(context: Context) -> Pad {
        let pad = Pad()
        pad.zoneAt = zoneAt
        pad.onChange = onChange
        pad.bounds.size = CGSize(width: side, height: side)
        return pad
    }

    func updateUIView(_ pad: Pad, context: Context) {
        pad.zoneAt = zoneAt
        pad.onChange = onChange
    }

    final class Pad: UIView {
        var zoneAt: ((CGFloat, CGFloat) -> Int32)?
        var onChange: (([Int32], [Int32]) -> Void)?
        private var active: Set<UITouch> = []

        override init(frame: CGRect) {
            super.init(frame: frame)
            isMultipleTouchEnabled = true
            backgroundColor = .clear
        }

        required init?(coder: NSCoder) { return nil }

        override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
            active.formUnion(touches)
            if touches.contains(where: { zone(of: $0) != 0 }) { KeyHaptics.tap() }
            emit(ended: [])
        }

        override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
            emit(ended: [])
        }

        override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
            let ended = touches.map { zone(of: $0) }
            active.subtract(touches)
            emit(ended: ended)
        }

        override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
            active.subtract(touches)
            emit(ended: [])
        }

        private func emit(ended: [Int32]) {
            onChange?(active.map { zone(of: $0) }, ended)
        }

        private func zone(of touch: UITouch) -> Int32 {
            let point = touch.location(in: self)
            return zoneAt?(point.x, point.y) ?? 0
        }
    }
}

/// Hardware keyboard: A/Left, S/Return, D/Right. Auto-repeat is ignored by the caller.
struct KeyCatcher: UIViewControllerRepresentable {
    var onKey: (String, Bool) -> Void

    func makeUIViewController(context: Context) -> KeyController {
        let controller = KeyController()
        controller.onKey = onKey
        return controller
    }

    func updateUIViewController(_ controller: KeyController, context: Context) {
        controller.onKey = onKey
    }

    final class KeyController: UIViewController {
        var onKey: ((String, Bool) -> Void)?
        override var canBecomeFirstResponder: Bool { true }
        override func viewDidAppear(_ animated: Bool) {
            super.viewDidAppear(animated)
            becomeFirstResponder()
        }
        override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
            guard handle(presses, down: true) else {
                super.pressesBegan(presses, with: event)
                return
            }
        }
        override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
            guard handle(presses, down: false) else {
                super.pressesEnded(presses, with: event)
                return
            }
        }
        override func pressesCancelled(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
            _ = handle(presses, down: false)
            super.pressesCancelled(presses, with: event)
        }
        private func handle(_ presses: Set<UIPress>, down: Bool) -> Bool {
            var matched = false
            for press in presses {
                guard let key = press.key else { continue }
                let name = key.charactersIgnoringModifiers.isEmpty ? arrow(key.keyCode) : key.charactersIgnoringModifiers
                if name == "a" || name == "A" || name == "s" || name == "S" || name == "d" || name == "D"
                    || name == "\r" || name == "left" || name == "right" {
                    onKey?(name, down)
                    matched = true
                }
            }
            return matched
        }
        private func arrow(_ code: UIKeyboardHIDUsage) -> String {
            switch code {
            case .keyboardLeftArrow: return "left"
            case .keyboardRightArrow: return "right"
            default: return ""
            }
        }
    }
}

private func keyIndex(_ name: String) -> Int? {
    switch name.lowercased() {
    case "a", "left": return 0
    case "s", "\r", "return": return 1
    case "d", "right": return 2
    default: return nil
    }
}
