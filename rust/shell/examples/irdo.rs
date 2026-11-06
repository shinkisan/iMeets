//! Drive two toys with key scripts after the みーつ submenu, then bridge IR.
//!
//! Usage: POCKET_MEETS_ROM=... irdo <a-dir> <b-dir> <seconds> <hour> <a-keys> <b-keys> [lead]

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
fn hour_now(s: &mut Session, hour: f64) {
    let care = s.care_at();
    let day = (care / 86_400.0).floor() * 86_400.0;
    let at = day + hour * 3600.0;
    s.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((at as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
}
fn open_submenu(s: &mut Session) -> String {
    for _ in 0..14 {
        let t = text(s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(s, 1);
    }
    tap(s, 0);
    step(s, 1.0);
    for _ in 0..16 {
        if text(s).contains("ツーしん") {
            break;
        }
        tap(s, 0);
    }
    tap(s, 1);
    step(s, 2.0);
    tap(s, 1);
    step(s, 2.0);
    text(s)
}
fn keys(s: &mut Session, script: &str, tag: &str) {
    let list: Vec<usize> = script
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    for (i, &k) in list.iter().enumerate() {
        tap(s, k);
        step(s, 1.0);
        eprintln!("{tag} key {i} ({k}) -> {:?}", text(s));
    }
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let a_dir = std::env::args().nth(1).expect("a");
    let b_dir = std::env::args().nth(2).expect("b");
    let secs: f64 = std::env::args()
        .nth(3)
        .map(|v| v.parse().unwrap())
        .unwrap_or(40.0);
    let hour: f64 = std::env::args()
        .nth(4)
        .map(|v| v.parse().unwrap())
        .unwrap_or(10.0);
    let a_keys = std::env::args().nth(5).unwrap_or_default();
    let b_keys = std::env::args().nth(6).unwrap_or_default();
    let lead: f64 = std::env::args()
        .nth(7)
        .map(|v| v.parse().unwrap())
        .unwrap_or(0.5);
    let mut a = Session::open_care(Path::new(&rom), Path::new(&a_dir), false).unwrap();
    let mut b = Session::open_care(Path::new(&rom), Path::new(&b_dir), false).unwrap();
    hour_now(&mut a, hour);
    hour_now(&mut b, hour);
    step(&mut a, 2.0);
    step(&mut b, 2.0);
    step(&mut a, 15.0);
    step(&mut b, 15.0);
    a.display_awake(true);
    b.display_awake(true);
    step(&mut a, 2.0);
    step(&mut b, 2.0);
    a.machine.stop_on_bluetooth = false;
    b.machine.stop_on_bluetooth = false;

    eprintln!("A submenu -> {:?}", open_submenu(&mut a));
    eprintln!("B submenu -> {:?}", open_submenu(&mut b));
    keys(&mut a, &a_keys, "A");
    keys(&mut b, &b_keys, "B");
    eprintln!("A before start -> {:?}", text(&mut a));
    eprintln!("B before start -> {:?}", text(&mut b));

    let _ = a.key(1, true);
    step(&mut a, 0.2);
    let _ = a.key(1, false);
    step(&mut a, lead);
    let _ = b.key(1, true);
    step(&mut b, 0.2);
    let _ = b.key(1, false);
    step(&mut b, 1.0);
    step(&mut a, 1.0);
    eprintln!("A after start -> {:?}", text(&mut a));
    eprintln!("B after start -> {:?}", text(&mut b));

    let tab: Vec<u16> = (0..256)
        .map(|i| {
            let mut c = i as u16;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    (c >> 1) ^ 0x8408
                } else {
                    c >> 1
                };
            }
            c
        })
        .collect();
    let mut p = vec![0xFEu8];
    p.extend(std::iter::repeat(0).take(21));
    let mut crc = 0xFFFFu16;
    for &byte in &p {
        crc = (crc >> 8) ^ tab[((crc ^ byte as u16) & 0xFF) as usize];
    }
    let c = !crc;
    p.push((c & 0xFF) as u8);
    p.push((c >> 8) as u8);
    let mut f = vec![0xC0];
    f.extend(p);
    f.push(0xC0);
    for byte in &f {
        a.machine.sys.uart_receive(*byte);
        b.machine.sys.uart_receive(*byte);
    }

    let mut px = [0u32; 16_384];
    let ticks = (secs / 0.02) as usize;
    let mut frames = 0usize;
    let mut answered = false;
    for i in 0..ticks {
        step(&mut a, 0.02);
        step(&mut b, 0.02);
        let fa = a.machine.sys.uart_take();
        let fb = b.machine.sys.uart_take();
        for x in &fa {
            b.machine.sys.uart_receive(*x);
        }
        for x in &fb {
            a.machine.sys.uart_receive(*x);
        }
        if !fa.is_empty() || !fb.is_empty() {
            frames += fa.len() + fb.len();
            if frames < 300 || i % 200 == 0 {
                eprintln!(
                    "t={:5.2} A->B {:02X?} B->A {:02X?}",
                    i as f64 * 0.02,
                    fa,
                    fb
                );
            }
        }
        if !answered && i > 30 {
            a.frame(&mut px);
            let ta = pocket_meets::menu_text::read(&px);
            b.frame(&mut px);
            let tb = pocket_meets::menu_text::read(&px);
            if ta.contains("うけとる")
                || tb.contains("うけとる")
                || ta.contains("あげる？")
                || tb.contains("あげる？")
            {
                eprintln!("i={i} prompt A[{ta}] B[{tb}]");
                let _ = a.key(1, true);
                step(&mut a, 0.2);
                let _ = a.key(1, false);
                step(&mut a, 1.5);
                answered = true;
            }
        }
        if i % 200 == 0 {
            a.frame(&mut px);
            let ta = pocket_meets::menu_text::read(&px);
            b.frame(&mut px);
            let tb = pocket_meets::menu_text::read(&px);
            eprintln!("i={i:4} A[{ta}] B[{tb}]");
        }
    }
    eprintln!("bytes={frames}");
    let _ = a.save();
    let _ = b.save();
}
