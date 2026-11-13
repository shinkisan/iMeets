import SwiftUI
import UIKit

struct ToyView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var toolbarConfig: ToolbarConfig
    @Environment(\.colorScheme) private var colorScheme
    @State private var exitStart: Date?
    @State private var confirmLanDisconnect = false
    @State private var confirmResumeForLan = false
    @State private var showToolbarEditor = false
    @State private var showCareInfo = false
    @State private var downKeys: Set<Int> = []
    @State private var flashActive = false
    @AppStorage("pm.liveTranslation") private var liveTranslation = false
    @State private var ocrText = ""
    @State private var glyphSource = false
    @State private var fontMatchable: Bool?
    @AppStorage("pm.playerName") private var playerName = ""
    @AppStorage("pm.pip") private var pipEnabled = false
    /// Latest More-menu actions. The menu view itself does not redraw with
    /// the LCD, so taps read this object instead of a closure frozen earlier.
    @State private var moreRelay = ToyMoreRelay()

    var body: some View {
        let guide = showingGuide
        let actions = availableActions(guide: guide)
        ZStack {
            backgroundColor.ignoresSafeArea()
            if look == 2 {
                smallStage(guide: guide)
            } else if isMinimal {
                minimalStage
            } else {
                deviceStage
            }
            VStack {
                toolbar(actions)
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
        .sheet(isPresented: $model.showLan) { LanView() }
        .sheet(isPresented: $showCareInfo) { CareInfoView(engine: model.engine, english: english) }
        .onChange(of: showCareInfo) { open in
            if open {
                model.engine.releaseKeys()
                downKeys.removeAll()
            }
        }
        .alert("断开连接？", isPresented: $confirmLanDisconnect) {
            Button("断开", role: .destructive) { model.lan.stop() }
            Button("取消", role: .cancel) {}
        }
        .alert(
            english ? "Resume Care to Connect?" : "联机需要恢复养育",
            isPresented: $confirmResumeForLan
        ) {
            Button(english ? "Resume & Connect" : "恢复并联机") {
                model.setPaused(false)
                model.showLan = true
            }
            Button(english ? "Cancel" : "取消", role: .cancel) {}
        } message: {
            Text(
                english
                    ? "Care is currently paused. Multiplayer connection requires resuming the pet. Continue?"
                    : "当前养育已处于暂停状态。联机通信需要先恢复宠物运行，是否继续？"
            )
        }
        .background(KeyCatcher { name, down in
            guard !showCareInfo, let index = keyIndex(name) else { return }
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
        case 3, 4:
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
            shots: false,
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
            if shown > 0 {
                ProgressView(value: shown, total: max(total, 1))
                    .progressViewStyle(.linear)
                    .tint(Color.pmAccent)
                Text((english ? "Caught up " : "已补齐 ") + catchupText(shown)
                    + (english ? " of " : " / 共 ") + catchupText(total))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            } else {
                ProgressView()
                    .controlSize(.regular)
                    .frame(maxWidth: .infinity)
            }
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

    private func toolbar(_ available: Set<ToyToolbarAction>) -> some View {
        HStack(spacing: 8) {
            Spacer(minLength: 0)
            ForEach(toolbarConfig.visible(available: available)) { action in
                toolbarAction(action)
            }
            moreButton(available)
        }
        .padding(.horizontal, 20)
        .padding(.top, 4)
        .sheet(isPresented: $showToolbarEditor) {
            ToolbarEditorView(
                config: toolbarConfig,
                available: available,
                lanConnected: model.lan.connected,
                english: english
            )
        }
    }

    /// Actions the current look can draw. The editor greys out the rest instead
    /// of dropping them, so a layout survives a look change. Callers pass the
    /// guide flag in so the engine is asked once per body, not once per button.
    private func availableActions(guide: Bool) -> Set<ToyToolbarAction> {
        ToyToolbarAction.availability(
            look: look,
            screenshots: model.prefs.screenshots,
            cameraReady: shutterAllowed(guide: guide)
        )
    }

    private var english: Bool { model.prefs.language == "en" }

    @ViewBuilder
    private func toolbarAction(_ action: ToyToolbarAction) -> some View {
        switch action {
        case .info:
            chromeButton("info.circle", action.title(english)) { showCareInfo = true }
        case .camera:
            chromeButton("camera.fill", model.t("screenshots_label")) {
                model.captureScreen(bypassGuide: false)
            }
        case .appearance:
            chromeButton(
                isNight ? "moon.fill" : "sun.max.fill",
                model.t(isNight ? "appearance_night" : "appearance_shell")
            ) { toggleNight() }
        case .pause:
            let pauseLabel: String = {
                if model.lan.connected {
                    return english ? "Pause care (disabled during link)" : "暂停养育（联机中已禁用）"
                }
                return model.t(model.prefs.care_paused ? "resume_care" : "pause_care")
            }()
            chromeButton(
                model.prefs.care_paused ? "play.circle" : "pause.circle",
                pauseLabel,
                active: model.prefs.care_paused,
                disabled: model.lan.connected
            ) {
                guard !model.lan.connected else { return }
                model.setPaused(!model.prefs.care_paused)
            }
        case .translate:
            translateButton
        case .lan:
            lanButton
        case .pip:
            chromeButton(
                pipEnabled ? "pip.fill" : "pip",
                pipTitle,
                active: pipEnabled
            ) {
                togglePiP()
            }
        }
    }

    @ViewBuilder
    private var translateButton: some View {
        if #available(iOS 18.0, *) {
            Button {
                liveTranslation.toggle()
            } label: {
                Image(systemName: liveTranslation ? "character.bubble.fill" : "character.bubble")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(liveTranslation ? Color.pmAccent : chromeColor)
                    .frame(width: 44, height: 44)
                    .pmGlass(Circle(), interactive: true)
                    .contentShape(Circle())
            }
            .accessibilityLabel(ocrTitle)
        }
    }

    /// More carries whatever did not fit in the bar, then the editing and
    /// settings entries. Hidden buttons stay reachable from here, as HIG asks.
    ///
    /// The popup is a UIKit menu. SwiftUI's own `Menu` restarts its glass
    /// morph on every LCD frame, so the open popup flashes. The button keeps
    /// the same menu object until an entry actually changes.
    private func moreButton(_ available: Set<ToyToolbarAction>) -> some View {
        let extra = toolbarConfig.more(available: available)
        let entries = extra.map { action in
            ToyMoreEntry(
                action: action,
                title: moreTitle(action),
                symbol: action.symbol,
                disabled: action == .pause && model.lan.connected
            )
        }
        moreRelay.perform = { perform($0) }
        moreRelay.editToolbar = { showToolbarEditor = true }
        moreRelay.openSettings = { model.page = .settings }
        return ToyMenuButton(
            model: ToyMenuModel(
                entries: entries,
                night: isNight,
                english: english,
                editTitle: english ? "Edit Toolbar" : "编辑顶栏",
                settingsTitle: model.t("setup_settings")
            ),
            relay: moreRelay
        )
        .frame(width: 44, height: 44)
    }

    private func moreTitle(_ action: ToyToolbarAction) -> String {
        switch action {
        case .pause:
            if model.lan.connected {
                return model.t("pause_care") + (english ? " · In link" : " · 联机中")
            }
            return model.t(model.prefs.care_paused ? "resume_care" : "pause_care")
        case .lan where model.lan.connected:
            return action.title(english) + (english ? " · Connected" : " · 已连接")
        default:
            return action.title(english)
        }
    }

    private func perform(_ action: ToyToolbarAction) {
        switch action {
        case .info:
            showCareInfo = true
        case .camera:
            model.captureScreen(bypassGuide: false)
        case .appearance:
            toggleNight()
        case .pause:
            guard !model.lan.connected else { return }
            model.setPaused(!model.prefs.care_paused)
        case .translate:
            if #available(iOS 18.0, *) { liveTranslation.toggle() }
        case .lan:
            if model.lan.connected {
                confirmLanDisconnect = true
            } else if model.prefs.care_paused {
                confirmResumeForLan = true
            } else {
                model.showLan = true
            }
        case .pip:
            togglePiP()
        }
    }

    private func togglePiP() {
        pipEnabled.toggle()
        model.setPiP(pipEnabled)
        if pipEnabled {
            model.notice = model.t("pip_background_toast")
        } else {
            model.notice = model.t("pip_disabled_toast")
        }
    }

    /// The look that is actually drawn. "Follow system" only has two
    /// outcomes: the shell look in light mode, night in dark mode.
    private var look: Int {
        ToyLook.resolve(model.prefs, dark: colorScheme == .dark)
    }

    private var isNight: Bool { look == 3 || look == 4 }
    private var isMinimal: Bool { look == 1 || look == 4 }

    private func toggleNight() {
        var next = model.prefs
        if isMinimal {
            next.minimal_follow_system = false
            next.appearance = isNight ? 1 : 4
        } else {
            next.shell_follow_system = false
            next.appearance = isNight ? 0 : 3
        }
        _ = model.engine.write(next)
        model.reload()
    }

    private var lanButton: some View {
        Button {
            if model.lan.connected {
                confirmLanDisconnect = true
            } else if model.prefs.care_paused {
                confirmResumeForLan = true
            } else {
                model.showLan = true
            }
        } label: {
            if model.lan.connected {
                HStack(spacing: 6) {
                    Circle().fill(Color.green).frame(width: 7, height: 7)
                    Text("面对面").font(.system(size: 13, weight: .semibold))
                }
                .foregroundStyle(chromeColor)
                .padding(.horizontal, 12)
                .frame(height: 44)
                .pmGlass(Capsule(), interactive: true)
                .contentShape(Capsule())
            } else {
                Image(systemName: "antenna.radiowaves.left.and.right")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(chromeColor)
                    .frame(width: 44, height: 44)
                    .pmGlass(Circle(), interactive: true)
                    .contentShape(Circle())
            }
        }
        .accessibilityLabel("面对面联机")
    }

    private var chromeColor: Color {
        isNight ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83)
    }

    private var ocrTitle: String {
        model.prefs.language == "en" ? "Live translation" : "实时翻译"
    }

    private var pipTitle: String {
        english ? "Picture in Picture" : "画中画"
    }

    private func chromeButton(
        _ symbol: String,
        _ label: String,
        active: Bool = false,
        disabled: Bool = false,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(
                    disabled
                        ? chromeColor.opacity(0.35)
                        : (active ? Color.pmAccent : chromeColor)
                )
                .frame(width: 44, height: 44)
                .pmGlass(Circle(), interactive: !disabled)
                .contentShape(Circle())
        }
        .disabled(disabled)
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
                            .fill(isNight ? Color(rgb: 0x2E2748) : Color(rgb: 0xF0D6E1))
                            .padding(-13)
                    )
                HStack(spacing: max(24, key * 0.62)) {
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
                .foregroundStyle(isNight ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83))
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
            night: isNight,
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
                    screenshots: false
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
        if held.contains(5) && model.engine.exitAllowed(ready: model.engine.hasCurrent(), shelf: model.shelfOpen, guide: guide) {
            if exitStart == nil { exitStart = Date() }
            if let exitStart, Date().timeIntervalSince(exitStart) >= 1 {
                var next = model.prefs
                next.appearance = 1
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

private struct ToyMoreEntry: Equatable, Identifiable {
    let action: ToyToolbarAction
    let title: String
    let symbol: String
    var disabled: Bool = false
    var id: ToyToolbarAction { action }
}

/// Holds the current More-menu actions. Taps read it so the menu view can
/// stay still across LCD frames without keeping a stale closure.
private final class ToyMoreRelay {
    var perform: (ToyToolbarAction) -> Void = { _ in }
    var editToolbar: () -> Void = {}
    var openSettings: () -> Void = {}
}

private struct ToyMenuModel: Equatable {
    var entries: [ToyMoreEntry]
    var night: Bool
    var english: Bool
    var editTitle: String
    var settingsTitle: String
}

/// UIKit menu anchored to a glass circle. `render` leaves the presented menu
/// alone while the LCD republishes frames; replacing `button.menu` on an open
/// popup is what makes it flash.
private struct ToyMenuButton: UIViewRepresentable {
    var model: ToyMenuModel
    var relay: ToyMoreRelay

    func makeUIView(context: Context) -> ToyMenuHost {
        let host = ToyMenuHost()
        host.setContentHuggingPriority(.required, for: .horizontal)
        host.setContentHuggingPriority(.required, for: .vertical)
        host.setContentCompressionResistancePriority(.required, for: .horizontal)
        host.setContentCompressionResistancePriority(.required, for: .vertical)
        return host
    }

    func updateUIView(_ host: ToyMenuHost, context: Context) {
        host.render(model, relay: relay)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: ToyMenuHost, context: Context) -> CGSize? {
        CGSize(width: 44, height: 44)
    }
}

private final class ToyMenuHost: UIView {
    private let button = UIButton(type: .custom)
    private var rendered: ToyMenuModel?

    override init(frame: CGRect) {
        super.init(frame: frame)
        button.showsMenuAsPrimaryAction = true
        button.preferredMenuElementOrder = .fixed
        button.translatesAutoresizingMaskIntoConstraints = false
        addSubview(button)
        NSLayoutConstraint.activate([
            button.leadingAnchor.constraint(equalTo: leadingAnchor),
            button.trailingAnchor.constraint(equalTo: trailingAnchor),
            button.topAnchor.constraint(equalTo: topAnchor),
            button.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { nil }

    override var intrinsicContentSize: CGSize { CGSize(width: 44, height: 44) }

    func render(_ model: ToyMenuModel, relay: ToyMoreRelay) {
        let previous = rendered
        rendered = model
        if previous?.night != model.night || button.configuration == nil {
            button.configuration = Self.chrome(night: model.night)
        }
        let menuChanged = previous?.entries != model.entries
            || previous?.editTitle != model.editTitle
            || previous?.settingsTitle != model.settingsTitle
        if menuChanged || button.menu == nil {
            button.menu = Self.menu(model, relay: relay)
        }
        if previous?.english != model.english || button.accessibilityLabel == nil {
            button.accessibilityLabel = model.english ? "More" : "更多"
            button.accessibilityHint = model.english
                ? "Hidden buttons, edit toolbar and settings"
                : "隐藏的按钮、编辑顶栏和设置"
        }
    }

    private static func chrome(night: Bool) -> UIButton.Configuration {
        let color = UIColor(
            red: CGFloat(night ? 0xD9 : 0xA5) / 255,
            green: CGFloat(night ? 0xCD : 0x6F) / 255,
            blue: CGFloat(night ? 0xF4 : 0x83) / 255,
            alpha: 1
        )
        let image = UIImage(
            systemName: "ellipsis",
            withConfiguration: UIImage.SymbolConfiguration(pointSize: 17, weight: .semibold)
        )?.withTintColor(color, renderingMode: .alwaysOriginal)
        if #available(iOS 26.0, *) {
            var config = UIButton.Configuration.glass()
            config.image = image
            config.baseForegroundColor = color
            config.cornerStyle = .capsule
            return config
        }
        var config = UIButton.Configuration.plain()
        config.image = image
        config.baseForegroundColor = color
        config.background.visualEffect = UIBlurEffect(style: .systemChromeMaterial)
        config.background.cornerRadius = 22
        config.cornerStyle = .capsule
        return config
    }

    private static func menuIcon(_ name: String) -> UIImage? {
        let accent = UIColor(red: 194.0 / 255, green: 84.0 / 255, blue: 121.0 / 255, alpha: 1)
        return UIImage(systemName: name)?.withTintColor(accent, renderingMode: .alwaysOriginal)
    }

    private static func menu(_ model: ToyMenuModel, relay: ToyMoreRelay) -> UIMenu {
        let actions = model.entries.map { entry in
            let action = UIAction(title: entry.title, image: menuIcon(entry.symbol)) { _ in
                relay.perform(entry.action)
            }
            if entry.disabled {
                action.attributes = .disabled
            }
            return action
        }
        let edit = UIAction(
            title: model.editTitle,
            image: menuIcon("slider.horizontal.3")
        ) { _ in
            relay.editToolbar()
        }
        let settings = UIAction(
            title: model.settingsTitle,
            image: menuIcon("gearshape")
        ) { _ in
            relay.openSettings()
        }
        let tail = UIMenu(options: .displayInline, children: [edit, settings])
        let children: [UIMenuElement] = actions.isEmpty
            ? [tail]
            : [UIMenu(options: .displayInline, children: actions), tail]
        return UIMenu(children: children)
    }
}
