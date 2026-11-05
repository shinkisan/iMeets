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
    // Stub: mov r0,#2; ldr r3,=set_flag; bx r3
    let stub = 0x13F00usize;
    let code: [u8; 16] = [0x02,0x00,0xA0,0xE3, 0x04,0x30,0x9F,0xE5, 0x13,0xFF,0x2F,0xE1, 0xE0,0xD7,0x18,0x20];
    s.machine.sys.sram[stub..stub+16].copy_from_slice(&code);
    let app = 0x11A5Cusize;
    s.machine.sys.sram[app+0x24..app+0x28].copy_from_slice(&0xF8013F00u32.to_le_bytes());
    s.machine.sys.sram[app+0x28..app+0x2C].copy_from_slice(&1u32.to_le_bytes());
    // Enable the packet callback gate: [game_state+0xF00] = 1.
    let rd0 = u32::from_le_bytes(s.machine.sys.sram[0x10208..0x1020C].try_into().unwrap()) as usize;
    if rd0 >= 0xF8000000 {
        let b = rd0 - 0xF8000000;
        s.machine.sys.sram[b + 0xF00] = 1;
        let v: u32 = std::env::args().nth(2).and_then(|x| x.parse().ok()).unwrap_or(1);
        s.machine.sys.sram[b + 0x2C..b + 0x30].copy_from_slice(&v.to_le_bytes());
        println!("gate f00=1 +2c={v}");
    }
    let tab: Vec<u16> = (0..256).map(|i| { let mut c=i as u16; for _ in 0..8 { c = if c&1!=0 {(c>>1)^0x8408} else {c>>1}; } c }).collect();
    let text = std::env::args().nth(3).unwrap_or_default();
    let mut p = vec![0xFEu8];
    for b in text.as_bytes() { if p.len() < 22 { p.push(*b); } }
    p.extend(std::iter::repeat(0x20).take(22 - p.len()));
    let mut crc=0xFFFFu16; for &x in &p { crc=(crc>>8)^tab[((crc^x as u16)&0xFF) as usize]; }
    let c=!crc; p.push((c&0xFF) as u8); p.push((c>>8) as u8);
    let mut f=vec![0xC0]; f.extend(p); f.push(0xC0); f.extend([0xFF,0xFF,0xFF]);
    for byte in &f { let _ = s.machine.call_arm2(0x20190E3C, 0xF8011A5C, *byte as u32, 100_000); }
    let rd = |o: usize| u32::from_le_bytes(s.machine.sys.sram[o..o+4].try_into().unwrap());
    let g = rd(0x1020C) as usize;
    println!("g={:08X} g+f8={:08X} g+dc={:08X}", g, if g>=0xF8000000 {rd(g-0xF8000000+0xF8)} else {0}, if g>=0xF8000000 {rd(g-0xF8000000+0xDC)} else {0});
    let base = 0xF8011EDCusize - 0xF8000000;
    println!("node {:02X?}", &s.machine.sys.sram[base..base+0x30]);
    let g2 = u32::from_le_bytes(s.machine.sys.sram[0x10208..0x1020C].try_into().unwrap()) as usize;
    let c2 = if g2>=0xF8000000 { u32::from_le_bytes(s.machine.sys.sram[g2-0xF8000000+0x2C..g2-0xF8000000+0x30].try_into().unwrap()) } else {0};
    println!("after feed 2c={c2}");
    println!("after feed flags={:02X?} 14={:02X} 16={:02X}", &s.machine.sys.sram[0x11A54..0x11A58], s.machine.sys.sram[app+0x14], s.machine.sys.sram[app+0x16]);
    let mut px = [0u32; 16_384];
    for i in 0..40 {
        step(&mut s, 0.02);
        if i % 10 == 0 { s.frame(&mut px); println!("t={} [{}] flags={:02X?}", i as f64*0.02, pocket_meets::menu_text::read(&px), &s.machine.sys.sram[0x11A54..0x11A58]); }
    }
}
