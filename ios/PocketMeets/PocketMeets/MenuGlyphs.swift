import Foundation

/// Glyphs extracted from the status/settings menu. This is a template table,
/// not OCR: each glyph is compared pixel-by-pixel against the game font.
struct MenuGlyph {
    let char: String
    let width: Int
    let rows: [String]
}

enum MenuGlyphs {
    static let templates: [MenuGlyph] = [
        MenuGlyph(char: "じ", width: 11, rows: ["###...##.##", "###...##.##", "###........", "###........", "###........", "###........", "###.....###", "###.....###", "###.....###", "####...####", ".#########.", "..#######.."]),
        MenuGlyph(char: "ょ", width: 9, rows: [".........", ".........", "....###..", "....###..", "....#####", "....#####", "....###..", ".######..", "########.", "###.#####", "#########", ".#####..."]),
        MenuGlyph(char: "う", width: 11, rows: ["..######...", "..######...", "...........", "..#######..", "##########.", "###...#####", ".......####", ".......####", ".......####", "......#####", "...#######.", "...######.."]),
        MenuGlyph(char: "た", width: 11, rows: ["..###......", "..###......", "#########..", "#########..", "..###......", "..###.#####", ".####.#####", ".###.......", "####.......", "####.###...", "###..######", "###...#####"]),
        MenuGlyph(char: "い", width: 11, rows: ["...........", "###...###..", "###...####.", "###....###.", "###....####", "###.....###", "###.....###", "###.##..###", "######..###", "######..###", ".####......", "..##......."]),
        MenuGlyph(char: "せ", width: 11, rows: [".###...###.", ".###...###.", "###########", "###########", ".###...###.", ".###...###.", ".###..####.", ".###..###..", ".####......", ".#########.", "..########.", "..........."]),
        MenuGlyph(char: "っ", width: 9, rows: [".........", ".........", "..#####..", "########.", "###..####", "......###", "......###", "......###", ".....####", "..######.", "..#####..", "........."]),
        MenuGlyph(char: "て", width: 11, rows: ["###########", "###########", ".....####..", "....####...", "...####....", "..####.....", "..####.....", "..####.....", "..#####....", "...########", ".....######", "..........."]),
        MenuGlyph(char: "い", width: 11, rows: ["###...###..", "###...####.", "###....###.", "###....####", "###.....###", "###.....###", "###.##..###", "######..###", "######..###", ".####......", "..##.......", "..........."]),
    ]

    /// Read the two-line menu label from a 128×128 ARGB frame.
    static func read(_ pixels: [UInt32], sweets: Bool) -> String? {
        guard pixels.count == 128 * 128 else { return nil }
        if sweets {
            let decoded = pixels.withUnsafeBufferPointer { buffer in
                CString.take(pm_menu_text(buffer.baseAddress, buffer.count))
            }
            if !decoded.isEmpty { return decoded }
        }
        func ink(_ x: Int, _ y: Int) -> Bool {
            let p = pixels[y * 128 + x]
            let r = Int((p >> 16) & 0xFF), g = Int((p >> 8) & 0xFF), b = Int(p & 0xFF)
            return r < 100 && g < 100 && b > 60
        }
        var bands: [(Int, Int)] = []
        var start = -1
        for y in 18...60 {
            let count = (30...100).reduce(0) { $0 + (ink($1, y) ? 1 : 0) }
            if count >= 2 && start < 0 { start = y }
            if count < 2 && start >= 0 { bands.append((start, y - 1)); start = -1 }
        }
        if start >= 0 { bands.append((start, 60)) }
        guard bands.count >= 2 else { return nil }
        var lines: [String] = []
        for band in bands.prefix(2) {
            let (y0, y1) = band
            var runs: [(Int, Int)] = []
            var s = -1
            for x in 30...105 {
                let any = (y0...y1).contains { ink(x, $0) }
                if any && s < 0 { s = x }
                if !any && s >= 0 { runs.append((s, x - 1)); s = -1 }
            }
            if s >= 0 { runs.append((s, 105)) }
            var text = ""
            for (rx0, rx1) in runs {
                let w = rx1 - rx0 + 1
                var best: (Int, String)?
                for t in templates {
                    if abs(t.width - w) > 1 { continue }
                    var dist = 0
                    for r in 0..<min(12, y1 - y0 + 1) {
                        for c in 0..<min(w, t.rows[r].count) {
                            let tc = t.rows[r][t.rows[r].index(t.rows[r].startIndex, offsetBy: c)]
                            let on = ink(rx0 + c, y0 + r)
                            if (tc == "#") != on { dist += 1 }
                        }
                    }
                    if best == nil || dist < best!.0 { best = (dist, t.char) }
                }
                guard let pick = best, pick.0 <= 10 else { return nil }
                text += pick.1
            }
            lines.append(text)
        }
        let joined = lines.joined(separator: " ")
        return joined == "じょうたい せってい" ? joined : nil
    }
}
