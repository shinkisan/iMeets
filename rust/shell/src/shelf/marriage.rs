//! Spouse builder. Signature addresses come from a scan of the firmware image.
//!
//! TODO(tamago-marriage-layout): SPEC §14.13. The 39334-byte message writes
//! the magic, the declared length, the kind-like byte 22 at offset 76, and
//! the checksum. Name, color, DNA, and preview offsets are not specified.
//! TODO(stashed-pc): the PC word in the first 64 CPU bytes is set to
//! `marriageEvent`, which is also stored in r15. The spec says the field is
//! adjusted and does not name a different address.
//! TODO(trampoline-word-order): the six words follow the order the spec
//! lists them: previous mode, address 545259776, then the four instruction words.
//! TODO(part-pixels): SPEC §14.14. A part file whose length equals the pixel
//! span is copied. No pixel codec is applied.

use crate::copy::{self, Lang};
use crate::shelf::slots::MemWrite;

pub const MESSAGE_ADDR: u32 = 545_063_936;
pub const MESSAGE_LEN: usize = 39_334;
pub const CHARACTER_ADDR: u32 = 543_096_832;
pub const CHARACTER_LEN: usize = 32_768;
pub const CPU_ADDR: u32 = 545_259_776;
pub const TRAMPOLINE_ADDR: u32 = 545_259_520;
pub const SEX_OFFSET: usize = 262;

pub const PARTS: [PartSpan; 4] = [
    PartSpan {
        key: 'B',
        count: 14,
        header: 16,
        pixel: 1024,
        end: 14416,
    },
    PartSpan {
        key: 'M',
        count: 19,
        header: 72,
        pixel: 14416,
        end: 21248,
    },
    PartSpan {
        key: 'A',
        count: 11,
        header: 148,
        pixel: 21248,
        end: 29184,
    },
    PartSpan {
        key: 'K',
        count: 7,
        header: 192,
        pixel: 29184,
        end: 32768,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct PartSpan {
    pub key: char,
    pub count: u8,
    pub header: usize,
    pub pixel: usize,
    pub end: usize,
}

impl PartSpan {
    pub fn span(self) -> usize {
        self.end - self.pixel
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sigs {
    pub parameter: u32,
    pub receive_type: u32,
    pub marriage_event: u32,
}

const CODE_END: usize = 2_097_152;
const PARAM_WORDS: [i32; 9] = [
    536_870_915,
    -670_356_645,
    1_241_909_401,
    844_306_514,
    5_994_512,
    1_074_995_718,
    213_272,
    1_075_322_883,
    1_187_006_320,
];
const MARRIAGE_WORDS: [i32; 12] = [
    -382_906_376,
    -442_556_416,
    -442_552_320,
    -444_391_424,
    -494_786_558,
    -442_552_320,
    -444_391_424,
    -476_041_215,
    -442_556_416,
    -440_258_560,
    -442_556_416,
    -440_258_560,
];
const MARRIAGE_MASKS: [i32; 12] = [
    -1, -4096, -4096, -1, -1, -4096, -1, -1, -4096, -1, -4096, -1,
];

fn missing() -> String {
    copy::text(Lang::En, "sigs_unavailable").to_string()
}

fn word(image: &[u8], at: usize) -> Option<i32> {
    let bytes = image.get(at..at + 4)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

fn half(image: &[u8], at: usize) -> Option<i32> {
    let bytes = image.get(at..at + 2)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?) as i32)
}

/// Top 20 bits `0xF80` select SRAM. The shelf offset is the low 20 bits.
fn sram_off(addr: i32) -> Result<u32, String> {
    if (addr & -1_048_576) == -134_217_728 {
        Ok((addr as u32) & 0x000F_FFFF)
    } else {
        Err(missing())
    }
}

/// PC-relative load. The pool word sits 8 bytes after the instruction, plus
/// the low 12 bits. After the top-16 mask only `0xE59F0000` can match.
fn literal(image: &[u8], at: usize) -> Result<i32, String> {
    let insn = word(image, at).ok_or_else(missing)?;
    let top = insn & -65_536;
    if top == -442_564_608 || top == -442_556_416 || top == -442_552_320 {
        let pool = at + 8 + (insn as u32 & 4095) as usize;
        word(image, pool).ok_or_else(missing)
    } else {
        Err(missing())
    }
}

fn matches_at(image: &[u8], at: usize, words: &[i32], masks: Option<&[i32]>) -> bool {
    words.iter().enumerate().all(|(n, expected)| {
        let Some(got) = word(image, at + n * 4) else {
            return false;
        };
        let mask = masks.map(|m| m[n]).unwrap_or(-1);
        got & mask == *expected
    })
}

/// One hit inside the first 2 MiB. The cursor advances by `step` after a
/// partial match as well as after a hit.
fn find(image: &[u8], words: &[i32], masks: Option<&[i32]>, step: usize) -> Result<usize, String> {
    let Some(span) = words.len().checked_mul(4) else {
        return Err(missing());
    };
    if words.is_empty() || step == 0 || span > CODE_END || image.len() < CODE_END {
        return Err(missing());
    }
    if masks.is_some_and(|m| m.len() != words.len()) {
        return Err(missing());
    }
    let last = CODE_END - span;
    let mut hit = None;
    let mut at = 0usize;
    while at <= last {
        if matches_at(image, at, words, masks) {
            if hit.is_some() {
                return Err(missing());
            }
            hit = Some(at);
        }
        at += step;
    }
    hit.ok_or_else(missing)
}

fn receive_type(image: &[u8], hit: usize, marriage_event: i32) -> Result<u32, String> {
    if hit < 14 {
        return Err(missing());
    }
    let mut found: Option<u32> = None;
    let mut site = hit - 8192.min(hit);
    while site < hit - 14 {
        match receive_at(image, site, marriage_event)? {
            None => {}
            Some(value) => {
                if found.is_some_and(|prev| prev != value) {
                    return Err(missing());
                }
                found = Some(value);
            }
        }
        site += 2;
    }
    found.ok_or_else(missing)
}

fn receive_at(image: &[u8], site: usize, marriage_event: i32) -> Result<Option<u32>, String> {
    let Some(h0) = half(image, site) else {
        return Ok(None);
    };
    let Some(h2) = half(image, site + 2) else {
        return Ok(None);
    };
    let Some(h4) = half(image, site + 4) else {
        return Ok(None);
    };
    let Some(h6) = half(image, site + 6) else {
        return Ok(None);
    };
    let Some(h12) = half(image, site + 12) else {
        return Ok(None);
    };
    if (h0 & 0xFF00) != 19_200
        || h2 != 30_875
        || h4 != 11_009
        || (h6 & 0xFF00) != 53_504
        || (h12 & 0xFF00) != 19_200
    {
        return Ok(None);
    }
    let event_at = ((site + 16) & !3) + (h12 as usize & 255) * 4;
    if word(image, event_at).ok_or_else(missing)? != marriage_event {
        return Ok(None);
    }
    let ptr_at = ((site + 4) & !3) + (h0 as usize & 255) * 4;
    let off = sram_off(word(image, ptr_at).ok_or_else(missing)?)?;
    Ok(Some(off + 3))
}

fn pair_pattern() -> ([i32; 36], [i32; 36]) {
    const SIZES: [i32; 9] = [2048, 2048, 16_384, 8192, 8192, 8192, 16_384, 32_768, 65_536];
    const IDS: [i32; 9] = [5218, 5219, 1415, 2151, 3948, 1397, 2159, 625, 626];
    let mut words = [0i32; 36];
    let mut masks = [0i32; 36];
    for i in 0..9 {
        words[i * 4] = SIZES[i];
        words[i * 4 + 1] = IDS[i];
        masks[i * 4] = -1;
        masks[i * 4 + 1] = -1;
    }
    (words, masks)
}

fn ramp_words() -> [i32; 137] {
    let mut words = [0i32; 137];
    for i in 0..137 {
        words[i] = if i < 128 {
            (65_536 * i) as i32
        } else {
            ((i - 127) * 4096 + 8_323_072) as i32
        };
    }
    words
}

fn gate_words() -> [i32; 10] {
    let mut bytes = [0u8; 40];
    for i in 0..23 {
        bytes[i] = (i + 54) as u8;
    }
    for i in 0..16 {
        bytes[i + 24] = (i + 37) as u8;
    }
    let mut words = [0i32; 10];
    for i in 0..10 {
        words[i] = i32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    }
    words
}

/// Locate `parameter`, `receiveType`, and `marriageEvent` in a firmware image.
/// An image shorter than 2 MiB, a second hit, or a failed SRAM check is rejected.
pub fn locate_firmware(image: &[u8]) -> Result<Sigs, String> {
    if image.len() < CODE_END {
        return Err(missing());
    }
    let param_at = find(image, &PARAM_WORDS, None, 2)?;
    let parameter = sram_off(word(image, param_at + 36).ok_or_else(missing)?)?;
    let hit = find(image, &MARRIAGE_WORDS, Some(&MARRIAGE_MASKS), 4)?;
    if literal(image, hit + 4)? != 544_932_864 {
        return Err(missing());
    }
    if sram_off(literal(image, hit + 68)?)? != parameter {
        return Err(missing());
    }
    if word(image, hit + 72).ok_or_else(missing)? & -4096 != -439_144_448 {
        return Err(missing());
    }
    let marriage_event = 0x2000_0000u32.wrapping_add(hit as u32);
    let receive_type = receive_type(image, hit, marriage_event as i32)?;
    let (pairs, pair_masks) = pair_pattern();
    let _ = find(image, &pairs, Some(&pair_masks), 4)?;
    let ramp = ramp_words();
    let _ = find(image, &ramp, None, 4)?;
    let gate = gate_words();
    let _ = find(image, &gate, None, 1)?;
    Ok(Sigs {
        parameter,
        receive_type,
        marriage_event,
    })
}

const DNA: [[[u8; 8]; 4]; 4] = [
    [
        [0, 1, 10, 11, 8, 9, 2, 3],
        [8, 9, 2, 3, 0, 1, 10, 11],
        [0, 9, 10, 3, 8, 1, 2, 11],
        [8, 1, 2, 11, 0, 9, 10, 3],
    ],
    [
        [0, 1, 10, 11, 12, 9, 6, 3],
        [8, 9, 2, 3, 4, 1, 14, 11],
        [0, 9, 10, 3, 12, 1, 6, 11],
        [8, 1, 2, 11, 4, 9, 14, 3],
    ],
    [
        [0, 1, 10, 11, 12, 1, 2, 11],
        [8, 1, 2, 11, 4, 9, 10, 3],
        [0, 1, 10, 11, 12, 9, 2, 3],
        [8, 9, 2, 3, 4, 1, 10, 11],
    ],
    [
        [0, 9, 10, 3, 8, 1, 6, 11],
        [8, 1, 2, 11, 0, 9, 14, 3],
        [0, 1, 10, 11, 8, 9, 6, 3],
        [8, 9, 2, 3, 0, 1, 14, 11],
    ],
];

pub fn dna_row(n: u32) -> [u8; 8] {
    let n = n % 100;
    let group = (n / 25) as usize;
    let digit = n % 10;
    let col = if digit <= 3 { digit } else { (digit % 3) + 1 };
    DNA[group][col as usize]
}

pub fn dna_bytes(mode: u8, n: u32, random8: [u8; 8]) -> Result<[u8; 8], String> {
    match mode {
        0 => Ok(dna_row(n)),
        1 => Ok([8; 8]),
        2 => Ok(random8.map(|b| b % 12)),
        _ => Err("inheritance mode is 0, 1, or 2".into()),
    }
}

pub fn resolve_sex(arg: u8, random_bit: u8) -> Result<u8, String> {
    match arg {
        0 | 1 => Ok(arg),
        2 => Ok(random_bit & 1),
        _ => Err("sex is 0, 1, or random".into()),
    }
}

/// Low 8 bits of `sram[parameter + 82]`, the flag at `parameter + 12`, and
/// the state byte at `parameter + 4`.
pub fn growth_error(stage: u8, flag12: u8, state4: u8) -> Result<(), &'static str> {
    if state4 > 1 {
        return Err(copy::text(Lang::En, "invalid_pet"));
    }
    let pending = flag12 & 0x80 != 0;
    match stage {
        4 => Ok(()),
        1 | 2 | 3 if pending => Err(copy::text(Lang::En, "evolution_pending")),
        0..=3 => Err(copy::text(Lang::En, "marriage_young")),
        _ => Err(copy::text(Lang::En, "growth_unavailable")),
    }
}

pub fn home_screen_ok(cpu: &[u8]) -> bool {
    if cpu.len() != 180 {
        return false;
    }
    let pc = u32::from_le_bytes(cpu[60..64].try_into().unwrap());
    let mode = u32::from_le_bytes(cpu[64..68].try_into().unwrap());
    (mode & 0x1F) == 19 && (mode & 128) == 0 && (0x2000_0000..0x2020_0000).contains(&pc)
}

pub fn validate_character(blob: &[u8]) -> Result<(), String> {
    if blob.len() != CHARACTER_LEN {
        return Err("character must be 32768 bytes".into());
    }
    if blob[SEX_OFFSET] > 1 {
        return Err("character sex byte must be 0 or 1".into());
    }
    Ok(())
}

pub fn build_character(
    indices: [u8; 4],
    pixels: [Option<&[u8]>; 4],
    sex: u8,
) -> Result<Vec<u8>, String> {
    if sex > 1 {
        return Err("character sex byte must be 0 or 1".into());
    }
    let mut blob = vec![0u8; CHARACTER_LEN];
    for (i, span) in PARTS.iter().enumerate() {
        if indices[i] >= span.count {
            return Err(format!("part {} index is out of range", span.key));
        }
        blob[span.header] = indices[i];
        if let Some(px) = pixels[i] {
            if px.len() != span.span() {
                // TODO(part-pixels): do not invent a decoder for other lengths.
                return Err(copy::text(Lang::En, "pose_unknown").into());
            }
            blob[span.pixel..span.end].copy_from_slice(px);
        }
    }
    blob[SEX_OFFSET] = sex;
    validate_character(&blob)?;
    Ok(blob)
}

pub fn spouse_message() -> Vec<u8> {
    let mut msg = vec![0u8; MESSAGE_LEN];
    msg[..6].copy_from_slice(b"TAMAGO");
    msg[72..76].copy_from_slice(&(MESSAGE_LEN as u32).to_be_bytes());
    msg[76] = 22;
    let sum = msg[..MESSAGE_LEN - 2]
        .iter()
        .fold(0u32, |a, b| a.wrapping_add(*b as u32));
    let c = (sum & 0xFFFF) as u16;
    msg[MESSAGE_LEN - 2..].copy_from_slice(&c.to_be_bytes());
    msg
}

pub fn trampoline(previous_mode: u32) -> [u8; 24] {
    let words = [
        previous_mode,
        CPU_ADDR,
        (-442_564_600i32) as u32,
        (-517_410_816i32) as u32,
        (-442_511_356i32) as u32,
        (-392_298_497i32) as u32,
    ];
    let mut out = [0u8; 24];
    for (i, w) in words.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    out
}

#[derive(Clone, Debug)]
pub enum Edit {
    Write(MemWrite),
    Register { index: u8, value: u32 },
    Cpsr(u32),
}

#[derive(Debug)]
pub struct MarriagePlan {
    pub edits: Vec<Edit>,
    pub sex: u8,
    pub dna: [u8; 8],
}

pub struct Choices {
    pub parts: [u8; 4],
    pub pixels: [Option<Vec<u8>>; 4],
    pub colors: Vec<u8>,
    pub sex: u8,
    pub mode: u8,
    pub dna_n: u32,
    pub random_sex: u8,
    pub random_dna: [u8; 8],
}

pub fn marriage_plan(
    sram: &[u8],
    cpu: &[u8],
    sigs: Option<Sigs>,
    choices: &Choices,
) -> Result<MarriagePlan, String> {
    let Some(sigs) = sigs else {
        // Fail closed. Do not skip the growth gate and do not write.
        return Err(copy::text(Lang::En, "growth_unavailable").into());
    };
    if choices.colors.len() > 3 || choices.colors.iter().any(|c| *c > 12) {
        return Err("colors must be 0–12".into());
    }
    let p = sigs.parameter as usize;
    let stage = *sram
        .get(p + 82)
        .ok_or_else(|| copy::text(Lang::En, "growth_unavailable").to_string())?;
    let flag = *sram.get(p + 12).unwrap_or(&0);
    let state = *sram.get(p + 4).unwrap_or(&0xFF);
    growth_error(stage, flag, state).map_err(|e| e.to_string())?;
    if !home_screen_ok(cpu) {
        return Err(copy::text(Lang::En, "home_screen").into());
    }
    let sex = resolve_sex(choices.sex, choices.random_sex)?;
    let dna = dna_bytes(choices.mode, choices.dna_n, choices.random_dna)?;
    let px = [
        choices.pixels[0].as_deref(),
        choices.pixels[1].as_deref(),
        choices.pixels[2].as_deref(),
        choices.pixels[3].as_deref(),
    ];
    let character = build_character(choices.parts, px, sex)?;
    validate_character(&character)?;
    let message = spouse_message();
    // Colors, DNA, the TAMA name, and the preview are not given offsets.
    let _ = (choices.colors.as_slice(), dna);
    let mut cpu64 = cpu[..64].to_vec();
    cpu64[60..64].copy_from_slice(&sigs.marriage_event.to_le_bytes());
    let mode = u32::from_le_bytes(cpu[64..68].try_into().unwrap());
    let cpsr = mode & !0x20;
    let mut edits = vec![
        Edit::Write(MemWrite {
            address: MESSAGE_ADDR,
            bytes: message,
        }),
        Edit::Write(MemWrite {
            address: CHARACTER_ADDR,
            bytes: character,
        }),
        Edit::Write(MemWrite {
            address: CPU_ADDR,
            bytes: cpu64,
        }),
        Edit::Write(MemWrite {
            address: TRAMPOLINE_ADDR,
            bytes: trampoline(mode).to_vec(),
        }),
        Edit::Write(MemWrite {
            address: crate::shelf::slots::SRAM_BASE + sigs.receive_type,
            bytes: vec![22],
        }),
        Edit::Register {
            index: 14,
            value: TRAMPOLINE_ADDR,
        },
        Edit::Cpsr(cpsr),
        Edit::Register {
            index: 15,
            value: sigs.marriage_event,
        },
    ];
    let _ = &mut edits;
    Ok(MarriagePlan { edits, sex, dna })
}

pub fn stage_name(stage: u8) -> Option<&'static str> {
    match stage {
        0 => Some(copy::text(Lang::En, "stage_egg")),
        1 => Some(copy::text(Lang::En, "stage_baby")),
        2 => Some(copy::text(Lang::En, "stage_child")),
        3 => Some(copy::text(Lang::En, "stage_teen")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adult_cpu() -> Vec<u8> {
        let mut cpu = vec![0u8; 180];
        cpu[60..64].copy_from_slice(&0x2000_0048u32.to_le_bytes());
        cpu[64..68].copy_from_slice(&19u32.to_le_bytes());
        cpu
    }

    fn choices() -> Choices {
        Choices {
            parts: [0, 0, 0, 0],
            pixels: [None, None, None, None],
            colors: vec![1, 2, 12],
            sex: 2,
            mode: 0,
            dna_n: 0,
            random_sex: 1,
            random_dna: [0; 8],
        }
    }

    #[test]
    fn growth_dna_home_and_fail_closed_without_sigs() {
        assert_eq!(dna_row(0), [0, 1, 10, 11, 8, 9, 2, 3]);
        assert_eq!(dna_row(25), [8, 1, 2, 11, 4, 9, 14, 3]);
        assert_eq!(dna_row(99), [8, 1, 2, 11, 0, 9, 14, 3]);
        assert_eq!(dna_bytes(1, 0, [0; 8]).unwrap(), [8; 8]);
        assert!(dna_bytes(2, 0, [11, 13, 0, 1, 2, 3, 4, 5]).unwrap()[1] < 12);
        assert_eq!(growth_error(4, 0, 0), Ok(()));
        assert_eq!(growth_error(4, 0, 1), Ok(()));
        assert!(growth_error(2, 0x80, 0)
            .unwrap_err()
            .contains("evolution is pending"));
        assert!(growth_error(3, 0, 1).unwrap_err().contains("adult form"));
        assert!(growth_error(0, 0, 0).unwrap_err().contains("adult form"));
        assert_eq!(stage_name(0), Some("Egg"));
        assert_eq!(stage_name(3), Some("Teen"));
        assert!(growth_error(9, 0, 0).unwrap_err().contains("unavailable"));
        assert!(growth_error(4, 0, 2)
            .unwrap_err()
            .contains("invalid pet state"));

        let mut cpu = adult_cpu();
        assert!(home_screen_ok(&cpu));
        cpu[64] = 19 | 128;
        assert!(!home_screen_ok(&cpu));
        cpu = adult_cpu();
        cpu[60..64].copy_from_slice(&0x1999_0000u32.to_le_bytes());
        assert!(!home_screen_ok(&cpu));
        assert_eq!(home_screen_ok(&cpu[..10]).then_some(0), None);

        let err = marriage_plan(&[], &adult_cpu(), None, &choices()).unwrap_err();
        assert_eq!(err, "Pet growth state is unavailable.");

        let mut sram = vec![0u8; 256];
        sram[82] = 4;
        sram[4] = 0;
        let plan = marriage_plan(
            &sram,
            &adult_cpu(),
            Some(Sigs {
                parameter: 0,
                receive_type: 20,
                marriage_event: 0x2000_1000,
            }),
            &choices(),
        )
        .unwrap();
        assert_eq!(plan.sex, 1);
        assert_eq!(plan.dna, dna_row(0));
        assert!(plan.edits.len() >= 4);
        match &plan.edits[0] {
            Edit::Write(w) => {
                assert_eq!(w.address, MESSAGE_ADDR);
                assert_eq!(w.bytes.len(), MESSAGE_LEN);
                assert_eq!(&w.bytes[..6], b"TAMAGO");
                assert_eq!(w.bytes[76], 22);
                assert_eq!(
                    u32::from_be_bytes(w.bytes[72..76].try_into().unwrap()),
                    MESSAGE_LEN as u32
                );
                // Unknown interior stays zero. Offset 100 is not a specified field.
                assert_eq!(w.bytes[100], 0);
            }
            _ => panic!("message write"),
        }
        let words = trampoline(19);
        assert_eq!(u32::from_le_bytes(words[0..4].try_into().unwrap()), 19);
        assert_eq!(
            u32::from_le_bytes(words[4..8].try_into().unwrap()),
            CPU_ADDR
        );
        assert_eq!(
            u32::from_le_bytes(words[8..12].try_into().unwrap()),
            (-442_564_600i32) as u32
        );

        let mut young = sram.clone();
        young[82] = 1;
        young[12] = 0x80;
        let err = marriage_plan(
            &young,
            &adult_cpu(),
            Some(Sigs {
                parameter: 0,
                receive_type: 1,
                marriage_event: 1,
            }),
            &choices(),
        )
        .unwrap_err();
        assert!(err.contains("evolution is pending"));

        let blob = build_character([1, 2, 3, 4], [None, None, None, None], 0).unwrap();
        assert_eq!(blob[SEX_OFFSET], 0);
        assert_eq!(blob.len(), 32_768);
        assert!(validate_character(&blob).is_ok());
        let mut bad = blob.clone();
        bad[SEX_OFFSET] = 2;
        assert!(validate_character(&bad).is_err());
        assert!(build_character([0, 0, 0, 0], [Some(&[1, 2, 3]), None, None, None], 0).is_err());
    }

    fn put_word(image: &mut [u8], at: usize, value: i32) {
        image[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_half(image: &mut [u8], at: usize, value: u16) {
        image[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// A 2 MiB image with one of each shelf anchor. The marriage literal pool
    /// is placed past the 12-word site so the masked load can still match.
    fn planted() -> Vec<u8> {
        const PARAM_AT: usize = 0x1000;
        const MARRIAGE_AT: usize = 0x8000;
        const RECEIVE_AT: usize = MARRIAGE_AT - 0x100;
        const PAIRS_AT: usize = 0x1_1000;
        const RAMP_AT: usize = 0x1_2000;
        const GATE_AT: usize = 0x1_3000;
        let mut image = vec![0u8; CODE_END];
        for (n, word) in PARAM_WORDS.iter().enumerate() {
            put_word(&mut image, PARAM_AT + n * 4, *word);
        }
        put_word(&mut image, PARAM_AT + 36, 0xF800_0100u32 as i32);

        for (n, word) in MARRIAGE_WORDS.iter().enumerate() {
            put_word(&mut image, MARRIAGE_AT + n * 4, *word);
        }
        // Low 12 bits select the pool at hit+76. The mask ignores those bits.
        put_word(
            &mut image,
            MARRIAGE_AT + 4,
            (MARRIAGE_WORDS[1] & -4096) | 64,
        );
        put_word(&mut image, MARRIAGE_AT + 76, 544_932_864);
        put_word(&mut image, MARRIAGE_AT + 68, -442_564_608 | 32);
        put_word(&mut image, MARRIAGE_AT + 108, 0xF800_0100u32 as i32);
        put_word(&mut image, MARRIAGE_AT + 72, -439_144_448);

        put_half(&mut image, RECEIVE_AT, 0x4B03);
        put_half(&mut image, RECEIVE_AT + 2, 30_875);
        put_half(&mut image, RECEIVE_AT + 4, 11_009);
        put_half(&mut image, RECEIVE_AT + 6, 0xD100);
        put_half(&mut image, RECEIVE_AT + 12, 0x4B01);
        put_word(&mut image, RECEIVE_AT + 16, 0xF800_0014u32 as i32);
        put_word(
            &mut image,
            RECEIVE_AT + 20,
            (0x2000_0000 + MARRIAGE_AT as u32) as i32,
        );

        let (pairs, _) = pair_pattern();
        for (n, word) in pairs.iter().enumerate() {
            put_word(&mut image, PAIRS_AT + n * 4, *word);
        }
        for (n, word) in ramp_words().iter().enumerate() {
            put_word(&mut image, RAMP_AT + n * 4, *word);
        }
        let mut gate = [0u8; 40];
        for i in 0..23 {
            gate[i] = (i + 54) as u8;
        }
        for i in 0..16 {
            gate[i + 24] = (i + 37) as u8;
        }
        image[GATE_AT..GATE_AT + 40].copy_from_slice(&gate);
        image
    }

    #[test]
    fn locate_firmware_reads_planted_signatures() {
        const MARRIAGE_AT: usize = 0x8000;
        let image = planted();
        let sigs = locate_firmware(&image).unwrap();
        assert_eq!(sigs.parameter, 0x100);
        assert_eq!(sigs.receive_type, 23);
        assert_eq!(sigs.marriage_event, 0x2000_0000 + MARRIAGE_AT as u32);

        let mut duplicated = image.clone();
        for (n, word) in PARAM_WORDS.iter().enumerate() {
            put_word(&mut duplicated, 0x4000 + n * 4, *word);
        }
        put_word(&mut duplicated, 0x4000 + 36, 0xF800_0100u32 as i32);
        let err = locate_firmware(&duplicated).unwrap_err();
        assert!(err.contains("shelf signatures"));

        let mut broken_pool = image.clone();
        put_word(&mut broken_pool, MARRIAGE_AT + 108, 0xF800_0200u32 as i32);
        assert!(locate_firmware(&broken_pool).is_err());

        let mut broken_gate = image;
        broken_gate[0x1_3000] = 0;
        assert!(locate_firmware(&broken_gate).is_err());
        assert!(locate_firmware(&[0; 16])
            .unwrap_err()
            .contains("shelf signatures"));
        assert!(locate_firmware(&vec![0u8; CODE_END]).is_err());
    }
}
