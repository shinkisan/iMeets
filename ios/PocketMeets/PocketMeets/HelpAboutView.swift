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
                    Button { model.closeHelp() } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done"))
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
            }
            .navigationTitle(model.t("language_title"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button { model.showLanguage = false } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done"))
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
                    Image(systemName: "checkmark").foregroundStyle(.primary)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

struct AboutView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        Form {
            Section {
                Text(model.t("about_simple_body"))
                Link(destination: URL(string: "https://github.com/oreo-fox/nijiiro-egg")!) {
                    Text(model.t("about_core_line"))
                }
                Link(destination: URL(string: "https://github.com/kanatallow/Open-Meets")!) {
                    Text(model.t("about_sibling_line"))
                }
            }
        }
        .navigationTitle(model.t("about_title"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button { model.closeAbout() } label: { Image(systemName: "chevron.backward").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("back"))
            }
        }
    }
}
