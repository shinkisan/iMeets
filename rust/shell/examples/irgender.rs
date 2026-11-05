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
    for _ in 0..6 {
        let t = text(&mut s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(&mut s, 2);
    }
    tap(&mut s, 0);
    step(&mut s, 1.0);
    for _ in 0..16 {
        if text(&mut s).contains("じょうたい せってい") {
            break;
        }
        tap(&mut s, 0);
    }
    tap(&mut s, 1);
    tap(&mut s, 1);
    println!("profile -> {:?}", text(&mut s));
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let mut rgb = Vec::with_capacity(16_384 * 3);
    for p in &px {
        rgb.push(((p >> 16) & 0xff) as u8);
        rgb.push(((p >> 8) & 0xff) as u8);
        rgb.push((p & 0xff) as u8);
    }
    let _ = std::fs::write("/tmp/gender.rgb", &rgb);
}
