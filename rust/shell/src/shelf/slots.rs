//! Download slots. RAM offsets are written at `0x20000000 + offset`.
//! Occupancy bits use an SRAM `parameter` from the shelf signature scan.
//! Food, wallpaper, game, and station do not need that scan.

use crate::copy::{self, Lang};
use crate::shelf::tamago::{self, Cat, Tamago};

pub const RAM_BASE: u32 = 0x2000_0000;
pub const SRAM_BASE: u32 = 0xF800_0000;
pub const EXTRA_FF: u32 = 545_193_984;
pub const EXTRA_FF_LEN: usize = 32_768;

#[derive(Clone, Copy, Debug)]
pub struct Table {
    pub base: usize,
    pub count: usize,
    pub size: usize,
    /// Absolute occupancy index of slot 0. None when the category has no bits.
    pub occ0: Option<u32>,
}

pub fn table(cat: Cat) -> Table {
    match cat {
        Cat::Meal => Table {
            base: 6_356_992,
            count: 20,
            size: 2048,
            occ0: None,
        },
        Cat::Snack => Table {
            base: 6_422_528,
            count: 20,
            size: 2048,
            occ0: None,
        },
        Cat::Accessory => Table {
            base: 6_553_600,
            count: 16,
            size: 8192,
            occ0: Some(37),
        },
        Cat::Item => Table {
            base: 6_823_936,
            count: 23,
            size: 8192,
            occ0: Some(54),
        },
        Cat::Wallpaper => Table {
            base: 7_208_960,
            count: 8,
            size: 16384,
            occ0: None,
        },
        Cat::Game => Table {
            base: 7_340_032,
            count: 2,
            size: 32768,
            occ0: None,
        },
        Cat::Station => Table {
            base: 7_405_568,
            count: 2,
            size: 65536,
            occ0: None,
        },
    }
}

#[derive(Clone, Debug)]
pub struct MemWrite {
    pub address: u32,
    pub bytes: Vec<u8>,
}

pub fn set_occ_bits(byte: u8, abs_index: u32) -> u8 {
    let shift = ((abs_index % 4) * 2) as u8;
    (byte & !(0b11 << shift)) | (1 << shift)
}

fn occ_byte_index(parameter: u32, abs_index: u32) -> usize {
    parameter as usize + 83 + (abs_index / 4) as usize
}

fn bits_clear(sram: &[u8], parameter: u32, abs_index: u32) -> bool {
    let i = occ_byte_index(parameter, abs_index);
    if i >= sram.len() {
        return false;
    }
    let shift = ((abs_index % 4) * 2) as u8;
    (sram[i] >> shift) & 0b11 == 0
}

pub fn place(
    ram: &[u8],
    sram: &[u8],
    resource: &Tamago,
    parameter: Option<u32>,
) -> Result<(usize, Vec<MemWrite>), String> {
    let table = table(resource.cat);
    if resource.bytes.len() > table.size {
        return Err("resource is larger than the slot".into());
    }
    if table.occ0.is_some() && parameter.is_none() {
        return Err(copy::text(Lang::En, "sigs_unavailable").into());
    }
    let prepared = tamago::prepare(resource.clone());
    let mut empty: Option<usize> = None;
    for slot in 0..table.count {
        let off = table.base + slot * table.size;
        if off + table.size > ram.len() {
            return Err(copy::text(Lang::En, "sigs_unavailable").into());
        }
        let cell = &ram[off..off + table.size];
        let abs = table.occ0.map(|b| b + slot as u32);
        if cell.first() == Some(&b'T') && cell.len() >= 92 {
            let kind = i32::from_be_bytes(cell[78..82].try_into().unwrap());
            let id = u16::from_be_bytes(cell[90..92].try_into().unwrap());
            if kind == prepared.normalized && id == prepared.id {
                return Err(copy::text(Lang::En, "already_sent").into());
            }
        }
        let blank = cell.iter().all(|b| *b == 0xFF);
        let bits_ok = match abs {
            Some(i) => bits_clear(sram, parameter.unwrap(), i),
            None => true,
        };
        if blank && bits_ok && empty.is_none() {
            empty = Some(slot);
        }
    }
    let Some(slot) = empty else {
        return Err(copy::text(Lang::En, "slots_full").into());
    };
    let off = table.base + slot * table.size;
    let mut cell = vec![0xFF; table.size];
    cell[..prepared.bytes.len()].copy_from_slice(&prepared.bytes);
    let mut writes = vec![MemWrite {
        address: RAM_BASE + off as u32,
        bytes: cell,
    }];
    if let Some(base) = table.occ0 {
        let abs = base + slot as u32;
        let i = occ_byte_index(parameter.unwrap(), abs);
        let cur = sram.get(i).copied().unwrap_or(0);
        writes.push(MemWrite {
            address: SRAM_BASE + i as u32,
            bytes: vec![set_occ_bits(cur, abs)],
        });
    }
    if resource.cat.needs_extra_ff() {
        writes.push(MemWrite {
            address: EXTRA_FF,
            bytes: vec![0xFF; EXTRA_FF_LEN],
        });
    }
    Ok((slot, writes))
}

/// Little-endian balance word. Add clamps at 99999.
pub fn balance_word(current: u32, add: bool, amount: i64) -> Result<u32, &'static str> {
    if current > 99_999 || !(0..=99_999).contains(&amount) {
        return Err(copy::text(Lang::En, "balance_range"));
    }
    if add {
        Ok((current as u64 + amount as u64).min(99_999) as u32)
    } else {
        Ok(amount as u32)
    }
}

pub fn balance_bytes(word: u32) -> [u8; 4] {
    word.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shelf::tamago::build;

    fn region() -> (Vec<u8>, Vec<u8>) {
        (vec![0xFF; 8_388_608], vec![0u8; 4096])
    }

    #[test]
    fn duplicate_full_balance_and_food_without_parameter() {
        let (mut ram, sram) = region();
        let meal = build(-2_130_640_639, 9, 12, 516, &[]);
        let t = tamago::parse(&meal).unwrap();
        let (slot, writes) = place(&ram, &sram, &t, None).unwrap();
        assert_eq!(slot, 0);
        assert!(writes.iter().all(|w| w.address >= RAM_BASE));
        let dest = (writes[0].address - RAM_BASE) as usize;
        ram[dest..dest + writes[0].bytes.len()].copy_from_slice(&writes[0].bytes);
        let err = place(&ram, &sram, &t, None).unwrap_err();
        assert_eq!(err, "Already sent.");

        // Fill every meal slot with a different id.
        let (mut ram, sram) = region();
        let meals = table(Cat::Meal);
        for i in 0..meals.count {
            let off = meals.base + i * meals.size;
            ram[off] = b'T';
            ram[off + 78..off + 82].copy_from_slice(&(-2_130_640_383i32).to_be_bytes());
            ram[off + 90..off + 92].copy_from_slice(&(1000 + i as u16).to_be_bytes());
        }
        let err = place(&ram, &sram, &t, None).unwrap_err();
        assert_eq!(err, "download slots are full; tidy them in the game");

        let acc = build(-2_130_562_048, 3, 0, 516, &[]);
        let a = tamago::parse(&acc).unwrap();
        assert!(place(&ram, &sram, &a, None)
            .unwrap_err()
            .contains("shelf signatures"));
        let (_slot, writes) = place(&vec![0xFF; 8_388_608], &sram, &a, Some(0)).unwrap();
        assert!(writes
            .iter()
            .any(|w| w.address == SRAM_BASE + (83 + 37 / 4) as u32));
        assert_eq!(set_occ_bits(0, 37), 1 << ((37 % 4) * 2));
        assert_eq!(set_occ_bits(0xFF, 0) & 0b11, 0b01);

        assert_eq!(balance_word(10, true, 20).unwrap(), 30);
        assert_eq!(balance_word(99_990, true, 20).unwrap(), 99_999);
        assert_eq!(balance_word(1, false, 0).unwrap(), 0);
        assert_eq!(balance_word(1, false, 99_999).unwrap(), 99_999);
        assert_eq!(
            balance_word(100_000, false, 1),
            Err("balance must be 0–99999")
        );
        assert_eq!(balance_word(1, false, -1), Err("balance must be 0–99999"));
        assert_eq!(
            balance_word(1, true, 100_000),
            Err("balance must be 0–99999")
        );
        assert_eq!(table(Cat::Meal).base, 6_356_992);
        assert_eq!(table(Cat::Snack).base, 6_422_528);
        assert_ne!(table(Cat::Meal).base, table(Cat::Snack).base);
    }
}
