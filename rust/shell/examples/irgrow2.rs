//! Replay an away gap so the toy's own clock walks it minute by minute.
//!
//! `irpair` proves the infrared link. This example is for preparing a test
//! save: one hour of replay grows a baby into a child, and the status page
//! then shows the new character (and the points earned while away).
//!
//! Usage: POCKET_MEETS_ROM=... irgrow2 <save-dir> <hours>

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
    let hours: f64 = std::env::args().nth(2).map(|v| v.parse().unwrap()).unwrap_or(1.0);
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    if s.asleep() {
        tap(&mut s, 1);
    }
    println!("start asleep={} text={:?} care_at={:.0}", s.asleep(), text(&mut s), s.care_at());

    let gap = hours * 3600.0;
    let start = s.care_at() - gap;
    s.care.care_at = start;
    s.machine.sys.rtc.base_ticks = (start * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((start as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
    let now = start + gap;
    let mut n = 0;
    loop {
        let remain = s.care_sync(now, 100.0);
        n += 1;
        if remain < 0.0 {
            println!("error during catch-up");
            break;
        }
        if remain <= 0.001 || n > 5000 {
            break;
        }
        if n % 36 == 0 {
            println!("  replayed {}h", n as f64 * 100.0 / 3600.0);
        }
    }
    println!("catch-up calls={n} asleep={} text={:?}", s.asleep(), text(&mut s));
    if s.asleep() {
        tap(&mut s, 1);
        step(&mut s, 5.0);
    }
    println!("after wake text={:?}", text(&mut s));
    let _ = s.save();
}
