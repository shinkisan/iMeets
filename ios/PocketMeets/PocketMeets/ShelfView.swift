import ImageIO
import SwiftUI
import UniformTypeIdentifiers

private enum ShelfDestination: Hashable {
    case category(String)
    case item(ShelfItem)
    case part(Int)
}

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
    @State private var materialParts: [[ShelfPart]] = []
    @State private var materialSelection: [Int] = []
    @State private var colors = [0, 0, 0]
    @State private var sex = 2
    @State private var mode = 0
    @State private var spouseName = ""

    private let categories = ["wallpaper", "food", "item", "accessory", "game", "station"]
    private let partLimits = [14, 19, 11, 7]
    private let partTitles = ["part_body", "part_face", "part_head", "part_extra"]
    private let partIcons = ["figure.stand", "face.smiling", "crown", "sparkles"]

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
        .navigationDestination(for: ShelfDestination.self) { destination in
            switch destination {
            case .category(let tab):
                ShelfCategoryView(tab: tab, title: label(tab), icon: symbol(tab))
            case .item(let item):
                itemDetail(item)
            case .part(let index):
                partList(index)
            }
        }
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button { close() } label: {
                    Image(systemName: "chevron.backward")
                        .font(.system(size: 17, weight: .semibold))
                }
                .accessibilityLabel(model.t("back"))
            }
        }
        .pmBottomBar {
            if section == 0 { bottomBar }
        }
        .onAppear {
            model.shelfOpen = true
            model.engine.setShelfOpen(true)
            restoreCharacter()
        }
        .task(id: model.prefs.shelf_pack) { loadMaterialParts() }
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
        let picker = Picker(model.t("shelf_title"), selection: $section) {
            Text(model.t("tab_item")).tag(0)
            Text(model.t("shelf_marriage")).tag(1)
            Text(model.t("shelf_import")).tag(2)
        }
        return Group {
            if #available(iOS 27.0, *) {
                picker.pickerStyle(.tabs)
            } else {
                picker.pickerStyle(.segmented)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    private var categoriesHome: some View {
        Group {
            if model.prefs.shelf_pack.isEmpty {
                List {
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
                }
                .listStyle(.insetGrouped)
            } else {
                ShelfGrid {
                    ForEach(categories, id: \.self) { tab in
                        let title = label(tab)
                        let icon = symbol(tab)
                        NavigationLink(value: ShelfDestination.category(tab)) {
                            ShelfTile(title: title) {
                                Image(systemName: icon)
                                    .font(.system(size: 34))
                                    .foregroundStyle(Color.pmAccent)
                                    .accessibilityHidden(true)
                            }
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    private func itemDetail(_ item: ShelfItem) -> some View {
        Form {
            Section {
                ShelfArtwork(url: item.thumbnailURL, icon: symbol(item.kind), unavailable: model.t("shelf_no_preview"))
                    .padding(16)
                    .frame(maxWidth: .infinity)
                    .frame(height: 224)
                LabeledContent(model.t("shelf_category"), value: label(item.kind))
                LabeledContent("ID", value: "\(item.giftId)")
                if item.showsPrice {
                    LabeledContent(model.t("shelf_price"), value: item.priceLabel)
                }
            } footer: {
                Text(sendLine(item.kind))
            }
        }
        .pmBottomBar {
            sendBar {
                model.notice = model.engine.shelfSend(item.path)
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
            } footer: {
                if materialSelection.isEmpty {
                    Text(model.t("shelf_parts_missing"))
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
        }
        .pmBottomBar { sendBar(action: marry) }
    }

    private func sendBar(action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(model.t("send_action"))
                .frame(maxWidth: .infinity)
        }
        .pmProminent()
        .controlSize(.large)
        .frame(maxWidth: 480)
        .padding(.horizontal, 20)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity)
    }

    private func partLink(_ index: Int) -> some View {
        NavigationLink(value: ShelfDestination.part(index)) {
            if let part = selectedMaterialPart(index) {
                LabeledContent(model.t(partTitles[index])) {
                    HStack(spacing: 8) {
                        ShelfArtwork(url: part.thumbnailURL, icon: partIcons[index], emptyPart: .icon)
                            .frame(width: 40, height: 40)
                            .accessibilityHidden(true)
                        Text(verbatim: "\(part.id)")
                    }
                }
            } else {
                LabeledContent(model.t(partTitles[index]), value: "\(parts[index])")
            }
        }
    }

    private func partList(_ index: Int) -> some View {
        Group {
            if materialSelection.indices.contains(index) {
                let partLabel = model.t("shelf_part_number")
                let emptyLabel = model.t("shelf_part_empty")
                ShelfGrid {
                    ForEach(materialParts[index]) { part in
                        let selected = part.id == materialSelection[index]
                        Button {
                            materialSelection[index] = part.id
                            saveCharacter()
                        } label: {
                            ShelfTile(title: partLabel + String(part.id)) {
                                ShelfArtwork(url: part.thumbnailURL, icon: partIcons[index], emptyPart: .labeled(emptyLabel))
                                    .padding(8)
                                    .allowsHitTesting(false)
                            }
                            .overlay(alignment: .topTrailing) {
                                if selected {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundStyle(Color.pmAccent)
                                        .padding(8)
                                        .allowsHitTesting(false)
                                        .accessibilityHidden(true)
                                }
                            }
                        }
                        .buttonStyle(.plain)
                        .accessibilityAddTraits(selected ? .isSelected : [])
                    }
                }
            } else {
                List {
                    ForEach(0..<partLimits[index], id: \.self) { value in
                        Button {
                            parts[index] = value
                            saveCharacter()
                        } label: {
                            HStack {
                                Text(verbatim: "\(value)")
                                Spacer()
                                if parts[index] == value { Image(systemName: "checkmark") }
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(model.t(partTitles[index]))
        .navigationBarTitleDisplayMode(.inline)
    }

    private func selectedMaterialPart(_ index: Int) -> ShelfPart? {
        guard materialSelection.indices.contains(index) else { return nil }
        return materialParts[index].first { $0.id == materialSelection[index] }
    }

    private func loadMaterialParts() {
        let groups = model.engine.shelfParts()
        guard groups.count == partTitles.count, groups.allSatisfy({ !$0.isEmpty }) else {
            materialSelection = []
            materialParts = []
            return
        }
        let saved = CString.object(model.prefs.shelf_character)
        let ids = jsonInts(saved["material_parts"])
        materialParts = groups
        materialSelection = groups.enumerated().compactMap { index, group in
            if ids.indices.contains(index), group.contains(where: { $0.id == ids[index] }) {
                return ids[index]
            }
            return group.first?.id
        }
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
                Text(model.t("import_character_note"))
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
        if materialSelection.isEmpty {
            parts = partLimits.map { Int.random(in: 0..<$0) }
        } else {
            materialSelection = materialParts.compactMap { $0.randomElement()?.id }
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
        guard let text = characterJSON(forSending: true) else { return }
        model.notice = model.engine.shelfMarry(text)
        saveCharacter()
    }

    private func characterJSON(forSending: Bool = false) -> String? {
        var body: [String: Any] = [
            "parts": parts,
            "colors": colors,
            "sex": sex,
            "mode": mode,
            "original": false,
            "name": spouseName
        ]
        if !materialSelection.isEmpty {
            body["material_parts"] = materialSelection
            body["material_pack"] = model.prefs.shelf_pack
            if forSending {
                body["dna_n"] = Int.random(in: 0..<100)
                body["random_sex"] = Int.random(in: 0..<2)
                body["random_dna"] = (0..<8).map { _ in Int.random(in: 0..<12) }
            }
        }
        guard let data = try? JSONSerialization.data(withJSONObject: body) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private func saveCharacter() {
        guard let stored = characterJSON() else { return }
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

private struct ShelfGrid<Content: View>: View {
    @Environment(\.dynamicTypeSize) private var typeSize
    @ViewBuilder var content: Content

    var body: some View {
        let minimumWidth: CGFloat = typeSize.isAccessibilitySize ? 280 : 148
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: minimumWidth), spacing: 12, alignment: .top)],
                      spacing: 12) {
                content
            }
            .padding(16)
        }
        .background(Color(uiColor: .systemGroupedBackground))
    }
}

private struct ShelfTile<Artwork: View>: View {
    let title: String
    var subtitle: String? = nil
    @ViewBuilder var artwork: Artwork

    var body: some View {
        VStack(spacing: 8) {
            artwork
                .frame(maxWidth: .infinity)
                .frame(height: 88)
                .background(.quaternary.opacity(0.45), in: RoundedRectangle(cornerRadius: 10))
            Text(title)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.primary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            if let subtitle {
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity)
        .background(Color(uiColor: .secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 16))
        .contentShape(RoundedRectangle(cornerRadius: 16))
    }
}

private struct ShelfCategoryView: View {
    @EnvironmentObject private var model: AppModel
    @State private var items: [ShelfItem] = []
    @State private var status: String?

    let tab: String
    let title: String
    let icon: String

    var body: some View {
        Group {
            if let status {
                Text(status)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(24)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if items.isEmpty {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ShelfGrid {
                    ForEach(items) { item in
                        NavigationLink(value: ShelfDestination.item(item)) {
                            ShelfTile(title: item.name, subtitle: item.showsPrice ? item.priceLabel : nil) {
                                ShelfArtwork(url: item.thumbnailURL, icon: icon)
                                    .padding(8)
                                    .allowsHitTesting(false)
                                    .accessibilityHidden(true)
                            }
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
        .background(Color(uiColor: .systemGroupedBackground))
        .navigationTitle(title)
        .navigationBarTitleDisplayMode(.inline)
        .task(id: [model.prefs.shelf_pack, model.prefs.shelf_language]) {
            let listed = model.engine.shelfList(tab: tab, language: model.prefs.shelf_language)
            items = listed.items
            status = nil
            if !listed.ok {
                status = listed.error
            } else if listed.items.isEmpty {
                status = model.t("shelf_layer_empty")
            }
        }
    }
}

private struct ShelfArtwork: View {
    enum EmptyPart {
        case icon
        case labeled(String)
    }

    private enum Phase {
        case loading
        case missing
        case transparent
        case image(CGImage)
    }

    let url: URL?
    let icon: String
    var unavailable: String? = nil
    var emptyPart: EmptyPart? = nil
    @State private var phase = Phase.loading

    var body: some View {
        ZStack {
            switch phase {
            case .image(let image):
                Image(decorative: image, scale: 1)
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
            case .transparent:
                VStack(spacing: 4) {
                    Image(systemName: "circle.slash")
                        .font(.system(size: 22, weight: .medium))
                        .accessibilityHidden(true)
                    if case .labeled(let text) = emptyPart {
                        Text(text)
                            .font(.caption)
                            .multilineTextAlignment(.center)
                    }
                }
                .foregroundStyle(.secondary)
            case .loading, .missing:
                VStack(spacing: 8) {
                    Image(systemName: icon)
                        .font(.system(size: 28))
                        .accessibilityHidden(true)
                    if case .missing = phase, let unavailable {
                        Text(unavailable)
                            .font(.footnote)
                            .multilineTextAlignment(.center)
                    }
                }
                .foregroundStyle(.secondary)
            }
        }
        .task(id: url) {
            guard let url else {
                phase = .missing
                return
            }
            phase = .loading
            let detectTransparency = emptyPart != nil
            let decoded = await Task.detached(priority: .utility) { () -> Phase in
                let options: [CFString: Any] = [
                    kCGImageSourceCreateThumbnailFromImageAlways: true,
                    kCGImageSourceCreateThumbnailWithTransform: true,
                    kCGImageSourceThumbnailMaxPixelSize: 672,
                    kCGImageSourceShouldCacheImmediately: true
                ]
                guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
                      let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
                else { return .missing }
                if detectTransparency, Self.isTransparent(image) { return .transparent }
                return .image(image)
            }.value
            guard !Task.isCancelled else { return }
            phase = decoded
        }
    }

    private nonisolated static func isTransparent(_ image: CGImage) -> Bool {
        let bytesPerRow = (image.width * 4 + 15) & ~15
        var pixels = [UInt8](repeating: 0, count: bytesPerRow * image.height)
        return pixels.withUnsafeMutableBytes { bytes in
            guard let context = CGContext(
                data: bytes.baseAddress, width: image.width, height: image.height,
                bitsPerComponent: 8, bytesPerRow: bytesPerRow, space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
            ) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
            return stride(from: 3, to: bytes.count, by: 4).allSatisfy { bytes[$0] == 0 }
        }
    }
}
