use pocket_meets::session::Session;
use std::path::Path;
fn step(s: &mut Session, sec: f64) {
    let mut l = sec;
    while l > 0.0 {
        s.step(l.min(0.02));
        l -= 0.02;
    }
}
fn text(s: &mut Session) -> String {
    let mut px = [0u32; 16_384];
    s.frame(&mut px);
    pocket_meets::menu_text::read(&px)
}
fn tap(s: &mut Session, k: usize) {
    let _ = s.key(k, true);
    step(s, 0.2);
    let _ = s.key(k, false);
    step(s, 0.55);
}
fn home(s: &mut Session) {
    let t0 = text(s);
    if t0.contains("AM") || t0.contains("PM") || t0.contains(":") {
        tap(s, 0);
        step(s, 2.0);
    }
    for _ in 0..6 {
        let t = text(s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(s, 2);
    }
    if !text(s).is_empty() && text(s) != "あああああああ" {
        step(s, 12.0);
    }
    tap(s, 0);
    step(s, 1.0);
}
fn goto(s: &mut Session, label: &str) -> bool {
    home(s);
    for _ in 0..14 {
        let t = text(s);
        if t.contains(label) {
            step(s, 1.0);
            if text(s).contains(label) {
                tap(s, 1);
                return true;
            }
        }
        tap(s, 0);
    }
    false
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let action = std::env::args().nth(2).unwrap_or_else(|| "feed".into());
    let hour: Option<f64> = std::env::args().nth(3).and_then(|v| v.parse().ok());
    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    if let Some(h) = hour {
        let care = s.care_at();
        let day = (care / 86_400.0).floor() * 86_400.0;
        let at = day + h * 3600.0;
        s.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
        s.machine.sys.rtc.base_time = s.machine.sys.game_time;
        s.machine.sys.rtc_irq.tod_base = ((at as i64) % 86_400).rem_euclid(86_400);
        s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
    }
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    s.display_awake(true);
    step(&mut s, 2.0);
    if text(&mut s).contains("つづきから") {
        tap(&mut s, 1);
        step(&mut s, 8.0);
    }
    s.machine.sys.sram[0x0D854..0x0D856].copy_from_slice(&9999u16.to_le_bytes());
    println!("start -> {:?} asleep={}", text(&mut s), s.asleep());
    match action.as_str() {
        "feed" => {
            println!("goto food: {}", goto(&mut s, "ごはん・おやつ"));
            println!("  food -> {:?}", text(&mut s));
            tap(&mut s, 1);
            println!("  fridge -> {:?}", text(&mut s));
            for i in 0..10 {
                tap(&mut s, 1);
                step(&mut s, 1.2);
                println!("  bite {i} -> {:?}", text(&mut s));
            }
            step(&mut s, 3.0);
        }
        "med" => {
            println!("goto med: {}", goto(&mut s, "きゅうきゅうばこ"));
            println!("  med -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 3.0);
            println!("  after -> {:?}", text(&mut s));
        }
        "propose" => {
            println!("goto comm: {}", goto(&mut s, "ツーしん"));
            for attempt in 0..3 {
                tap(&mut s, 1);
                step(&mut s, 1.5);
                let t = text(&mut s);
                if t.contains("あそぶ") {
                    break;
                }
                println!("attempt {attempt}: {:?}", t);
                tap(&mut s, 2);
                step(&mut s, 1.0);
                tap(&mut s, 0);
                step(&mut s, 0.8);
            }
            println!("submenu -> {:?}", text(&mut s));
            tap(&mut s, 0);
            step(&mut s, 1.0);
            tap(&mut s, 0);
            step(&mut s, 1.0);
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("propose -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("start -> {:?}", text(&mut s));
        }
        "play" => {
            println!("goto play: {}", goto(&mut s, "たからばこ"));
            tap(&mut s, 1);
            step(&mut s, 1.5);
            println!("  box -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("  use -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("  after -> {:?}", text(&mut s));
        }
        "bath" => {
            println!("goto bath: {}", goto(&mut s, "トイレ・おふろ"));
            tap(&mut s, 1);
            step(&mut s, 1.5);
            println!("  bath menu -> {:?}", text(&mut s));
            tap(&mut s, 0);
            step(&mut s, 1.0);
            println!("  move -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("  after -> {:?}", text(&mut s));
        }
        "clean" => {
            println!("goto clean: {}", goto(&mut s, "トイレ・おふろ"));
            tap(&mut s, 1);
            step(&mut s, 1.5);
            println!("  clean menu -> {:?}", text(&mut s));
            tap(&mut s, 1);
            step(&mut s, 2.0);
            println!("  after -> {:?}", text(&mut s));
        }
        _ => println!("unknown action"),
    }
    let _ = s.save();
}
