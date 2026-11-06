//! Patch one SRAM byte and save, for character/gender experiments.
//!
//! Usage: POCKET_MEETS_ROM=... irpatchid <save> <offset> <value>

use pocket_meets::session::Session;
use std::path::Path;
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let off = usize::from_str_radix(
        std::env::args()
            .nth(2)
            .expect("offset")
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let val = u8::from_str_radix(
        std::env::args()
            .nth(3)
            .expect("value")
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let old = s.machine.sys.sram[off];
    s.machine.sys.sram[off] = val;
    let _ = s.save();
    println!("{off:05X}: {old:02X} -> {val:02X}");
}
