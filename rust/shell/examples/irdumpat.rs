//! Dump a byte range of SRAM as hex.
//!
//! Usage: POCKET_MEETS_ROM=... irdumpat <save-dir> <offset> <len>

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
    let len = usize::from_str_radix(
        std::env::args()
            .nth(3)
            .unwrap_or_else(|| "64".into())
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let sram = &s.machine.sys.sram;
    for row in 0..((len + 15) / 16) {
        let at = off + row * 16;
        let end = (at + 16).min(off + len);
        print!("{at:05X}:");
        for i in at..end {
            print!(" {:02X}", sram[i]);
        }
        println!();
    }
}
