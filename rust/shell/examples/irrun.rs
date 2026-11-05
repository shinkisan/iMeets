//! Drive the in-game IR link with a one-sided virtual partner.
use std::path::Path;
use pocket_meets::session::Session;

fn step(s: &mut Session, seconds: f64) { let mut l = seconds; while l > 0.0 { s.step(l.min(0.02)); l -= 0.02; } }
fn press(s: &mut Session, k: usize) { let _ = s.key(k, true); step(s, 0.20); let _ = s.key(k, false); step(s, 0.55); }

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();
    let seed: u8 = std::env::args().nth(3).and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok()).unwrap_or(0);
    let mut s = Session::open_care(Path::new(&rom), Path::new(&saves), false).unwrap();
    step(&mut s, 2.0);
    for _ in 0..5 { press(&mut s, 0); }
    press(&mut s, 1);
    press(&mut s, 1);
    s.machine.stop_on_bluetooth = false;
    press(&mut s, 1);
    step(&mut s, 0.5);
    eprintln!("wait pc={:08X} irq14_handler={:08X}", s.machine.cpu.r[15], s.machine.sys.peek32(0xF800A630));
    s.machine.sys.uart_receive(seed);
    eprintln!("fed {seed:02X}");
    let mut px = [0u32; 16_384];
    for i in 0..120 {
        step(&mut s, 0.02);
        let out = s.machine.sys.uart_take();
        if !out.is_empty() {
            let hex: Vec<String> = out.iter().map(|b| format!("{b:02X}")).collect();
            eprintln!("t={:5.2} pc={:08X} tx {}", i as f64 * 0.02, s.machine.cpu.r[15], hex.join(" "));
        }
        if i == 50 {
            std::fs::write("/tmp/opencode/sram-wait.bin", &s.machine.sys.sram).unwrap();
            eprintln!("wait dump pc={:08X}", s.machine.cpu.r[15]);
        }
        if i % 20 == 0 {
            s.frame(&mut px);
            eprintln!("  t={:4.1} pc={:08X} screen=[{}] handler={:08X}", i as f64 * 0.02, s.machine.cpu.r[15], pocket_meets::menu_text::read(&px), s.machine.sys.peek32(0xF800A630));
        }
    }
    std::fs::write("/tmp/opencode/sram-run.bin", &s.machine.sys.sram).unwrap();
    println!("dumped");
}
