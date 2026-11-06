//! Decodes the game message buffer. Codes are 16-bit little-endian in RAM;
//! the ROM stores the same values big-endian. Page 0x04 is kana, ordered by
//! gojuon, with voiced/small kana after the base table.

fn kana(code: u16) -> Option<&'static str> {
    // 0x045B is the centered dot used between menu words.
    if code == 0x045B {
        return Some("・");
    }
    Some(match code {
        0x0401 => "あ",
        0x0402 => "い",
        0x0403 => "う",
        0x0404 => "え",
        0x0405 => "お",
        0x0406 => "か",
        0x0407 => "き",
        0x0408 => "く",
        0x0409 => "け",
        0x040A => "こ",
        0x040B => "さ",
        0x040C => "し",
        0x040D => "す",
        0x040E => "せ",
        0x040F => "そ",
        0x0410 => "た",
        0x0411 => "ち",
        0x0412 => "つ",
        0x0413 => "て",
        0x0414 => "と",
        0x0415 => "な",
        0x0416 => "に",
        0x0417 => "ぬ",
        0x0418 => "ね",
        0x0419 => "の",
        0x041A => "は",
        0x041B => "ひ",
        0x041C => "ふ",
        0x041D => "へ",
        0x041E => "ほ",
        0x041F => "ま",
        0x0420 => "み",
        0x0421 => "む",
        0x0422 => "め",
        0x0423 => "も",
        0x0424 => "や",
        0x0425 => "ゆ",
        0x0426 => "よ",
        0x0427 => "ら",
        0x0428 => "り",
        0x0429 => "る",
        0x042A => "れ",
        0x042B => "ろ",
        0x042C => "わ",
        0x042D => "を",
        0x042E => "ん",
        0x042F => "ぁ",
        0x0430 => "ぃ",
        0x0431 => "ぅ",
        0x0432 => "ぇ",
        0x0433 => "ぉ",
        0x0434 => "っ",
        0x0435 => "ゃ",
        0x0436 => "ゅ",
        0x0437 => "ょ",
        0x0438 => "が",
        0x0439 => "ぎ",
        0x043A => "ぐ",
        0x043B => "げ",
        0x043C => "ご",
        0x043D => "ざ",
        0x043E => "じ",
        0x043F => "ず",
        0x0440 => "ぜ",
        0x0441 => "ぞ",
        0x0442 => "だ",
        0x0443 => "ぢ",
        0x0444 => "づ",
        0x0445 => "で",
        0x0446 => "ど",
        0x0447 => "ば",
        0x0448 => "び",
        0x0449 => "ぶ",
        0x044A => "べ",
        0x044B => "ぼ",
        0x044C => "ぱ",
        0x044D => "ぴ",
        0x044E => "ぷ",
        0x044F => "ぺ",
        0x0450 => "ぽ",
        0x0451 => "ー",
        0x046D => "0",
        0x046E => "1",
        0x046F => "2",
        0x0470 => "3",
        0x0471 => "4",
        0x0472 => "5",
        0x0473 => "6",
        0x0474 => "7",
        0x0475 => "8",
        0x0476 => "9",
        0x0452 => "〜",
        0x0453 => "…",
        0x0454 => "、",
        0x0455 => "。",
        0x0456 => "（",
        0x0457 => "）",
        0x0458 => "「",
        0x0459 => "」",
        0x045A => ".",
        0x045C => "！",
        0x045D => "？",
        0x045E => "＆",
        0x045F => "○",
        0x0460 => "×",
        0x0461 => "♥",
        0x0462 => "✿",
        0x0464 => "🌀",
        0x0466 => "✧",
        0x046B => "G",
        0x0463 => "★",
        0x0465 => "♪",
        0x0467 => "↑",
        0x0468 => "↓",
        0x0469 => "→",
        0x046A => "←",
        0x046C => " ",
        0x0477 => "ア",
        0x0478 => "イ",
        0x0479 => "ウ",
        0x047A => "エ",
        0x047B => "オ",
        0x047C => "カ",
        0x047D => "キ",
        0x047E => "ク",
        0x047F => "ケ",
        0x0480 => "コ",
        0x0481 => "サ",
        0x0482 => "シ",
        0x0483 => "ス",
        0x0484 => "セ",
        0x0485 => "ソ",
        0x0486 => "タ",
        0x0487 => "チ",
        0x0488 => "ツ",
        0x0489 => "テ",
        0x048A => "ト",
        0x048B => "ナ",
        0x048C => "ニ",
        0x048D => "ヌ",
        0x048E => "ネ",
        0x048F => "ノ",
        0x0490 => "ハ",
        0x0491 => "ヒ",
        0x0492 => "フ",
        0x0493 => "ヘ",
        0x0494 => "ホ",
        0x0495 => "マ",
        0x0496 => "ミ",
        0x0497 => "ム",
        0x0498 => "メ",
        0x0499 => "モ",
        0x049A => "ヤ",
        0x049B => "ユ",
        0x049C => "ヨ",
        0x049D => "ラ",
        0x049E => "リ",
        0x049F => "ル",
        0x04A0 => "レ",
        0x04A1 => "ロ",
        0x04A2 => "ワ",
        0x04A3 => "ヲ",
        0x04A4 => "ン",
        0x04A5 => "ァ",
        0x04A6 => "ィ",
        0x04A7 => "ゥ",
        0x04A8 => "ェ",
        0x04A9 => "ォ",
        0x04AA => "ッ",
        0x04AB => "ャ",
        0x04AC => "ュ",
        0x04AD => "ョ",
        0x04AE => "ガ",
        0x04AF => "ギ",
        0x04B0 => "グ",
        0x04B1 => "ゲ",
        0x04B2 => "ゴ",
        0x04B3 => "ザ",
        0x04B4 => "ジ",
        0x04B5 => "ズ",
        0x04B6 => "ゼ",
        0x04B7 => "ゾ",
        0x04B8 => "ダ",
        0x04B9 => "ヂ",
        0x04BA => "ヅ",
        0x04BB => "デ",
        0x04BC => "ド",
        0x04BD => "バ",
        0x04BE => "ビ",
        0x04BF => "ブ",
        0x04C0 => "ベ",
        0x04C1 => "ボ",
        0x04C2 => "パ",
        0x04C3 => "ピ",
        0x04C4 => "プ",
        0x04C5 => "ペ",
        0x04C6 => "ポ",
        0x04E0 => "A",
        0x04E1 => "B",
        0x04E2 => "C",
        0x04E3 => "D",
        0x04E4 => "E",
        0x04E5 => "F",
        0x04E6 => "G",
        0x04E7 => "H",
        0x04E8 => "I",
        0x04E9 => "J",
        0x04EA => "K",
        0x04EB => "L",
        0x04EC => "M",
        0x04ED => "N",
        0x04EE => "O",
        0x04EF => "P",
        0x04F0 => "Q",
        0x04F1 => "R",
        0x04F2 => "S",
        0x04F3 => "T",
        0x04F4 => "U",
        0x04F5 => "V",
        0x04F6 => "W",
        0x04F7 => "X",
        0x04F8 => "Y",
        0x04F9 => "Z",
        0x04C7 => "ヴ",
        // Pictograms are represented by approximate Unicode equivalents.
        0x04D0 => "😀",
        0x04D1 => "😣",
        0x04D2 => "😖",
        0x04D3 => "😑",
        0x04D4 => "😵",
        0x04D5 => "😢",
        0x04D6 => "🐰",
        0x04D7 => "🎮",
        0x04D8 => "🎂",
        0x04D9 => "🎁",
        0x04DA => "📱",
        0x04DB => "🏠",
        0x04DC => "👜",
        0x04DD => "🍙",
        0x04DE => "🍰",
        0x04DF => "✨",
        0x04FA => "%",
        0x04FB => "-",
        0x04FD => "ー",
        0x04FE => "↵",
        0x04FC => "+",
        // Extended Latin glyphs drawn by the Sweets firmware's 0x10 page.
        0x1001 => "É",
        0x1002 => "È",
        0x1003 => "Ê",
        0x1004 => "Ë",
        0x1005 => "Ï",
        0x1006 => "Ù",
        0x1007 => "Ü",
        0x1801 => "À",
        0x1802 => "Á",
        0x1803 => "Â",
        0x1804 => "Ã",
        0x1805 => "Ç",
        0x1806 => "É",
        0x1807 => "Ê",
        0x1808 => "Í",
        0x1809 => "Ó",
        0x180A => "Ô",
        0x180B => "Õ",
        0x180C => "Ú",
        0x1C01 => "Ä",
        0x1C02 => "Ö",
        0x1C03 => "Ü",
        0x2001 => "Ą",
        0x2002 => "Ć",
        0x2003 => "Ę",
        0x2004 => "Ł",
        0x2005 => "Ń",
        0x2006 => "Ó",
        0x2007 => "Ś",
        0x2008 => "Ź",
        0x2009 => "Ż",
        _ => return None,
    })
}

fn scan_at(sram: &[u8], start: usize) -> Option<String> {
    let hi = 0xE000.min(sram.len());
    if start + 4 > hi {
        return None;
    }
    let mut j = start;
    let mut codes = 0usize;
    while j + 2 <= hi {
        let w = u16::from_le_bytes([sram[j], sram[j + 1]]);
        // The Sweets renderer also accepts mapped extended-Latin pages.
        // Unknown non-kana pages remain terminators instead of being guessed.
        if w == 0x0400 || w <= 0x00ff || ((w >> 8) != 4 && kana(w).is_none()) {
            break;
        }
        codes += 1;
        j += 2;
    }
    if codes < 2 || j + 2 > hi {
        return None;
    }
    let term = u16::from_le_bytes([sram[j], sram[j + 1]]);
    if term != 0x0400 && term > 0x00ff {
        return None;
    }
    let mut out = String::new();
    let mut known = 0usize;
    for i in 0..codes {
        let at = start + i * 2;
        let w = u16::from_le_bytes([sram[at], sram[at + 1]]);
        match kana(w) {
            Some(ch) => {
                out.push_str(ch);
                known += 1;
            }
            None => out.push('□'),
        }
    }
    if known == 0 {
        return None;
    }
    Some(out.trim_matches('□').trim().to_string())
}

/// The game builds the current screen text in the 2-byte buffer at
/// 0xF800D700. The first byte is a header, so the codes normally start at
/// 0xF800D701. Try the four nearby offsets and keep the longest valid run.
pub fn message_text(sram: &[u8]) -> String {
    let mut best = String::new();
    for start in [0xD700usize, 0xD701, 0xD702, 0xD703] {
        if let Some(text) = scan_at(sram, start) {
            if text.chars().count() > best.chars().count() {
                best = text;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_small_kana_and_middle_dot() {
        let expected = ["ぁ", "ぃ", "ぅ", "ぇ", "ぉ", "っ", "ゃ", "ゅ", "ょ"];
        for (i, ch) in expected.iter().enumerate() {
            assert_eq!(kana(0x042F + i as u16), Some(*ch));
        }
        assert_eq!(kana(0x045B), Some("・"));
        assert_eq!(kana(0x046C), Some(" "));
    }

    #[test]
    fn decodes_known_buffers() {
        let mut sram = vec![0u8; 0x10000];
        let codes = [
            0x0419u16, 0x040B, 0x0403, 0x0401, 0x0401, 0x0401, 0x0401, 0x0401,
        ];
        for (i, c) in codes.iter().enumerate() {
            let at = 0xD701 + i * 2;
            sram[at..at + 2].copy_from_slice(&c.to_le_bytes());
        }
        sram[0xD701 + codes.len() * 2] = 1;
        assert_eq!(message_text(&sram), "のさうあああああ");
    }

    #[test]
    fn reads_player_name_with_zeroed_last_high_byte() {
        let mut sram = vec![0u8; 0xE000];
        let name = [0x0406u16, 0x0407, 0x046C, 0x0408];
        for (i, code) in name.iter().enumerate() {
            let at = 0xD8AF + i * 2;
            sram[at] = (code & 0xFF) as u8;
            sram[at + 1] = if i + 1 == name.len() {
                0
            } else {
                (code >> 8) as u8
            };
        }
        assert_eq!(player_name(&sram), "かき く");
        assert_eq!(player_name(&vec![0u8; 0xE000]), "");
    }

    #[test]
    fn decodes_mixed_kana_and_extended_latin() {
        let mut sram = vec![0u8; 0x10000];
        for (i, code) in [0x0406u16, 0x1001, 0x2004, 0x0402].iter().enumerate() {
            let at = 0xD701 + i * 2;
            sram[at..at + 2].copy_from_slice(&code.to_le_bytes());
        }
        sram[0xD701 + 8] = 1;
        assert_eq!(message_text(&sram), "かÉŁい");
    }
}

/// Player name from the Sweets game state. The field holds 16-bit character
/// codes terminated by zero; an unreadable field yields an empty string.
pub fn player_name(sram: &[u8]) -> String {
    const NAME_AT: usize = 0xD8AF;
    let mut out = String::new();
    let mut at = NAME_AT;
    while at + 1 < sram.len() && out.chars().count() < 12 {
        let raw = u16::from_le_bytes([sram[at], sram[at + 1]]);
        if raw == 0 {
            break;
        }
        // The field's last character keeps only its low byte; the high byte is
        // zeroed. Kana codes live on page 0x04.
        let code = if raw >> 8 == 0 { 0x0400 | raw } else { raw };
        match kana(code) {
            Some(ch) => out.push_str(ch),
            None => return String::new(),
        }
        at += 2;
    }
    out.trim().to_string()
}
