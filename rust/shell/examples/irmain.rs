//! Cycle the main menu and print each page.
//!
//! Usage: POCKET_MEETS_ROM=... irmain <save-dir> <hour> [steps]

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
    step(s, 0.7);
}
fn shot(s: &mut Session, tag: &str) {
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let mut rgb = Vec::with_capacity(16_384 * 3);
    for p in &px {
        rgb.push(((p >> 16) & 0xff) as u8);
        rgb.push(((p >> 8) & 0xff) as u8);
        rgb.push((p & 0xff) as u8);
    }
    let _ = std::fs::write(format!("/tmp/{tag}.rgb"), &rgb);
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let hour: f64 = std::env::args()
        .nth(2)
        .map(|v| v.parse().unwrap())
        .unwrap_or(10.0);
    let steps: usize = std::env::args()
        .nth(3)
        .map(|v| v.parse().unwrap())
        .unwrap_or(24);
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
        tap(&mut s, 1);
    }
    tap(&mut s, 0);
    step(&mut s, 1.0);
    for i in 0..steps {
        println!("{i} -> {:?}", text(&mut s));
        shot(&mut s, &format!("main-{i}"));
        tap(&mut s, 0);
        step(&mut s, 0.4);
    }
    let _ = s.save();
}
