//! Open a save and capture the main screen, to tell a live pet from a grave.
//!
//! Usage: POCKET_MEETS_ROM=... iralive <save-dir> <tag>

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
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let tag = std::env::args().nth(2).unwrap_or_else(|| "alive".into());
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    if s.asleep() {
        let _ = s.key(1, true);
        step(&mut s, 0.2);
        let _ = s.key(1, false);
        step(&mut s, 5.0);
    }
    for _ in 0..6 {
        let t = text(&mut s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        let _ = s.key(2, true);
        step(&mut s, 0.2);
        let _ = s.key(2, false);
        step(&mut s, 0.8);
    }
    let _ = s.key(0, true);
    step(&mut s, 0.2);
    let _ = s.key(0, false);
    step(&mut s, 1.0);
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let mut rgb = Vec::with_capacity(16_384 * 3);
    for p in &px {
        rgb.push(((p >> 16) & 0xff) as u8);
        rgb.push(((p >> 8) & 0xff) as u8);
        rgb.push((p & 0xff) as u8);
    }
    let _ = std::fs::write(format!("/tmp/{tag}.rgb"), &rgb);
    println!("text={:?}", text(&mut s));
}
