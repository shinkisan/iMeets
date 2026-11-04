//! TAMAGO gift files. Only the fields named in the spec are interpreted.
//!
//! TODO(tamago-layout): SPEC §14.13. Bytes other than the magic, declared
//! length, kind, id, price, and checksum are copied through unchanged.

use crate::copy::{self, Lang};

pub const MIN_LEN: usize = 516;
pub const PREVIEW_ACCESSORY: usize = 1478;
pub const PREVIEW_OTHER: usize = 516;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    Meal,
    Snack,
    Accessory,
    Item,
    Wallpaper,
    Game,
    Station,
}

impl Cat {
    pub fn tab(self) -> &'static str {
        match self {
            Cat::Meal | Cat::Snack => "food",
            Cat::Accessory => "accessory",
            Cat::Item => "item",
            Cat::Wallpaper => "wallpaper",
            Cat::Game => "game",
            Cat::Station => "station",
        }
    }

    pub fn occupancy(self) -> bool {
        matches!(self, Cat::Accessory | Cat::Item)
    }

    pub fn needs_extra_ff(self) -> bool {
        matches!(self, Cat::Game | Cat::Station)
    }

    pub fn shows_price(self) -> bool {
        matches!(self, Cat::Meal | Cat::Snack | Cat::Wallpaper)
    }
}

#[derive(Clone, Debug)]
pub struct Tamago {
    pub bytes: Vec<u8>,
    pub kind: i32,
    pub normalized: i32,
    pub cat: Cat,
    pub id: u16,
    pub price: u16,
}

pub fn checksum(data: &[u8]) -> u16 {
    if data.len() < 2 {
        return 0;
    }
    let sum = data[..data.len() - 2].iter().fold(0u32, |a, b| a.wrapping_add(*b as u32));
    (sum & 0xFFFF) as u16
}

pub fn write_checksum(data: &mut [u8]) {
    let n = data.len();
    if n < 2 {
        return;
    }
    let c = checksum(data);
    data[n - 2..n].copy_from_slice(&c.to_be_bytes());
}

pub fn normalize(kind: i32) -> Option<(i32, Cat)> {
    Some(match kind {
        -2_130_640_639 | -2_130_640_383 => (-2_130_640_383, Cat::Meal),
        -2_130_640_638 | -2_130_637_566 => (-2_130_637_566, Cat::Snack),
        -2_130_561_792 | -2_130_562_048 => (-2_130_562_048, Cat::Accessory),
        -2_130_433_535 => (-2_130_433_535, Cat::Item),
        -2_130_239_744 => (-2_130_239_744, Cat::Wallpaper),
        -1_811_784_959 => (-1_811_784_959, Cat::Game),
        -1_811_790_078 => (-1_811_790_078, Cat::Station),
        _ => return None,
    })
}

pub fn parse(data: &[u8]) -> Result<Tamago, String> {
    if data.len() < MIN_LEN || &data[..6] != b"TAMAGO" {
        return Err("not a TAMAGO resource".into());
    }
    let declared = u32::from_be_bytes(data[72..76].try_into().unwrap());
    let kind = i32::from_be_bytes(data[78..82].try_into().unwrap());
    let (normalized, cat) = normalize(kind).ok_or_else(|| "not a TAMAGO resource".to_string())?;
    // Station files declare either the file length or length − 2. Both occur.
    let full = data.len() as u32;
    let length_ok = if cat == Cat::Station {
        declared == full || declared == full.saturating_sub(2)
    } else {
        declared == full
    };
    if !length_ok {
        return Err("not a TAMAGO resource".into());
    }
    let stored = u16::from_be_bytes(data[data.len() - 2..].try_into().unwrap());
    if stored != checksum(data) {
        return Err("not a TAMAGO resource".into());
    }
    let id = u16::from_be_bytes(data[90..92].try_into().unwrap());
    let price = u16::from_be_bytes(data[118..120].try_into().unwrap());
    Ok(Tamago { bytes: data.to_vec(), kind, normalized, cat, id, price })
}

/// Rewrite the normalized kind and the checksum. Other bytes stay.
pub fn prepare(mut t: Tamago) -> Tamago {
    t.bytes[78..82].copy_from_slice(&t.normalized.to_be_bytes());
    write_checksum(&mut t.bytes);
    t.kind = t.normalized;
    t
}

pub fn send_sentence(cat: Cat) -> &'static str {
    let key = if matches!(cat, Cat::Meal | Cat::Snack) { "food_sentence" } else { "chest_sentence" };
    copy::text(Lang::En, key)
}

pub fn preview_len(cat: Cat) -> Option<usize> {
    match cat {
        Cat::Game | Cat::Station => None,
        Cat::Accessory => Some(PREVIEW_ACCESSORY),
        _ => Some(PREVIEW_OTHER),
    }
}

/// Build a structurally valid TAMAGO. `body` is copied after the price field
/// when it fits. That copy is not a pixel codec.
pub fn build(cat_kind: i32, id: u16, price: u16, len: usize, body: &[u8]) -> Vec<u8> {
    let len = len.max(MIN_LEN);
    let mut data = vec![0u8; len];
    data[..6].copy_from_slice(b"TAMAGO");
    let declared = if normalize(cat_kind).map(|(_, c)| c == Cat::Station).unwrap_or(false) {
        (len as u32).saturating_sub(2)
    } else {
        len as u32
    };
    data[72..76].copy_from_slice(&declared.to_be_bytes());
    data[78..82].copy_from_slice(&cat_kind.to_be_bytes());
    data[90..92].copy_from_slice(&id.to_be_bytes());
    data[118..120].copy_from_slice(&price.to_be_bytes());
    let room = len.saturating_sub(122);
    let n = body.len().min(room);
    if n > 0 {
        data[120..120 + n].copy_from_slice(&body[..n]);
    }
    write_checksum(&mut data);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_kind_and_price() {
        let raw = build(-2_130_640_639, 0x1234, 50, 516, b"pixels");
        let t = parse(&raw).unwrap();
        assert_eq!(t.normalized, -2_130_640_383);
        assert_eq!(t.cat, Cat::Meal);
        assert_eq!(t.id, 0x1234);
        assert_eq!(t.price, 50);
        assert!(t.bytes[120..126].starts_with(b"pixels"));
        let placed = prepare(t);
        assert_eq!(i32::from_be_bytes(placed.bytes[78..82].try_into().unwrap()), placed.normalized);
        assert!(parse(&placed.bytes).is_ok());

        let mut bad = raw.clone();
        bad[10] ^= 0xFF;
        assert!(parse(&bad).is_err());

        let station = build(-1_811_790_078, 7, 0, 516, &[]);
        let s = parse(&station).unwrap();
        assert_eq!(s.cat, Cat::Station);
        assert_eq!(u32::from_be_bytes(station[72..76].try_into().unwrap()), 514);
        let mut full_len = station.clone();
        let n = full_len.len() as u32;
        full_len[72..76].copy_from_slice(&n.to_be_bytes());
        write_checksum(&mut full_len);
        let full = parse(&full_len).unwrap();
        assert_eq!(full.cat, Cat::Station);
        assert_eq!(full.id, 7);
        let mut wrong = station.clone();
        wrong[72..76].copy_from_slice(&1u32.to_be_bytes());
        write_checksum(&mut wrong);
        assert!(parse(&wrong).is_err());

        assert_eq!(send_sentence(Cat::Snack), "sent to the restaurant menu to be bought");
        assert_eq!(send_sentence(Cat::Wallpaper), "sent to your chest to use or wear");
        assert_eq!(send_sentence(Cat::Game), "sent to your chest to use or wear");
        assert_eq!(preview_len(Cat::Accessory), Some(1478));
        assert_eq!(preview_len(Cat::Item), Some(516));
        assert_eq!(preview_len(Cat::Game), None);
    }
}
