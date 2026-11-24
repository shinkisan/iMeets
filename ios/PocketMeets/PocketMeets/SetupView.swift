import SwiftUI
import UniformTypeIdentifiers

struct SetupView: View {
    @EnvironmentObject private var model: AppModel
    @State private var pickingRom = false
    @State private var pickingPack = false

    var body: some View {
        Form {
            Section {
                Button { pickingRom = true } label: {
                    Label(model.t("pick_rom"), systemImage: "doc.badge.plus")
                }
                Button { model.showLanguage = true } label: {
                    Label(model.t("language_title"), systemImage: "globe")
                }
                Button { model.page = .about } label: {
                    Label(model.t("about_title"), systemImage: "info.circle")
                }
            }
            Section {
                Toggle(model.t("shelf_enable"), isOn: shelfEnabled)
                Button { pickingPack = true } label: {
                    Label(model.t("load_pack_optional"), systemImage: "shippingbox")
                }
                if model.prefs.shelf_enabled {
                    Button { model.page = .shelf } label: {
                        Label(model.t("shelf_title"), systemImage: "books.vertical")
                    }
                }
            } header: {
                Text(model.t("shelf_title"))
            } footer: {
                Text(model.t("setup_shelf_body"))
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

    private var shelfEnabled: Binding<Bool> {
        Binding(get: { model.prefs.shelf_enabled }, set: { on in
            var next = model.prefs
            next.shelf_enabled = on
            _ = model.engine.write(next)
            model.reload()
        })
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
                    Button { model.markNotesShown() } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done"))
                }
            }
        }
    }
}
