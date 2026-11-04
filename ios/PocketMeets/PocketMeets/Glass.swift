import SwiftUI

/// Liquid Glass building blocks for the iOS 26/27 look.
///
/// The app deploys to iOS 16, so each helper keeps the material, corner and
/// shadow it replaced when the glass APIs are unavailable. Only floating
/// controls (toolbar buttons, the shutter, sheets over the toy, the shelf bar)
/// use glass; lists and forms keep the system's own material.
extension View {
    @ViewBuilder
    func pmGlass(_ shape: some Shape, interactive: Bool = false, fallback: Material = .regularMaterial) -> some View {
        if #available(iOS 26.0, *) {
            glassEffect(interactive ? Glass.regular.interactive() : Glass.regular, in: shape)
        } else {
            background(fallback, in: shape)
                .overlay(shape.stroke(Color.primary.opacity(0.08), lineWidth: 0.5))
                .shadow(color: .black.opacity(0.10), radius: 12, y: 4)
        }
    }

    func pmGlassCard(_ corner: CGFloat = 26, fallback: Material = .regularMaterial) -> some View {
        pmGlass(RoundedRectangle(cornerRadius: corner, style: .continuous), fallback: fallback)
    }

    /// A primary action that floats over content. Rows inside a Form keep the
    /// plain bordered style instead.
    @ViewBuilder
    func pmProminent() -> some View {
        if #available(iOS 26.0, *) {
            buttonStyle(.glassProminent)
        } else {
            buttonStyle(.borderedProminent)
        }
    }
}

extension View {
    /// Pins a control strip to the top safe area. On iOS 26 the strip becomes
    /// a system glass bar that content scrolls beneath; older systems get a
    /// material strip.
    @ViewBuilder
    func pmTopBar<Bar: View>(@ViewBuilder _ bar: () -> Bar) -> some View {
        if #available(iOS 26.0, *) {
            safeAreaBar(edge: .top, spacing: 0) {
                bar()
            }
        } else {
            safeAreaInset(edge: .top, spacing: 0) {
                bar().background(.bar)
            }
        }
    }
}

/// Keeps nearby glass shapes from overlapping. A no-op before iOS 26.
struct PMGlassGroup<Content: View>: View {
    var spacing: CGFloat?
    @ViewBuilder var content: () -> Content

    var body: some View {
        if #available(iOS 26.0, *) {
            GlassEffectContainer(spacing: spacing) { content() }
        } else {
            content()
        }
    }
}
