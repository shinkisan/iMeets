//! Read/replay a private Sweets snapshot into a new research directory.
//! ROM input.t18s new-output-dir profile|cross|resume|morning|same-minute|sleep-resume|fragmented-pause [seconds]
//! resume/morning simulate the four RTC/TOD base writes; they do not invoke UI pause.
//! Session::step ignores care.paused; these replay modes intentionally use step, not pump.
//! Never writes to the source snapshot or a live app container.
use pocket_meets::session::Session;
use std::path::{Path, PathBuf};

fn step(s: &mut Session, seconds: f64) {
    let mut left = seconds;
    while left > 0.000_001 {
        let q = left.min(0.02);
        assert_eq!(s.step(q), 0, "firmware stopped: {}", s.error);
        left -= q;
    }
}
fn text(s: &mut Session) -> String {
    let mut pixels = [0u32; 16384];
    s.frame(&mut pixels);
    pocket_meets::menu_text::read(&pixels)
}
fn tap(s: &mut Session, key: usize) {
    s.key(key, true).unwrap();
    step(s, 0.2);
    s.key(key, false).unwrap();
    step(s, 0.7);
}
fn clock(s: &mut Session, hour: f64) {
    let old = s.machine.sys.rtc.ticks(s.machine.sys.game_time) as f64 / 32768.0;
    let at = (old / 86400.0).floor() * 86400.0 + hour * 3600.0;
    s.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = (at as i64).rem_euclid(86400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
}
fn capture(s: &mut Session, out: &Path, name: &str) {
    std::fs::write(out.join(format!("{name}.sram")), &s.machine.sys.sram).unwrap();
    tg18::snapshot::Snapshot::capture(&s.machine)
        .save(out.join(format!("{name}.t18s")).to_str().unwrap())
        .unwrap();
    tg18::png::write(
        out.join(format!("{name}.png")).to_str().unwrap(),
        128,
        128,
        &s.machine.lcd_rgb(),
        2,
    )
    .unwrap();
    let screen = text(s);
    println!(
        "{name} sim={:.3} rtc={:.3} stage={} text={:?}",
        s.seconds(),
        s.machine.sys.rtc.ticks(s.machine.sys.game_time) as f64 / 32768.0,
        s.machine.sys.sram[0xd886],
        screen
    );
    println!(
        "  age={} G={} off={} ctrl16={:02x} date={:02x?}",
        u16::from_le_bytes(s.machine.sys.sram[0xd86a..0xd86c].try_into().unwrap()),
        u32::from_le_bytes(s.machine.sys.sram[0xd850..0xd854].try_into().unwrap()),
        s.machine.sys.powered_off,
        s.machine.sys.sram[0xd84a],
        &s.machine.sys.sram[0xd8c1..0xd8c5]
    );
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() >= 5,
        "ROM input.t18s new-output-dir profile|cross|resume [seconds]"
    );
    let image = std::fs::read(&args[1]).unwrap();
    assert!(
        image.len() == 8 * 1024 * 1024 && tg18::code_hash(&image) == 0x28ad63c34fa9e369,
        "only the verified Sweets v062 code is supported"
    );
    assert!(
        [
            "profile",
            "cross",
            "resume",
            "morning",
            "same-minute",
            "sleep-resume",
            "fragmented-pause"
        ]
        .contains(&args[4].as_str()),
        "unknown mode"
    );
    let out = PathBuf::from(&args[3]);
    assert!(!out.exists(), "output must be a new directory");
    std::fs::create_dir_all(&out).unwrap();
    let saves = out.join("Saves");
    let stem = saves.join("fw_tg18_jp_v062_2019-09-12_sweets");
    std::fs::create_dir_all(&stem).unwrap();
    std::fs::copy(&args[2], stem.join("autosave.t18s")).unwrap();
    let mut s = Session::open_care(Path::new(&args[1]), &saves, true).unwrap();
    s.machine.print_log = false;
    s.machine.sys.turbo = 1.0;
    s.display_awake(true);
    clock(&mut s, 10.0);
    step(&mut s, 2.0);
    for _ in 0..10 {
        let t = text(&mut s);
        if t.is_empty() {
            break;
        }
        tap(&mut s, 2);
    }
    capture(&mut s, &out, "baseline");
    match args[4].as_str() {
        "profile" => {
            tap(&mut s, 0);
            for _ in 0..16 {
                if text(&mut s).contains("じょうたい せってい") {
                    break;
                }
                tap(&mut s, 0);
            }
            tap(&mut s, 1);
            capture(&mut s, &out, "state-menu");
            tap(&mut s, 1);
            capture(&mut s, &out, "profile");
            s.machine.sys.watch_read = Some((0xf800d800, 0xf800d900));
            s.machine.sys.watch_read_left = 20000;
            tap(&mut s, 0);
            s.machine.sys.watch_read = None;
            capture(&mut s, &out, "stats");
        }
        "fragmented-pause" => {
            step(&mut s, 30.0);
            capture(&mut s, &out, "first-30s");
            s.care_pause(true, tg18::unix_now());
            let cycles = s.machine.executed;
            let game_time = s.machine.sys.game_time;
            s.pump(&pocket_meets::session::PumpIn {
                now_unix: tg18::unix_now() + 86400.0,
                dt: 86400.0,
                visible: true,
                ..Default::default()
            });
            assert_eq!(s.machine.executed, cycles);
            assert_eq!(s.machine.sys.game_time, game_time);
            capture(&mut s, &out, "paused-large-gap");
            s.care_pause(false, tg18::unix_now());
            capture(&mut s, &out, "resumed-clock");
            step(&mut s, 30.0);
            capture(&mut s, &out, "second-30s");
        }
        "sleep-resume" => {
            clock(&mut s, 23.0 + 59.0 / 60.0);
            step(&mut s, 1.0);
            s.display_awake(false);
            step(&mut s, 45.0);
            capture(&mut s, &out, "before-sleep-resume");
            clock(&mut s, 34.0);
            s.display_awake(true);
            step(&mut s, 20.0);
            capture(&mut s, &out, "after-sleep-resume");
        }
        "cross" | "resume" | "morning" | "same-minute" => {
            clock(&mut s, 23.0 + 59.0 / 60.0 + 50.0 / 3600.0);
            step(&mut s, 1.0);
            capture(&mut s, &out, "before-midnight");
            s.machine.sys.watch_write = Some((0xf800d834, 0xf800d900));
            s.machine.sys.watch_write_left = 20000;
            if args[4] == "resume" {
                clock(&mut s, 24.0 + 0.1 / 3600.0);
            }
            if args[4] == "morning" {
                clock(&mut s, 34.0);
            }
            if args[4] == "same-minute" {
                clock(&mut s, 47.0 + 59.0 / 60.0 + 51.0 / 3600.0);
            }
            let duration = args.get(5).and_then(|x| x.parse().ok()).unwrap_or(25.0);
            step(&mut s, duration);
            s.machine.sys.watch_write = None;
            capture(&mut s, &out, "after-midnight");
        }
        _ => panic!("unknown mode"),
    }
}
