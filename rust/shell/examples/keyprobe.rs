//! Probes the firmware key scan: phase sensitivity and minimum hold.
use std::path::Path;
use pocket_meets::session::Session;

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

fn open(rom: &str, saves: &str) -> Session {
    let mut s = Session::open_care(Path::new(rom), Path::new(saves), false).unwrap();
    for _ in 0..150 {
        s.step(0.02);
    }
    if s.asleep() {
        let _ = s.key(1, true);
        for _ in 0..20 { s.step(0.02); }
        let _ = s.key(1, false);
        for _ in 0..60 { s.step(0.02); }
    }
    let _ = s.key(0, true);
    for _ in 0..20 { s.step(0.02); }
    let _ = s.key(0, false);
    for _ in 0..120 { s.step(0.02); }
    s
}

fn probe(rom: &str, saves: &str, offset_ms: u64, hold_ms: u64) -> Option<u64> {
    let mut s = open(rom, saves);
    for _ in 0..offset_ms { s.step(0.001); }
    let before = hash(&mut s);
    let t0 = s.seconds();
    s.machine.sys.live_keys[0] = true;
    let mut changed = None;
    let mut held = 0;
    for _ in 0..400 {
        s.step(0.001);
        held += 1;
        if held >= hold_ms {
            s.machine.sys.live_keys[0] = false;
        }
        if hash(&mut s) != before {
            changed = Some(((s.seconds() - t0) * 1000.0).round() as u64);
            break;
        }
    }
    s.machine.sys.live_keys[0] = false;
    changed
}

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    for offset in [0, 7, 15, 23, 31, 47] {
        println!("offset {:2} ms, hold 200 ms -> {:?} ms", offset, probe(&rom, &saves, offset, 200));
    }
    for hold in [1, 4, 8, 16, 24, 32, 48, 64] {
        println!("offset 0 ms, hold {:3} ms -> {:?} ms", hold, probe(&rom, &saves, 0, hold));
    }
}
