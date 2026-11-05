//! Try a marriage proposal between two adult toys over the virtual IR link.
//!
//! Device A proposes (`プロポーズ`), device B waits (`あそぶ`).
//!
//! Usage: POCKET_MEETS_ROM=... irmarry <a-dir> <b-dir> [seconds]

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
fn home(s: &mut Session) {
    for _ in 0..14 {
        let t = text(s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(s, 1);
    }
    tap(s, 0);
    step(s, 1.0);
}
fn open_submenu(s: &mut Session) -> String {
    home(s);
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
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let a_dir = std::env::args().nth(1).expect("a");
    let b_dir = std::env::args().nth(2).expect("b");
    let secs: f64 = std::env::args().nth(3).map(|v| v.parse().unwrap()).unwrap_or(40.0);
    let hour: f64 = std::env::args().nth(4).map(|v| v.parse().unwrap()).unwrap_or(10.0);
    let a_opt: usize = std::env::args().nth(5).map(|v| v.parse().unwrap()).unwrap_or(2);
    let b_opt: usize = std::env::args().nth(6).map(|v| v.parse().unwrap()).unwrap_or(0);
    let lead: f64 = std::env::args().nth(7).map(|v| v.parse().unwrap()).unwrap_or(0.5);
    let decline: bool = std::env::args().nth(8).as_deref() == Some("decline");
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

    let sa = open_submenu(&mut a);
    let sb = open_submenu(&mut b);
    eprintln!("A submenu: {sa:?}");
    eprintln!("B submenu: {sb:?}");

    // Move each cursor to its chosen option (0 あそぶ, 1 アイテム, 2 プロポーズ, 3 おでかけ).
    for _ in 0..a_opt {
        tap(&mut a, 0);
    }
    for _ in 0..b_opt {
        tap(&mut b, 0);
    }
    eprintln!("A option {a_opt}, B option {b_opt}");
    step(&mut a, 1.0);
    step(&mut b, 1.0);

    // Both confirm and wait for the link.
    let _ = a.key(1, true);
    let _ = b.key(1, true);
    step(&mut a, 0.2);
    step(&mut b, 0.2);
    let _ = a.key(1, false);
    let _ = b.key(1, false);
    step(&mut a, 2.0);
    step(&mut b, 2.0);
    eprintln!("A after confirm: {:?}", text(&mut a));
    eprintln!("B after confirm: {:?}", text(&mut b));

    // The waiting screen needs one more B on each side.
    step(&mut a, 3.0);
    step(&mut b, 3.0);
    if text(&mut a).contains("たいき") && text(&mut b).contains("たいき") {
        let _ = a.key(1, true);
        step(&mut a, 0.2);
        let _ = a.key(1, false);
        step(&mut a, lead);
        let _ = b.key(1, true);
        step(&mut b, 0.2);
        let _ = b.key(1, false);
        step(&mut b, 1.0);
        step(&mut a, 1.0);
        eprintln!("A after start: {:?}", text(&mut a));
        eprintln!("B after start: {:?}", text(&mut b));
    }

    // Kick both sides with the same well-formed frame the play harness uses.
    let tab: Vec<u16> = (0..256)
        .map(|i| {
            let mut c = i as u16;
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0x8408 } else { c >> 1 };
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

    let cap = std::env::var("IRCAP_DIR").ok();
    let cancel_at: Option<usize> = std::env::var("IRCANCEL_AT").ok().and_then(|v| v.parse().ok());
    let drop_every: Option<usize> = std::env::var("IRDROP_EVERY").ok().and_then(|v| v.parse().ok());
    let drop_at: Option<usize> = std::env::var("IRDROP_AT").ok().and_then(|v| v.parse().ok());
    let mut sent_all = 0usize;
    let mut cap_ab: Vec<u8> = Vec::new();
    let mut cap_ba: Vec<u8> = Vec::new();

    let mut px = [0u32; 16_384];
    let ticks = (secs / 0.02) as usize;
    let mut frames = 0usize;
    let mut answered = false;
    for i in 0..ticks {
        step(&mut a, 0.02);
        step(&mut b, 0.02);
        let fa = a.machine.sys.uart_take();
        let fb = b.machine.sys.uart_take();
        if i < 5 {
            eprintln!("i={i} nowA={:.2} nowB={:.2} fa={} fb={} firstA={:02X?} firstB={:02X?}", a.machine.now(), b.machine.now(), fa.len(), fb.len(), &fa[..fa.len().min(12)], &fb[..fb.len().min(12)]);
        }
        for x in &fa {
            sent_all += 1;
            if let Some(n) = drop_every {
                if sent_all % n == 0 {
                    continue;
                }
            }
            if drop_at == Some(sent_all) {
                eprintln!("drop byte {sent_all} A->B");
                continue;
            }
            b.machine.sys.uart_receive(*x);
        }
        for x in &fb {
            sent_all += 1;
            if let Some(n) = drop_every {
                if sent_all % n == 0 {
                    continue;
                }
            }
            if drop_at == Some(sent_all) {
                eprintln!("drop byte {sent_all} B->A");
                continue;
            }
            a.machine.sys.uart_receive(*x);
        }
        if cap.is_some() {
            cap_ab.extend_from_slice(&fa);
            cap_ba.extend_from_slice(&fb);
        }
        if !fa.is_empty() || !fb.is_empty() {
            frames += fa.len() + fb.len();
            if frames < 400 || i % 100 == 0 {
                eprintln!("t={:5.2} A->B {:02X?} B->A {:02X?}", i as f64 * 0.02, fa, fb);
            }
        }
        if let Some(c) = cancel_at {
            if i == c {
                eprintln!("i={i} cancel C on A");
                let _ = a.key(2, true);
                step(&mut a, 0.2);
                let _ = a.key(2, false);
                step(&mut a, 1.0);
            }
        }
        if !answered && i > 30 {
            b.frame(&mut px);
            let tb = pocket_meets::menu_text::read(&px);
            if tb.contains("けっこん する") {
                eprintln!("i={i} answer screen: {tb:?}");
                step(&mut b, 1.0);
                if decline {
                    let _ = b.key(0, true);
                    step(&mut b, 0.2);
                    let _ = b.key(0, false);
                    step(&mut b, 0.6);
                }
                let _ = b.key(1, true);
                step(&mut b, 0.2);
                let _ = b.key(1, false);
                step(&mut b, 1.5);
                b.frame(&mut px);
                eprintln!("answer after B: {:?}", pocket_meets::menu_text::read(&px));
                answered = true;
            }
        }
        if i % 100 == 0 {
            a.frame(&mut px);
            let ta = pocket_meets::menu_text::read(&px);
            let mut rgb = Vec::with_capacity(16_384 * 3);
            for p in &px { rgb.push(((p >> 16) & 0xff) as u8); rgb.push(((p >> 8) & 0xff) as u8); rgb.push((p & 0xff) as u8); }
            let _ = std::fs::write(format!("/tmp/mar-a-{i}.rgb"), &rgb);
            b.frame(&mut px);
            let tb = pocket_meets::menu_text::read(&px);
            let mut rgb = Vec::with_capacity(16_384 * 3);
            for p in &px { rgb.push(((p >> 16) & 0xff) as u8); rgb.push(((p >> 8) & 0xff) as u8); rgb.push((p & 0xff) as u8); }
            let _ = std::fs::write(format!("/tmp/mar-b-{i}.rgb"), &rgb);
            eprintln!("i={i:4} A[{ta}] B[{tb}]");
        }
    }
    eprintln!("bytes={frames}");
    if let Some(dir) = cap {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(format!("{dir}/a-to-b.bin"), &cap_ab);
        let _ = std::fs::write(format!("{dir}/b-to-a.bin"), &cap_ba);
        eprintln!("captured A->B={} B->A={}", cap_ab.len(), cap_ba.len());
    }
    let _ = a.save();
    let _ = b.save();
}
