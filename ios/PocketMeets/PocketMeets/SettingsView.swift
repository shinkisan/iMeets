import SwiftUI
import UIKit
import UniformTypeIdentifiers

struct SettingsView: View {
    let toolbarCapacity: Int
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var toolbarConfig: ToolbarConfig
    @Environment(\.colorScheme) private var colorScheme
    @State private var importingSave = false
    @State private var confirmDelete = false
    @State private var sheet: SettingsSheet?
    @ScaledMetric(relativeTo: .body) private var iconColumn: CGFloat = 22
    @AppStorage("pm.liveTranslation") private var liveTranslation = false
    @AppStorage("pm.pip") private var pipEnabled = false

    private enum SettingsSheet: String, Identifiable {
        case look
        case toolbar
        var id: String { rawValue }
    }

    var body: some View {
        Form {
            Section(model.t("settings_display")) {
                navRow("paintpalette", model.t("help_look_title"), lookName(model.prefs.appearance)) { sheet = .look }
                Toggle(isOn: bind(\.dot_matrix)) { settingLabel("circle.grid.3x3", model.t("dot_matrix_label")) }
                Toggle(isOn: bind(\.follow_system_sleep)) { settingLabel("sun.max", model.t("keep_display_lit")) }
                Toggle(isOn: bind(\.screenshots)) { settingLabel("camera", model.t("screenshots_label")) }
                if #available(iOS 18.0, *) {
                    Toggle(isOn: $liveTranslation) { settingLabel("text.viewfinder", ocrTitle) }
                }
                navRow("slider.horizontal.3", editToolbarTitle, toolbarSummary) { sheet = .toolbar }
            }
            Section(model.t("settings_sound")) {
                Toggle(isOn: soundOn) { settingLabel("speaker.wave.2", model.t("sound_label")) }
                Toggle(isOn: hapticsBinding) { settingLabel("hand.tap", model.t("key_haptics_label")) }
                navRow("bell", model.t("notifications_title"), model.notificationStatusTitle) {
                    model.manageNotifications()
                }
                EventNotifySection()
            }
            Section(model.t("settings_system")) {
                Toggle(isOn: pauseBinding) { settingLabel("pause.circle", model.t("pause_care")) }
                    .disabled(model.lan.connected)
                Toggle(isOn: $pipEnabled) { settingLabel("pip", pipTitle) }
                    .onChange(of: pipEnabled) { on in model.setPiP(on) }
                Toggle(isOn: bind(\.fast_keys)) { settingLabel("bolt", model.t("fast_keys_label")) }
                    .disabled(!model.engine.usesSweetsFont())
            }
            Section {
                actionRow("square.and.arrow.down", model.t("save_now")) { model.notice = model.engine.saveNow() }
                actionRow("tray.and.arrow.down", model.t("import_save")) { importingSave = true }
                actionRow("square.and.arrow.up", model.t("export_backup")) { exportSave() }
                navRow("arrow.triangle.2.circlepath", model.t("switch_rom"), nil) { model.page = .library }
                Button(role: .destructive) { confirmDelete = true } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "trash").frame(width: iconColumn).accessibilityHidden(true)
                        Text(model.t("delete_save"))
                        Spacer(minLength: 0)
                    }
                }
            } header: {
                Text(model.t("progress_section"))
            }
            Section(model.t("settings_other")) {
                navRow(
                    "books.vertical",
                    model.t("shelf_title"),
                    model.t(model.prefs.shelf_enabled ? "shelf_status_on" : "shelf_status_off")
                ) { model.page = .shelf }
                navRow("globe", model.t("language_title"), model.prefs.language == "zh" ? "简体中文" : "English") { model.showLanguage = true }
                navRow("info.circle", model.t("about_title"), nil) { model.page = .about }
            }
        }
        .navigationTitle(model.t("setup_settings"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button { model.page = .toy } label: { Image(systemName: "chevron.backward").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("back")) } }
        .sheet(item: $sheet) { which in
            switch which {
            case .look:
                LookSheet()
            case .toolbar:
                ToolbarEditorView(
                    config: toolbarConfig,
                    available: toolbarAvailable,
                    capacity: toolbarCapacity,
                    lanConnected: model.lan.connected,
                    english: model.prefs.language == "en"
                )
            }
        }
        .fileImporter(isPresented: $importingSave, allowedContentTypes: [.data]) { result in
            guard case let .success(url) = result else { return }
            guard let path = model.copyFile(from: url, name: "incoming.t18s") else {
                model.notice = model.t("import_failed")
                return
            }
            model.notice = model.engine.importSave(from: path)
        }
        .alert(model.t("delete_alert_title"), isPresented: $confirmDelete) {
            Button(model.t("cancel"), role: .cancel) { }
            Button(model.t("delete_yes"), role: .destructive) {
                model.notice = model.engine.deleteCurrent()
            }
        } message: {
            Text(model.t("delete_confirm"))
        }
        .onAppear {
            model.refreshNotificationPermission()
            if ProcessInfo.processInfo.arguments.contains("-pmToolbar") {
                sheet = .toolbar
            }
        }
    }

    /// One fixed icon column keeps every title in the same place, whether the
    /// row is a toggle, a navigation link or an action.
    private func settingLabel(_ symbol: String, _ title: String) -> some View {
        HStack(spacing: 12) {
            Image(systemName: symbol)
                .frame(width: iconColumn)
                .foregroundStyle(Color.pmAccent)
                .accessibilityHidden(true)
            Text(title)
        }
    }

    private func navRow(_ symbol: String, _ title: String, _ value: String?, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: symbol)
                    .frame(width: iconColumn)
                    .foregroundStyle(Color.pmAccent)
                    .accessibilityHidden(true)
                Text(title)
                    .foregroundStyle(Color(uiColor: .label))
                Spacer(minLength: 8)
                if let value {
                    Text(value)
                        .foregroundStyle(Color(uiColor: .secondaryLabel))
                        .lineLimit(1)
                }
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(Color(uiColor: .tertiaryLabel))
                    .accessibilityHidden(true)
            }
        }
    }

    private func actionRow(_ symbol: String, _ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: symbol)
                    .frame(width: iconColumn)
                    .accessibilityHidden(true)
                Text(title)
                    .foregroundStyle(Color(uiColor: .label))
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var ocrTitle: String {
        model.prefs.language == "en" ? "Live translation" : "实时翻译"
    }

    private var pipTitle: String {
        model.prefs.language == "en" ? "Picture in Picture" : "后台画中画"
    }

    private var editToolbarTitle: String {
        model.prefs.language == "en" ? "Edit Toolbar" : "编辑顶栏"
    }

    private var toolbarAvailable: Set<ToyToolbarAction> {
        ToyToolbarAction.availability(
            look: ToyLook.resolve(model.prefs, dark: colorScheme == .dark),
            screenshots: model.prefs.screenshots,
            cameraReady: model.hasSession && !model.shelfOpen,
            shelf: model.prefs.shelf_enabled
        )
    }

    private var toolbarSummary: String {
        let count = toolbarConfig.visible(available: toolbarAvailable, capacity: toolbarCapacity).count
        return model.prefs.language == "en" ? "\(count) buttons" : "\(count) 个按钮"
    }

    /// Family plus the way its day/night form is picked.
    private func lookName(_ value: Int) -> String {
        if value == 2 { return model.t("small_mode") }
        let minimal = value == 1 || value == 4
        let follows = minimal ? model.prefs.minimal_follow_system : model.prefs.shell_follow_system
        let mode = follows
            ? model.t("appearance_auto")
            : model.t(value == 3 || value == 4 ? "appearance_night" : "appearance_day")
        let family = model.t(minimal ? "appearance_minimal" : "appearance_shell")
        return "\(family) · \(mode)"
    }

    private var soundOn: Binding<Bool> {
        Binding(get: { !model.prefs.muted }, set: { on in
            var next = model.prefs
            next.muted = !on
            _ = model.engine.write(next)
            model.reload()
        })
    }

    private var hapticsBinding: Binding<Bool> {
        Binding(get: { model.prefs.key_haptics }, set: { on in
            var next = model.prefs
            next.key_haptics = on
            KeyHaptics.enabled = on
            _ = model.engine.write(next)
            model.reload()
            // A sample so the switch itself shows whether the device vibrates.
            KeyHaptics.tap()
        })
    }

    private var pauseBinding: Binding<Bool> {
        Binding(get: { model.prefs.care_paused }, set: { model.setPaused($0) })
    }

    private func bind(_ keyPath: WritableKeyPath<Prefs, Bool>) -> Binding<Bool> {
        Binding(get: { model.prefs[keyPath: keyPath] }, set: { value in
            var next = model.prefs
            next[keyPath: keyPath] = value
            _ = model.engine.write(next)
            model.reload()
        })
    }

    private func exportSave() {
        let name = model.engine.exportName()
        let url = model.engine.cacheURL.appendingPathComponent(name)
        let result = model.engine.exportSave(to: url.path)
        guard result.ok else { model.notice = result.message; return }
        // A save-to-files picker lets the person choose the destination folder
        // instead of dropping the backup into whatever the share sheet picks.
        let picker = UIDocumentPickerViewController(forExporting: [url], asCopy: true)
        picker.shouldShowFileExtensions = true
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        let host = scene?.windows.first { $0.isKeyWindow }?.rootViewController ?? scene?.windows.first?.rootViewController
        host?.present(picker, animated: true)
    }
}

/// Expandable care reminders. The metric is a menu; the threshold is a wheel.
private struct EventNotifySection: View {
    @EnvironmentObject private var model: AppModel
    @State private var wheel: Int?
    @ScaledMetric(relativeTo: .body) private var iconColumn: CGFloat = 22

    private var english: Bool { model.prefs.language == "en" }
    private var rules: [EventRule] { model.prefs.event_rules }

    var body: some View {
        DisclosureGroup {
            Text(english
                 ? "While care is running, closing the simulator schedules a notification for each condition the pet is expected to meet. Only the verified Japanese Sweets ROM can estimate the time."
                 : "未暂停时，关闭模拟器后，若预计宠物会满足下面任一条件，就发送通知。仅已验证的日版 Sweets 会估算时间。")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if rules.isEmpty {
                HStack {
                    Text(english ? "Add a condition" : "添加条件")
                        .foregroundStyle(Color(uiColor: .label))
                    Spacer(minLength: 8)
                    plusButton
                }
            }
            ForEach(rules.indices, id: \.self) { index in
                ruleRow(index)
                if wheel == index {
                    Picker(english ? "Threshold" : "阈值", selection: valueBinding(index)) {
                        ForEach(EventRule.range(for: rules[index].metric), id: \.self) { value in
                            Text("\(value)").tag(value)
                        }
                    }
                    .pickerStyle(.wheel)
                    .labelsHidden()
                    .frame(height: 132)
                }
            }
        } label: {
            HStack(spacing: 12) {
                Image(systemName: "bell.badge")
                    .frame(width: iconColumn)
                    .foregroundStyle(Color.pmAccent)
                    .accessibilityHidden(true)
                Text(english ? "Event notifications" : "事件通知")
            }
        }
    }

    private func ruleRow(_ index: Int) -> some View {
        HStack(spacing: 6) {
            rowText(english ? "When" : "当")
            metricMenu(index)
            rowText(compareText(rules[index].metric))
            numberButton(index)
            if let tail = tailText(rules[index].metric) {
                rowText(tail)
            }
            Spacer(minLength: 4)
            if index == rules.count - 1, rules.count < EventRule.limit {
                plusButton
            }
            Button {
                remove(index)
            } label: {
                Image(systemName: "minus.circle.fill")
                    .font(.title3)
                    .foregroundStyle(Color(uiColor: .secondaryLabel))
            }
            .buttonStyle(.borderless)
            .layoutPriority(1)
            .accessibilityLabel(english ? "Remove condition" : "删除条件")
        }
    }

    private func rowText(_ text: String) -> some View {
        Text(text)
            .foregroundStyle(Color(uiColor: .label))
            .lineLimit(1)
            .minimumScaleFactor(0.7)
    }

    private func metricMenu(_ index: Int) -> some View {
        Menu {
            ForEach(["hunger", "mood", "potty"], id: \.self) { metric in
                Button {
                    setMetric(index, metric)
                } label: {
                    if rules[index].metric == metric {
                        Label(metricTitle(metric), systemImage: "checkmark")
                    } else {
                        Text(metricTitle(metric))
                    }
                }
            }
        } label: {
            HStack(spacing: 2) {
                Text(metricTitle(rules[index].metric))
                Image(systemName: "chevron.up.chevron.down")
                    .font(.caption2.weight(.semibold))
                    .accessibilityHidden(true)
            }
            .foregroundStyle(Color(uiColor: .label))
            .lineLimit(1)
            .minimumScaleFactor(0.7)
        }
        .buttonStyle(.borderless)
        .menuOrder(.fixed)
        .accessibilityLabel(english ? "Condition" : "条件")
    }

    private func numberButton(_ index: Int) -> some View {
        Button {
            wheel = wheel == index ? nil : index
        } label: {
            Text("\(rules[index].below)")
                .font(.body.monospacedDigit())
                .foregroundStyle(Color(uiColor: .label))
                .frame(minWidth: 32)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Color(uiColor: .secondarySystemFill), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.borderless)
        .accessibilityLabel(english ? "Threshold \(rules[index].below)" : "阈值 \(rules[index].below)")
    }

    private var plusButton: some View {
        Button(action: add) {
            Image(systemName: "plus.circle.fill")
                .font(.title3)
                .foregroundStyle(Color.pmAccent)
        }
        .buttonStyle(.borderless)
        .layoutPriority(1)
        .accessibilityLabel(english ? "Add condition" : "添加条件")
    }

    private func valueBinding(_ index: Int) -> Binding<Int> {
        Binding(
            get: {
                guard rules.indices.contains(index) else { return 1 }
                return rules[index].below
            },
            set: { value in
                var next = rules
                guard next.indices.contains(index) else { return }
                next[index].below = value
                commit(next)
            }
        )
    }

    private func setMetric(_ index: Int, _ metric: String) {
        var next = rules
        next[index].metric = metric
        next[index] = next[index].clamped()
        commit(next)
    }

    private func add() {
        guard rules.count < EventRule.limit else { return }
        var next = rules
        next.append(EventRule.fresh(among: rules))
        commit(next)
        wheel = nil
        model.refreshNotificationPermission(requestIfNeeded: true)
    }

    private func remove(_ index: Int) {
        guard rules.indices.contains(index) else { return }
        var next = rules
        next.remove(at: index)
        if wheel == index {
            wheel = nil
        } else if let wheel, wheel > index {
            self.wheel = wheel - 1
        }
        commit(next)
    }

    private func commit(_ rules: [EventRule]) {
        var next = model.prefs
        next.event_rules = rules.map { $0.clamped() }
        // The wheel fires once per detent. Write the rules through without
        // reloading the ROM library on every notch.
        model.prefs = next
        _ = model.engine.write(next)
    }

    private func metricTitle(_ metric: String) -> String {
        switch metric {
        case "mood": return english ? "Affection" : "亲密度"
        case "potty": return english ? "Poop" : "排便"
        default: return english ? "Hunger" : "饥饿度"
        }
    }

    private func compareText(_ metric: String) -> String {
        if metric == "potty" { return english ? "is within" : "前" }
        return english ? "is below" : "小于"
    }

    private func tailText(_ metric: String) -> String? {
        if metric == "potty" { return english ? "min" : "分钟时" }
        return english ? nil : "时"
    }
}

struct ShelfSwitchForm: View {
    @EnvironmentObject private var model: AppModel
    @State private var pickingPack = false

    var body: some View {
        Form {
            Section(model.t("shelf_title")) {
                Toggle(model.t("shelf_enable"), isOn: shelfEnabled)
                Text(packStatus)
                Button(model.t("shelf_replace")) { pickingPack = true }
                Button(model.t("shelf_open_action")) {
                    model.page = .shelf
                }
                Text(model.t("shelf_small_note")).font(.footnote)
            }
        }
        .fileImporter(isPresented: $pickingPack, allowedContentTypes: [.archive, .data]) { result in
            guard case let .success(url) = result else { return }
            model.notice = model.t("shelf_loading")
            guard let path = model.copyFile(from: url, name: "shelf-import-\(UUID().uuidString).zip") else {
                model.notice = model.t("cannot_read_zip")
                return
            }
            let message = model.engine.shelfInstall(path)
            try? FileManager.default.removeItem(atPath: path)
            model.reload()
            model.notice = message.isEmpty ? model.t("shelf_loaded") : message
        }
    }

    private var packStatus: String {
        let pack = model.prefs.shelf_pack
        if pack.isEmpty { return model.t("shelf_not_loaded") }
        let url = model.engine.root
            .appendingPathComponent("TamaShelf")
            .appendingPathComponent(pack)
            .appendingPathComponent("shelf.json")
        guard let data = try? Data(contentsOf: url),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else {
            return model.t("shelf_loaded")
        }
        let count = (object["items"] as? [Any])?.count ?? 0
        return "\(name) · \(count)\(model.t("gift_suffix"))"
    }

    private var shelfEnabled: Binding<Bool> {
        Binding(get: { model.prefs.shelf_enabled }, set: { on in
            var next = model.prefs
            next.shelf_enabled = on
            _ = model.engine.write(next)
            model.reload()
        })
    }
}

struct LookSheet: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section(model.t("appearance_shell")) {
                    styleRow(model.t("appearance_day"), 0, family: 0)
                    styleRow(model.t("appearance_night"), 3, family: 0)
                    followRow(family: 0)
                }
                Section(model.t("appearance_minimal")) {
                    styleRow(model.t("appearance_day"), 1, family: 1)
                    styleRow(model.t("appearance_night"), 4, family: 1)
                    followRow(family: 1)
                }
                Section {
                    Button { write(appearance: 2) } label: {
                        HStack {
                            Text(model.t("small_mode")).foregroundStyle(Color(uiColor: .label))
                            Spacer()
                            if model.prefs.appearance == 2 {
                                Image(systemName: "checkmark").foregroundStyle(Color.pmAccent)
                            }
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
            .navigationTitle(model.t("help_look_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button { dismiss() } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
        }
    }

    /// 0 = shell family, 1 = minimal family, 2 = the small-screen page.
    private var activeFamily: Int {
        switch model.prefs.appearance {
        case 1, 4: return 1
        case 2: return 2
        default: return 0
        }
    }

    private func styleRow(_ title: String, _ value: Int, family: Int) -> some View {
        let follows = family == 0 ? model.prefs.shell_follow_system : model.prefs.minimal_follow_system
        let checked = activeFamily == family && !follows && model.prefs.appearance == value
        return Button {
            var next = model.prefs
            next.appearance = value
            if family == 0 { next.shell_follow_system = false } else { next.minimal_follow_system = false }
            commit(next)
        } label: {
            HStack {
                Text(title).foregroundStyle(Color(uiColor: .label))
                Spacer()
                if checked { Image(systemName: "checkmark").foregroundStyle(Color.pmAccent) }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func followRow(family: Int) -> some View {
        let follows = family == 0 ? model.prefs.shell_follow_system : model.prefs.minimal_follow_system
        let checked = activeFamily == family && follows
        return Button {
            var next = model.prefs
            if family == 0 {
                next.shell_follow_system = true
                if !(next.appearance == 0 || next.appearance == 3) { next.appearance = 0 }
            } else {
                next.minimal_follow_system = true
                if !(next.appearance == 1 || next.appearance == 4) { next.appearance = 1 }
            }
            commit(next)
        } label: {
            HStack {
                Text(model.t("appearance_auto")).foregroundStyle(Color(uiColor: .label))
                Spacer()
                if checked { Image(systemName: "checkmark") }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func write(appearance: Int) {
        var next = model.prefs
        next.appearance = appearance
        commit(next)
    }

    private func commit(_ next: Prefs) {
        _ = model.engine.write(next)
        model.reload()
    }
}
