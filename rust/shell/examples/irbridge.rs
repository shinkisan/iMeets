use std::path::Path;
use pocket_meets::session::Session;
const ROM: &str = "/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin";
fn step(s: &mut Session, seconds: f64) { let mut l = seconds; while l > 0.0 { s.step(l.min(0.02)); l -= 0.02; } }
fn press(s: &mut Session, k: usize) { let _ = s.key(k, true); step(s, 0.20); let _ = s.key(k, false); step(s, 0.55); }
fn to_link(s: &mut Session) { step(s, 2.0); for _ in 0..5 { press(s, 0); } press(s, 1); press(s, 1); s.machine.stop_on_bluetooth = false; }
fn main() {
    let da = std::env::args().nth(1).unwrap();
    let db = std::env::args().nth(2).unwrap();
    let mut a = Session::open_care(Path::new(ROM), Path::new(&da), false).unwrap();
    let mut b = Session::open_care(Path::new(ROM), Path::new(&db), false).unwrap();
    to_link(&mut a); to_link(&mut b);
    let _ = a.key(1, true); let _ = b.key(1, true); step(&mut a, 0.2); step(&mut b, 0.2);
    let _ = a.key(1, false); let _ = b.key(1, false);
    step(&mut a, 1.0); step(&mut b, 1.0);
    let _ = a.key(1, true); let _ = b.key(1, true); step(&mut a, 0.2); step(&mut b, 0.2);
    let _ = a.key(1, false); let _ = b.key(1, false);
    let app = 0x11A5Cusize;
    let buf = 0x13E00usize;
    for (i, x) in a.machine.sys.sram[buf..buf+24].iter_mut().enumerate() { *x = if i == 0 { 0xFE } else { i as u8 }; }
    for s in [&mut a, &mut b] {
        let stub: [u8; 20] = [0x08,0x30,0x9F,0xE5, 0x02,0x00,0xA0,0xE3, 0x13,0xFF,0x2F,0xE1, 0x00,0x00,0x00,0x00, 0xE0,0xD7,0x18,0x20];
        s.machine.sys.sram[0x13F00..0x13F14].copy_from_slice(&stub);
        s.machine.sys.sram[app+0x24..app+0x28].copy_from_slice(&0xF8013F00u32.to_le_bytes());
        s.machine.sys.sram[app+0x28..app+0x2C].copy_from_slice(&1u32.to_le_bytes());
        s.machine.sys.uart[8..12].copy_from_slice(&0xF070u32.to_le_bytes());
        s.machine.sys.sram[app+0x4B4] = 2;
        s.machine.sys.sram[app+0x4B0..app+0x4B4].copy_from_slice(&(0xF8000000u32 + buf as u32).to_le_bytes());
        let _ = s.machine.call_arm_ret(0x2018DC28, 0, 300_000);
    }
    let mut px = [0u32; 16_384];
    for i in 0..500 {
        a.machine.sys.uart[8..12].copy_from_slice(&0xF070u32.to_le_bytes());
        b.machine.sys.uart[8..12].copy_from_slice(&0xF070u32.to_le_bytes());
        for z in [&mut a, &mut b] {
            z.machine.sys.sram[0x11A5C+0x24..0x11A5C+0x28].copy_from_slice(&0xF8013F00u32.to_le_bytes());
            z.machine.sys.sram[0x11A5C+0x28..0x11A5C+0x2C].copy_from_slice(&1u32.to_le_bytes());
        }
        step(&mut a, 0.02); step(&mut b, 0.02);
        if i >= 50 {
            for z in [&mut a, &mut b] {
                let r = u32::from_le_bytes(z.machine.sys.sram[0x1020C..0x10210].try_into().unwrap()) as usize;
                if r >= 0xF8000000 {
                    let bb = r - 0xF8000000;
                    z.machine.sys.sram[bb+0x2C..bb+0x30].copy_from_slice(&1u32.to_le_bytes());
                    z.machine.sys.sram[bb+0xF00] = 1;
                }
            }
        }
        let fa = a.machine.sys.uart_take(); let fb = b.machine.sys.uart_take();
        if !fa.is_empty() { for x in &fa { b.machine.sys.uart_receive(*x); } }
        if !fb.is_empty() { for x in &fb { a.machine.sys.uart_receive(*x); } }
        if i % 50 == 0 {
            a.frame(&mut px); let x = pocket_meets::menu_text::read(&px);
            b.frame(&mut px); let y = pocket_meets::menu_text::read(&px);
            let rd = |z: &Session, o: usize| u32::from_le_bytes(z.machine.sys.sram[o..o+4].try_into().unwrap());
            let ga = rd(&a, 0x10208) as usize; let gb = rd(&b, 0x10208) as usize;
            let ca = if ga>=0xF8000000 { u32::from_le_bytes(a.machine.sys.sram[ga-0xF8000000+0x2C..ga-0xF8000000+0x30].try_into().unwrap()) } else {0};
            let cb = if gb>=0xF8000000 { u32::from_le_bytes(b.machine.sys.sram[gb-0xF8000000+0x2C..gb-0xF8000000+0x30].try_into().unwrap()) } else {0};
            println!("t={:4.1} A[{}] B[{}] 2cA={ca} 2cB={cb} tx={} rxB={} fA={:02X?} fB={:02X?}", i as f64*0.02, x, y, fa.len(), b.machine.sys.uart_rx.len(), &a.machine.sys.sram[0x11A54..0x11A58], &b.machine.sys.sram[0x11A54..0x11A58]);
        }
    }
}
