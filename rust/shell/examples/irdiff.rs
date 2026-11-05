//! Diff the SRAM of two saves and print differing runs.
//!
//! Used to locate fields such as the gotchi points and the in-game clock.
//!
//! Usage: POCKET_MEETS_ROM=... irdiff <save-a> <save-b>

use pocket_meets::session::Session;
use std::path::Path;
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let pa = std::env::args().nth(1).expect("save A");
    let pb = std::env::args().nth(2).expect("save B");
    let a = Session::open_care(Path::new(&rom), Path::new(&pa), false).unwrap();
    let b = Session::open_care(Path::new(&rom), Path::new(&pb), false).unwrap();
    let sa = &a.machine.sys.sram;
    let sb = &b.machine.sys.sram;
    println!("sram len {} {}", sa.len(), sb.len());
    // Find locations where B holds 300 (0x012C) and A holds 0.
    for i in 0..sa.len().saturating_sub(4) {
        let wa = u32::from_le_bytes(sa[i..i + 4].try_into().unwrap());
        let wb = u32::from_le_bytes(sb[i..i + 4].try_into().unwrap());
        if wa == 0 && wb == 300 {
            println!("u32 300 at {:05X}: A={:02X?} B={:02X?}", i, &sa[i..i + 8], &sb[i..i + 8]);
        }
    }
    for i in 0..sa.len().saturating_sub(2) {
        let ha = u16::from_le_bytes(sa[i..i + 2].try_into().unwrap());
        let hb = u16::from_le_bytes(sb[i..i + 2].try_into().unwrap());
        if ha == 0 && hb == 300 {
            println!("u16 300 at {:05X}: A={:02X?} B={:02X?}", i, &sa[i..i + 8], &sb[i..i + 8]);
        }
    }
    // Print the first differing runs.
    let mut run: Option<usize> = None;
    let mut runs = 0;
    for i in 0..sa.len().min(sb.len()) {
        if sa[i] != sb[i] {
            if run.is_none() {
                run = Some(i);
            }
        } else if let Some(start) = run.take() {
            if i - start >= 2 {
                println!(
                    "diff {:05X}..{:05X} A={:02X?} B={:02X?}",
                    start,
                    i,
                    &sa[start..i.min(start + 16)],
                    &sb[start..i.min(start + 16)]
                );
                runs += 1;
                if runs >= 40 {
                    break;
                }
            }
        }
    }
}
