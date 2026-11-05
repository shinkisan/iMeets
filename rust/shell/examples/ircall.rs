use std::path::Path;
use pocket_meets::session::Session;
const ROM: &str = "/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin";
fn step(s: &mut Session, seconds: f64) { let mut l = seconds; while l > 0.0 { s.step(l.min(0.02)); l -= 0.02; } }
fn press(s: &mut Session, k: usize) { let _ = s.key(k, true); step(s, 0.20); let _ = s.key(k, false); step(s, 0.55); }
fn main() {
    let saves = std::env::args().nth(1).unwrap();
    let byte: u32 = std::env::args().nth(2).and_then(|v| v.parse().ok()).unwrap_or(1);
    let mut s = Session::open_care(Path::new(ROM), Path::new(&saves), false).unwrap();
    step(&mut s, 2.0);
    for _ in 0..5 { press(&mut s, 0); }
    press(&mut s, 1); press(&mut s, 1);
    s.machine.stop_on_bluetooth = false; s.machine.sys.uart_trace = true;
    let _ = s.key(1, true); step(&mut s, 0.2); let _ = s.key(1, false);
    step(&mut s, 0.5);
    let ok = s.machine.call_arm(0x2018DB90, byte, 500_000);
    println!("call ok={ok} fault={:?}", s.machine.cpu.fault);
    let mut px = [0u32; 16_384];
    for i in 0..300 {
        step(&mut s, 0.02);
        let tx = s.machine.sys.uart_take();
        if !tx.is_empty() { println!("t={:5.2} TX {:02X?}", i as f64*0.02, tx); }
        if i % 100 == 0 { s.frame(&mut px); println!("  t={:4.1} [{}]", i as f64*0.02, pocket_meets::menu_text::read(&px)); }
    }
}
