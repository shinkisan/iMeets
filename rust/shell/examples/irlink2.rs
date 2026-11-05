//! Two toys: Play, wait, then press B on both to start communication.
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
fn main() {
    let da = std::env::args().nth(1).unwrap();
    let db = std::env::args().nth(2).unwrap();
    let mut a = Session::open_care(Path::new(ROM), Path::new(&da), false).unwrap();
    let mut b = Session::open_care(Path::new(ROM), Path::new(&db), false).unwrap();
    to_link(&mut a); to_link(&mut b);
    // Play on both.
    let _ = a.key(1, true); let _ = b.key(1, true); step(&mut a, 0.2); step(&mut b, 0.2);
    let _ = a.key(1, false); let _ = b.key(1, false);
    step(&mut a, 1.0); step(&mut b, 1.0);
    let mut px = [0u32; 16_384];
    a.frame(&mut px); let sa = pocket_meets::menu_text::read(&px);
    b.frame(&mut px); let sb = pocket_meets::menu_text::read(&px);
    eprintln!("wait: A[{}] B[{}]", sa, sb);
    // Start communication, optionally with A leading.
    let lead: f64 = std::env::args().nth(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let _ = a.key(1, true); step(&mut a, 0.2); let _ = a.key(1, false);
    step(&mut a, lead);
    let _ = b.key(1, true); step(&mut b, 0.2); let _ = b.key(1, false);
    // Feed each side a valid packet once communication starts.
    let tab: Vec<u16> = (0..256).map(|i| { let mut c=i as u16; for _ in 0..8 { c = if c&1!=0 {(c>>1)^0x8408} else {c>>1}; } c }).collect();
    let mut p = vec![0xFEu8]; p.extend(std::iter::repeat(0).take(21));
    let mut crc=0xFFFFu16; for &b in &p { crc=(crc>>8)^tab[((crc^b as u16)&0xFF) as usize]; }
    let c=!crc; p.push((c&0xFF) as u8); p.push((c>>8) as u8);
    let mut f=vec![0xC0]; f.extend(p); f.push(0xC0);
    for byte in &f { a.machine.sys.uart_receive(*byte); b.machine.sys.uart_receive(*byte); }
    let idle_feed = std::env::args().nth(4).is_some();
    for i in 0..1500 {
        step(&mut a, 0.02); step(&mut b, 0.02);
        if idle_feed && i % 5 == 0 {
            a.machine.sys.uart_receive(0xFF); b.machine.sys.uart_receive(0xFF);
        }
        let fa = a.machine.sys.uart_take(); let fb = b.machine.sys.uart_take();
        for x in &fa { b.machine.sys.uart_receive(*x); }
        for x in &fb { a.machine.sys.uart_receive(*x); }
        if !fa.is_empty() || !fb.is_empty() { eprintln!("t={:5.2} A->B {:02X?} B->A {:02X?}", i as f64*0.02, fa, fb); }
        if i % 50 == 0 {
            a.frame(&mut px); let x = pocket_meets::menu_text::read(&px);
            b.frame(&mut px); let y = pocket_meets::menu_text::read(&px);
            eprintln!("t={:5.1} A[{}] B[{}]", i as f64*0.02, x, y);
        }
    }
}
