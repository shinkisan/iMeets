//! Host CPU needed per second of emulated play.
use pocket_meets::session::Session;
use std::path::Path;
use std::time::Instant;

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    let mut s = Session::open_care(Path::new(&rom), Path::new(&saves), false).unwrap();
    for _ in 0..100 {
        s.step(0.02);
    }
    if s.asleep() {
        let _ = s.key(1, true);
        for _ in 0..10 {
            s.step(0.02);
        }
        let _ = s.key(1, false);
        for _ in 0..80 {
            s.step(0.02);
        }
    }
    let emu_s: f64 = 20.0;
    let t0 = Instant::now();
    let mut left = emu_s;
    while left > 0.0 {
        let q = left.min(0.02);
        s.step(q);
        left -= q;
    }
    let host = t0.elapsed().as_secs_f64();
    println!(
        "{emu_s:.0}s emulated in {host:.2}s host => {:.0}% of one core, idle_skipped={}",
        host / emu_s * 100.0,
        s.machine.idle_skipped
    );
}
