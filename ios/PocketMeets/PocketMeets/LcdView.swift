import SwiftUI
import UIKit

/// The 128×128 frame. Dot-matrix is a shader on top of a copy. Asleep is a flat fill.
struct LcdView: View {
    var pixels: [UInt32]
    var asleep: Bool
    var dotMatrix: Bool
    var side: CGFloat
    var strength: Float

    var body: some View {
        Canvas { context, size in
            let image = LcdBitmap.image(pixels: pixels, asleep: asleep, dotMatrix: dotMatrix && !asleep, strength: strength, points: size)
            context.draw(Image(uiImage: image), in: CGRect(origin: .zero, size: size))
        }
        .frame(width: side, height: side)
        .accessibilityLabel("LCD")
    }
}

enum LcdBitmap {
    static func image(pixels: [UInt32], asleep: Bool, dotMatrix: Bool, strength: Float, points: CGSize) -> UIImage {
        let side = max(points.width, 128)
        let format = UIGraphicsImageRendererFormat()
        format.scale = 1
        format.opaque = true
        let renderer = UIGraphicsImageRenderer(size: CGSize(width: side, height: side), format: format)
        return renderer.image { ctx in
            if asleep {
                UIColor(red: 0x29 / 255, green: 0x2B / 255, blue: 0x2A / 255, alpha: 1).setFill()
                ctx.fill(CGRect(x: 0, y: 0, width: side, height: side))
                return
            }
            let cell = side / 128
            for y in 0..<128 {
                for x in 0..<128 {
                    let index = y * 128 + x
                    let argb = index < pixels.count ? pixels[index] : 0xFF000000
                    color(argb).setFill()
                    ctx.fill(CGRect(x: CGFloat(x) * cell, y: CGFloat(y) * cell, width: ceil(cell), height: ceil(cell)))
                }
            }
            guard dotMatrix, strength > 0 else { return }
            let gapV = CGFloat(0.21 * strength) * cell
            let gapH = CGFloat(0.18 * strength) * cell
            UIColor(white: 0, alpha: 0.85).setFill()
            if gapV > 0.2 {
                for y in 0..<128 {
                    ctx.fill(CGRect(x: 0, y: CGFloat(y + 1) * cell - gapV, width: side, height: gapV))
                }
            }
            if gapH > 0.2 {
                for x in 0..<128 {
                    ctx.fill(CGRect(x: CGFloat(x + 1) * cell - gapH, y: 0, width: gapH, height: side))
                }
            }
        }
    }

    private static func color(_ argb: UInt32) -> UIColor {
        let r = CGFloat((argb >> 16) & 0xFF) / 255
        let g = CGFloat((argb >> 8) & 0xFF) / 255
        let b = CGFloat(argb & 0xFF) / 255
        return UIColor(red: r, green: g, blue: b, alpha: 1)
    }
}

struct FlashOverlay: View {
    var token: Int
    @State private var progress: CGFloat = -0.3

    var body: some View {
        GeometryReader { geo in
            let band = geo.size.height * 0.28
            LinearGradient(
                colors: [Color.white.opacity(0), Color(white: 1, opacity: 0.9), Color.white.opacity(0)],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: band)
            .offset(y: geo.size.height * progress)
            .allowsHitTesting(false)
        }
        // The sweep must never paint outside the LCD. Without this the band
        // parks over the shell after the animation.
        .clipped()
        .onAppear { sweep() }
        .onChange(of: token) { _ in sweep() }
    }

    private func sweep() {
        let current = token
        progress = -0.3
        withAnimation(.linear(duration: 0.42)) { progress = 1.3 }
        // If the animation is interrupted (permission sheet, backgrounding)
        // finish the sweep anyway. An idle mid-flight value would leave a
        // translucent band on the screen.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) {
            guard token == current else { return }
            progress = 1.4
        }
    }
}
