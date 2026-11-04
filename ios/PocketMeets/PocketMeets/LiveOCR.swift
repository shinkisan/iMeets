import SwiftUI
import Translation

/// Floating caption under the toy. Shows the recognised Japanese line and the
/// on-device translation below it. Translation is skipped for lines that are
/// already mostly ASCII.
@available(iOS 18.0, *)
struct LiveCaptionPanel: View {
    var source: String
    var target: Locale.Language
    var localized: String? = nil
    var hideSource: Bool = false
    var placeholder: String = ""
    @State private var translated = ""
    @State private var translatedSource = ""
    @State private var session: TranslationSession?

    /// Lines the LCD repeats (menus, prompts, clock) are translated once and
    /// reused, so returning to a screen is instant.
    @MainActor private static var cache: [String: String] = [:]

    private var configuration: TranslationSession.Configuration {
        TranslationSession.Configuration(source: Locale.Language(identifier: "ja"), target: target)
    }

    private var currentTranslation: String? {
        translatedSource == source && !translated.isEmpty ? translated : nil
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if source.isEmpty {
                Text(placeholder)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.leading)
                    .lineLimit(4)
            } else {
                Text(localized ?? currentTranslation ?? source)
                    .font(.subheadline)
                    .multilineTextAlignment(.leading)
                    .lineLimit(4)
                if !hideSource, let shown = localized ?? currentTranslation, shown != source {
                    Text(source)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.leading)
                        .lineLimit(4)
                }
            }
        }
        .padding(14)
        .frame(maxWidth: 360, alignment: .leading)
        .pmGlassCard(20, fallback: .thickMaterial)
        .animation(.easeInOut(duration: 0.15), value: source)
        .translationTask(configuration) { session in
            await MainActor.run { self.session = session }
            await translate()
        }
        .task(id: source) { await translate() }
    }

    @MainActor
    private func translate() async {
        let text = source
        translatedSource = text
        translated = ""
        guard !text.isEmpty else {
            return
        }
        if let localized, !localized.isEmpty { return }
        let key = "\(target.minimalIdentifier)|\(text)"
        if let cached = Self.cache[key] {
            translated = cached
            return
        }
        if isMostlyASCII(text) {
            translated = text
            Self.cache[key] = text
            return
        }
        #if targetEnvironment(simulator)
        // The simulator ships without the Translation runtime.
        translated = text
        Self.cache[key] = text
        return
        #else
        guard let session else { return }
        do {
            let response = try await session.translate(text)
            // The LCD may have changed while the request was in flight.
            guard text == source else { return }
            translated = response.targetText
            if Self.cache.count > 200 { Self.cache.removeAll() }
            Self.cache[key] = response.targetText
        } catch {
            guard text == source else { return }
            translated = ""
        }
        #endif
    }

    private func isMostlyASCII(_ text: String) -> Bool {
        let scalars = text.unicodeScalars
        guard !scalars.isEmpty else { return true }
        let ascii = scalars.filter(\.isASCII).count
        return Double(ascii) / Double(scalars.count) > 0.8
    }
}
