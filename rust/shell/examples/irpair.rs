//! Two emulated toys, one infrared link.
//!
//! Runs the original firmware on both sides, with A's transmitter wired to B's
//! receiver and vice versa. The toys are started a fraction of a second apart,
//! the way two people pressing the button would be, so one side sends the link
//! request while the other is listening instead of both colliding forever.
//!
//! The run is a success when the firmware itself finishes the visit and shows
//! its goodbye / out-for-the-day message.

use pocket_meets::session::Session;
use std::path::{Path, PathBuf};

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
    let rom =
        std::env::var("POCKET_MEETS_ROM").expect("set POCKET_MEETS_ROM to a Sweets v062 dump");
    let rom = PathBuf::from(rom);
    let da = std::env::args().nth(1).expect("save A");
    let db = std::env::args().nth(2).expect("save B");
    let mut a = Session::open_care(&rom, Path::new(&da), false).unwrap();
    let mut b = Session::open_care(&rom, Path::new(&db), false).unwrap();
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
    let mut obex_frame: Option<Vec<u8>> = None;
    let mut tama_frame: Option<Vec<u8>> = None;
    let mut cap_a: Vec<u8> = Vec::new();
    let mut cap_b: Vec<u8> = Vec::new();
    let mut done = false;
    let mut last_text = String::new();

    for i in 0..2000 {
        step(&mut a, 0.02);
        step(&mut b, 0.02);
        let fa = a.machine.sys.uart_take();
        let fb = b.machine.sys.uart_take();
        cap_a.extend_from_slice(&fa);
        cap_b.extend_from_slice(&fb);
        frames += usize::from(!fa.is_empty()) + usize::from(!fb.is_empty());
        for buf in [&fa, &fb] {
            if buf.windows(4).any(|w| w == b"OBEX") {
                obex = true;
                if obex_frame.is_none() {
                    obex_frame = Some(buf.clone());
                }
            }
            if buf.windows(4).any(|w| w == b"TAMA") {
                tama = true;
                if tama_frame.is_none() {
                    tama_frame = Some(buf.clone());
                }
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

    println!("frames={frames} obex={obex} tama={tama} finished={done} last={last_text}");
    let _ = a.save();
    let _ = b.save();
    if let Some(f) = &obex_frame {
        println!(
            "first OBEX frame ({} bytes): {:02X?}",
            f.len(),
            &f[..f.len().min(48)]
        );
    }
    if let Some(f) = &tama_frame {
        println!(
            "first TAMA frame ({} bytes): {:02X?}",
            f.len(),
            &f[..f.len().min(48)]
        );
    }
    let scan = |name: &str, bytes: &[u8]| {
        let soi = bytes
            .windows(3)
            .filter(|w| *w == [0xFF, 0xD8, 0xFF])
            .count();
        let eoi = bytes.windows(2).filter(|w| *w == [0xFF, 0xD9]).count();
        let first = bytes.windows(3).position(|w| w == [0xFF, 0xD8, 0xFF]);
        let last = bytes.windows(2).rposition(|w| w == [0xFF, 0xD9]);
        let span = match (first, last) {
            (Some(a), Some(b)) if b > a => Some(b + 2 - a),
            _ => None,
        };
        println!(
            "{name}: {} bytes, JPEG SOI={soi} EOI={eoi} first_span={span:?}",
            bytes.len()
        );
        let _ = std::fs::write(format!("/tmp/{name}.bin"), bytes);
    };
    scan("ir-capture-a", &cap_a);
    scan("ir-capture-b", &cap_b);
    if done {
        println!("PASS: the two original firmwares completed an infrared visit");
    } else {
        println!("FAIL: the visit did not finish");
        std::process::exit(1);
    }
}
