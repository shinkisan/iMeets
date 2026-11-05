//! Brute-force the pet record for the gender byte.
//!
//! Opens a male save, flips one byte in the pet-data range, walks to the
//! profile page and hashes the bottom-right gender-symbol area. A changed
//! hash means the byte affects the symbol; the screenshot is saved for
//! inspection.
//!
//! Usage: POCKET_MEETS_ROM=... irbrute <save>

use pocket_meets::session::Session;
use std::path::Path;

fn step(s: &mut Session, sec: f64) {
    let mut l = sec;
    while l > 0.0 {
        s.step(l.min(0.02));
        l -= 0.02;
    }
}
fn text(s: &mut Session) -> String {
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    pocket_meets::menu_text::read(&px)
}
fn tap(s: &mut Session, k: usize) {
    let _ = s.key(k, true);
    step(s, 0.2);
    let _ = s.key(k, false);
    step(s, 0.55);
}
fn profile(s: &mut Session) -> Option<[u32; 16_384]> {
    step(s, 2.0);
    s.display_awake(true);
    step(s, 2.0);
    if text(s).contains(":") {
        tap(s, 0);
        step(s, 2.0);
    }
    for _ in 0..6 {
        let t = text(s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(s, 2);
    }
    tap(s, 0);
    step(s, 1.0);
    let mut found = false;
    for _ in 0..16 {
        if text(s).contains("じょうたい せってい") {
            found = true;
            break;
        }
        tap(s, 0);
    }
    if !found {
        return None;
    }
    tap(s, 1);
    tap(s, 1);
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    Some(px)
}
fn region_hash(px: &[u32; 16_384]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for y in 96..120 {
        for x in 100..124 {
            h ^= px[y * 128 + x] as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    }
    h
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let lo: usize = std::env::args()
        .nth(2)
        .and_then(|v| usize::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x0D7C0);
    let hi: usize = std::env::args()
        .nth(3)
        .and_then(|v| usize::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x0D880);

    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let base_px = profile(&mut s).expect("profile baseline");
    let baseline = region_hash(&base_px);
    println!("baseline region hash {baseline:016X}");
    drop(s);

    for off in lo..hi {
        let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
        let old = s.machine.sys.sram[off];
        s.machine.sys.sram[off] ^= 0xFF;
        if let Some(px) = profile(&mut s) {
            let h = region_hash(&px);
            if h != baseline {
                println!("offset {off:05X}: {old:02X} -> {:02X}, hash {h:016X}", old ^ 0xFF);
                let mut rgb = Vec::with_capacity(16_384 * 3);
                for p in &px {
                    rgb.push(((p >> 16) & 0xff) as u8);
                    rgb.push(((p >> 8) & 0xff) as u8);
                    rgb.push((p & 0xff) as u8);
                }
                let _ = std::fs::write(format!("/tmp/brute-{off:05X}.rgb"), &rgb);
            }
        } else {
            println!("offset {off:05X}: profile not reachable");
        }
    }
}
