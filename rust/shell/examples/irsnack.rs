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
fn home(s: &mut Session) {
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
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    s.display_awake(true);
    step(&mut s, 2.0);
    s.machine.sys.sram[0x0D854..0x0D856].copy_from_slice(&9999u16.to_le_bytes());
    home(&mut s);
    for _ in 0..14 {
        if text(&mut s).contains("ごはん・おやつ") {
            break;
        }
        tap(&mut s, 0);
    }
    tap(&mut s, 1);
    println!("food -> {:?}", text(&mut s));
    tap(&mut s, 0);
    println!("move -> {:?}", text(&mut s));
    tap(&mut s, 1);
    println!("open -> {:?}", text(&mut s));
    for i in 0..5 {
        tap(&mut s, 1);
        step(&mut s, 1.0);
        println!("B{i} -> {:?}", text(&mut s));
    }
    let _ = s.save();
}
