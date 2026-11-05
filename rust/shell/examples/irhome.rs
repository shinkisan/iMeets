//! Return to the room screen at a chosen hour and save.
//!
//! Usage: POCKET_MEETS_ROM=... irhome <save-dir> <hour> [tag]

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
    let hour: f64 = std::env::args().nth(2).map(|v| v.parse().unwrap()).unwrap_or(10.0);
    let tag = std::env::args().nth(3).unwrap_or_else(|| "home".into());
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    let care = s.care_at();
    let day = (care / 86_400.0).floor() * 86_400.0;
    let at = day + hour * 3600.0;
    s.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((at as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    s.display_awake(true);
    step(&mut s, 2.0);
    for _ in 0..14 {
        let t = text(&mut s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        let _ = s.key(1, true);
        step(&mut s, 0.2);
        let _ = s.key(1, false);
        step(&mut s, 0.9);
    }
    step(&mut s, 2.0);
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let mut rgb = Vec::with_capacity(16_384 * 3);
    for p in &px {
        rgb.push(((p >> 16) & 0xff) as u8);
        rgb.push(((p >> 8) & 0xff) as u8);
        rgb.push((p & 0xff) as u8);
    }
    let _ = std::fs::write(format!("/tmp/{tag}.rgb"), &rgb);
    println!("home -> {:?}", text(&mut s));
    let _ = s.save();
}
