//! Walk to the meet submenu and open the item-exchange screen.
//!
//! Usage: POCKET_MEETS_ROM=... irgiftprobe <save>

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
    s.display_awake(true);
    step(&mut s, 2.0);
    if text(&mut s).contains(":") {
        tap(&mut s, 0);
        step(&mut s, 2.0);
    }
    for _ in 0..4 {
        let t = text(&mut s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(&mut s, 2);
    }
    tap(&mut s, 0);
    step(&mut s, 1.0);
    for _ in 0..3 {
        tap(&mut s, 0);
    }
    tap(&mut s, 1);
    step(&mut s, 1.0);
    println!("comm -> {:?}", text(&mut s));
    tap(&mut s, 1);
    step(&mut s, 1.5);
    println!("submenu -> {:?}", text(&mut s));
    tap(&mut s, 0);
    step(&mut s, 1.0);
    tap(&mut s, 1);
    step(&mut s, 2.0);
    println!("gift -> {:?}", text(&mut s));
    tap(&mut s, 1);
    step(&mut s, 2.0);
    println!("start -> {:?}", text(&mut s));
}
