//! Replay the whole wall-clock gap, the way reopening the app does.
//!
//! `irgrow2` replays a chosen number of hours. This example uses the actual
//! current time, so the toy lands on today's date and, if that is daytime,
//! wakes up instead of stopping at the saved 6 a.m.
//!
//! Usage: POCKET_MEETS_ROM=... irgrow3 <save-dir>

use pocket_meets::session::Session;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let start = s.care_at();
    let gap = (now - start).max(0.0);
    println!("care_at={start:.0} now={now:.0} gap={:.1}h", gap / 3600.0);
    let mut n = 0;
    loop {
        let remain = s.care_sync(now, 100.0);
        n += 1;
        if remain < 0.0 {
            println!("error during catch-up");
            break;
        }
        if remain <= 0.001 || n > 100_000 {
            break;
        }
        if n % 360 == 0 {
            println!("  replayed {:.1}h", n as f64 * 100.0 / 3600.0);
        }
    }
    println!("calls={n} asleep={} text={:?}", s.asleep(), text(&mut s));
    let _ = s.save();
}
