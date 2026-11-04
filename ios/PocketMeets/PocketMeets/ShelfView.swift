import SwiftUI
import UniformTypeIdentifiers

struct ShelfView: View {
    @EnvironmentObject private var model: AppModel
    @SceneStorage("shelf.section") private var section = 0
    @State private var pickingPack = false
    @State private var pickingCustom = false
    @State private var customKind: Int32 = 0
    @State private var showMoney = false
    @State private var showPack = false
    @State private var balanceText = "10000"
    @State private var parts = [0, 0, 0, 0]
    @State private var colors = [0, 0, 0]
    @State private var sex = 2
    @State private var mode = 0
    @State private var spouseName = ""

    private let categories = ["wallpaper", "food", "item", "accessory", "game", "station"]
    private let partLimits = [14, 19, 11, 7]

    var body: some View {
        Group {
            switch section {
            case 1: marriage
            case 2: imports
            default: categoriesHome
            }
        }
        .pmTopBar { sectionPicker }
        .navigationTitle(model.t("shelf_title"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button(model.t("shelf_close")) { close() }
            }
        }
        .safeAreaInset(edge: .bottom) {
            if section == 0 { bottomBar }
        }
        .onAppear {
            model.shelfOpen = true
            model.engine.setShelfOpen(true)
            restoreCharacter()
        }
        .onDisappear {
            model.shelfOpen = false
            model.engine.setShelfOpen(false)
        }
        .sheet(isPresented: $showMoney) { moneySheet }
        .sheet(isPresented: $showPack) {
            NavigationStack {
                ShelfSwitchForm()
                    .navigationTitle(model.t("shelf_pack_row"))
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar { ToolbarItem(placement: .confirmationAction) { Button { showPack = false } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
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
        .fileImporter(isPresented: $pickingCustom, allowedContentTypes: [.data, .image, .archive]) { result in
            guard case let .success(url) = result else { return }
            guard let path = model.copyFile(from: url, name: "custom-\(UUID().uuidString)") else {
                model.notice = model.t("import_failed")
                return
            }
            let kind = customKind == 1 && url.pathExtension.lowercased() == "zip" ? Int32(3) : customKind
            model.notice = model.engine.customImport(path, kind: kind)
        }
    }

    private var sectionPicker: some View {
        Picker(model.t("shelf_title"), selection: $section) {
            Text(model.t("tab_item")).tag(0)
            Text(model.t("shelf_marriage")).tag(1)
            Text(model.t("shelf_import")).tag(2)
        }
        .pickerStyle(.segmented)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    private var categoriesHome: some View {
        List {
            if model.prefs.shelf_pack.isEmpty {
                VStack(spacing: 14) {
                    Image(systemName: "books.vertical")
                        .font(.system(size: 34))
                        .foregroundStyle(.secondary)
                    Text(model.t("shelf_fill"))
                        .multilineTextAlignment(.center)
                        .foregroundStyle(.secondary)
                    Button(model.t("shelf_replace")) { pickingPack = true }
                        .pmProminent()
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 28)
                .listRowBackground(Color.clear)
            } else {
                ForEach(categories, id: \.self) { name in
                    NavigationLink { categoryList(name) } label: {
                        Label(label(name), systemImage: symbol(name))
                    }
                }
            }
        }
        .listStyle(.insetGrouped)
    }

    private func categoryList(_ tab: String) -> some View {
        let listed = model.engine.shelfList(tab: tab, language: model.prefs.shelf_language)
        return List {
            if listed.items.isEmpty {
                Text(model.t("shelf_layer_empty"))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .listRowBackground(Color.clear)
            }
            ForEach(listed.items) { item in
                NavigationLink { itemDetail(item) } label: { Text(item.name) }
            }
        }
        .listStyle(.insetGrouped)
        .navigationTitle(label(tab))
    }

    private func itemDetail(_ item: ShelfItem) -> some View {
        Form {
            Section {
                LabeledContent("ID", value: "\(item.giftId)")
                if item.kind == "food" || item.kind == "wallpaper" {
                    LabeledContent(model.t("price_prefix"), value: "\(item.price) G")
                }
            } footer: {
                Text(sendLine(item.kind))
            }
            Section {
                Button {
                    model.notice = model.engine.shelfSend(item.path)
                } label: {
                    Text(model.t("send_action"))
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .listRowBackground(Color.clear)
            }
        }
        .navigationTitle(item.name)
        .navigationBarTitleDisplayMode(.inline)
    }

    private var marriage: some View {
        Form {
            Section {
                partLink(0)
                partLink(1)
                partLink(2)
                partLink(3)
                NavigationLink { moreOptions } label: {
                    Label(model.t("more_options"), systemImage: "slider.horizontal.3")
                }
            }
            Section {
                Picker(model.t("send_method"), selection: $mode) {
                    Text(model.t("send_marry")).tag(0)
                    Text(model.t("send_obtain")).tag(1)
                    Text(model.t("send_random")).tag(2)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                Text(modeLine)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            } header: {
                Text(model.t("send_method"))
            } footer: {
                Text(model.t("marry_gate"))
            }
            Section {
                Button(action: marry) {
                    Text(model.t("send_action") + " · " + modeName)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .listRowBackground(Color.clear)
            }
        }
    }

    private func partLink(_ index: Int) -> some View {
        let titles = [model.t("part_body"), model.t("part_face"), model.t("part_head"), model.t("part_extra")]
        return NavigationLink { partList(index) } label: {
            LabeledContent(titles[index], value: "\(parts[index])")
        }
    }

    private func partList(_ index: Int) -> some View {
        List {
            ForEach(0..<partLimits[index], id: \.self) { value in
                Button {
                    parts[index] = value
                    saveCharacter()
                } label: {
                    HStack {
                        Text("\(value)")
                        Spacer()
                        if parts[index] == value { Image(systemName: "checkmark") }
                    }
                }
            }
        }
        .navigationTitle([model.t("part_body"), model.t("part_face"), model.t("part_head"), model.t("part_extra")][index])
    }

    private var moreOptions: some View {
        Form {
            Section(model.t("sex_child")) {
                sexRow(model.t("sex_boy"), 0)
                sexRow(model.t("sex_girl"), 1)
                sexRow(model.t("sex_random"), 2)
            }
            Section(model.t("parts_colors")) {
                Button(model.t("random_character")) { randomParts() }
                Button(model.t("random_colors")) { randomColors() }
                ForEach(0..<3, id: \.self) { index in
                    NavigationLink { colorList(index) } label: {
                        Text("\(colorTitle(index))  \(model.t("color_\(colors[index])"))")
                    }
                }
            }
            Section(model.t("shelf_lang")) {
                languageRow("日本語", "jp")
                languageRow("English", "en")
                Text(model.t("shelf_lang_note")).font(.footnote)
            }
        }
        .navigationTitle(model.t("character_options"))
    }

    private func colorList(_ index: Int) -> some View {
        List {
            ForEach(0..<13, id: \.self) { value in
                Button {
                    colors[index] = value
                    saveCharacter()
                } label: {
                    HStack {
                        Text(model.t("color_\(value)")).foregroundStyle(.primary)
                        Spacer()
                        if colors[index] == value { Image(systemName: "checkmark") }
                    }
                }
            }
        }
        .navigationTitle(model.t("choose_color"))
    }

    private var imports: some View {
        Form {
            Section {
                Button { customKind = 2; pickingCustom = true } label: {
                    Label(model.t("import_resource"), systemImage: "doc")
                }
                Button { customKind = 0; pickingCustom = true } label: {
                    Label(model.t("import_wallpaper"), systemImage: "photo")
                }
                Button { customKind = 1; pickingCustom = true } label: {
                    Label(model.t("import_character"), systemImage: "person")
                }
            } header: {
                Text(model.t("import_collection"))
            } footer: {
                Text(model.t("import_character_note") + "\n" + model.t("shelf_layer_empty"))
            }
            Section {
                Button {
                    model.notice = model.engine.shelfUndo()
                } label: {
                    Label(model.t("restore_shelf"), systemImage: "arrow.uturn.backward")
                }
            }
        }
    }

    private var bottomBar: some View {
        PMGlassGroup(spacing: 0) {
            HStack(spacing: 0) {
                barButton(model.t("shelf_money"), "banknote") { showMoney = true }
                barButton(languageCaption, "character.book.closed") { toggleLanguage() }
                barButton(model.t("shelf_pack_row"), "shippingbox") { showPack = true }
            }
            .padding(.horizontal, 4)
            .padding(.vertical, 2)
            .pmGlass(Capsule(style: .continuous))
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 8)
    }

    private func barButton(_ title: String, _ symbol: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 3) {
                Image(systemName: symbol)
                    .font(.system(size: 19, weight: .medium))
                Text(title)
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .lineLimit(2)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 9)
            .foregroundStyle(.primary)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var moneySheet: some View {
        NavigationStack {
            Form {
                TextField("0–99999", text: $balanceText).keyboardType(.numberPad)
                Text(model.t("money_hint")).font(.footnote)
                Button(model.t("money_add")) { applyMoney(add: true) }
                Button(model.t("money_set")) { applyMoney(add: false) }
            }
            .navigationTitle(model.t("shelf_money"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button { showMoney = false } label: { Image(systemName: "checkmark").font(.system(size: 17, weight: .semibold)) }
                .accessibilityLabel(model.t("done")) } }
        }
    }

    private var languageCaption: String {
        let name = model.prefs.shelf_language == "en" ? "English" : "日本語"
        let title = model.prefs.language == "en" ? "Language" : "素材语言"
        return title + "\n" + name
    }

    private var modeName: String {
        [model.t("send_marry"), model.t("send_obtain"), model.t("send_random")][min(max(mode, 0), 2)]
    }

    private var modeLine: String {
        [model.t("send_marry_body"), model.t("send_obtain_body"), model.t("send_random_body")][min(max(mode, 0), 2)]
    }

    private func symbol(_ tab: String) -> String {
        switch tab {
        case "wallpaper": return "photo"
        case "food": return "fork.knife"
        case "item": return "cube.box"
        case "accessory": return "eyeglasses"
        case "game": return "gamecontroller"
        case "station": return "tram"
        default: return "square.grid.2x2"
        }
    }

    private func label(_ tab: String) -> String {
        switch tab {
        case "wallpaper": return model.t("tab_wallpaper")
        case "food": return model.t("tab_food")
        case "item": return model.t("tab_item")
        case "accessory": return model.t("tab_accessory")
        case "game": return model.t("tab_game")
        case "station": return model.t("tab_station")
        default: return tab
        }
    }

    private func sendLine(_ kind: String) -> String {
        switch kind {
        case "food": return model.t("send_food")
        case "wallpaper": return model.t("send_room")
        case "game", "station": return model.t("send_download")
        default: return model.t("send_wear")
        }
    }

    private func colorTitle(_ index: Int) -> String {
        [model.t("color_body"), model.t("color_head"), model.t("color_extra")][index]
    }

    private func sexRow(_ title: String, _ value: Int) -> some View {
        Button {
            sex = value
            saveCharacter()
        } label: {
            HStack {
                Text(title).foregroundStyle(.primary)
                Spacer()
                if sex == value { Image(systemName: "checkmark") }
            }
        }
    }

    private func languageRow(_ title: String, _ code: String) -> some View {
        Button { setShelfLanguage(code) } label: {
            HStack {
                Text(title).foregroundStyle(.primary)
                Spacer()
                if model.prefs.shelf_language == code { Image(systemName: "checkmark") }
            }
        }
    }

    private func toggleLanguage() {
        setShelfLanguage(model.prefs.shelf_language == "jp" ? "en" : "jp")
    }

    private func setShelfLanguage(_ code: String) {
        var next = model.prefs
        next.shelf_language = code
        _ = model.engine.write(next)
        model.reload()
    }

    private func randomParts() {
        for index in 0..<4 {
            parts[index] = Int.random(in: 0..<partLimits[index])
        }
        saveCharacter()
    }

    private func randomColors() {
        for index in 0..<3 { colors[index] = Int.random(in: 0...12) }
        saveCharacter()
    }

    private func applyMoney(add: Bool) {
        let amount = Int(balanceText) ?? -1
        model.notice = model.engine.shelfBalance(add: add, amount: amount)
    }

    private func marry() {
        let body: [String: Any] = [
            "parts": parts,
            "colors": colors,
            "sex": sex,
            "mode": mode,
            "dna_n": 0,
            "random_sex": 0,
            "original": false,
            "name": spouseName
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: body),
              let text = String(data: data, encoding: .utf8) else { return }
        model.notice = model.engine.shelfMarry(text)
        saveCharacter(text)
    }

    private func saveCharacter(_ text: String? = nil) {
        let stored: String
        if let text {
            stored = text
        } else {
            let body: [String: Any] = [
                "parts": parts, "colors": colors, "sex": sex, "mode": mode,
                "dna_n": 0, "random_sex": 0, "original": false, "name": spouseName
            ]
            guard let data = try? JSONSerialization.data(withJSONObject: body),
                  let encoded = String(data: data, encoding: .utf8) else { return }
            stored = encoded
        }
        var next = model.prefs
        next.shelf_character = stored
        _ = model.engine.write(next)
        model.reload()
    }

    private func restoreCharacter() {
        guard let data = model.prefs.shelf_character.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
        let storedParts = jsonInts(object["parts"])
        if storedParts.count == 4 {
            parts = zip(storedParts, partLimits).map { min(max($0.0, 0), $0.1 - 1) }
        }
        let storedColors = jsonInts(object["colors"])
        if !storedColors.isEmpty {
            colors = (0..<3).map { index in
                let value = index < storedColors.count ? storedColors[index] : 0
                return min(max(value, 0), 12)
            }
        }
        if let stored = jsonInt(object["sex"]) { sex = stored }
        if let stored = jsonInt(object["mode"]) { mode = min(max(stored, 0), 2) }
        if let stored = object["name"] as? String { spouseName = stored }
    }

    private func jsonInts(_ value: Any?) -> [Int] {
        if let stored = value as? [Int] { return stored }
        if let stored = value as? [NSNumber] { return stored.map(\.intValue) }
        if let stored = value as? [Any] {
            return stored.compactMap { jsonInt($0) }
        }
        return []
    }

    private func jsonInt(_ value: Any?) -> Int? {
        if let stored = value as? Int { return stored }
        if let stored = value as? NSNumber { return stored.intValue }
        return nil
    }

    private func close() {
        model.shelfOpen = false
        model.engine.setShelfOpen(false)
        model.page = model.engine.hasCurrent() ? .toy : .setup
    }
}
