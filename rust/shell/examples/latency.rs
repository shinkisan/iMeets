//! Measures key-to-display latency through the real pump path.
//!
//! Opens a save, goes to the status menu, presses A and reports how many
//! milliseconds pass before a changed LCD frame is handed to the UI.
//! cargo run --release --example latency -- ROM SAVES_DIR

use std::path::Path;
use pocket_meets::session::{PumpIn, Session};

fn tick(s: &mut Session, inp: &mut PumpIn) -> bool {
    inp.frame_ns = inp.monotonic_ms * 1_000_000;
    let out = s.pump(inp);
    let next = out.next_ms.max(1).min(100);
    inp.last_frame_ns = out.last_frame_ns;
    inp.monotonic_ms += next;
    inp.now_unix += next as f64 / 1000.0;
    inp.dt = next as f64 / 1000.0;
    out.drew_frame
}

fn run(s: &mut Session, inp: &mut PumpIn, ms: u64) {
    let end = inp.monotonic_ms + ms;
    while inp.monotonic_ms < end {
        tick(s, inp);
    }
}

fn hash(s: &mut Session) -> u64 {
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for v in px.iter() {
        h ^= *v as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    let mut s = Session::open_care(Path::new(&rom), Path::new(&saves), false).unwrap();
    let mut inp = PumpIn {
        now_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        dt: 0.008,
        visible: true,
        monotonic_ms: 0,
        phone_interactive: true,
        ..Default::default()
    };
    run(&mut s, &mut inp, 2500);
    if s.asleep() {
        let _ = s.key(1, true);
        run(&mut s, &mut inp, 200);
        let _ = s.key(1, false);
    }
    let _ = s.key(0, true);
    run(&mut s, &mut inp, 200);
    let _ = s.key(0, false);
    run(&mut s, &mut inp, 2500);

    let before = hash(&mut s);
    let start = inp.monotonic_ms;
    let _ = s.key(0, true);
    let mut drawn = None;
    for _ in 0..200 {
        if tick(&mut s, &mut inp) && hash(&mut s) != before {
            drawn = Some(inp.monotonic_ms - start);
            break;
        }
    }
    let _ = s.key(0, false);
    println!("key down -> first changed frame drawn: {:?} ms", drawn);
}
