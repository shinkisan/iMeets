//! Host cost of driving the pump at a given cadence.
use pocket_meets::session::{PumpIn, Session};
use std::path::Path;
use std::time::Instant;

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    let dt_ms: f64 = std::env::args().nth(3).unwrap().parse().unwrap();
    let seconds: f64 = std::env::args()
        .nth(4)
        .map(|v| v.parse().unwrap())
        .unwrap_or(20.0);

    let mut s = Session::open_care(Path::new(&rom), Path::new(&saves), false).unwrap();
    let mut inp = PumpIn {
        now_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        dt: dt_ms / 1000.0,
        visible: true,
        monotonic_ms: 0,
        phone_interactive: true,
        ..Default::default()
    };
    // Warm up and get the toy awake.
    for _ in 0..200 {
        inp.frame_ns = inp.monotonic_ms * 1_000_000;
        let o = s.pump(&inp);
        inp.last_frame_ns = o.last_frame_ns;
        inp.monotonic_ms += 8;
        inp.now_unix += 0.008;
    }
    if s.asleep() {
        let _ = s.key(1, true);
        for _ in 0..20 {
            inp.frame_ns = inp.monotonic_ms * 1_000_000;
            let o = s.pump(&inp);
            inp.last_frame_ns = o.last_frame_ns;
            inp.monotonic_ms += 8;
            inp.now_unix += 0.008;
        }
        let _ = s.key(1, false);
    }

    let dt = dt_ms / 1000.0;
    let calls = (seconds / dt).round() as u64;
    let t0 = Instant::now();
    let mut drawn = 0u64;
    for _ in 0..calls {
        inp.frame_ns = inp.monotonic_ms * 1_000_000;
        let out = s.pump(&inp);
        inp.last_frame_ns = out.last_frame_ns;
        inp.last_save_ms = out.last_save_ms;
        inp.last_catchup_save_ms = out.last_catchup_save_ms;
        inp.monotonic_ms += dt_ms as u64;
        inp.now_unix += dt;
        inp.dt = dt;
        if out.drew_frame {
            drawn += 1;
        }
    }
    let host = t0.elapsed().as_secs_f64();
    println!(
        "dt={:>2.0} ms: {:>4} pumps in {:.2}s host ({:.2}% core), {:.1} us/pump, {} frames",
        dt_ms,
        calls,
        host,
        host / seconds * 100.0,
        host / calls as f64 * 1e6,
        drawn
    );
}
