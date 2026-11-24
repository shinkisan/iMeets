import SwiftUI
import UIKit

enum ToyToolbarAction: String, CaseIterable, Identifiable {
    case info
    case camera
    case appearance
    case pause
    case translate
    case lan
    case pip
    case shelf

    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .info: return "info.circle"
        case .camera: return "camera.fill"
        case .appearance: return "sun.max"
        case .pause: return "pause.circle"
        case .translate: return "character.bubble"
        case .lan: return "antenna.radiowaves.left.and.right"
        case .pip: return "pip"
        case .shelf: return "books.vertical"
        }
    }

    func title(_ english: Bool) -> String {
        switch self {
        case .info: return english ? "Care information" : "养育信息"
        case .camera: return english ? "Screenshot" : "截图"
        case .appearance: return english ? "Theme" : "主题切换"
        case .pause: return english ? "Pause care" : "暂停养育"
        case .translate: return english ? "Live translation" : "实时翻译"
        case .lan: return english ? "Face to Face" : "面对面"
        case .pip: return english ? "Picture in Picture" : "画中画"
        case .shelf: return english ? "Tama Shelf" : "拓麻书架"
        }
    }

    var fixed: Bool { self == .lan }

    static let defaultOrder: [ToyToolbarAction] = [.info, .appearance, .pause, .lan, .translate, .camera, .pip, .shelf]
    static let defaultHidden: Set<ToyToolbarAction> = [.camera, .pip]

    /// Actions the current look and settings can actually draw. Everything
    /// else stays listed in the editor but is greyed out instead of vanishing,
    /// so a layout never looks like it lost an item.
    static func availability(
        look: Int,
        screenshots: Bool,
        cameraReady: Bool,
        shelf: Bool
    ) -> Set<ToyToolbarAction> {
        var set: Set<ToyToolbarAction> = [.info, .pause, .lan, .pip]
        // The small-screen look has no top bar of its own to switch.
        if look != 2 { set.insert(.appearance) }
        if look == 1 || look == 2, screenshots, cameraReady { set.insert(.camera) }
        if #available(iOS 18.0, *) { set.insert(.translate) }
        if shelf { set.insert(.shelf) }
        return set
    }
}

enum ToyToolbarMetrics {
    static let buttonSize: CGFloat = 44
    static let spacing: CGFloat = 8
    static let horizontalPadding: CGFloat = 20
    static let lanDotSize: CGFloat = 7
    static let lanSpacing: CGFloat = 6
    static let lanPadding: CGFloat = 12
    static let lanFontSize: CGFloat = 13
    private static let connectedWidths = (zh: measureLANWidth(english: false), en: measureLANWidth(english: true))

    static func connectedLANWidth(english: Bool) -> CGFloat {
        english ? connectedWidths.en : connectedWidths.zh
    }

    private static func measureLANWidth(english: Bool) -> CGFloat {
        let text = ToyToolbarAction.lan.title(english)
        let font = UIFont.systemFont(ofSize: lanFontSize, weight: .semibold)
        let textWidth = (text as NSString).size(withAttributes: [.font: font]).width
        return max(buttonSize, ceil(textWidth) + lanDotSize + lanSpacing + 2 * lanPadding)
    }

    static func capacity(width: CGFloat, lanConnected: Bool, english: Bool) -> Int {
        // More always has a button, and every action adds its width and a gap.
        let lanExtra = lanConnected ? connectedLANWidth(english: english) - buttonSize : 0
        let remaining = width - 2 * horizontalPadding - buttonSize - lanExtra
        guard remaining > 0 else { return 0 }
        return Int(remaining / (buttonSize + spacing))
    }
}

/// The look that is actually drawn. "Follow system" only has two outcomes:
/// the shell look in light mode, night in dark mode. Kept here so the toy bar
/// and the settings editor agree on which actions exist.
enum ToyLook {
    static func resolve(_ prefs: Prefs, dark: Bool) -> Int {
        let style = prefs.appearance
        switch style {
        case 0, 3:
            guard prefs.shell_follow_system else { return style }
            return dark ? 3 : 0
        case 1, 4:
            guard prefs.minimal_follow_system else { return style }
            return dark ? 4 : 1
        default:
            return style
        }
    }
}

final class ToolbarConfig: ObservableObject {
    @Published private(set) var order: [ToyToolbarAction]
    @Published private var hidden: Set<ToyToolbarAction>
    @Published private var vacantSlots: Set<ToyToolbarAction>

    private let orderKey = "pm.toolbar.order"
    private let hiddenKey = "pm.toolbar.hidden"
    private let vacantSlotsKey = "pm.toolbar.vacantSlots"
    private let defaults = UserDefaults.standard

    init() {
        let savedOrder = (defaults.array(forKey: orderKey) as? [String])?
            .compactMap(ToyToolbarAction.init(rawValue:))
        let savedHidden = (defaults.array(forKey: hiddenKey) as? [String])?
            .compactMap(ToyToolbarAction.init(rawValue:))
        let savedVacancies = (defaults.stringArray(forKey: vacantSlotsKey) ?? [])
            .compactMap(ToyToolbarAction.init(rawValue:))
        var order = savedOrder ?? ToyToolbarAction.defaultOrder
        // Ship the new information button on existing toolbars once. Preserve
        // the relative order of saved actions; overflow stays in More.
        if !order.contains(.info) { order.insert(.info, at: 0) }

        // Unknown or newly shipped actions are appended in their default slot;
        // a stale saved order never drops a button.
        for action in ToyToolbarAction.defaultOrder where !order.contains(action) {
            order.append(action)
        }
        for action in ToyToolbarAction.allCases where !order.contains(action) {
            order.append(action)
        }
        var hidden = Set(savedHidden ?? Array(ToyToolbarAction.defaultHidden))
        hidden.remove(.lan)
        self.order = order
        self.hidden = hidden
        self.vacantSlots = Set(savedVacancies).intersection(hidden)
        persist()
    }

    private func candidates(available: Set<ToyToolbarAction>) -> [ToyToolbarAction] {
        order.filter { available.contains($0) && (!hidden.contains($0) || vacantSlots.contains($0)) }
    }

    private func slots(available: Set<ToyToolbarAction>, capacity: Int) -> [ToyToolbarAction] {
        guard capacity > 0 else { return [] }
        var list = Array(candidates(available: available).prefix(capacity))
        if available.contains(.lan) && !list.contains(.lan) {
            if list.count == capacity {
                list.removeLast()
            }
            list.append(.lan)
        }
        return list
    }

    // Keep a slot for a removed action until the user adds a button. A change
    // in window width only affects how many of the saved slots fit.
    func visible(available: Set<ToyToolbarAction>, capacity: Int) -> [ToyToolbarAction] {
        slots(available: available, capacity: capacity).filter { !hidden.contains($0) }
    }

    func show(_ action: ToyToolbarAction, available: Set<ToyToolbarAction>, capacity: Int) {
        let slots = slots(available: available, capacity: capacity)
        let shown = slots.filter { !hidden.contains($0) }
        guard !action.fixed, available.contains(action), !shown.contains(action), shown.count < capacity else { return }

        if let vacancy = slots.first(where: { vacantSlots.contains($0) }), !slots.contains(action),
           let empty = order.firstIndex(of: vacancy), let occupied = order.firstIndex(of: action) {
            order.swapAt(empty, occupied)
            vacantSlots.remove(vacancy)
        }
        vacantSlots.remove(action)
        hidden.remove(action)
        persist()
    }

    func hide(_ action: ToyToolbarAction) {
        guard !action.fixed, !hidden.contains(action) else { return }
        hidden.insert(action)
        vacantSlots.insert(action)
        persist()
    }

    func moveBar(from: IndexSet, to: Int, available: Set<ToyToolbarAction>, capacity: Int) {
        let slots = slots(available: available, capacity: capacity)
        var list = slots.filter { !hidden.contains($0) }
        list.move(fromOffsets: from, toOffset: to)
        let moved = Set(list)
        var iterator = list.makeIterator()
        var next = order

        // Give a pinned LAN button its actual slot before rearranging the bar.
        if slots.last == .lan, let overflow = candidates(available: available).dropFirst(capacity - 1).first,
           overflow != .lan {
            next.removeAll { $0 == .lan }
            if let index = next.firstIndex(of: overflow) { next.insert(.lan, at: index) }
        }
        for index in next.indices where moved.contains(next[index]) {
            if let action = iterator.next() { next[index] = action }
        }
        order = next
        persist()
    }

    func reset() {
        order = ToyToolbarAction.defaultOrder
        hidden = ToyToolbarAction.defaultHidden
        vacantSlots = []
        persist()
    }

    private func persist() {
        defaults.set(order.map(\.rawValue), forKey: orderKey)
        defaults.set(hidden.map(\.rawValue), forKey: hiddenKey)
        defaults.set(vacantSlots.map(\.rawValue), forKey: vacantSlotsKey)
    }
}

private struct ToolbarRowFramesKey: PreferenceKey {
    static var defaultValue: [ToyToolbarAction: CGRect] = [:]
    static func reduce(
        value: inout [ToyToolbarAction: CGRect],
        nextValue: () -> [ToyToolbarAction: CGRect]
    ) {
        value.merge(nextValue(), uniquingKeysWith: { $1 })
    }
}

struct ToolbarEditorView: View {
    @ObservedObject var config: ToolbarConfig
    let available: Set<ToyToolbarAction>
    let capacity: Int
    let lanConnected: Bool
    let english: Bool

    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    @State private var dragging: ToyToolbarAction?
    @State private var dragOffset: CGFloat = 0
    /// Distance between two row centres; measured so the drag follows the
    /// finger at any Dynamic Type size.
    @State private var rowPitch: CGFloat = 66

    /// The editor is a sheet, so it follows the system appearance rather than
    /// the toy's night look. Its chrome must match the sheet, not the bar, or
    /// the night palette ends up pale lavender on a white sheet.
    private var chromeColor: Color {
        colorScheme == .dark ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83)
    }

    private var bar: [ToyToolbarAction] { config.visible(available: available, capacity: capacity) }
    /// Everything that is not on the bar: hidden actions, overflow and
    /// actions the current look cannot draw.
    private var rest: [ToyToolbarAction] {
        let onBar = Set(bar)
        return config.order.filter { !onBar.contains($0) }
    }
    private var barIsFull: Bool { bar.count >= capacity }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                preview
                Divider()
                    .padding(.horizontal, 20)
                List {
                    Section {
                        ForEach(bar) { action in
                            row(action)
                        }
                    }

                    if !rest.isEmpty {
                        Section {
                            ForEach(rest) { action in
                                row(action)
                            }
                        }
                    }

                    Section {
                        Button(role: .destructive) {
                            withAnimation(.spring(response: 0.3, dampingFraction: 0.9)) {
                                config.reset()
                            }
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "arrow.counterclockwise")
                                Text(english ? "Reset to Default" : "恢复默认")
                                Spacer(minLength: 0)
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.borderless)
                    }
                }
                .coordinateSpace(name: "toolbarList")
                .onPreferenceChange(ToolbarRowFramesKey.self) { frames in
                    let ys = frames.values.map(\.minY).sorted()
                    guard ys.count > 1 else { return }
                    let pitch = (ys[ys.count - 1] - ys[0]) / CGFloat(ys.count - 1)
                    if pitch > 1, abs(pitch - rowPitch) > 0.5 { rowPitch = pitch }
                }
            }
            .navigationTitle(english ? "Edit Toolbar" : "编辑顶栏")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button {
                        dismiss()
                    } label: {
                        Image(systemName: "chevron.backward")
                            .font(.system(size: 17, weight: .semibold))
                    }
                    .accessibilityLabel(english ? "Close" : "关闭")
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(english ? "Done" : "完成") { dismiss() }
                }
            }
        }
    }

    private var preview: some View {
        VStack(spacing: 8) {
            ViewThatFits(in: .horizontal) {
                previewButtons
                ScrollView(.horizontal, showsIndicators: false) {
                    previewButtons
                }
            }
            .frame(maxWidth: .infinity)
            .animation(.spring(response: 0.3, dampingFraction: 0.9), value: bar)

            Text(english ? "Preview" : "预览")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 14)
        .frame(maxWidth: .infinity)
        .background(Color(uiColor: .systemGroupedBackground))
    }

    private var previewButtons: some View {
        HStack(spacing: ToyToolbarMetrics.spacing) {
            ForEach(bar) { action in
                if action == .lan && lanConnected {
                    HStack(spacing: 5) {
                        Circle().fill(Color.green).frame(width: 6, height: 6)
                        Text(action.title(english))
                            .font(.system(size: 12, weight: .semibold))
                    }
                    .foregroundStyle(chromeColor)
                    .padding(.horizontal, 10)
                    .frame(height: 40)
                    .pmGlass(Capsule(), interactive: false)
                } else {
                    previewIcon(action.symbol)
                }
            }
            previewIcon("ellipsis")
        }
        .fixedSize(horizontal: true, vertical: false)
    }

    private func previewIcon(_ symbol: String) -> some View {
        Image(systemName: symbol)
            .font(.system(size: 16, weight: .semibold))
            .foregroundStyle(chromeColor)
            .frame(width: 40, height: 40)
            .pmGlass(Circle(), interactive: false)
    }

    private func row(_ action: ToyToolbarAction) -> some View {
        let onBar = bar.contains(action)
        let isAvailable = available.contains(action)
        let mustStay = action.fixed && isAvailable
        return HStack(spacing: 12) {
            // The drag handle sits at the trailing edge, so the add/remove
            // control goes first to keep both reachable in edit mode.
            accessory(action, onBar: onBar, isAvailable: isAvailable, mustStay: mustStay)
            Image(systemName: action.symbol)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(chromeColor)
                .frame(width: 24)
            VStack(alignment: .leading, spacing: 2) {
                Text(action.title(english))
                if !isAvailable {
                    Text(english ? "Unavailable here" : "当前不可用")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            Spacer(minLength: 8)
        }
        .opacity(isAvailable ? 1 : 0.55)
        // The row follows the finger; the order is committed on release so
        // the gesture survives a multi-row drag.
        .offset(y: dragging == action ? dragOffset : 0)
        // An overlay keeps the 44 pt grab area without making rows on the bar
        // taller than the rows in the card below.
        .overlay(alignment: .trailing) {
            if onBar {
                reorderHandle(action)
            }
        }
        .background(
            GeometryReader { geo in
                Color.clear.preference(
                    key: ToolbarRowFramesKey.self,
                    value: [action: geo.frame(in: .named("toolbarList"))]
                )
            }
        )
    }

    /// Our own handle, so a row can never be added without one. List's
    /// edit-mode handle is only attached to rows that exist when the section
    /// is built, which left freshly added buttons undraggable on iOS 27.
    private func reorderHandle(_ action: ToyToolbarAction) -> some View {
        Image(systemName: "line.3.horizontal")
            .font(.system(size: 15, weight: .semibold))
            .foregroundStyle(.tertiary)
            .frame(width: 44, height: 44)
            .contentShape(Rectangle())
            .highPriorityGesture(reorderGesture(action))
            .accessibilityLabel(english ? "Reorder" : "调整顺序")
            .accessibilityAction(named: Text(english ? "Move up" : "上移")) {
                move(action, by: -1)
            }
            .accessibilityAction(named: Text(english ? "Move down" : "下移")) {
                move(action, by: 1)
            }
    }

    private func move(_ action: ToyToolbarAction, by delta: Int) {
        guard let from = bar.firstIndex(of: action) else { return }
        let to = min(max(from + delta, 0), bar.count - 1)
        guard to != from else { return }
        withAnimation(.easeInOut(duration: 0.16)) {
            config.moveBar(
                from: IndexSet(integer: from),
                to: to > from ? to + 1 : to,
                available: available,
                capacity: capacity
            )
        }
    }

    private func reorderGesture(_ action: ToyToolbarAction) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                dragging = action
                dragOffset = value.translation.height
            }
            .onEnded { value in
                let steps = Int((value.translation.height / max(rowPitch, 1)).rounded())
                dragging = nil
                dragOffset = 0
                guard let from = bar.firstIndex(of: action) else { return }
                let to = min(max(from + steps, 0), bar.count - 1)
                guard to != from else { return }
                withAnimation(.easeInOut(duration: 0.18)) {
                    config.moveBar(
                        from: IndexSet(integer: from),
                        to: to > from ? to + 1 : to,
                        available: available,
                        capacity: capacity
                    )
                }
            }
    }

    @ViewBuilder
    private func accessory(
        _ action: ToyToolbarAction,
        onBar: Bool,
        isAvailable: Bool,
        mustStay: Bool
    ) -> some View {
        if mustStay {
            Image(systemName: "lock.fill")
                .font(.system(size: 15))
                .foregroundStyle(.tertiary)
                .frame(width: 24, height: 30)
                .accessibilityLabel(english ? "Always shown" : "始终显示")
        } else if onBar {
            Button {
                withAnimation(.spring(response: 0.3, dampingFraction: 0.9)) {
                    config.hide(action)
                }
            } label: {
                Image(systemName: "minus.circle.fill")
                    .font(.system(size: 20))
                    .foregroundStyle(Color.pmAccent)
                    .frame(width: 24, height: 30)
            }
            .buttonStyle(.borderless)
            .accessibilityLabel(english ? "Move to More" : "移到更多")
        } else {
            Button {
                withAnimation(.spring(response: 0.3, dampingFraction: 0.9)) {
                    config.show(action, available: available, capacity: capacity)
                }
            } label: {
                Image(systemName: "plus.circle.fill")
                    .font(.system(size: 20))
                    .foregroundStyle(isAvailable && !barIsFull ? Color.pmAccent : Color.secondary.opacity(0.5))
                    .frame(width: 24, height: 30)
            }
            .buttonStyle(.borderless)
            .disabled(!isAvailable || barIsFull)
            .accessibilityLabel(english ? "Add to toolbar" : "加入顶栏")
        }
    }
}
