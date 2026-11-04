import SwiftUI
import UIKit
import UniformTypeIdentifiers

struct SettingsView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var importingSave = false
    @State private var confirmDelete = false
    @State private var showLook = false
    @ScaledMetric(relativeTo: .body) private var iconColumn: CGFloat = 22
    @AppStorage("pm.liveTranslation") private var liveTranslation = false

    var body: some View {
        Form {
            Section(model.t("settings_display")) {
                navRow("paintpalette", model.t("help_look_title"), lookName(model.prefs.appearance)) { showLook = true }
                Toggle(isOn: bind(\.dot_matrix)) { settingLabel("circle.grid.3x3", model.t("dot_matrix_label")) }
                Toggle(isOn: bind(\.follow_system_sleep)) { settingLabel("sun.max", model.t("keep_display_lit")) }
                Toggle(isOn: bind(\.screenshots)) { settingLabel("camera", model.t("screenshots_label")) }
                if #available(iOS 18.0, *) {
                    Toggle(isOn: $liveTranslation) { settingLabel("text.viewfinder", ocrTitle) }
                }
            }
            Section(model.t("settings_sound")) {
                Toggle(isOn: soundOn) { settingLabel("speaker.wave.2", model.t("sound_label")) }
                Toggle(isOn: hapticsBinding) { settingLabel("hand.tap", model.t("key_haptics_label")) }
            }
            Section(model.t("settings_system")) {
                Toggle(isOn: enhancedBinding) { settingLabel("moon", model.t("residence_toggle")) }
                Toggle(isOn: pauseBinding) { settingLabel("pause.circle", model.t("pause_care")) }
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
                navRow("globe", model.t("language_title"), model.prefs.language == "zh" ? "简体中文" : "English") { model.showLanguage = true }
                navRow("info.circle", model.t("about_title"), nil) { model.page = .about }
            }
        }
        .navigationTitle(model.t("setup_settings"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button { model.page = .toy } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
        .sheet(isPresented: $showLook) { LookSheet() }
        .fileImporter(isPresented: $importingSave, allowedContentTypes: [.data]) { result in
            guard case let .success(url) = result else { return }
            guard let path = model.copyFile(from: url, name: "incoming.t18s") else {
                model.notice = model.t("import_failed")
                return
            }
            model.notice = model.engine.importSave(from: path)
        }
        .confirmationDialog(model.t("delete_confirm"), isPresented: $confirmDelete) {
            Button(model.t("delete_yes"), role: .destructive) {
                model.notice = model.engine.deleteCurrent()
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

    /// The look actually drawn: with following on, the family stays and the
    /// system picks its day or night form.
    private func lookName(_ value: Int) -> String {
        var shown = value
        if model.prefs.appearance_auto, [0, 1, 3, 4].contains(value) {
            let minimal = value == 1 || value == 4
            shown = colorScheme == .dark ? (minimal ? 4 : 3) : (minimal ? 1 : 0)
        }
        switch shown {
        case 1: return model.t("appearance_minimal")
        case 2: return model.t("small_mode")
        case 3: return model.t("appearance_night")
        case 4: return model.t("appearance_minimal_night")
        default: return model.t("appearance_shell")
        }
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

    private var enhancedBinding: Binding<Bool> {
        Binding(get: { model.prefs.background_care }, set: { model.setEnhanced($0) })
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
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    lookRow(model.t("appearance_shell"), 0)
                    lookRow(model.t("appearance_night"), 3)
                    lookRow(model.t("appearance_minimal"), 1)
                    lookRow(model.t("appearance_minimal_night"), 4)
                    lookRow(model.t("small_mode"), 2)
                    Toggle(isOn: autoAppearance) {
                        Text(model.t("appearance_auto"))
                    }
                }
            }
            .navigationTitle(model.t("help_look_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button { dismiss() } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
        }
    }

    private var autoAppearance: Binding<Bool> {
        Binding(get: { model.prefs.appearance_auto }, set: { on in
            var next = model.prefs
            next.appearance_auto = on
            // Following the system only means something for looks that have
            // a light and a dark form.
            if on && ![0, 1, 3, 4].contains(next.appearance) {
                next.appearance = 0
            }
            _ = model.engine.write(next)
            model.reload()
        })
    }

    /// With following on, these rows pick the family and the system picks its
    /// day or night form, so the tick marks the drawn look rather than the
    /// stored one.
    private var effectiveLook: Int {
        let style = model.prefs.appearance
        guard model.prefs.appearance_auto, [0, 1, 3, 4].contains(style) else { return style }
        let minimal = style == 1 || style == 4
        return colorScheme == .dark ? (minimal ? 4 : 3) : (minimal ? 1 : 0)
    }

    private func lookRow(_ title: String, _ value: Int) -> some View {
        Button {
            var next = model.prefs
            next.appearance = value
            _ = model.engine.write(next)
            model.reload()
        } label: {
            HStack {
                Text(title).foregroundStyle(Color(uiColor: .label))
                Spacer()
                if effectiveLook == value {
                    Image(systemName: "checkmark")
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

struct CareSheet: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section(model.t("help_care_title")) {
                    Toggle(model.t("residence_toggle"), isOn: Binding(get: { model.prefs.background_care }, set: { model.setEnhanced($0) }))
                    Text(model.t("ios_background")).font(.footnote)
                    Toggle(model.t("pause_care"), isOn: Binding(get: { model.prefs.care_paused }, set: { model.setPaused($0) }))
                    Text(model.t("alarm_inexact")).font(.footnote)
                    Text(model.t("long_absence")).font(.footnote)
                    Text(model.t("force_quit_help")).font(.footnote)
                }
            }
            .navigationTitle(model.t("help_care_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button { dismiss() } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
        }
    }
}
