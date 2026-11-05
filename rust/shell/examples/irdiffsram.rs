//! Diff the pet-area SRAM of two saves.
//!
//! Usage: POCKET_MEETS_ROM=... irdiffsram <save-a> <save-b> [lo] [hi]

use pocket_meets::session::Session;
use std::path::Path;

fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let a = std::env::args().nth(1).expect("save a");
    let b = std::env::args().nth(2).expect("save b");
    let lo = std::env::args()
        .nth(3)
        .map(|v| usize::from_str_radix(v.trim_start_matches("0x"), 16).unwrap())
        .unwrap_or(0x0D000);
    let hi = std::env::args()
        .nth(4)
        .map(|v| usize::from_str_radix(v.trim_start_matches("0x"), 16).unwrap())
        .unwrap_or(0x0E000);
    let sa = Session::open_care(Path::new(&rom), Path::new(&a), false).unwrap();
    let sb = Session::open_care(Path::new(&rom), Path::new(&b), false).unwrap();
    let x = &sa.machine.sys.sram;
    let y = &sb.machine.sys.sram;
    for i in lo..hi {
        if x[i] != y[i] {
            println!("{i:05X}: {a}={:02X} {b}={:02X}", x[i], y[i]);
        }
    }
}
