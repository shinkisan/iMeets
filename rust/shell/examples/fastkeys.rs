//! Compare button latency and menu walking with and without the fast-key
//! firmware patch. Usage: fastkeys ROM SAVES [patch]
use pocket_meets::session::Session;
use std::path::Path;

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
        for _ in 0..20 {
            s.step(0.02);
        }
        let _ = s.key(1, false);
        for _ in 0..60 {
            s.step(0.02);
        }
    }
    s
}

fn down_counter(s: &Session) -> u16 {
    let o = 0xF800_D778usize - 0xF800_0000;
    u16::from_le_bytes(s.machine.sys.sram[o..o + 2].try_into().unwrap())
}

fn measure(mut s: Session) -> (Option<u64>, Option<u64>) {
    for _ in 0..150 {
        s.step(0.02);
    }
    if s.asleep() {
        let _ = s.key(1, true);
        for _ in 0..20 {
            s.step(0.02);
        }
        let _ = s.key(1, false);
        for _ in 0..60 {
            s.step(0.02);
        }
    }
    // Open the status menu so the screen is static, then wait for it to settle.
    let _ = s.key(0, true);
    for _ in 0..20 {
        s.step(0.02);
    }
    let _ = s.key(0, false);
    for _ in 0..120 {
        s.step(0.02);
    }

    let before = hash(&mut s);
    let t0 = s.seconds();
    let _ = s.key(0, true);
    let mut accepted = None;
    let mut changed = None;
    for _ in 0..400 {
        s.step(0.001);
        let t = ((s.seconds() - t0) * 1000.0).round() as u64;
        if accepted.is_none() && down_counter(&s) == 50 {
            accepted = Some(t);
        }
        if changed.is_none() && hash(&mut s) != before {
            changed = Some(t);
            break;
        }
    }
    let _ = s.key(0, false);
    (accepted, changed)
}

fn walk(mut s: Session) -> String {
    for _ in 0..150 {
        s.step(0.02);
    }
    if s.asleep() {
        return "asleep".into();
    }
    let mut out = Vec::new();
    for _ in 0..8 {
        // A short tap, then let the screen settle.
        let _ = s.key(0, true);
        for _ in 0..4 {
            s.step(0.01);
        }
        let _ = s.key(0, false);
        for _ in 0..60 {
            s.step(0.01);
        }
        out.push(format!("{:016X}", hash(&mut s)));
    }
    out.join(" ")
}

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    let patched = std::env::args()
        .nth(3)
        .map(|v| v == "patch")
        .unwrap_or(false);

    let mut s = open(&rom, &saves);
    if patched {
        s.set_fast_keys(true);
    }
    let (a, c) = measure(s);
    println!("accepted at {:?} ms, first changed frame {:?} ms", a, c);
    let mut s = open(&rom, &saves);
    if patched {
        s.set_fast_keys(true);
    }
    println!("walk: {}", walk(s));
}
