//! Hatch a fresh egg into the female baby `ぷちもちっち`.
//!
//! The difference from `irhatch2` is one C press after the boot/wait screen,
//! before the name-entry B sequence. That selects a different name/keyboard
//! state and the resulting egg is `ぷちもちっち` with the female marker.
//!
//! Usage: POCKET_MEETS_ROM=... irhatch_female <empty-save-dir>

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
    let base = std::env::args().nth(1).expect("save dir");
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 5.0);
    println!("boot -> {:?}", text(&mut s));
    tap(&mut s, 2);
    step(&mut s, 1.0);
    for i in 0..60 {
        let t = text(&mut s);
        if t.contains("これでけってい") {
            println!("confirm at {i}: {:?}", t);
            tap(&mut s, 1);
            step(&mut s, 2.0);
            break;
        }
        tap(&mut s, 1);
        step(&mut s, 1.2);
        if i % 5 == 0 {
            println!("B{i} -> {:?}", text(&mut s));
        }
    }
    for i in 0..12 {
        step(&mut s, 5.0);
        println!(
            "wait {}s -> {:?} asleep={}",
            (i + 1) * 5,
            text(&mut s),
            s.asleep()
        );
    }
    let _ = s.save();
}
