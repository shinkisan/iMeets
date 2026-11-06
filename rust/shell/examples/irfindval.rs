//! Scan SRAM for a u16/u32 value in the pet/clock areas.
//!
//! Usage: POCKET_MEETS_ROM=... irfindval <save> <hex-value>

use pocket_meets::session::Session;
use std::path::Path;
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let val = u32::from_str_radix(
        std::env::args()
            .nth(2)
            .expect("value")
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let sram = &s.machine.sys.sram;
    for (lo, hi, label) in [
        (0x0D000usize, 0x0E000usize, "pet"),
        (0x0B000, 0x0C000, "clock"),
    ] {
        for i in lo..hi.saturating_sub(4) {
            let w = u32::from_le_bytes(sram[i..i + 4].try_into().unwrap());
            let h = u16::from_le_bytes(sram[i..i + 2].try_into().unwrap()) as u32;
            if w == val || h == val {
                println!(
                    "{label} {i:05X}: u32={w:08X} u16={h:04X} ctx={:02X?}",
                    &sram[i..i + 8]
                );
            }
        }
    }
}
