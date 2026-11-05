use std::path::Path;
use pocket_meets::session::Session;
const ROM: &str = "/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin";
fn step(s: &mut Session, seconds: f64) { let mut l = seconds; while l > 0.0 { s.step(l.min(0.02)); l -= 0.02; } }
fn press(s: &mut Session, k: usize) { let _ = s.key(k, true); step(s, 0.20); let _ = s.key(k, false); step(s, 0.55); }
fn main() {
    let saves = std::env::args().nth(1).unwrap();
    let mut s = Session::open_care(Path::new(ROM), Path::new(&saves), false).unwrap();
    step(&mut s, 2.0);
    for _ in 0..5 { press(&mut s, 0); }
    press(&mut s, 1); press(&mut s, 1);
    s.machine.stop_on_bluetooth = false;
    let _ = s.key(1, true); step(&mut s, 0.2); let _ = s.key(1, false);
    step(&mut s, 1.0);
    press(&mut s, 1);
    step(&mut s, 0.2);
    let app = 0xF8011A5Cu32;
    // Feed a valid frame straight into the protocol engine.
    let tab: Vec<u16> = (0..256).map(|i| { let mut c=i as u16; for _ in 0..8 { c = if c&1!=0 {(c>>1)^0x8408} else {c>>1}; } c }).collect();
    let mut p = vec![0xFEu8]; p.extend(std::iter::repeat(0).take(21));
    let mut crc=0xFFFFu16; for &x in &p { crc=(crc>>8)^tab[((crc^x as u16)&0xFF) as usize]; }
    let c=!crc; p.push((c&0xFF) as u8); p.push((c>>8) as u8);
    let mut f=vec![0xC0]; f.extend(p); f.push(0xC0);
    for (i, byte) in f.iter().enumerate() {
        let (ok, ret) = s.machine.call_arm2(0x20190E3C, app, *byte as u32, 100_000);
        println!("feed {i} {byte:02X} ok={ok} ret={ret:08X} 14={:02X} 16={:02X} 18={:02X} 24={:08X}",
            s.machine.sys.sram[0x11A5C+0x14], s.machine.sys.sram[0x11A5C+0x16], s.machine.sys.sram[0x11A5C+0x18],
            u32::from_le_bytes(s.machine.sys.sram[0x11A5C+0x24..0x11A5C+0x28].try_into().unwrap()));
    }
    for flag in 1..=4u32 {
        let _ = s.machine.call_arm(0x2018D7E0, flag, 50_000);
        step(&mut s, 0.3);
        let mut px = [0u32; 16_384]; s.frame(&mut px);
        println!("after flag {flag}: [{}] flags={:02X?}", pocket_meets::menu_text::read(&px), &s.machine.sys.sram[0x11A54..0x11A58]);
    }
    let app2 = 0x11A5Cusize;
    let rd = |o: usize| u32::from_le_bytes(s.machine.sys.sram[o..o+4].try_into().unwrap());
    println!("app f8={:08X} dc={:08X} c={:08X}", rd(app2+0xF8), rd(app2+0xDC), rd(app2+0x0C));
    let mut px = [0u32; 16_384]; s.frame(&mut px);
    println!("flags {:02X?} screen=[{}]", &s.machine.sys.sram[0x11A54..0x11A58], pocket_meets::menu_text::read(&px));
}
