//! Research capture: ROM snapshot output-dir seconds turbo sample-seconds.
//! Writes only to the specified research directory, never to the input snapshot.
use std::{path::Path, sync::Arc};
use tg18::{snapshot::Snapshot, Machine, Outcome, CPU_HZ};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() >= 5,
        "ROM snapshot output-dir seconds [turbo] [sample-seconds]"
    );
    let image = Arc::new(std::fs::read(&args[1]).unwrap());
    let mut m = Machine::new(image.clone(), &image, tg18::rtc_seconds_now());
    Snapshot::load(&args[2]).unwrap().restore(&mut m).unwrap();
    let profile = pocket_meets::potty::PottyProfile::for_image(&image)
        .expect("only the verified Sweets ROM is supported");
    m.print_log = false;
    m.never_sleep = true;
    m.sys.turbo = args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let duration: f64 = args[4].parse().unwrap();
    let interval: f64 = args.get(6).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let out = Path::new(&args[3]);
    std::fs::create_dir_all(out).unwrap();
    let start = m.now();
    if let Ok(spec) = std::env::var("PROBE_PRESS") {
        for press in spec.split(',') {
            let (time, key) = press.split_once(':').unwrap();
            m.press(
                start + time.parse::<f64>().unwrap(),
                tg18::Key::parse(key).unwrap(),
            );
        }
    }
    if std::env::var_os("PROBE_WATCH").is_some() {
        m.sys.watch_read = Some((0xf800d834, 0xf800dc00));
        m.sys.watch_read_left = 200000;
    }
    let mut at: f64 = 0.0;
    let mut logs = String::new();
    loop {
        let stem = out.join(format!("{:012}", (at * 1000.0).round() as u64));
        std::fs::write(stem.with_extension("sram"), &m.sys.sram[..0x20000]).unwrap();
        tg18::png::write(
            stem.with_extension("png").to_str().unwrap(),
            128,
            128,
            &m.lcd_rgb(),
            1,
        )
        .unwrap();
        Snapshot::capture(&m)
            .save(stem.with_extension("t18s").to_str().unwrap())
            .unwrap();
        for line in m.log.drain(..) {
            logs.push_str(&format!("{at:.2} {line}\n"));
        }
        println!(
            "{at:.2} game={:.3} pc={:08x} potty={:?}",
            m.sys.game_time,
            m.cpu.pc(),
            profile.read(&m)
        );
        if at >= duration {
            break;
        }
        at = (at + interval).min(duration);
        let limit = ((start + at) * CPU_HZ) as u64;
        while m.executed < limit {
            if let Some(addr) = m.idle_counter {
                m.sys.poke(addr, &[0]);
            }
            let next = (m.executed + (0.05 * CPU_HZ) as u64).min(limit);
            if m.sys.powered_off {
                m.press(m.now() + 0.001, tg18::Key::B);
                m.power_cycle(next);
            }
            if !m.sys.powered_off {
                let result = m.run_with_sleep(next);
                assert!(
                    matches!(result, Outcome::Limit | Outcome::PowerOff),
                    "stopped: {result:?}"
                );
            }
        }
        m.sys.sound.events.clear();
    }
    std::fs::write(out.join("firmware.log"), logs).unwrap();
}
