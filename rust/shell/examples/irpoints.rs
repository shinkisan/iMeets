//! Read the pet's profile and stats page (age, generation, gotchi points).
//!
//! Waits for any open menu to close, wakes the toy, then walks the main menu
//! to `じょうたい せってい` and flips to the stats page.
//!
//! Usage: POCKET_MEETS_ROM=... irpoints <save-dir>

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
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    if s.asleep() {
        tap(&mut s, 1);
        step(&mut s, 5.0);
    }
    println!("start -> {:?}", text(&mut s));
    tap(&mut s, 0);
    for i in 0..16 {
        let t = text(&mut s);
        if t.contains("じょうたい せってい") {
            break;
        }
        println!("A{i} -> {:?}", t);
        tap(&mut s, 0);
    }
    tap(&mut s, 1);
    println!("menu -> {:?}", text(&mut s));
    tap(&mut s, 1);
    println!("profile -> {:?}", text(&mut s));
    tap(&mut s, 0);
    println!("stats -> {:?}", text(&mut s));
}
