import SwiftUI
import UniformTypeIdentifiers

struct SetupView: View {
    @EnvironmentObject private var model: AppModel
    @State private var pickingRom = false

    var body: some View {
        Form {
            Section {
                Text(model.t("setup_no_bundle"))
            } header: {
                Text(model.t("setup_start"))
            }
            Section {
                Button { pickingRom = true } label: {
                    Label(model.t("pick_rom"), systemImage: "doc.badge.plus")
                }
            } header: {
                Text(model.t("setup_import"))
            } footer: {
                Text(model.t("setup_import_body"))
            }
            Section(model.t("setup_settings")) {
                Button { model.showLanguage = true } label: {
                    Label(model.t("language_title"), systemImage: "globe")
                }
                Button { model.showHelp = true } label: {
                    Label(model.t("help_title"), systemImage: "questionmark.circle")
                }
            }
            Section {
                Text(model.t("about_body"))
                Text(model.t("mit_notice"))
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .navigationTitle(model.t("app_name"))
        .navigationBarTitleDisplayMode(.inline)
        .fileImporter(isPresented: $pickingRom, allowedContentTypes: [.data, .archive]) { result in
            guard case let .success(url) = result else { return }
            guard let path = model.copyFile(from: url, name: "rom-import-\(UUID().uuidString).tmp") else {
                model.notice = model.t("import_failed_again")
                return
            }
            let imported = model.engine.importRom(from: path)
            try? FileManager.default.removeItem(atPath: path)
            model.reload()
            if imported.ok {
                model.enter()
            } else {
                model.notice = imported.error
            }
        }
    }

}

struct NotesView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        NavigationStack {
            Form {
                Section(model.t("notes_camera_title")) {
                    Text(model.t("notes_camera_body"))
                }
                Section(model.t("notes_hut_title")) {
                    Text(model.t("notes_hut_body"))
                }
                Section(model.t("notes_keep_title")) {
                    Text(model.t("notes_keep_body"))
                }
            }
            .navigationTitle(model.t("notes_heading"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button(model.t("done")) { model.markNotesShown() }
                }
            }
        }
    }
}
