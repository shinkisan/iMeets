//! Two emulated toys meeting over a virtual infrared link.
use std::path::Path;
use pocket_meets::session::Session;

const ROM: &str = "/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin";
fn step(s: &mut Session, seconds: f64) { let mut l = seconds; while l > 0.0 { s.step(l.min(0.02)); l -= 0.02; } }
fn press(s: &mut Session, k: usize) { let _ = s.key(k, true); step(s, 0.20); let _ = s.key(k, false); step(s, 0.55); }
fn to_link(s: &mut Session) {
    step(s, 2.0);
    for _ in 0..5 { press(s, 0); }
    press(s, 1); press(s, 1);
    s.machine.stop_on_bluetooth = false;
}
fn play(s: &mut Session) { let _ = s.key(1, true); step(s, 0.2); let _ = s.key(1, false); }

fn main() {
    let a_dir = std::env::args().nth(1).unwrap();
    let b_dir = std::env::args().nth(2).unwrap();
    let lead: f64 = std::env::args().nth(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let inject: Option<u8> = std::env::args().nth(4).and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    let mut a = Session::open_care(Path::new(ROM), Path::new(&a_dir), false).unwrap();
    let mut b = Session::open_care(Path::new(ROM), Path::new(&b_dir), false).unwrap();
    to_link(&mut a); to_link(&mut b);
    play(&mut a);
    step(&mut a, lead);
    play(&mut b);
    let mut px = [0u32; 16_384];
    let mut ticks = 0u32;
    for i in 0..1500 {
        step(&mut a, 0.02); step(&mut b, 0.02); ticks += 1;
        if inject.is_some() && ticks % 10 == 0 {
            for s in [&mut a, &mut b] {
                for _ in 0..10 { s.machine.sys.uart_receive(0xFF); }
                s.machine.sys.uart_receive(0xC0);
            }
        }
        let fa = a.machine.sys.uart_take(); let fb = b.machine.sys.uart_take();
        for x in &fa { b.machine.sys.uart_receive(*x); }
        for x in &fb { a.machine.sys.uart_receive(*x); }
        if !fa.is_empty() || !fb.is_empty() { eprintln!("t={:5.2} A->B {:02X?} B->A {:02X?}", i as f64*0.02, fa, fb); }
        if i == 100 {
            for (name, off) in [("drv", 0xA7C8usize), ("app", 0x11A58usize)] {
                let mut line = String::new();
                for k in 0..0x80 {
                    let v = u32::from_le_bytes(a.machine.sys.sram[off+k*4..off+k*4+4].try_into().unwrap());
                    if (0x2000_0000..0x2020_0000).contains(&v) || (0xF800_0000..0xF810_0000).contains(&v) {
                        line.push_str(&format!(" +{:02X}={:08X}", k*4, v));
                    }
                }
                eprintln!("{name}:{line}");
            }
        }
        if i % 50 == 0 {
            a.frame(&mut px); let sa = pocket_meets::menu_text::read(&px);
            b.frame(&mut px); let sb = pocket_meets::menu_text::read(&px);
            eprintln!("t={:5.1} A[{}] B[{}]", i as f64*0.02, sa, sb);
        }
    }
}
