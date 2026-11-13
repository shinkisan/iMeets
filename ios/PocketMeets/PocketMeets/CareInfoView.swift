import SwiftUI

struct CareInfoSnapshot: Decodable {
    let status: String
    let paused: Bool
    let data: Fields?

    struct Fields: Decodable {
        let age: Int
        let money: Int
        let growth: Growth
        let careMisses: Int
        let hungerIgnoreMinutes: Int?
        let moodIgnoreMinutes: Int?
        let fullness: Int
        let fullnessDropMinutes: Int?
        let mood: Int
        let moodMax: Int
        let moodDropMinutes: Int?
        let pottySeconds: Int
    }

    struct Growth: Decodable {
        let state: String
        let remainingMinutes: Int?
        let elapsedMinutes: Int?
        let requiredMinutes: Int?
    }

    static func decode(_ json: String) -> CareInfoSnapshot? {
        guard let data = json.data(using: .utf8) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(Self.self, from: data)
    }
}

struct CareInfoView: View {
    let engine: Engine
    let english: Bool
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var scenePhase
    @State private var snapshot: CareInfoSnapshot?
    @State private var readFailed = false

    private func text(_ zh: String, _ en: String) -> String { english ? en : zh }

    var body: some View {
        NavigationStack {
            List {
                if readFailed {
                    message(
                        "暂时无法读取养育信息，请稍后重试。",
                        "Care information could not be read. Please try again shortly."
                    )
                } else if let snapshot {
                    if snapshot.status == "ready", let data = snapshot.data {
                        fields(data, paused: snapshot.paused)
                    } else {
                        unavailable(snapshot.status)
                    }
                } else {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .navigationTitle(text("养育信息", "Care information"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button {
                        dismiss()
                    } label: {
                        Image(systemName: "checkmark")
                            .font(.system(size: 17, weight: .semibold))
                    }
                    .accessibilityLabel(text("完成", "Done"))
                }
            }
        }
        .task(id: scenePhase) {
            // SwiftUI cancels the task on dismissal. No background polling.
            guard scenePhase == .active else { return }
            while !Task.isCancelled {
                snapshot = engine.careInfo()
                readFailed = snapshot == nil
                do { try await Task.sleep(nanoseconds: 1_000_000_000) }
                catch { return }
            }
        }
    }

    @ViewBuilder
    private func fields(_ data: CareInfoSnapshot.Fields, paused: Bool) -> some View {
        if paused {
            Section {
                pmLabel(text("养育已暂停 · 计时已冻结", "Care paused · timers frozen"), systemImage: "pause.circle.fill")
            }
        }

        alertsSection(data)

        Section(text("状态与消耗", "Needs & care")) {
            fullnessRow(data)
            moodRow(data)
            valueRow(text("养育失误", "Care mistakes"), systemImage: "exclamationmark.bubble", value: text("\(data.careMisses) 次", "\(data.careMisses)"))
        }

        Section(text("基本信息", "Profile")) {
            growthRow(data.growth)
            valueRow(text("年龄", "Age"), systemImage: "calendar", value: text("\(data.age) 岁", "\(data.age) yr"))
            valueRow(text("金钱", "Money"), systemImage: "banknote", value: "\(data.money) G")
        }
    }

    @ViewBuilder
    private func alertsSection(_ data: CareInfoSnapshot.Fields) -> some View {
        let hasUrgent = data.pottySeconds > 0
            || data.hungerIgnoreMinutes != nil
            || data.moodIgnoreMinutes != nil

        if hasUrgent {
            Section {
                if data.pottySeconds > 0 {
                    alertRow(
                        icon: "toilet.fill",
                        title: text("快去厕所", "Toilet needed"),
                        countdown: text("还剩 \(data.pottySeconds) 秒", "\(data.pottySeconds)s left")
                    )
                }
                if let minutes = data.hungerIgnoreMinutes {
                    alertRow(
                        icon: "fork.knife",
                        title: text("饥饿呼叫中", "Hunger call"),
                        countdown: text("\(minutes) 分钟后记失误", "Mistake in \(minutes)m")
                    )
                }
                if let minutes = data.moodIgnoreMinutes {
                    alertRow(
                        icon: "face.smiling",
                        title: text("心情呼叫中", "Mood call"),
                        countdown: text("\(minutes) 分钟后记失误", "Mistake in \(minutes)m")
                    )
                }
            } header: {
                Text(text("现在需要处理", "Needs attention"))
            }
        } else {
            Section {
                pmLabel(text("当前无呼叫，暂无便意", "No active calls · no toilet urge"), systemImage: "checkmark.circle.fill")
            }
        }
    }

    private func alertRow(
        icon: String,
        title: String,
        countdown: String
    ) -> some View {
        HStack {
            pmLabel(title, systemImage: icon)
            Spacer(minLength: 8)
            Text(countdown)
                .font(.body)
                .foregroundStyle(.primary)
                .monospacedDigit()
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }

    private func fullnessRow(_ data: CareInfoSnapshot.Fields) -> some View {
        let clamped = max(0, min(6, data.fullness))

        return VStack(alignment: .leading, spacing: 8) {
            HStack {
                pmLabel(text("饱食度", "Fullness"), systemImage: "fork.knife")
                Spacer(minLength: 8)
                Text(fullnessValueText(clamped, dropMinutes: data.fullnessDropMinutes))
                    .font(.body)
                    .foregroundStyle(.primary)
                    .monospacedDigit()
            }

            HStack(spacing: 6) {
                ForEach(0..<6, id: \.self) { index in
                    Capsule()
                        .fill(index < clamped ? Color.pmAccent : Color.secondary.opacity(0.2))
                        .frame(height: 8)
                }
            }
            .accessibilityHidden(true)
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }

    private func fullnessValueText(_ fullness: Int, dropMinutes: Int?) -> String {
        if fullness == 0 {
            return text("0/6（已空腹）", "0/6 (empty)")
        }
        if let minutes = dropMinutes {
            return text("\(fullness)/6（\(minutes)分钟后下降）", "\(fullness)/6 (drops in \(minutes)m)")
        }
        return "\(fullness)/6"
    }

    private func moodRow(_ data: CareInfoSnapshot.Fields) -> some View {
        let maxMood = max(1, data.moodMax)
        let clamped = max(0, min(maxMood, data.mood))

        return VStack(alignment: .leading, spacing: 8) {
            HStack {
                pmLabel(text("心情", "Mood"), systemImage: "face.smiling")
                Spacer(minLength: 8)
                Text(moodValueText(data.mood, maxMood: data.moodMax, dropMinutes: data.moodDropMinutes))
                    .font(.body)
                    .foregroundStyle(.primary)
                    .monospacedDigit()
            }

            ProgressView(value: Double(clamped), total: Double(maxMood))
                .tint(Color.pmAccent)
                .accessibilityHidden(true)
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }

    private func moodValueText(_ mood: Int, maxMood: Int, dropMinutes: Int?) -> String {
        if mood == 0 {
            return "\(mood)/\(maxMood)"
        }
        if let minutes = dropMinutes {
            return text("\(mood)/\(maxMood)（\(minutes)分钟后下降）", "\(mood)/\(maxMood) (drops in \(minutes)m)")
        }
        return "\(mood)/\(maxMood)"
    }

    private func valueRow(_ title: String, systemImage: String, value: String) -> some View {
        HStack {
            pmLabel(title, systemImage: systemImage)
            Spacer()
            Text(value)
                .font(.body)
                .foregroundStyle(.primary)
                .monospacedDigit()
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder
    private func growthRow(_ growth: CareInfoSnapshot.Growth) -> some View {
        switch growth.state {
        case "growing", "blocked":
            valueRow(
                text("距下一阶段", "Until next stage"),
                systemImage: "leaf.fill",
                value: growth.state == "blocked"
                    ? text("异常状态已暂停", "Paused (condition)")
                    : duration(growth.remainingMinutes)
            )

        case "pending":
            pmLabel(text("成长条件已满，等待进化", "Ready to evolve"), systemImage: "hourglass")
                .padding(.vertical, 2)

        case "mature":
            pmLabel(text("已成年 · 无下一阶段", "Adult · no next stage"), systemImage: "crown.fill")
                .padding(.vertical, 2)

        default:
            pmLabel(text("成长时间暂不可用", "Growth timer unavailable"), systemImage: "questionmark.circle")
                .padding(.vertical, 2)
        }
    }

    private func pmLabel(_ title: String, systemImage: String) -> some View {
        Label {
            Text(title)
                .font(.body)
                .foregroundStyle(.primary)
        } icon: {
            Image(systemName: systemImage)
                .font(.body)
                .foregroundStyle(Color.pmAccent)
        }
    }

    private func duration(_ minutes: Int?) -> String {
        guard let minutes else { return text("暂不可用", "Unavailable") }
        if minutes < 60 { return text("还需 \(minutes) 分钟", "\(minutes)m left") }
        let hours = minutes / 60
        let mins = minutes % 60
        if mins == 0 { return text("还需 \(hours) 小时", "\(hours)h left") }
        return text("还需 \(hours) 小时 \(mins) 分", "\(hours)h \(mins)m left")
    }

    @ViewBuilder
    private func unavailable(_ status: String) -> some View {
        switch status {
        case "unsupported_rom":
            Section {
                pmLabel(text("此 ROM 暂不支持养育信息", "Care information is not supported for this ROM"), systemImage: "info.circle")
                Text(text(
                    "目前仅支持已验证的日版 Sweets v062（2019-09-12）固件。",
                    "Only the verified Japanese Sweets v062 (2019-09-12) firmware is currently supported."
                ))
                .font(.body)
                .foregroundStyle(.primary)
            }
        case "asleep":
            message("固件正在休眠，实时参数暂不可读；唤醒后自动刷新。", "The firmware is asleep. Live values will refresh after it wakes.")
        case "catching_up":
            message("正在补算离线养育，完成后自动刷新。", "Offline care is being replayed. Values will refresh when it finishes.")
        case "no_session":
            message("请先加载并启动一个宠物。", "Load and start a pet first.")
        case "stopped":
            message("当前会话已停止，养育信息暂不可用。", "The session has stopped. Care information is unavailable.")
        default:
            message("固件参数尚未就绪，稍后自动刷新。", "Firmware values are not ready yet. They will refresh automatically.")
        }
    }

    private func message(_ zh: String, _ en: String) -> some View {
        Section { Text(text(zh, en)).foregroundStyle(.primary) }
    }
}
