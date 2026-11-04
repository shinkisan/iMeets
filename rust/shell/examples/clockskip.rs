//! What does a multi-day RTC jump actually change?
//!
//! Two identical runs from the same save; the second one moves the clock
//! forward three days halfway through. Everything else (inputs, timing,
//! seeds) is identical, so the diff is exactly what the firmware does when
//! it sees the date jump.
use std::path::Path;
use pocket_meets::session::Session;

struct Stats {
    frame: u64,
    sram: Vec<u8>,
    ram: Vec<u8>,
    flash_writes: u64,
    frames: u64,
    asleep: bool,
}

fn run(rom: &str, saves: &str, jump: f64) -> Stats {
    let mut s = Session::open_care(Path::new(rom), Path::new(saves), false).unwrap();
    for _ in 0..150 { s.step(0.02); }
    if s.asleep() {
        let _ = s.key(1, true);
        for _ in 0..10 { s.step(0.02); }
        let _ = s.key(1, false);
        for _ in 0..120 { s.step(0.02); }
    }
    println!("awake before test: {}", !s.asleep());
    // Walk to the clock app: A opens the menu, A picks the first cell.
    let _ = s.key(0, true);
    for _ in 0..10 { s.step(0.02); }
    let _ = s.key(0, false);
    for _ in 0..40 { s.step(0.02); }
    let _ = s.key(0, true);
    for _ in 0..10 { s.step(0.02); }
    let _ = s.key(0, false);
    for _ in 0..60 { s.step(0.02); }

    // Start both runs from exactly the same clock.
    let fixed = 500_000_000i64 * 32768;
    s.machine.sys.rtc.base_ticks = fixed;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    let writes0 = s.machine.sys.flash.writes;

    if jump != 0.0 {
        s.machine.sys.rtc.base_ticks += (jump * 32768.0) as i64;
    }
    for _ in 0..150 { s.step(0.02); }

    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let frame = px.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, v| (h ^ *v as u64).wrapping_mul(0x100_0000_01b3));

    Stats {
        frame,
        sram: s.machine.sys.sram.clone(),
        ram: s.machine.sys.ram.clone(),
        flash_writes: s.machine.sys.flash.writes - writes0,
        frames: s.machine.sys.lcd.frames,
        asleep: s.asleep(),
    }
}

fn diff(label: &str, a: &[u8], b: &[u8], base: u32) {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            let at = base + i as u32;
            match runs.last_mut() {
                Some((start, end)) if at == *end + 1 => *end = at,
                _ => runs.push((at, at)),
            }
        }
    }
    let bytes: usize = runs.iter().map(|(s, e)| (e - s + 1) as usize).sum();
    println!("{label}: {} bytes differ in {} runs", bytes, runs.len());
    for (s, e) in runs.iter().take(12) {
        println!("   {:08X}..{:08X}", s, e);
    }
}

/// Set the clock before the firmware boots (what `align_clock_for_open` does
/// before the toy wakes), then wake it and let it settle.
fn run_boot(rom: &str, saves: &str, jump: f64) -> Stats {
    let mut s = Session::open_care(Path::new(rom), Path::new(saves), false).unwrap();
    let asleep = s.asleep();
    let fixed = 500_000_000i64 * 32768;
    s.machine.sys.rtc.base_ticks = fixed + (jump * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    if asleep {
        let _ = s.key(1, true);
        for _ in 0..10 { s.step(0.02); }
        let _ = s.key(1, false);
    }
    for _ in 0..150 { s.step(0.02); }
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    let frame = px.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, v| (h ^ *v as u64).wrapping_mul(0x100_0000_01b3));
    Stats {
        frame,
        sram: s.machine.sys.sram.clone(),
        ram: s.machine.sys.ram.clone(),
        flash_writes: s.machine.sys.flash.writes,
        frames: s.machine.sys.lcd.frames,
        asleep: s.asleep(),
    }
}

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let a_dir = std::env::args().nth(2).unwrap();
    let b_dir = std::env::args().nth(3).unwrap();
    let days: f64 = std::env::args().nth(4).map(|v| v.parse().unwrap()).unwrap_or(3.0);

    let a = run(&rom, &a_dir, 0.0);
    let b = run(&rom, &b_dir, days * 86_400.0);

    println!("control: writes={} frames={} asleep={} lcd={:016X}", a.flash_writes, a.frames, a.asleep, a.frame);
    println!("jump {:.0}d: writes={} frames={} asleep={} lcd={:016X}", days, b.flash_writes, b.frames, b.asleep, b.frame);
    diff("sram", &a.sram, &b.sram, 0xF800_0000);
    diff("ram ", &a.ram, &b.ram, 0x2000_0000);

    // Same jump, but applied before the toy boots, which is how the clock
    // reaches the firmware's own schedule.
    let boot_a = run_boot(&rom, &a_dir, 0.0);
    let boot_b = run_boot(&rom, &b_dir, days * 86_400.0);
    println!("boot control: lcd={:016X} asleep={}", boot_a.frame, boot_a.asleep);
    println!("boot jump   : lcd={:016X} asleep={}", boot_b.frame, boot_b.asleep);
    diff("boot sram", &boot_a.sram, &boot_b.sram, 0xF800_0000);
    diff("boot ram ", &boot_a.ram, &boot_b.ram, 0x2000_0000);
}
