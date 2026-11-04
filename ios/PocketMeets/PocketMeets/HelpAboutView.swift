import SwiftUI
import UIKit

struct HelpView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        NavigationStack {
            Form {
                help(model.t("help_controls_title"), model.t("help_controls_body"))
                help(model.t("help_save_title"), model.t("help_save_body"))
                help(model.t("help_care_title"), model.t("help_care_body"))
                help(model.t("help_pause_title"), model.t("help_pause_body"))
                help(model.t("help_look_title"), model.t("help_look_body"))
                help(model.t("help_link_title"), model.t("help_link_body"))
                help(model.t("help_rom_title"), model.t("help_rom_body"))
                Section {
                    Button(model.engine.supportURL()) {
                        if let url = URL(string: model.engine.supportURL()) {
                            UIApplication.shared.open(url)
                        }
                    }
                }
            }
            .navigationTitle(model.t("help_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button(model.t("done")) { model.closeHelp() }
                }
            }
        }
    }

    private func help(_ title: String, _ body: String) -> some View {
        Section(title) { Text(body) }
    }
}

struct LanguageView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        NavigationStack {
            List {
                languageRow("简体中文", code: "zh")
                languageRow("English", code: "en")
                Text(model.t("language_note"))
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            .navigationTitle(model.t("language_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button(model.t("done")) { model.showLanguage = false }
                }
            }
        }
    }

    private func languageRow(_ title: String, code: String) -> some View {
        Button {
            model.setLanguage(code)
        } label: {
            HStack {
                Text(title).foregroundStyle(.primary)
                Spacer()
                if model.prefs.language == code {
                    Image(systemName: "checkmark")
                }
            }
        }
    }
}

struct AboutView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        Form {
            Section(model.t("app_name")) {
                Text(model.t("about_body"))
                LabeledContent(model.t("version"), value: model.engine.releaseText())
                LabeledContent(model.t("build"), value: "\(model.engine.buildNumber())")
                LabeledContent("Mixing", value: model.engine.mixingVersion())
            }
            Section {
                Text(model.t("mit_notice"))
                Text(model.t("about_license"))
                Text(model.t("about_font"))
            }
            Section {
                Text(model.t("about_drawn"))
                    .foregroundStyle(.secondary)
            }
        }
        .navigationTitle(model.t("about_title"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button(model.t("back")) { model.page = .settings }
            }
        }
    }
}
