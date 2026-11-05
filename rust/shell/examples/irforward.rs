//! Replay the next N hours forward, with the RTC alarm re-armed every minute.
//!
//! The save's `care_at` is the last accounted second. Because the alarm is
//! enabled, the sleep code cold-boots the toy at each alarm, which refreshes
//! the game's calendar and lets the pet age/evolve.
//!
//! Usage: POCKET_MEETS_ROM=... irforward <save-dir> <hours>

use pocket_meets::session::Session;
use std::path::Path;

fn step(s: &mut Session, sec: f64) {
    let mut l = sec;
    while l > 0.0 {
        s.step(l.min(0.02));
        l -= 0.02;
    }
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let hours: f64 = std::env::args().nth(2).map(|v| v.parse().unwrap()).unwrap_or(24.0);
    let alarm_secs: f64 = std::env::args().nth(3).map(|v| v.parse().unwrap()).unwrap_or(60.0);
    let hour: Option<f64> = std::env::args().nth(4).and_then(|v| v.parse().ok());
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    let start = s.care_at();
    let now = start + hours * 3600.0;
    s.care.care_at = start;
    let rtc_at = match hour {
        Some(h) => (start / 86_400.0).floor() * 86_400.0 + h * 3600.0,
        None => start,
    };
    s.machine.sys.rtc.base_ticks = (rtc_at * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((rtc_at as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
    let alarm = s.machine.sys.rtc.ticks(s.machine.sys.game_time) + (alarm_secs * 32768.0) as i64;
    for i in 0..6 {
        s.machine.sys.rtc.regs[0x20 + i] = ((alarm >> (8 * i)) & 0xff) as u8;
    }
    s.machine.sys.rtc.regs[0x40] |= 6;
    let mut n = 0;
    loop {
        let remain = s.care_sync(now, 100.0);
        n += 1;
        if remain < 0.0 {
            println!("error");
            break;
        }
        if remain <= 0.001 || n > 10_000 {
            break;
        }
    }
    println!("calls={n} care_at={:.0} asleep={}", s.care_at(), s.asleep());
    let _ = s.save();
}
