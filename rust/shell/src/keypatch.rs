//! Optional "fast keys": shrink the firmware's key debounce.
//!
//! Sweets v062 runs three near-identical key scanners out of SRAM at
//! `0xF80048D0`, `0xF8004B2C` and `0xF8004D34`. Each one counts consecutive
//! samples where the key reads down and accepts the press at 40 (the scan
//! runs at 1 kHz, so ~40 ms), then pins the counter and needs 50 up-samples
//! before it re-arms. That is the whole of the 63 ms the button feels: 40 ms
//! of debounce plus the time the game takes to draw the new screen.
//!
//! The scanners are copies the firmware keeps in SRAM, so the patch is a
//! handful of immediate bytes there. Nothing on disk changes and the ROM the
//! user picked is untouched; the guard below refuses to write anything unless
//! every site still holds the exact instruction it expects, so other
//! firmwares are left alone.

use tg18::Machine;

/// SRAM the firmware's code and pointers live in.
const SRAM_BASE: u32 = 0xF800_0000;

/// `cmp r3, #imm` immediates: (address, expected, replacement).
///
/// 40 -> 2 samples for the press, 39 -> 1 for its companion branch (the
/// counter saturates once it is past the accept point) and 49 -> 9 for the
/// release, which only bounds how fast the same key can be tapped again.
const SITES: [(u32, u8, u8); 9] = [
    (0xF800_490E, 0x28, 0x02),
    (0xF800_4916, 0x27, 0x01),
    (0xF800_4A08, 0x31, 0x09),
    (0xF800_4B6A, 0x28, 0x02),
    (0xF800_4B72, 0x27, 0x01),
    (0xF800_4C7A, 0x31, 0x09),
    (0xF800_4D70, 0x28, 0x02),
    (0xF800_4D78, 0x27, 0x01),
    (0xF800_4E5E, 0x31, 0x09),
];

fn byte(m: &Machine, addr: u32) -> Option<u8> {
    let o = addr.checked_sub(SRAM_BASE)? as usize;
    m.sys.sram.get(o).copied()
}

/// True when every scanner site holds the instruction this patch knows.
pub fn recognized(m: &Machine) -> bool {
    SITES
        .iter()
        .all(|&(addr, want, _)| byte(m, addr) == Some(want))
}

/// True when the patch is in place (either just applied or already there).
pub fn apply(m: &mut Machine) -> bool {
    if SITES
        .iter()
        .all(|&(addr, _, new)| byte(m, addr) == Some(new))
    {
        return true;
    }
    if !recognized(m) {
        return false;
    }
    for &(addr, _, new) in SITES.iter() {
        let o = (addr - SRAM_BASE) as usize;
        m.sys.sram[o] = new;
    }
    true
}

/// Put the original immediates back. The firmware's sleep/wake path needs
/// them: with the press accepted after two samples, a wake press lands while
/// the toy is still powering up and it goes straight back to sleep.
pub fn restore(m: &mut Machine) -> bool {
    if SITES
        .iter()
        .all(|&(addr, want, _)| byte(m, addr) == Some(want))
    {
        return true;
    }
    if !SITES
        .iter()
        .all(|&(addr, _, new)| byte(m, addr) == Some(new))
    {
        return false;
    }
    for &(addr, want, _) in SITES.iter() {
        let o = (addr - SRAM_BASE) as usize;
        m.sys.sram[o] = want;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom_machine() -> Machine {
        // A machine with no firmware: the guard must refuse it.
        let image = vec![0u8; 0x1000];
        Machine::new(image.clone().into(), &image, 0.0)
    }

    #[test]
    fn a_machine_without_the_scan_routines_is_left_alone() {
        let mut m = rom_machine();
        assert!(!recognized(&m));
        assert!(!apply(&mut m));
        assert_eq!(byte(&m, 0xF800_490E), Some(0x00));
    }

    #[test]
    fn the_scanner_bytes_are_rewritten_only_when_all_match() {
        let mut m = rom_machine();
        for &(addr, want, _) in SITES.iter() {
            m.sys.sram[(addr - SRAM_BASE) as usize] = want;
        }
        assert!(recognized(&m));
        assert!(apply(&mut m));
        for &(addr, _, new) in SITES.iter() {
            assert_eq!(byte(&m, addr), Some(new));
        }
        // Idempotent.
        assert!(apply(&mut m));
        // And reversible, so the sleep/wake path can run unpatched.
        assert!(restore(&mut m));
        for &(addr, want, _) in SITES.iter() {
            assert_eq!(byte(&m, addr), Some(want));
        }
        assert!(restore(&mut m));
    }
}
