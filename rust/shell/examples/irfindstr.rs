//! Scan SRAM for a byte pattern.
//!
//! Usage: POCKET_MEETS_ROM=... irfindstr <save> "3c 04 3d 04 ..."

use pocket_meets::session::Session;
use std::path::Path;
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let pat: Vec<u8> = std::env::args()
        .nth(2)
        .expect("hex bytes")
        .split_whitespace()
        .map(|x| u8::from_str_radix(x, 16).unwrap())
        .collect();
    let s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let sram = &s.machine.sys.sram;
    let mut i = 0;
    while i + pat.len() <= sram.len() {
        if sram[i..i + pat.len()] == pat[..] {
            let lo = i.saturating_sub(32);
            let hi = (i + pat.len() + 64).min(sram.len());
            println!("hit {i:05X}: {:02X?}", &sram[lo..hi]);
            i += pat.len();
        } else {
            i += 1;
        }
    }
}
