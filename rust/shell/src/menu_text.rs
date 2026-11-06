//! Deterministic Sweets font matching. Unrecognized glyphs can leave a partial
//! line; ambiguous small glyphs require a unique dictionary match.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const WIDTH: usize = 128;
const CELL_W: usize = 12;
const CELL_H: usize = 14;
// Sweets status units are proportional bitmaps and extend beyond a 12px cell.
const STATUS_UNITS: [(&str, [&str; CELL_H]); 3] = [
    (
        "代目",
        [
            ".....##..###.##..##########",
            "....###..###.##..##########",
            "...####..###.....###...####",
            "..####...###.....###...####",
            ".#####.########..##########",
            ".#####.########..##########",
            "...###...###.....###...####",
            "...###...###.....###...####",
            "...###...###.....##########",
            "...###...###.....##########",
            "...###...###.....###...####",
            "...###...###.....###...####",
            "...###...######..##########",
            "...###....#####..##########",
        ],
    ),
    (
        "才",
        [
            "......####..",
            "......####..",
            ".###########",
            ".###########",
            "......####..",
            "......####..",
            "......######",
            "......######",
            ".....#####..",
            ".#########..",
            ".####.####..",
            "......####..",
            ".....#####..",
            ".....#####..",
        ],
    ),
    (
        "G",
        [
            "............",
            ".....###....",
            "...########.",
            "..#########.",
            ".###.###....",
            ".###.###....",
            ".###.#######",
            ".###.#######",
            ".###.###.###",
            ".###.###.###",
            ".###.###.###",
            "..#########.",
            "...#######..",
            ".....###....",
        ],
    ),
];

// Month/day units on the profile page share a baseline with the preceding digit.
const DATE_UNITS: [(&str, [&str; CELL_H]); 2] = [
    (
        "月",
        [
            "...##########",
            "...##########",
            "...###....###",
            "...###....###",
            "...##########",
            "...##########",
            "...###....###",
            "...###....###",
            "...##########",
            "...##########",
            "...###....###",
            "...###....###",
            "..###...#####",
            ".###.....####",
        ],
    ),
    (
        "日",
        [
            ".###########",
            ".###########",
            ".###.....###",
            ".###.....###",
            ".###.....###",
            ".###.....###",
            ".###########",
            ".###########",
            ".###.....###",
            ".###.....###",
            ".###.....###",
            ".###.....###",
            ".###########",
            ".###########",
        ],
    ),
];

// The game draws the m!x brand as one proportional, 34-pixel-wide mark.
const MIX_MARK: [&str; CELL_H] = [
    "..................................",
    ".................####.............",
    ".................####.............",
    "##.#####.#####...####..###.....###",
    "###############..####..####...####",
    "###############..####...####.####.",
    "####..####..###..####....#######..",
    "###...###...###...##......#####...",
    "###...###...###...##.......###....",
    "###...###...###...........#####...",
    "###...###...###...##.....#######..",
    "###...###...###..####...####.####.",
    "###...###...###..####..####...####",
    "###...###...###...##...###.....###",
];

fn glyphs() -> &'static HashMap<[u16; CELL_H], char> {
    static GLYPHS: OnceLock<HashMap<[u16; CELL_H], char>> = OnceLock::new();
    GLYPHS.get_or_init(|| {
        let entries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-menu-glyphs.json"
        ))
        .expect("font data");
        let mut out = HashMap::new();
        for entry in entries {
            let ch = entry["char"].as_str().unwrap().chars().next().unwrap();
            let x = entry["x"].as_i64().unwrap() - 58;
            let y = entry["y"].as_i64().unwrap() - 21;
            let mut cell = [0u16; CELL_H];
            for (dy, row) in entry["rows"].as_array().unwrap().iter().enumerate() {
                for (dx, c) in row.as_str().unwrap().bytes().enumerate() {
                    if c == b'#' {
                        let xx = (x + dx as i64) as usize;
                        let yy = (y + dy as i64) as usize;
                        assert!(xx < CELL_W && yy < CELL_H);
                        cell[yy] |= 1 << xx;
                    }
                }
            }
            if let Some(previous) = out.insert(cell, ch) {
                assert_eq!(previous, ch, "ambiguous font data");
            }
        }
        out
    })
}

// Glyph bitmaps trimmed to their ink bounds. The 12-pixel window below only
// matches a glyph when the following glyph is far enough away; a narrow glyph
// such as ょ in a tight proportional line never forms a clean window. These
// trimmed shapes let the line joiner recover such glyphs from the gap between
// two matched runs.
struct Trimmed {
    w: usize,
    h: usize,
    rows: [u16; CELL_H],
    ch: char,
}

fn cell_bbox(cell: &[u16; CELL_H]) -> (usize, usize, usize, usize) {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (CELL_W, 0usize, CELL_H, 0usize);
    for y in 0..CELL_H {
        for x in 0..CELL_W {
            if cell[y] & (1 << x) != 0 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    (min_x, max_x, min_y, max_y)
}

fn trimmed() -> &'static Vec<Trimmed> {
    static TRIMMED: OnceLock<Vec<Trimmed>> = OnceLock::new();
    TRIMMED.get_or_init(|| {
        let mut seen: HashMap<(usize, usize, [u16; CELL_H]), Option<char>> = HashMap::new();
        for (cell, ch) in glyphs() {
            let (x0, x1, y0, y1) = cell_bbox(cell);
            let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
            let mut rows = [0u16; CELL_H];
            for y in y0..=y1 {
                let mut bits = 0u16;
                for x in x0..=x1 {
                    if cell[y] & (1 << x) != 0 {
                        bits |= 1 << (x - x0);
                    }
                }
                rows[y - y0] = bits;
            }
            seen.entry((w, h, rows))
                .and_modify(|seen| {
                    if *seen != Some(*ch) {
                        *seen = None;
                    }
                })
                .or_insert(Some(*ch));
        }
        seen.into_iter()
            .filter_map(|((w, h, rows), ch)| ch.map(|ch| Trimmed { w, h, rows, ch }))
            .collect()
    })
}

// Decode at most one glyph from the ink between two matched runs on one line.
// Used when a proportional glyph is too narrow for a clean 12-pixel window.
fn glyph_in_gap(ink: &[[bool; WIDTH]; WIDTH], y: usize, start: usize, stop: usize) -> Option<char> {
    if stop <= start || stop > WIDTH || y + CELL_H > WIDTH {
        return None;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (WIDTH, 0usize, WIDTH, 0usize);
    for yy in y..y + CELL_H {
        for xx in start..stop {
            if ink[yy][xx] {
                min_x = min_x.min(xx);
                max_x = max_x.max(xx);
                min_y = min_y.min(yy);
                max_y = max_y.max(yy);
            }
        }
    }
    if max_x < min_x {
        return None;
    }
    let (w, h) = (max_x - min_x + 1, max_y - min_y + 1);
    if w > CELL_W || h > CELL_H {
        return None;
    }
    let mut rows = [0u16; CELL_H];
    for dy in 0..h {
        for dx in 0..w {
            if ink[min_y + dy][min_x + dx] {
                rows[dy] |= 1 << dx;
            }
        }
    }
    trimmed()
        .iter()
        .find(|t| t.w == w && t.h == h && t.rows == rows)
        .map(|t| t.ch)
}

// The name-entry keyboard draws its kana list on a solid pink bar. That list
// is a selection UI, not message text, so lines sitting on it are dropped.
fn on_kana_bar(pixels: &[u32], y: usize, x: usize, chars: usize) -> bool {
    let width = (chars * CELL_W).min(WIDTH - x);
    if width == 0 || y >= WIDTH {
        return false;
    }
    let (mut pink, mut total) = (0usize, 0usize);
    for yy in y..(y + CELL_H).min(WIDTH) {
        for xx in x..x + width {
            let p = pixels[yy * WIDTH + xx];
            let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
            total += 1;
            if r >= 245 && (185..=225).contains(&g) && (195..=245).contains(&b) {
                pink += 1;
            }
        }
    }
    total > 0 && pink * 2 > total
}

// A 4:3 nearest-neighbour enlargement of the 12px grid produces 16px
// cells. These templates support enlarged renderings without relying on the
// text occupying any particular part of the LCD.
fn scaled_glyphs() -> &'static HashMap<[u16; 19], char> {
    static SCALED: OnceLock<HashMap<[u16; 19], char>> = OnceLock::new();
    SCALED.get_or_init(|| {
        let mut out = HashMap::new();
        for (source, ch) in glyphs() {
            let mut rows = [0u16; 19];
            for y in 0..19 {
                for x in 0..16 {
                    if source[y * 3 / 4] & (1 << (x * 3 / 4)) != 0 {
                        rows[y] |= 1 << x;
                    }
                }
            }
            if let Some(previous) = out.insert(rows, *ch) {
                assert_eq!(previous, *ch, "ambiguous scaled font data");
            }
        }
        out
    })
}

// Cache horizontal ink windows for the whole frame. Consecutive windows share
// all but one pixel, so computing them incrementally avoids rescanning every
// pixel of every overlapping candidate cell.
fn ink_windows(ink: &[[bool; WIDTH]; WIDTH], width: usize) -> [[u16; WIDTH]; WIDTH] {
    debug_assert!((1..=16).contains(&width));
    let mut windows = [[0u16; WIDTH]; WIDTH];
    for y in 0..WIDTH {
        let mut bits = 0u16;
        for x in 0..width {
            if ink[y][x] {
                bits |= 1 << x;
            }
        }
        for x in 0..=WIDTH - width {
            windows[y][x] = bits;
            if x + width < WIDTH {
                bits = (bits >> 1) | (u16::from(ink[y][x + width]) << (width - 1));
            }
        }
    }
    windows
}

fn is_pictogram(c: char) -> bool {
    matches!(
        c,
        '😀' | '😣'
            | '😖'
            | '😑'
            | '😵'
            | '😢'
            | '🐰'
            | '🎮'
            | '🎂'
            | '🎁'
            | '📱'
            | '🏠'
            | '👜'
            | '🍙'
            | '🍰'
    )
}

fn scaled_lines(ink: &[[bool; WIDTH]; WIDTH]) -> Vec<(usize, usize, usize, String)> {
    const W: usize = 16;
    const H: usize = 19;
    let windows = ink_windows(ink, W);
    let font = scaled_glyphs();
    let mut found = [[None; WIDTH]; WIDTH];
    for y in 0..=WIDTH - H {
        for x in 0..=WIDTH - W {
            let mut rows = [0u16; H];
            for dy in 0..H {
                rows[dy] = windows[y + dy][x];
            }
            found[y][x] = font.get(&rows).copied();
        }
    }
    let mut segments = Vec::new();
    for y in 0..=WIDTH - H {
        for x in 0..=WIDTH - W {
            if found[y][x].is_none() || (x >= W && found[y][x - W].is_some()) {
                continue;
            }
            let mut text = String::new();
            let mut end = x;
            while end <= WIDTH - W {
                let Some(ch) = found[y][end] else { break };
                text.push(ch);
                end += W;
            }
            segments.push((y, x, end, text));
        }
    }
    let mut lines = Vec::new();
    let mut index = 0;
    while index < segments.len() {
        let (y, x, mut end, mut text) = segments[index].clone();
        index += 1;
        while index < segments.len() && segments[index].0 == y && segments[index].1 >= end {
            let gap = segments[index].1 - end;
            if !(gap <= 3 || (14..=18).contains(&gap))
                || (y..y + H).any(|yy| (end..segments[index].1).any(|xx| ink[yy][xx]))
            {
                break;
            }
            if gap > 0 {
                text.push(' ');
            }
            text.push_str(&segments[index].3);
            end = segments[index].2;
            index += 1;
        }
        let adjacent_ink = |start: usize, stop: usize| {
            (y..y + H)
                .map(|yy| (start..stop).filter(|xx| ink[yy][*xx]).count())
                .sum::<usize>()
                > 4
        };
        if text.chars().filter(|c| *c != ' ').count() >= 2
            && text.chars().any(char::is_alphanumeric)
            && !adjacent_ink(x.saturating_sub(W), x)
            && !adjacent_ink(end.min(WIDTH), (end + W).min(WIDTH))
        {
            lines.push((y, x, end, text));
        }
    }
    lines
}

/// Return recognized lines from a 128×128 ARGB LCD buffer.
pub fn read(pixels: &[u32]) -> String {
    if pixels.len() < WIDTH * WIDTH {
        return String::new();
    }
    let mut ink = [[false; WIDTH]; WIDTH];
    for y in 0..WIDTH {
        for x in 0..WIDTH {
            let p = pixels[y * WIDTH + x];
            let r = (p >> 16) & 255;
            let g = (p >> 8) & 255;
            let b = p & 255;
            // Enabled and disabled Sweets menu labels use dark blue and grey.
            ink[y][x] = (r < 100 && g < 100 && b > 60)
                || ((192..=202).contains(&r)
                    && (189..=199).contains(&g)
                    && (192..=202).contains(&b));
        }
    }
    let windows = ink_windows(&ink, CELL_W);
    let font = glyphs();
    let mut found = [[None; WIDTH]; WIDTH];
    for y in 0..=WIDTH - CELL_H {
        for x in 0..=WIDTH - CELL_W {
            let mut cell = [0u16; CELL_H];
            for dy in 0..CELL_H {
                cell[dy] = windows[y + dy][x];
            }
            found[y][x] = font.get(&cell).copied();
        }
    }
    let mut lines: Vec<(usize, usize, String)> = clock_lines(&ink);
    let mut columns = vertical_lines(pixels);
    columns.sort_by_key(|(y, _, _)| *y);
    let mut i = 0;
    while i < columns.len() {
        let top = columns[i].0;
        let mut end = i + 1;
        while end < columns.len() && columns[end].0 - top <= 3 {
            end += 1;
        }
        let mut band = columns[i..end].to_vec();
        band.sort_by_key(|(_, x, _)| std::cmp::Reverse(*x));
        let mut j = 0;
        while j < band.len() {
            let (mut y, mut left, mut text) = band[j].clone();
            let mut previous_x = left;
            j += 1;
            while j < band.len() && previous_x - band[j].1 <= 32 {
                text.push(' ');
                text.push_str(&band[j].2);
                y = y.min(band[j].0);
                previous_x = band[j].1;
                left = left.min(previous_x);
                j += 1;
            }
            lines.push((y, left, text));
        }
        i = end;
    }
    // Find individual same-baseline runs first; short runs can be meaningful
    // when separated by a genuine blank cell or proportional spacing.
    let mut segments: Vec<(usize, usize, usize, String)> = Vec::new();
    for y in 0..=WIDTH - CELL_H {
        for x in 0..=WIDTH - CELL_W {
            if found[y][x].is_none() || (x >= CELL_W && found[y][x - CELL_W].is_some()) {
                continue;
            }
            let mut word = String::new();
            let mut end = x;
            while end <= WIDTH - CELL_W {
                match found[y][end] {
                    Some(ch) => word.push(ch),
                    None => break,
                }
                end += CELL_W;
            }
            let mix = (0..=2).find(|gap| {
                let at = end + gap;
                at + 34 <= WIDTH
                    && MIX_MARK.iter().enumerate().all(|(dy, row)| {
                        row.bytes()
                            .enumerate()
                            .all(|(dx, p)| ink[y + dy][at + dx] == (p == b'#'))
                    })
            });
            if let Some(gap) = mix {
                word.push_str("m!x");
                end += gap + 34;
            }
            segments.push((y, x, end, word));
        }
    }
    segments.sort_by_key(|(y, x, _, _)| (*y, *x));
    let mut i = 0;
    while i < segments.len() {
        let (y, x, mut end, mut word) = segments[i].clone();
        i += 1;
        while i < segments.len() && segments[i].0 == y && segments[i].1 >= end {
            let gap = segments[i].1 - end;
            if !(gap <= 2 || (10..=14).contains(&gap)) {
                break;
            }
            // A separated numeric value is often a different status column.
            if gap >= 10
                && word.chars().all(|c| c.is_ascii_digit())
                    != segments[i].3.chars().all(|c| c.is_ascii_digit())
            {
                break;
            }
            // Name-entry characters occupy adjacent green boxes separated by
            // a thin white rule. That rule is not a space in the name.
            let green_box = |at: usize| {
                (y..y + CELL_H)
                    .flat_map(|yy| (at..at + CELL_W).map(move |cx| pixels[yy * WIDTH + cx]))
                    .filter(|p| {
                        let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
                        (170..=210).contains(&r) && g >= 225 && (160..=210).contains(&b)
                    })
                    .count()
                    >= 24
            };
            let clean_gap = (y..y + CELL_H).all(|yy| {
                (end..segments[i].1).all(|cx| {
                    let p = pixels[yy * WIDTH + cx];
                    !ink[yy][cx]
                        && ((p >> 16) & 255) >= 215
                        && ((p >> 8) & 255) >= 215
                        && (p & 255) >= 215
                })
            });
            if !clean_gap {
                // An empty cell inside the green name field is a literal space
                // in the name (the game lets names contain one).
                let empty_name_box = gap <= 14
                    && end + CELL_W <= WIDTH
                    && green_box(end)
                    && (y..y + CELL_H).all(|yy| (end..segments[i].1).all(|cx| !ink[yy][cx]));
                if empty_name_box {
                    word.push(' ');
                    word.push_str(&segments[i].3);
                    end = segments[i].2;
                    i += 1;
                    continue;
                }
                // A narrow proportional glyph never forms its own window.
                // Recover it from the ink between the two matched runs.
                match glyph_in_gap(&ink, y, end, segments[i].1) {
                    Some(ch) => {
                        word.push(ch);
                        word.push_str(&segments[i].3);
                        end = segments[i].2;
                        i += 1;
                        continue;
                    }
                    None => break,
                }
            }
            let name_box_rule = gap <= 2 && green_box(end - CELL_W) && green_box(segments[i].1);
            if gap > 0 && !name_box_rule {
                word.push(' ');
            }
            word.push_str(&segments[i].3);
            end = segments[i].2;
            i += 1;
        }
        let count = word.chars().filter(|c| *c != ' ').count();
        let pictograms = word.chars().filter(|c| is_pictogram(*c)).count();
        let isolated_pictogram = count == 1
            && pictograms == 1
            && segments.iter().any(|(sy, sx, se, other)| {
                *sy == y
                    && (x.abs_diff(*se) <= 32 || end.abs_diff(*sx) <= 32)
                    && other.chars().filter(|c| is_pictogram(*c)).count() >= 2
            });
        // The name keyboard also has a page with just the return symbol.
        // Its pink option cell and green name boxes provide context for this
        // otherwise easily confused single glyph.
        let keyboard_return = count == 1
            && word == "↵"
            && (y..y + CELL_H)
                .flat_map(|yy| (x..end).map(move |cx| pixels[yy * WIDTH + cx]))
                .filter(|p| {
                    let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
                    r >= 245 && (180..=215).contains(&g) && b >= 210
                })
                .count()
                >= 24
            && (0..y)
                .flat_map(|yy| (0..WIDTH).map(move |cx| pixels[yy * WIDTH + cx]))
                .filter(|p| {
                    let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
                    (170..=210).contains(&r) && g >= 225 && (160..=210).contains(&b)
                })
                .count()
                >= 100;
        if (!word.chars().any(char::is_alphanumeric)
            && pictograms < 2
            && !isolated_pictogram
            && !keyboard_return)
            || (count < 2 && !isolated_pictogram && !keyboard_return)
        {
            continue;
        }
        // Reject fragments adjacent to unexplained text ink. Coloured cursor
        // icons and decorations are not treated as unknown text.
        let suspicious = |start: usize, stop: usize| {
            let mut text_ink = 0usize;
            let mut other = 0usize;
            for yy in y..y + CELL_H {
                for cx in start..stop {
                    if ink[yy][cx] {
                        text_ink += 1;
                        continue;
                    }
                    let p = pixels[yy * WIDTH + cx];
                    if ((p >> 16) & 255) < 215 || ((p >> 8) & 255) < 215 || (p & 255) < 215 {
                        other += 1;
                    }
                }
            }
            text_ink > 4 && other == 0
        };
        if suspicious(x.saturating_sub(CELL_W), x)
            || suspicious(end.min(WIDTH), (end + CELL_W).min(WIDTH))
        {
            continue;
        }
        lines.push((y, x, word));
    }
    // A value is accepted only when an entire unit bitmap and at least one
    // adjacent digit match. This avoids treating isolated decorative digits as
    // status text. Values with unrecognized units are left unread.
    for y in 0..=WIDTH - CELL_H {
        for &(unit, pattern) in &STATUS_UNITS {
            let unit_width = pattern[0].len();
            for x in 18..=WIDTH - unit_width {
                let matches = pattern.iter().enumerate().all(|(dy, row)| {
                    row.bytes()
                        .enumerate()
                        .all(|(dx, p)| ink[y + dy][x + dx] == (p == b'#'))
                });
                if !matches {
                    continue;
                }
                let start = (x.saturating_sub(18)..=x.saturating_sub(10))
                    .find(|&px| found[y][px].is_some_and(|ch| ch.is_ascii_digit()));
                let Some(mut px) = start else {
                    continue;
                };
                let mut digits = vec![found[y][px].unwrap()];
                while px >= CELL_W {
                    match found[y][px - CELL_W] {
                        Some(ch) if ch.is_ascii_digit() => {
                            digits.push(ch);
                            px -= CELL_W;
                        }
                        _ => break,
                    }
                }
                digits.reverse();
                let text = digits.into_iter().collect::<String>() + unit;
                lines.push((y, px, text));
            }
        }
    }
    let mut date_parts = Vec::new();
    for y in 2..=WIDTH - CELL_H {
        for &(unit, pattern) in &DATE_UNITS {
            let unit_width = pattern[0].len();
            for x in 12..=WIDTH - unit_width {
                if !pattern.iter().enumerate().all(|(dy, row)| {
                    row.bytes()
                        .enumerate()
                        .all(|(dx, p)| ink[y + dy][x + dx] == (p == b'#'))
                }) {
                    continue;
                }
                let mut px = x - 12;
                let Some(digit) = found[y - 2][px].filter(|ch| ch.is_ascii_digit()) else {
                    continue;
                };
                let mut digits = vec![digit];
                while px >= CELL_W {
                    match found[y - 2][px - CELL_W] {
                        Some(ch) if ch.is_ascii_digit() => {
                            digits.push(ch);
                            px -= CELL_W;
                        }
                        _ => break,
                    }
                }
                digits.reverse();
                date_parts.push((y - 2, px, digits.into_iter().collect::<String>() + unit));
            }
        }
    }
    // A pair of date units provides context for a short heading immediately
    // above the date; isolated glyphs elsewhere remain suppressed.
    for &(y, x, ref month) in &date_parts {
        if !month.ends_with('月')
            || !date_parts
                .iter()
                .any(|(dy, dx, day)| *dy == y && *dx > x && day.ends_with('日'))
        {
            continue;
        }
        let mut headings = Vec::new();
        for hy in y.saturating_sub(20)..y.saturating_sub(10) {
            for hx in 0..x {
                if let Some(ch) = found[hy][hx] {
                    headings.push((hy, hx, ch));
                }
            }
        }
        if headings.len() == 1 {
            let (hy, hx, ch) = headings[0];
            lines.push((hy, hx, ch.to_string()));
        }
    }
    lines.extend(date_parts);
    let large_count = lines.len();
    for (sy, sx, text) in small_candidates(pixels, &ink, &lines) {
        let end = sx + text.chars().count() * 9;
        let overlaps = lines.iter().take(large_count).any(|(ly, lx, large)| {
            let large_end = lx + large.chars().count() * CELL_W;
            sy < ly + CELL_H && *ly < sy + 9 && sx < large_end && *lx < end
        });
        if !overlaps {
            lines.push((sy, sx, text));
        }
    }
    for (y, x, end, text) in scaled_lines(&ink) {
        lines.retain(|(ly, lx, _)| !(*ly >= y && *ly < y + 19 && *lx >= x && *lx < end));
        lines.push((y, x, text));
    }
    lines.retain(|(y, x, text)| !on_kana_bar(pixels, *y, *x, text.chars().count()));
    lines.sort_by_key(|(y, x, _)| (*y, *x));
    lines
        .into_iter()
        .map(|(_, _, text)| text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, has_fixtures))]
mod tests {
    use super::*;
    #[test]
    fn rolling_windows_match_direct_pixel_reads() {
        let mut ink = [[false; WIDTH]; WIDTH];
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                ink[y][x] = (x * 17 + y * 31 + x * y) % 11 < 4;
            }
        }
        for width in [1, 9, 12, 16] {
            let windows = ink_windows(&ink, width);
            for y in 0..WIDTH {
                for x in 0..=WIDTH - width {
                    let expected =
                        (0..width).fold(0u16, |bits, dx| bits | (u16::from(ink[y][x + dx]) << dx));
                    assert_eq!(windows[y][x], expected, "{width} at {x},{y}");
                }
            }
        }
    }
    fn fixture_pixels(screen: &serde_json::Value) -> Vec<u32> {
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        let x0 = screen["x"].as_u64().unwrap() as usize;
        let y0 = screen["y"].as_u64().unwrap() as usize;
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                let color = match ch {
                    b'w' => 0xffffffff,
                    b'c' => 0xff83e2ff,
                    b'l' => 0xffdee6ff,
                    b'p' => 0xfffffac5,
                    b'i' => 0xffffcae6,
                    b'n' => 0xffbdf6b4,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => continue,
                };
                pixels[(y0 + y) * WIDTH + x0 + x] = color;
            }
        }
        pixels
    }

    #[test]
    fn real_frames_do_not_use_the_synthetic_scaled_path() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        for (index, screen) in screens.iter().enumerate() {
            let pixels = fixture_pixels(screen);
            let mut ink = [[false; WIDTH]; WIDTH];
            for y in 0..WIDTH {
                for x in 0..WIDTH {
                    let p = pixels[y * WIDTH + x];
                    let r = (p >> 16) & 255;
                    let g = (p >> 8) & 255;
                    let b = p & 255;
                    ink[y][x] = (r < 100 && g < 100 && b > 60)
                        || ((192..=202).contains(&r)
                            && (189..=199).contains(&g)
                            && (192..=202).contains(&b));
                }
            }
            let scaled = scaled_lines(&ink);
            assert!(
                scaled.is_empty(),
                "screen {index} ({:?}) unexpectedly matched 16px: {scaled:?}",
                screen["label"]
            );
            let windows = ink_windows(&ink, 16);
            let font = scaled_glyphs();
            let mut cells = 0;
            for y in 0..=WIDTH - 19 {
                for x in 0..=WIDTH - 16 {
                    let mut rows = [0u16; 19];
                    for dy in 0..19 {
                        rows[dy] = windows[y + dy][x];
                    }
                    if font.contains_key(&rows) {
                        cells += 1;
                    }
                }
            }
            // A single wide glyph (for example ー) can accidentally fit a
            // scaled cell; a 16px line still requires at least two cells and
            // is rejected by scaled_lines above. More than one stray cell in
            // a frame would be a new false-positive case to investigate.
            assert!(
                cells <= 1,
                "screen {index} ({:?}) has {cells} 16px glyph cells",
                screen["label"]
            );
        }
    }

    #[test]
    fn reads_multiline_large_text_screens() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/large-text-screens.json"
        ))
        .unwrap();
        for (index, screen) in screens.iter().enumerate() {
            let pixels = fixture_pixels(screen);
            assert_eq!(
                read(&pixels),
                screen["label"].as_str().unwrap(),
                "screen {index}"
            );
        }
    }

    #[test]
    fn reads_independent_menu_frames() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        for (index, screen) in screens.into_iter().enumerate() {
            let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
            let x0 = screen["x"].as_u64().unwrap() as usize;
            let y0 = screen["y"].as_u64().unwrap() as usize;
            for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
                for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                    if ch == b'w' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xffffffff;
                    }
                    if ch == b'c' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xff83e2ff;
                    }
                    if ch == b'l' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xffdee6ff;
                    }
                    if ch == b'p' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xfffffac5;
                    }
                    if ch == b'i' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xffffcae6;
                    }
                    if ch == b'n' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xffbdf6b4;
                    }
                    if ch == b'#' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xff000080;
                    }
                    if ch == b'g' {
                        pixels[(y0 + y) * WIDTH + x0 + x] = 0xffc5c2c5;
                    }
                }
            }
            // Some annotated screens record a known incomplete result separately.
            let expected = screen["recognized"]
                .as_str()
                .or_else(|| screen["label"].as_str())
                .unwrap();
            assert_eq!(read(&pixels), expected, "screen {index}: {expected}");
        }
    }
    #[test]
    fn reads_new_text_at_another_position() {
        let mut pixels = [0xffffffff; WIDTH * WIDTH];
        let font = glyphs();
        for (i, ch) in ['あ', 'い', 'う'].iter().enumerate() {
            let rows = font.iter().find(|(_, value)| *value == ch).unwrap().0;
            for y in 0..CELL_H {
                for x in 0..CELL_W {
                    if rows[y] & (1 << x) != 0 {
                        pixels[(72 + y) * WIDTH + 13 + i * CELL_W + x] = 0xff000080;
                    }
                }
            }
        }
        assert_eq!(read(&pixels), "あいう");
    }
    #[test]
    fn reads_two_character_names_without_dictionary_entries() {
        let mut pixels = [0xffffffff; WIDTH * WIDTH];
        for (i, ch) in ['き', 'ぬ'].iter().enumerate() {
            let rows = glyphs().iter().find(|(_, value)| *value == ch).unwrap().0;
            for y in 0..CELL_H {
                for x in 0..CELL_W {
                    if rows[y] & (1 << x) != 0 {
                        pixels[(43 + y) * WIDTH + 31 + i * CELL_W + x] = 0xff000080;
                    }
                }
            }
        }
        assert!(!local_words().contains("きぬ"));
        assert_eq!(read(&pixels), "きぬ");
    }
    #[test]
    fn reads_16px_nearest_neighbour_scaled_text() {
        let mut pixels = [0xffffffff; WIDTH * WIDTH];
        for (index, ch) in ['か', 'た', 'な'].iter().enumerate() {
            let rows = scaled_glyphs()
                .iter()
                .find(|(_, value)| *value == ch)
                .unwrap()
                .0;
            for y in 0..19 {
                for x in 0..16 {
                    if rows[y] & (1 << x) != 0 {
                        pixels[(47 + y) * WIDTH + 21 + index * 16 + x] = 0xff000080;
                    }
                }
            }
        }
        assert_eq!(read(&pixels), "かたな");
        for y in 50..54 {
            for x in 70..73 {
                pixels[y * WIDTH + x] = 0xff000080;
            }
        }
        assert!(!read(&pixels).contains("かたな"));
    }
    #[test]
    fn joins_scaled_text_across_small_spacing() {
        let mut pixels = [0xffffffff; WIDTH * WIDTH];
        for (ch, left) in [('か', 9), ('た', 25), ('な', 43), ('に', 59)] {
            let rows = scaled_glyphs()
                .iter()
                .find(|(_, value)| **value == ch)
                .unwrap()
                .0;
            for y in 0..19 {
                for x in 0..16 {
                    if rows[y] & (1 << x) != 0 {
                        pixels[(47 + y) * WIDTH + left + x] = 0xff000080;
                    }
                }
            }
        }
        assert_eq!(read(&pixels), "かた なに");
    }
    #[test]
    fn reads_another_single_character_status_name() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens
            .iter()
            .find(|s| s["label"] == "か ふわつのっち")
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => 0xff00ff00,
                };
            }
        }
        let font: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .unwrap();
        let glyph = font.iter().find(|g| g["char"] == "き").unwrap();
        for (y, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[(78 + y) * WIDTH + 11 + x] =
                    if ch == b'#' { 0xff000080 } else { 0xffffffff };
            }
        }
        assert_eq!(read(&pixels), "き ふわつのっち");
    }
    #[test]
    fn reads_small_dialogue_with_a_blank_cell() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens.iter().find(|s| s["label"] == "あ").unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => 0xff00ff00,
                };
            }
        }
        let font: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .unwrap();
        let glyph = font.iter().find(|g| g["char"] == "い").unwrap();
        for (y, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                if ch == b'#' {
                    pixels[(44 + y) * WIDTH + 47 + x] = 0xff000080;
                }
            }
        }
        assert_eq!(read(&pixels), "あ い");
        for y in 44..53 {
            for x in 47..57 {
                pixels[y * WIDTH + x] = 0xffffffff;
            }
        }
        for (y, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                if ch == b'#' {
                    pixels[(44 + y) * WIDTH + 56 + x] = 0xff000080;
                }
            }
        }
        assert_eq!(read(&pixels), "あ  い");
    }
    #[test]
    fn reads_dense_small_glyph_followed_by_left_overhang() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens
            .iter()
            .find(|s| s["label"] == "か🎁 ふわつのっち")
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => 0xff00ff00,
                };
            }
        }
        let font: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .unwrap();
        let glyph = font.iter().find(|g| g["char"] == "ロ").unwrap();
        for (y, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                if ch == b'#' {
                    pixels[(78 + y) * WIDTH + 29 + x] = 0xff000080;
                }
            }
        }
        assert_eq!(read(&pixels), "か🎁ロ ふわつのっち");
        pixels[78 * WIDTH + 29] = 0xff000080; // Corrupt the final glyph.
        assert_eq!(read(&pixels), "ふわつのっち");
    }
    #[test]
    fn duplicate_extended_codes_do_not_make_small_glyph_ambiguous() {
        let font: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .unwrap();
        let mut pixels = [0xffffffff; WIDTH * WIDTH];
        for (index, code) in ["0406", "1001"].iter().enumerate() {
            let glyph = font.iter().find(|g| g["code"] == *code).unwrap();
            for (y, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
                for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                    if ch == b'#' {
                        pixels[(40 + y) * WIDTH + 11 + index * 9 + x] = 0xff000080;
                    }
                }
            }
        }
        assert_eq!(read(&pixels), "かÉ");
    }
    #[test]
    fn unknown_name_suffix_is_not_a_complete_single_character_name() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens
            .iter()
            .find(|s| s["label"] == "か🎁 ふわつのっち")
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => 0xff00ff00,
                };
            }
        }
        pixels[78 * WIDTH + 20] = 0xff000080; // Make the gift shape unknown.
        assert_eq!(read(&pixels), "ふわつのっち");
    }
    #[test]
    fn reads_pictograms_in_a_text_run() {
        for icon in [
            '😀', '😣', '😖', '😑', '😵', '😢', '🐰', '🎮', '🎂', '🎁', '📱', '🏠', '👜', '🍙',
            '🍰',
        ] {
            let mut pixels = [0xffffffff; WIDTH * WIDTH];
            for (i, ch) in ['あ', 'い', icon, 'う', 'え'].iter().enumerate() {
                let rows = glyphs().iter().find(|(_, value)| *value == ch).unwrap().0;
                for y in 0..CELL_H {
                    for x in 0..CELL_W {
                        if rows[y] & (1 << x) != 0 {
                            pixels[(40 + y) * WIDTH + 13 + i * CELL_W + x] = 0xff000080;
                        }
                    }
                }
            }
            assert_eq!(read(&pixels), format!("あい{icon}うえ"));
        }
    }
    #[test]
    fn recognizes_two_independent_vertical_columns() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let source = screens
            .iter()
            .find(|v| {
                v["label"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("まだであってないよ"))
            })
            .unwrap();
        let mut pixels = vec![0xffffffff; WIDTH * WIDTH];
        for (y, row) in source["rows"].as_array().unwrap().iter().enumerate() {
            let row = row.as_str().unwrap().as_bytes();
            for x in 6..21 {
                let pixel = match row[x] {
                    b'c' => 0xff83e2ff,
                    b'w' => 0xffffffff,
                    _ => 0xff00ff00,
                };
                pixels[y * WIDTH + x] = pixel;
                pixels[y * WIDTH + x + 30] = pixel;
            }
        }
        let lines = vertical_lines(&pixels);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|(_, _, s)| s == "まだであってないよ"));
    }
    #[test]
    fn reads_vertical_columns_from_right_to_left() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let source = screens
            .iter()
            .find(|v| {
                v["label"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("まだであってないよ"))
            })
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in source["rows"].as_array().unwrap().iter().enumerate() {
            let row = row.as_str().unwrap().as_bytes();
            for x in 6..21 {
                let pixel = match row[x] {
                    b'c' => 0xff83e2ff,
                    b'w' => 0xffffffff,
                    _ => 0xff00ff00,
                };
                pixels[y * WIDTH + x] = pixel;
                pixels[y * WIDTH + x + 30] = if y < 47 { pixel } else { 0xff83e2ff };
            }
        }
        assert_eq!(read(&pixels), "まだで まだであってないよ");
    }
    #[test]
    fn status_units_follow_changing_digits() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens
            .iter()
            .find(|screen| {
                screen["label"]
                    .as_str()
                    .is_some_and(|s| s.contains("1代目"))
            })
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'l' => 0xffdee6ff,
                    b'#' => 0xff000080,
                    b'g' => 0xffc5c2c5,
                    _ => 0xff00ff00,
                };
            }
        }
        for (x, y, ch) in [(69, 62, '2'), (81, 62, '7'), (93, 80, '8')] {
            let rows = glyphs().iter().find(|(_, value)| **value == ch).unwrap().0;
            for dy in 0..CELL_H {
                for dx in 0..CELL_W {
                    pixels[(y + dy) * WIDTH + x + dx] = if rows[dy] & (1 << dx) != 0 {
                        0xff000080
                    } else {
                        0xffffffff
                    };
                }
            }
        }
        assert_eq!(read(&pixels), "あいじょー おなか せだい 27代目 8才 0G");
    }
    #[test]
    fn family_card_units_follow_changing_digits() {
        let screens: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-full-screens.json"
        ))
        .unwrap();
        let screen = screens
            .iter()
            .find(|s| s["label"] == "1代目 ふわつのっち 0才")
            .unwrap();
        let mut pixels = vec![0xff00ff00; WIDTH * WIDTH];
        for (y, row) in screen["rows"].as_array().unwrap().iter().enumerate() {
            for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                pixels[y * WIDTH + x] = match ch {
                    b'w' => 0xffffffff,
                    b'p' => 0xfffffac5,
                    b'#' => 0xff000080,
                    _ => 0xff00ff00,
                };
            }
        }
        let font: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .unwrap();
        for (ch, x, y, background) in [('2', 109, 1, 0xfffffac5), ('3', 43, 84, 0xffffffff)] {
            let glyph = font.iter().find(|g| g["char"] == ch.to_string()).unwrap();
            for (dy, row) in glyph["rows"].as_array().unwrap().iter().enumerate() {
                for (dx, p) in row.as_str().unwrap().bytes().enumerate() {
                    if p == b'#' {
                        pixels[(y + dy) * WIDTH + x + dx] = 0xff000080;
                    } else if dx < 9 {
                        pixels[(y + dy) * WIDTH + x + dx] = background;
                    }
                }
            }
        }
        assert_eq!(read(&pixels), "2代目 ふわつのっち 3才");
        let one = font.iter().find(|g| g["char"] == "1").unwrap();
        for (dy, row) in one["rows"].as_array().unwrap().iter().enumerate() {
            for (dx, p) in row.as_str().unwrap().bytes().enumerate() {
                if p == b'#' {
                    pixels[(1 + dy) * WIDTH + 100 + dx] = 0xff000080;
                }
            }
        }
        assert_eq!(read(&pixels), "12代目 ふわつのっち 3才");
    }
    #[test]
    fn ambiguous_small_glyph_uses_dictionary_only_when_unique() {
        let cells = vec![
            vec!['ヘ'],
            vec!['ア'],
            vec!['サ'],
            vec!['コ', 'ロ'],
            vec!['ン'],
        ];
        assert_eq!(choose_small_word(&cells).as_deref(), Some("ヘアサロン"));
        let unknown = vec![vec!['ぬ'], vec!['コ', 'ロ'], vec!['ぬ']];
        assert_eq!(choose_small_word(&unknown), None);
    }
    #[test]
    fn sweets_font_gate_follows_the_session_rom() {
        let mut session = crate::session::Session::bare(vec![0; 4]);
        assert_eq!(crate::ffi::pm_current_is_sweets(std::ptr::null_mut()), 0);
        assert_eq!(crate::ffi::pm_current_is_sweets(&mut session), 0);
        session.stem = tg18::roms::KNOWN
            .iter()
            .find(|rom| rom.slug == "sweets")
            .unwrap()
            .stem();
        assert_eq!(crate::ffi::pm_current_is_sweets(&mut session), 1);
        session.stem = tg18::roms::KNOWN
            .iter()
            .find(|rom| rom.slug == "fairy")
            .unwrap()
            .stem();
        assert_eq!(crate::ffi::pm_current_is_sweets(&mut session), 0);
    }
    #[test]
    fn ffi_checks_length_and_handles_empty_frame() {
        use std::ffi::CStr;
        unsafe {
            let p = crate::ffi::pm_menu_text(std::ptr::null(), WIDTH * WIDTH);
            assert_eq!(CStr::from_ptr(p).to_bytes(), b"");
            crate::ffi::pm_string_free(p);
            let pixels = [0xffffffff; WIDTH * WIDTH];
            let p = crate::ffi::pm_menu_text(pixels.as_ptr(), 1);
            assert_eq!(CStr::from_ptr(p).to_bytes(), b"");
            crate::ffi::pm_string_free(p);
        }
    }
    #[test]
    fn empty_screen_is_not_text() {
        assert_eq!(read(&[0xffffffff; WIDTH * WIDTH]), "");
        assert_eq!(read(&[]), "");
    }
}

// Resolve ambiguous cells only when exactly one whole word is in the local
// translation dictionary. Unknown or multiply matching words stay unread.
fn choose_small_word(cells: &[Vec<char>]) -> Option<String> {
    if cells.iter().any(Vec::is_empty) {
        return None;
    }
    let mut paths = vec![String::new()];
    for chars in cells {
        if paths.len().saturating_mul(chars.len()) > 128 {
            return None;
        }
        paths = paths
            .iter()
            .flat_map(|path| {
                chars.iter().map(move |ch| {
                    let mut next = path.clone();
                    next.push(*ch);
                    next
                })
            })
            .collect();
    }
    if paths.len() == 1 {
        return paths.pop();
    }
    let mut matches = paths
        .into_iter()
        .filter(|word| local_words().contains(word));
    let best = matches.next()?;
    if matches.next().is_none() {
        Some(best)
    } else {
        None
    }
}

fn local_words() -> &'static HashSet<String> {
    static WORDS: OnceLock<HashSet<String>> = OnceLock::new();
    WORDS.get_or_init(|| {
        include_str!("../../../ios/PocketMeets/PocketMeets/Translations.swift")
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                let rest = line.strip_prefix('"')?;
                let (word, tail) = rest.split_once('"')?;
                if tail.trim_start().starts_with(':') {
                    Some(word.to_string())
                } else {
                    None
                }
            })
            .collect()
    })
}

// Small glyphs can extend into an adjacent 9-pixel cell.
fn small_candidates(
    pixels: &[u32],
    ink: &[[bool; WIDTH]; WIDTH],
    large_lines: &[(usize, usize, String)],
) -> Vec<(usize, usize, String)> {
    type SmallFont = HashMap<[u16; 9], Vec<([u16; 9], char)>>;
    static SMALL_FONT: OnceLock<SmallFont> = OnceLock::new();
    let font = SMALL_FONT.get_or_init(|| {
        let data: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-small-glyphs.json"
        ))
        .expect("small font data");
        let mut font: SmallFont = HashMap::new();
        for item in data {
            let mut rows = [0u16; 9];
            for (y, row) in item["rows"].as_array().unwrap().iter().enumerate() {
                for (x, ch) in row.as_str().unwrap().bytes().enumerate() {
                    if ch == b'#' {
                        rows[y] |= 1 << x;
                    }
                }
            }
            let core = rows.map(|r| r & 0x1fe);
            font.entry(core)
                .or_default()
                .push((rows, item["char"].as_str().unwrap().chars().next().unwrap()));
        }
        // Units on the family profile card use the same 9px advance as the
        // small font, but are separate bitmaps rather than page-04 codes.
        for (ch, pattern) in [
            (
                '才',
                [
                    "..........",
                    "...#......",
                    "...#......",
                    "#######...",
                    "...#......",
                    "...###....",
                    "..##......",
                    "##.#......",
                    "...#......",
                ],
            ),
            (
                '代',
                [
                    "..........",
                    "....#.#.#.",
                    "...#..#...",
                    "..##.####.",
                    ".#.#..#...",
                    "...#..#...",
                    "...#..#...",
                    "...#...##.",
                    "..........",
                ],
            ),
            (
                '目',
                [
                    "..........",
                    ".####.....",
                    ".#..#.....",
                    ".####.....",
                    ".#..#.....",
                    ".####.....",
                    ".#..#.....",
                    ".####.....",
                    "..........",
                ],
            ),
        ] {
            let mut rows = [0u16; 9];
            for (y, row) in pattern.iter().enumerate() {
                for (x, p) in row.bytes().enumerate() {
                    if p == b'#' {
                        rows[y] |= 1 << x;
                    }
                }
            }
            font.entry(rows.map(|r| r & 0x1fe))
                .or_default()
                .push((rows, ch));
        }
        font
    });
    static CLIPPED_FONT: OnceLock<SmallFont> = OnceLock::new();
    let clipped_font = CLIPPED_FONT.get_or_init(|| {
        let mut clipped = SmallFont::new();
        for values in font.values() {
            for (rows, ch) in values {
                let mut variant = *rows;
                variant[1] = 0;
                clipped
                    .entry(variant.map(|r| r & 0x1fe))
                    .or_default()
                    .push((variant, *ch));
            }
        }
        clipped
    });
    let mut found: Vec<Vec<Option<Vec<char>>>> = vec![vec![None; WIDTH]; WIDTH];
    let mut clipped: Vec<Vec<Option<Vec<char>>>> = vec![vec![None; WIDTH]; WIDTH];
    for y in 0..=WIDTH - 9 {
        for x in 0..=WIDTH - 10 {
            let mut rows = [0u16; 9];
            let mut white = 0;
            for dy in 0..9 {
                for dx in 0..10 {
                    let p = pixels[(y + dy) * WIDTH + x + dx];
                    let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
                    if ink[y + dy][x + dx] {
                        rows[dy] |= 1 << dx;
                    }
                    if r >= 215 && g >= 215 && b >= 185 {
                        white += 1;
                    }
                }
            }
            // Dense pictograms can occupy over half of a cell (the gift uses
            // 52 of 90 pixels). Keep enough light background for confidence
            // without discarding valid dense glyphs.
            if white < 30 {
                continue;
            }
            if rows[1] & 0x1fe == 0 {
                if let Some(options) = clipped_font.get(&rows.map(|r| r & 0x1fe)) {
                    let mut viable: Vec<char> = options
                        .iter()
                        .filter(|(pattern, _)| {
                            pattern
                                .iter()
                                .zip(rows)
                                .all(|(a, b)| (a & 0x1ff) == (b & 0x1ff))
                        })
                        .map(|(_, ch)| *ch)
                        .collect();
                    viable.sort_unstable();
                    viable.dedup();
                    if !viable.is_empty() {
                        clipped[y][x] = Some(viable);
                    }
                }
            }
            if let Some(options) = font.get(&rows.map(|r| r & 0x1fe)) {
                let mut viable: Vec<char> = options
                    .iter()
                    // The last column may contain the next glyph's left
                    // overhang. The other nine columns must match exactly;
                    // accepting extra ink there makes ロ look like コ and U
                    // look like J when the word is absent from the lexicon.
                    .filter(|(pattern, _)| {
                        pattern
                            .iter()
                            .zip(rows)
                            .all(|(a, b)| (a & 0x1ff) == (b & 0x1ff))
                    })
                    .map(|(_, ch)| *ch)
                    .collect();
                viable.sort_unstable();
                viable.dedup();
                if !viable.is_empty() {
                    found[y][x] = Some(viable);
                }
            }
        }
    }
    let mut lines = Vec::new();
    for y in 0..=WIDTH - 9 {
        let blank_cell = |start: usize| {
            start + 9 <= WIDTH
                && (y..y + 9).all(|yy| {
                    (start..start + 9).all(|cx| {
                        let p = pixels[yy * WIDTH + cx];
                        !ink[yy][cx]
                            && (p >> 16) & 255 >= 215
                            && (p >> 8) & 255 >= 215
                            && p & 255 >= 185
                    })
                })
        };
        for x in 0..=WIDTH - 10 {
            if found[y][x].is_none() || (x >= 9 && found[y][x - 9].is_some()) {
                continue;
            }
            let mut previous = x;
            let mut continues_previous = false;
            while previous >= 9 && x - previous < 18 && blank_cell(previous - 9) {
                previous -= 9;
                if previous >= 9 && found[y][previous - 9].is_some() {
                    continues_previous = true;
                    break;
                }
            }
            if continues_previous {
                continue;
            }
            let mut cells = Vec::new();
            let mut xx = x;
            while xx <= WIDTH - 10 {
                match &found[y][xx] {
                    Some(chars) => cells.push(chars.clone()),
                    None => {
                        let mut next = xx;
                        while next <= WIDTH - 10 && next - xx < 18 && blank_cell(next) {
                            next += 9;
                        }
                        if cells.is_empty() || next > WIDTH - 10 || found[y][next].is_none() {
                            break;
                        }
                        for _ in 0..(next - xx) / 9 {
                            cells.push(vec![' ']);
                        }
                        xx = next;
                        continue;
                    }
                }
                xx += 9;
            }
            // A single name character on the status card is directly above
            // the larger species name. Require that independent layout cue;
            // isolated glyph-shaped decoration elsewhere is not enough.
            let one_character_name = cells.len() == 1
                && large_lines.iter().any(|(ly, lx, _)| {
                    *ly > y && (9..=20).contains(&(*ly - y)) && x.abs_diff(*lx) <= 2
                });
            // Dialogue can reveal one character at a time. A speech bubble
            // supplies independent context: solid borders above and below the
            // white text band at the same horizontal position.
            let border = |row: usize| (x..x + 9).all(|cx| ink[row][cx]);
            let one_character_dialogue = cells.len() == 1
                && (y.saturating_sub(7)..y).any(border)
                && (y + 9..(y + 16).min(WIDTH)).any(border);
            // Adjacent dark ink on a light background may be an unknown
            // glyph. Do not present the recognized part as a complete name.
            let suspicious = |start: usize, stop: usize| {
                let mut dark = 0;
                let mut other = 0;
                for yy in y..y + 9 {
                    for cx in start..stop {
                        if ink[yy][cx] {
                            dark += 1;
                        } else {
                            let p = pixels[yy * WIDTH + cx];
                            if (p >> 16) & 255 < 215 || (p >> 8) & 255 < 215 || p & 255 < 185 {
                                other += 1;
                            }
                        }
                    }
                }
                dark > 4 && other == 0
            };
            if suspicious(x.saturating_sub(9), x) || suspicious(xx.min(WIDTH), (xx + 9).min(WIDTH))
            {
                continue;
            }
            if cells.len() >= 2 || one_character_name || one_character_dialogue {
                if let Some(word) = choose_small_word(&cells) {
                    if word.chars().any(char::is_alphanumeric) {
                        lines.push((y, x, word));
                    }
                }
            }
        }
    }
    // On family cards the generation number sits directly above the small
    // 代目 label, unlike the inline generation value on the status page.
    let mut merged_numbers = Vec::new();
    for line in &mut lines {
        let (unit_y, unit_x, text) = line;
        if text != "代目" || *unit_y < 9 {
            continue;
        }
        let mut candidates = Vec::new();
        for y in unit_y.saturating_sub(12)..=unit_y.saturating_sub(9) {
            for x in unit_x.saturating_sub(18)..=(*unit_x + 18).min(WIDTH - 10) {
                if x >= 9 && found[y][x - 9].is_some() {
                    continue;
                }
                let mut xx = x;
                let mut digits = String::new();
                while xx <= WIDTH - 10 {
                    let Some(options) = &found[y][xx] else { break };
                    let [digit] = options.as_slice() else { break };
                    if !digit.is_ascii_digit() {
                        break;
                    }
                    digits.push(*digit);
                    xx += 9;
                }
                if !digits.is_empty() {
                    candidates.push((y, x, digits));
                }
            }
        }
        if let [(y, x, digits)] = candidates.as_slice() {
            merged_numbers.push((*y, *x, digits.clone()));
            *unit_y = *y;
            *unit_x = *x;
            *text = format!("{digits}代目");
        }
    }
    lines.retain(|(y, x, text)| {
        !merged_numbers
            .iter()
            .any(|(dy, dx, digits)| y == dy && x == dx && text == digits)
    });
    // Some profile-panel lines are clipped one pixel at their top edge.
    // Accept this weaker match only for a known whole word immediately below
    // an independently matched small-font line.
    for y in 0..=WIDTH - 9 {
        for x in 0..=WIDTH - 10 {
            if clipped[y][x].is_none() || (x >= 9 && clipped[y][x - 9].is_some()) {
                continue;
            }
            if !lines
                .iter()
                .any(|(py, px, _)| y > *py && (9..=12).contains(&(y - py)) && x.abs_diff(*px) <= 1)
            {
                continue;
            }
            let mut cells = Vec::new();
            let mut xx = x;
            while xx <= WIDTH - 10 {
                match &clipped[y][xx] {
                    Some(chars) => cells.push(chars.clone()),
                    None => {
                        // Punctuation may be inset one pixel in this panel.
                        if xx + 1 <= WIDTH - 10 {
                            let marks: Vec<char> = clipped[y][xx + 1]
                                .as_ref()
                                .into_iter()
                                .flatten()
                                .copied()
                                .filter(|c| *c == '！' || *c == '？')
                                .collect();
                            if !marks.is_empty() {
                                cells.push(marks);
                            }
                        }
                        break;
                    }
                }
                xx += 9;
            }
            if cells.len() >= 3 {
                if let Some(word) = choose_small_word(&cells) {
                    if local_words().contains(&word) {
                        lines.push((y, x, word));
                    }
                }
            }
        }
    }
    lines
}

// The clock uses separate date and time fonts. Locate an entire AM/PM marker
// first, then decode the adjacent fields; blinking cells are left unread.
fn clock_lines(ink: &[[bool; WIDTH]; WIDTH]) -> Vec<(usize, usize, String)> {
    #[derive(Clone)]
    struct Glyph {
        text: String,
        kind: String,
        rows: Vec<Vec<bool>>,
    }
    static FONT: OnceLock<Vec<Glyph>> = OnceLock::new();
    let font = FONT.get_or_init(|| {
        let entries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-clock-glyphs.json"
        ))
        .expect("clock font data");
        entries
            .into_iter()
            .map(|v| Glyph {
                text: v["char"].as_str().unwrap().to_owned(),
                kind: v["size"].as_str().unwrap().to_owned(),
                rows: v["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r.as_str().unwrap().bytes().map(|p| p == b'#').collect())
                    .collect(),
            })
            .collect()
    });
    let matches = |g: &Glyph, x: usize, y: usize| -> bool {
        y + g.rows.len() <= WIDTH
            && x + g.rows[0].len() <= WIDTH
            && g.rows.iter().enumerate().all(|(dy, row)| {
                row.iter()
                    .enumerate()
                    .all(|(dx, p)| ink[y + dy][x + dx] == *p)
            })
    };
    let digit = |kind: &str, x: usize, y: usize| -> Option<char> {
        let mut it = font.iter().filter(|g| g.kind == kind && matches(g, x, y));
        let first = it.next()?;
        if it.next().is_some() {
            None
        } else {
            first.text.chars().next()
        }
    };
    let blank = |x: usize, y: usize, w: usize, h: usize| -> bool {
        x + w <= WIDTH && y + h <= WIDTH && (y..y + h).all(|yy| (x..x + w).all(|xx| !ink[yy][xx]))
    };
    let mut out = Vec::new();
    for y in 13..WIDTH - 24 {
        for x in 3..WIDTH - 37 {
            let Some(marker) = font.iter().find(|g| g.kind == "ampm" && matches(g, x, y)) else {
                continue;
            };
            let hour_tens = digit("clock", x - 3, y + 13);
            let hour_ones = digit("clock", x + 6, y + 13);
            let minute_tens = digit("clock", x + 20, y + 13);
            let minute_ones = digit("clock", x + 29, y + 13);
            let first = hour_tens
                .map(|c| c.to_string())
                .or_else(|| blank(x - 3, y + 13, 8, 14).then(String::new));
            if let (Some(first), Some(h), Some(m1), Some(m2)) =
                (first.as_deref(), hour_ones, minute_tens, minute_ones)
            {
                let hour = format!("{first}{h}");
                let minute = format!("{m1}{m2}");
                if hour.parse::<u8>().is_ok_and(|v| (1..=12).contains(&v))
                    && minute.parse::<u8>().is_ok_and(|v| v < 60)
                {
                    let mut time = format!("{} {hour}:{minute}", marker.text);
                    if let (Some(s1), Some(s2)) =
                        (digit("date", x + 40, y + 17), digit("date", x + 47, y + 17))
                    {
                        let seconds = format!("{s1}{s2}");
                        if seconds.parse::<u8>().is_ok_and(|v| v < 60) {
                            time.push_str(&format!(":{seconds}"));
                        }
                    }
                    out.push((y, x, time));
                }
            } else if blank(x - 3, y + 13, 8, 14) && blank(x + 6, y + 13, 8, 14) {
                if let (Some(m1), Some(m2)) = (minute_tens, minute_ones) {
                    let minute = format!("{m1}{m2}");
                    if minute.parse::<u8>().is_ok_and(|v| v < 60) {
                        out.push((y, x, format!("{} :{minute}", marker.text)));
                    }
                }
            } else if blank(x + 20, y + 13, 8, 14) && blank(x + 29, y + 13, 8, 14) {
                if let (Some(first), Some(h)) = (first.as_deref(), hour_ones) {
                    let hour = format!("{first}{h}");
                    if hour.parse::<u8>().is_ok_and(|v| (1..=12).contains(&v)) {
                        out.push((y, x, format!("{} {hour}:", marker.text)));
                    }
                }
            }
            let month_tens = digit("date", x - 1, y - 13);
            let month_ones = digit("date", x + 6, y - 13);
            let day_tens = digit("date", x + 18, y - 13);
            let day_ones = digit("date", x + 26, y - 13);
            let mfirst = month_tens
                .map(|c| c.to_string())
                .or_else(|| blank(x - 1, y - 13, 6, 10).then(String::new));
            let dfirst = day_tens
                .map(|c| c.to_string())
                .or_else(|| blank(x + 18, y - 13, 6, 10).then(String::new));
            if let (Some(mfirst), Some(m), Some(dfirst), Some(d)) =
                (mfirst.as_deref(), month_ones, dfirst.as_deref(), day_ones)
            {
                let month = format!("{mfirst}{m}");
                let day = format!("{dfirst}{d}");
                if month.parse::<u8>().is_ok_and(|v| (1..=12).contains(&v))
                    && day.parse::<u8>().is_ok_and(|v| (1..=31).contains(&v))
                {
                    out.push((y - 13, x - 1, format!("{month}/{day}")));
                }
            } else if blank(x - 1, y - 13, 6, 10) && blank(x + 6, y - 13, 6, 10) {
                if let (Some(dfirst), Some(d)) = (dfirst.as_deref(), day_ones) {
                    let day = format!("{dfirst}{d}");
                    if day.parse::<u8>().is_ok_and(|v| (1..=31).contains(&v)) {
                        out.push((y - 13, x - 1, format!("/{day}")));
                    }
                }
            } else if blank(x + 18, y - 13, 6, 10) && blank(x + 26, y - 13, 6, 10) {
                if let (Some(mfirst), Some(m)) = (mfirst.as_deref(), month_ones) {
                    let month = format!("{mfirst}{m}");
                    if month.parse::<u8>().is_ok_and(|v| (1..=12).contains(&v)) {
                        out.push((y - 13, x - 1, format!("{month}/")));
                    }
                }
            }
        }
    }
    out
}

// White text on the Sweets cyan sidebar is laid out vertically. Match cropped
// glyphs against both foreground and background, then link adjacent glyphs.
fn vertical_lines(pixels: &[u32]) -> Vec<(usize, usize, String)> {
    struct Shape {
        ch: char,
        rows: Vec<Vec<bool>>,
    }
    static FONT: OnceLock<Vec<Shape>> = OnceLock::new();
    let font = FONT.get_or_init(|| {
        let entries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tools/fixtures/sweets-menu-glyphs.json"
        ))
        .unwrap();
        let mut out = Vec::new();
        for item in entries {
            let rows: Vec<Vec<bool>> = item["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().bytes().map(|c| c == b'#').collect())
                .collect();
            let points: Vec<(usize, usize)> = rows
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter()
                        .enumerate()
                        .filter_map(move |(x, p)| p.then_some((x, y)))
                })
                .collect();
            if points.is_empty() {
                continue;
            }
            let (x0, x1) = (
                points.iter().map(|p| p.0).min().unwrap(),
                points.iter().map(|p| p.0).max().unwrap(),
            );
            let (y0, y1) = (
                points.iter().map(|p| p.1).min().unwrap(),
                points.iter().map(|p| p.1).max().unwrap(),
            );
            if x1 - x0 + 1 < 8 || y1 - y0 + 1 < 8 {
                continue;
            }
            out.push(Shape {
                ch: item["char"].as_str().unwrap().chars().next().unwrap(),
                rows: rows[y0..=y1]
                    .iter()
                    .map(|row| row[x0..=x1].to_vec())
                    .collect(),
            });
        }
        out
    });
    let cyan = |p: u32| {
        let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
        (120..=145).contains(&r) && (210..=240).contains(&g) && b >= 245
    };
    let white = |p: u32| {
        let (r, g, b) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
        r >= 245 && g >= 245 && b >= 245
    };
    let mut candidates: Vec<(usize, usize, usize, char)> = Vec::new();
    for shape in font {
        let h = shape.rows.len();
        let w = shape.rows[0].len();
        for y in 0..=WIDTH - h {
            for x in 0..=WIDTH - w {
                // A cyan pixel just left of the glyph establishes the
                // sidebar background before attempting a costly comparison.
                if x == 0 || !cyan(pixels[y * WIDTH + x - 1]) {
                    continue;
                }
                let mut bg = 0usize;
                let mut ok = true;
                for (dy, row) in shape.rows.iter().enumerate() {
                    for (dx, want) in row.iter().enumerate() {
                        let p = pixels[(y + dy) * WIDTH + x + dx];
                        if *want {
                            if !white(p) {
                                ok = false;
                                break;
                            }
                        } else if cyan(p) {
                            bg += 1;
                        } else {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        break;
                    }
                }
                if ok && bg >= w * h / 5 {
                    candidates.push((y, x, h, shape.ch));
                }
            }
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    let mut best: Vec<Vec<usize>> = vec![Vec::new(); candidates.len()];
    for i in (0..candidates.len()).rev() {
        let (y, x, h, _) = candidates[i];
        let next = (i + 1..candidates.len())
            .filter(|&j| {
                let (ny, nx, _, _) = candidates[j];
                nx.abs_diff(x) <= 2 && ny >= y + h && ny <= y + h + 4
            })
            .max_by_key(|&j| {
                (
                    best[j].len(),
                    best[j].iter().map(|&k| candidates[k].2).sum::<usize>(),
                )
            });
        best[i].push(i);
        if let Some(j) = next {
            let tail = best[j].clone();
            best[i].extend(tail);
        }
    }
    best.sort_by_key(|path| {
        std::cmp::Reverse((
            path.len(),
            path.iter().map(|&k| candidates[k].2).sum::<usize>(),
        ))
    });
    let mut selected: Vec<(usize, usize, usize, String)> = Vec::new();
    for path in best {
        if path.len() < 3 {
            break;
        }
        let first = candidates[path[0]];
        let last = candidates[*path.last().unwrap()];
        let end = last.0 + last.2;
        if selected
            .iter()
            .any(|(y, x, bottom, _)| x.abs_diff(first.1) <= 4 && first.0 < *bottom && *y < end)
        {
            continue;
        }
        selected.push((
            first.0,
            first.1,
            end,
            path.iter().map(|&i| candidates[i].3).collect(),
        ));
    }
    selected
        .into_iter()
        .map(|(y, x, _, text)| (y, x, text))
        .collect()
}
