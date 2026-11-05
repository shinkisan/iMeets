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
    let hours: f64 = std::env::args().nth(2).map(|v| v.parse().unwrap()).unwrap_or(1.0);
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    step(&mut s, 2.0);
    let gap = hours * 3600.0;
    let start = s.care_at() - gap;
    s.care.care_at = start;
    s.machine.sys.rtc.base_ticks = (start * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((start as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
    // Wake once a minute, the way the toy's own sleep alarm does.
    let alarm = s.machine.sys.rtc.ticks(s.machine.sys.game_time) + 60 * 32768;
    for i in 0..6 {
        s.machine.sys.rtc.regs[0x20 + i] = ((alarm >> (8 * i)) & 0xff) as u8;
    }
    s.machine.sys.rtc.regs[0x40] |= 6;
    println!("alarm set enable={:02X} alarm={alarm} now_ticks={}", s.machine.sys.rtc.regs[0x40], s.machine.sys.rtc.ticks(s.machine.sys.game_time));
    let now = start + gap;
    let mut n = 0;
    loop {
        let remain = s.care_sync(now, 100.0);
        n += 1;
        if remain < 0.0 {
            println!("error");
            break;
        }
        if remain <= 0.001 || n > 5000 {
            break;
        }
    }
    println!("calls={n} asleep={}", s.asleep());
    let _ = s.save();
}
