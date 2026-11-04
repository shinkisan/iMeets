import SwiftUI
import UniformTypeIdentifiers

struct LibraryView: View {
    @EnvironmentObject private var model: AppModel
    @State private var picking = false

    var body: some View {
        List {
            Section {
                if model.library.isEmpty {
                    Text(model.t("no_rom_imported"))
                        .foregroundStyle(.secondary)
                }
                ForEach(model.library, id: \.self) { stem in
                    Button {
                        model.notice = model.t("switch_starting")
                        let path = model.engine.root.appendingPathComponent("ROMs").appendingPathComponent("\(stem).bin").path
                        let message = model.engine.switchRom(path)
                        model.notice = message
                        model.reload()
                    } label: {
                        Label(stem, systemImage: "externaldrive")
                            .foregroundStyle(.primary)
                    }
                }
            }
            Section {
                Button { picking = true } label: {
                    Label(model.t("choose_rom"), systemImage: "doc.badge.plus")
                }
            }
        }
        .listStyle(.insetGrouped)
        .navigationTitle(model.t("switch_rom"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button(model.t("back")) { model.page = .settings } } }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.data, .archive]) { result in
            guard case let .success(url) = result else { return }
            guard let path = model.copyFile(from: url, name: "rom-import-\(UUID().uuidString).tmp") else {
                model.notice = model.t("import_failed_again")
                return
            }
            let imported = model.engine.importRom(from: path)
            try? FileManager.default.removeItem(atPath: path)
            model.reload()
            model.notice = imported.ok ? (imported.current.map { model.t("loaded_prefix") + $0 } ?? "") : imported.error
        }
        .onAppear { model.reload() }
    }
}
