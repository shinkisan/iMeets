import AudioToolbox
import CoreHaptics
import SwiftUI
import UIKit

/// Device shell in the same 600×700 drawing space as the Android toy.
/// The LCD occupies the window: 55% of the width, top at 26.5% of the height.
enum EggMetrics {
    static func bodyHeight(_ width: CGFloat) -> CGFloat { width * 7 / 6 }

    static func screen(bodyWidth: CGFloat) -> CGRect {
        let height = bodyHeight(bodyWidth)
        let side = bodyWidth * 0.55
        return CGRect(x: (bodyWidth - side) / 2, y: 0.265 * height, width: side, height: side)
    }

    static func keySide(bodyWidth: CGFloat) -> CGFloat {
        min(64, max(44, bodyWidth * 0.105))
    }

    static func keyCenter(index: Int, bodyWidth: CGFloat, screen: CGRect) -> CGPoint {
        let height = bodyHeight(bodyWidth)
        let key = keySide(bodyWidth: bodyWidth)
        let gap = max(10 + key, bodyWidth / 6)
        let y = max(0.805 * height, screen.maxY + 12 + key / 2) + (index == 1 ? 0.03 * height : 0)
        return CGPoint(x: bodyWidth / 2 + CGFloat(index - 1) * gap, y: y)
    }

    static func shutter(bodyWidth: CGFloat) -> CGRect {
        let scale = bodyWidth / 600
        let width = max(48, 100 * scale)
        let height = max(40, 70 * scale)
        return CGRect(
            x: 300 * scale - width / 2,
            y: 105 * scale - height / 2,
            width: width,
            height: height
        )
    }
}

struct EggShell: View {
    var screen: CGRect
    var night: Bool = false

    var body: some View {
        Canvas { context, size in
            let scale = size.width / 600
            func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
                CGPoint(x: x * scale, y: y * scale)
            }
            let egg = eggPath(p)
            var shadow = context
            shadow.translateBy(x: 0, y: 7 * scale)
            shadow.fill(egg, with: .color(night ? Color(rgb: 0x07050F) : Color(rgb: 0xE8D6D8)))
            context.fill(egg, with: .linearGradient(
                Gradient(colors: night
                    ? [Color(rgb: 0x3E3368), Color(rgb: 0x2A2148), Color(rgb: 0x1A1430)]
                    : [Color(rgb: 0xFFE6ED), Color(rgb: 0xF4BFD1), Color(rgb: 0xE4A1BA)]),
                startPoint: p(100, 20),
                endPoint: p(460, 680)
            ))
            context.stroke(egg, with: .color(night ? Color(rgb: 0x6C59A2) : Color(rgb: 0xD78FA9)), lineWidth: 2 * scale)

            var inner = context
            inner.translateBy(x: 300 * scale, y: 350 * scale)
            inner.scaleBy(x: 0.897, y: 0.91)
            inner.translateBy(x: -300 * scale, y: -350 * scale)
            inner.fill(egg, with: .linearGradient(
                Gradient(colors: night
                    ? [Color(rgb: 0x262139), Color(rgb: 0x11101A)]
                    : [Color(rgb: 0xFFFEFA), Color(rgb: 0xFFF3F4)]),
                startPoint: p(300, 0),
                endPoint: p(300, 700)
            ))
            inner.stroke(egg, with: .color(night ? Color(rgb: 0x40385C) : Color(rgb: 0xFFFAFC)), lineWidth: 4 * scale)

            context.fill(bowPath(p), with: .color(night ? Color(rgb: 0x7D61B8) : Color(rgb: 0xD97194)))
            let knot = 12 * scale
            context.fill(
                Path(ellipseIn: CGRect(x: 300 * scale - knot, y: 108 * scale - knot, width: knot * 2, height: knot * 2)),
                with: .color(night ? Color(rgb: 0xB89BE8) : Color(rgb: 0xF6A9C0))
            )
            context.fill(sparkle(p(174, 154), 8 * scale), with: .color(night ? Color(rgb: 0x6D5BA8) : Color(rgb: 0xF1D5DC)))
            context.fill(sparkle(p(426, 158), 7 * scale), with: .color(night ? Color(rgb: 0x6D5BA8) : Color(rgb: 0xF1D5DC)))
            context.fill(sparkle(p(115, 340), 7 * scale), with: .color(night ? Color(rgb: 0x6D5BA8) : Color(rgb: 0xF1D5DC)))
            context.fill(sparkle(p(487, 345), 8 * scale), with: .color(night ? Color(rgb: 0x6D5BA8) : Color(rgb: 0xF1D5DC)))
            for dot in [p(154, 202), p(447, 215), p(130, 412), p(470, 405)] {
                let r = 3 * scale
                context.fill(Path(ellipseIn: CGRect(x: dot.x - r, y: dot.y - r, width: r * 2, height: r * 2)), with: .color(night ? Color(rgb: 0x463B70) : Color(rgb: 0xE8C6D1)))
            }
            bezel(context, screen: screen, night: night)
        }
        .allowsHitTesting(false)
    }

    private func eggPath(_ p: (CGFloat, CGFloat) -> CGPoint) -> Path {
        var path = Path()
        path.move(to: p(300, 10))
        path.addCurve(to: p(27, 363), control1: p(161, 10), control2: p(47, 193))
        path.addCurve(to: p(300, 690), control1: p(4, 554), control2: p(100, 688))
        path.addCurve(to: p(573, 363), control1: p(500, 688), control2: p(596, 554))
        path.addCurve(to: p(300, 10), control1: p(553, 193), control2: p(439, 10))
        path.closeSubpath()
        return path
    }

    private func bowPath(_ p: (CGFloat, CGFloat) -> CGPoint) -> Path {
        var path = Path()
        path.move(to: p(298, 105))
        path.addCurve(to: p(254, 108), control1: p(277, 82), control2: p(247, 81))
        path.addCurve(to: p(299, 113), control1: p(248, 135), control2: p(279, 129))
        path.addCurve(to: p(346, 108), control1: p(320, 132), control2: p(352, 136))
        path.addCurve(to: p(298, 105), control1: p(353, 79), control2: p(321, 81))
        path.closeSubpath()
        return path
    }

    private func sparkle(_ center: CGPoint, _ radius: CGFloat) -> Path {
        let pull = radius * 0.22
        var path = Path()
        path.move(to: CGPoint(x: center.x, y: center.y - radius))
        path.addQuadCurve(to: CGPoint(x: center.x + radius, y: center.y), control: CGPoint(x: center.x + pull, y: center.y - pull))
        path.addQuadCurve(to: CGPoint(x: center.x, y: center.y + radius), control: CGPoint(x: center.x + pull, y: center.y + pull))
        path.addQuadCurve(to: CGPoint(x: center.x - radius, y: center.y), control: CGPoint(x: center.x - pull, y: center.y + pull))
        path.addQuadCurve(to: CGPoint(x: center.x, y: center.y - radius), control: CGPoint(x: center.x - pull, y: center.y - pull))
        path.closeSubpath()
        return path
    }

    private func bezel(_ context: GraphicsContext, screen: CGRect, night: Bool) {
        let pad = screen.width * 0.035
        let corner = screen.width * 0.06
        var frame = screen.insetBy(dx: -pad, dy: -pad)
        var ctx = context
        ctx.fill(Path(roundedRect: frame, cornerRadius: corner), with: .color(night ? Color(rgb: 0x2E2748) : Color(rgb: 0xEAC1CF)))
        let inner = min(pad * 0.45, screen.width * 0.016)
        frame = frame.insetBy(dx: inner, dy: inner)
        ctx.fill(Path(roundedRect: frame, cornerRadius: corner * 0.8), with: .color(night ? Color(rgb: 0x0B0A14) : Color(rgb: 0xFFFCF8)))
        let dark = screen.insetBy(dx: -screen.width * 0.012, dy: -screen.width * 0.012)
        ctx.fill(Path(roundedRect: dark, cornerRadius: max(3, screen.width * 0.018)), with: .color(night ? Color(rgb: 0x04030A) : Color(rgb: 0x8B757A)))
    }
}

struct ShellKey: View {
    var title: String
    var side: CGFloat
    var minimal: Bool
    var night: Bool = false
    var accessibility: String
    var onChange: (Bool) -> Void
    @State private var pressed = false
    @State private var held = false
    @State private var pressStarted = Date.distantPast
    @State private var pressSeq = 0

    var body: some View {
        ZStack {
            // The key sits proud of the shell; pressing lets it sink.
            Circle()
                .fill(LinearGradient(colors: fill, startPoint: .top, endPoint: .bottom))
                .shadow(
                    color: .black.opacity(held ? 0.07 : 0.20),
                    radius: held ? 1.5 : 4.5,
                    y: held ? 0.5 : 2.5
                )
            Circle().stroke(border, lineWidth: 1)
            Text(title)
                .font(.system(size: side * 0.32, weight: .medium))
                .foregroundStyle(label)
            Circle().fill(Color.black.opacity(held ? 0.12 : 0))
        }
        .frame(width: side, height: side)
        .scaleEffect(held ? 0.92 : 1)
        .animation(.easeOut(duration: held ? 0.06 : 0.14), value: held)
        .contentShape(Circle())
        .overlay(KeyTouch(onDown: press, onUp: release))
        .accessibilityLabel(accessibility)
        .accessibilityAddTraits(.isButton)
        .accessibilityAction { tap() }
        .onAppear { KeyHaptics.prepare() }
    }

    private var fill: [Color] {
        if minimal && night { return [Color(rgb: 0x4A3C74), Color(rgb: 0x2A2148)] }
        if minimal { return [Color(rgb: 0xC25479), Color(rgb: 0xB74168)] }
        if night { return [Color(rgb: 0x3E3268), Color(rgb: 0x211A3B)] }
        return [Color.white, Color(rgb: 0xFFF3F5)]
    }

    private var border: Color {
        if minimal && night { return Color(rgb: 0x6E5BA8) }
        if minimal { return Color(rgb: 0xB43E65) }
        if night { return Color(rgb: 0x6E5BA8) }
        return Color(rgb: 0xE3B3C4)
    }

    private var label: Color {
        if minimal { return Color.white }
        return night ? Color(rgb: 0xD9CDF4) : Color(rgb: 0xA56F83)
    }

    // The key's own touch handling is instant, so the pressed look and the
    // click land on the same frame as the finger.
    private func press() {
        guard !pressed else { return }
        pressed = true
        held = true
        pressStarted = Date()
        pressSeq += 1
        KeyHaptics.tap()
        onChange(true)
    }

    private func release() {
        guard pressed else { return }
        pressed = false
        onChange(false)
        // A quick tap would flash the pressed look for two frames and read as
        // "nothing happened", so keep it on screen for a moment longer. The
        // key itself is released immediately.
        let remaining = 0.07 - Date().timeIntervalSince(pressStarted)
        pressSeq += 1
        let token = pressSeq
        if remaining <= 0 {
            held = false
            return
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + remaining) {
            if pressSeq == token { held = false }
        }
    }

    /// VoiceOver activation: a full press and release.
    private func tap() {
        press()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.12) { release() }
    }
}

/// Reports touch-down and touch-up the moment they happen.
///
/// SwiftUI's `DragGesture` only calls `onChanged` once it has recognised the
/// gesture, which made the pressed state arrive noticeably after the finger.
/// A plain `UIView` gets `touchesBegan` immediately, and it also gives us the
/// exact moment to fire the haptic.
private struct KeyTouch: UIViewRepresentable {
    var onDown: () -> Void
    var onUp: () -> Void

    func makeUIView(context: Context) -> TouchView {
        let view = TouchView()
        view.backgroundColor = .clear
        view.isMultipleTouchEnabled = false
        view.onDown = onDown
        view.onUp = onUp
        return view
    }

    func updateUIView(_ view: TouchView, context: Context) {
        view.onDown = onDown
        view.onUp = onUp
    }

    final class TouchView: UIView {
        var onDown: (() -> Void)?
        var onUp: (() -> Void)?
        private var down = false

        override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
            guard !down else { return }
            down = true
            KeyHaptics.prepare()
            onDown?()
        }

        override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
            guard down else { return }
            down = false
            onUp?()
        }

        override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
            guard down else { return }
            down = false
            onUp?()
        }

        /// Keep the round hit area instead of the view's square bounds.
        override func point(inside point: CGPoint, with event: UIEvent?) -> Bool {
            let radius = min(bounds.width, bounds.height) / 2
            let dx = point.x - bounds.midX
            let dy = point.y - bounds.midY
            return dx * dx + dy * dy <= radius * radius
        }
    }
}

/// One shared impact generator per key press. `prepare` keeps the taptic
/// engine warm so the click is not late; the system still honours the user's
/// haptics setting.
enum KeyHaptics {
    /// Mirrors the "button vibration" preference.
    static var enabled = true

    /// Intensity of the click, 0...1. The system sound fallback below is a
    /// fixed "peek" boom, so Core Haptics is what makes a softer click
    /// possible at all.
    private static let intensity: Float = 0.4
    private static let sharpness: Float = 0.5

    private static var engine: CHHapticEngine?
    private static var pattern: CHHapticPattern?

    /// Fallback for devices without a Taptic Engine, or if the haptic engine
    /// refuses to start: the low-level system sound the Camera app uses.
    private static let fallbackSound: SystemSoundID = 1519

    static func prepare() {
        guard enabled, engine == nil else { return }
        guard CHHapticEngine.capabilitiesForHardware().supportsHaptics else { return }
        do {
            let engine = try CHHapticEngine()
            // Our own AVAudioEngine holds the audio session; haptics-only mode
            // keeps the click off that path.
            engine.playsHapticsOnly = true
            engine.isAutoShutdownEnabled = false
            engine.stoppedHandler = { _ in KeyHaptics.engine = nil }
            engine.resetHandler = { KeyHaptics.engine = nil }
            try engine.start()
            KeyHaptics.engine = engine
            pattern = try? CHHapticPattern(events: [
                CHHapticEvent(eventType: .hapticTransient, parameters: [
                    .init(parameterID: .hapticIntensity, value: intensity),
                    .init(parameterID: .hapticSharpness, value: sharpness),
                ], relativeTime: 0)
            ], parameters: [])
        } catch {
            KeyHaptics.engine = nil
            pattern = nil
        }
    }

    static func tap() {
        guard enabled else { return }
        if engine == nil { prepare() }
        guard let engine, let pattern else {
            AudioServicesPlaySystemSound(fallbackSound)
            return
        }
        do {
            let player = try engine.makePlayer(with: pattern)
            try player.start(atTime: CHHapticTimeImmediate)
        } catch {
            KeyHaptics.engine = nil
            AudioServicesPlaySystemSound(fallbackSound)
        }
    }
}

/// Deep night sky behind the toy: indigo gradient, a soft moon and fixed
/// pin-prick stars. Kept in Canvas so it does not need image assets.
struct NightSky: View {
    var body: some View {
        ZStack {
            LinearGradient(
                colors: [Color(rgb: 0x070A18), Color(rgb: 0x14102A), Color(rgb: 0x090713)],
                startPoint: .top,
                endPoint: .bottom
            )
            Canvas { context, size in
                let moon = CGPoint(x: size.width * 0.78, y: size.height * 0.14)
                context.fill(
                    Path(ellipseIn: CGRect(x: moon.x - 44, y: moon.y - 44, width: 88, height: 88)),
                    with: .radialGradient(
                        Gradient(colors: [Color(rgb: 0xF6E9C8).opacity(0.20), .clear]),
                        center: moon,
                        startRadius: 10,
                        endRadius: 44
                    )
                )
                context.fill(
                    Path(ellipseIn: CGRect(x: moon.x - 19, y: moon.y - 19, width: 38, height: 38)),
                    with: .color(Color(rgb: 0xF5EBD0))
                )
                context.fill(
                    Path(ellipseIn: CGRect(x: moon.x - 12, y: moon.y - 23, width: 30, height: 30)),
                    with: .color(Color(rgb: 0x14102A).opacity(0.92))
                )
                for i in 0..<52 {
                    let x = CGFloat((i * 73 + 17) % 101) / 101 * size.width
                    let y = CGFloat((i * 37 + 11) % 97) / 97 * size.height
                    let r = CGFloat(0.7 + CGFloat(i % 3) * 0.7)
                    context.fill(
                        Path(ellipseIn: CGRect(x: x, y: y, width: r, height: r)),
                        with: .color(.white.opacity(i % 5 == 0 ? 0.75 : 0.35))
                    )
                }
            }
        }
    }
}

extension Color {
    /// Shell pink. Controls and row icons use this instead of the system
    /// accent so a pressed or rebuilt row cannot fall back to blue.
    static let pmAccent = Color(rgb: 0xC25479)

    init(rgb: UInt32) {
        self.init(
            red: Double((rgb >> 16) & 0xFF) / 255,
            green: Double((rgb >> 8) & 0xFF) / 255,
            blue: Double(rgb & 0xFF) / 255
        )
    }
}
