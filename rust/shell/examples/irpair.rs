//! Two emulated toys, one infrared link.
//!
//! Runs the original firmware on both sides, with A's transmitter wired to B's
//! receiver and vice versa. The toys are started a fraction of a second apart,
//! the way two people pressing the button would be, so one side sends the link
//! request while the other is listening instead of both colliding forever.
//!
//! The run is a success when the firmware itself finishes the visit and shows
//! its goodbye / out-for-the-day message.

use std::path::Path;
use pocket_meets::session::Session;

const ROM: &str = "/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin";

fn step(s: &mut Session, seconds: f64) {
    let mut l = seconds;
    while l > 0.0 {
        s.step(l.min(0.02));
        l -= 0.02;
    }
}

fn press(s: &mut Session, k: usize) {
    let _ = s.key(k, true);
    step(s, 0.20);
    let _ = s.key(k, false);
    step(s, 0.55);
}

fn to_comm(s: &mut Session) {
    step(s, 2.0);
    for _ in 0..5 {
        press(s, 0);
    }
    press(s, 1);
    press(s, 1);
    s.machine.stop_on_bluetooth = false;
    for _ in 0..2 {
        let _ = s.key(1, true);
        step(s, 0.2);
        let _ = s.key(1, false);
        step(s, 1.0);
    }
}

fn main() {
    let da = std::env::args().nth(1).expect("save A");
    let db = std::env::args().nth(2).expect("save B");
    let mut a = Session::open_care(Path::new(ROM), Path::new(&da), false).unwrap();
    let mut b = Session::open_care(Path::new(ROM), Path::new(&db), false).unwrap();
    to_comm(&mut a);
    to_comm(&mut b);

    for s in [&mut a, &mut b] {
        let _ = s.key(1, true);
        step(s, 0.2);
        let _ = s.key(1, false);
    }
    // A starts first; B is still in its wait screen when A's frame goes out.
    step(&mut a, 0.20);

    let mut px = [0u32; 16_384];
    let mut frames = 0usize;
    let mut obex = false;
    let mut tama = false;
    let mut done = false;
    let mut last_text = String::new();

    for i in 0..2000 {
        step(&mut a, 0.02);
        step(&mut b, 0.02);
        let fa = a.machine.sys.uart_take();
        let fb = b.machine.sys.uart_take();
        frames += usize::from(!fa.is_empty()) + usize::from(!fb.is_empty());
        for buf in [&fa, &fb] {
            if buf.windows(4).any(|w| w == b"OBEX") {
                obex = true;
            }
            if buf.windows(4).any(|w| w == b"TAMA") {
                tama = true;
            }
        }
        for x in &fa {
            b.machine.sys.uart_receive(*x);
        }
        for x in &fb {
            a.machine.sys.uart_receive(*x);
        }
        if i % 25 == 24 {
            a.frame(&mut px);
            let ta = pocket_meets::menu_text::read(&px);
            b.frame(&mut px);
            let tb = pocket_meets::menu_text::read(&px);
            last_text = format!("A[{ta}] B[{tb}]");
            if i % 100 == 99 {
                println!("t={:.1} {last_text} frames={frames}", (i + 1) as f64 * 0.02);
            }
            if ta.contains("バイバイ") || tb.contains("バイバイ") {
                done = true;
                println!("t={:.1} {last_text}", (i + 1) as f64 * 0.02);
                break;
            }
        }
    }

    println!(
        "frames={frames} obex={obex} tama={tama} finished={done} last={last_text}"
    );
    if done {
        println!("PASS: the two original firmwares completed an infrared visit");
    } else {
        println!("FAIL: the visit did not finish");
        std::process::exit(1);
    }
}
