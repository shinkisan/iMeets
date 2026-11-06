//! Two toys over a plain TCP socket, for local LAN debugging.
//!
//! One process hosts (`host`), the other connects (`join`). After a small
//! line-based handshake both navigate to the `みーつ` submenu, synchronise
//! the second B press, then forward raw UART bytes over the socket.
//!
//! Usage:
//!   POCKET_MEETS_ROM=... irlan host <save> <port> <seconds> <hour> [option]
//!   POCKET_MEETS_ROM=... irlan join <save> <port> <seconds> <hour> [option]
//!
//! option: 0 あそぶ (default), 2 プロポーズ (host proposes, join waits)
//!
//! `IRLAN_ANSWER=decline` makes the joining side answer `しない`.

use pocket_meets::session::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

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
    step(s, 0.7);
}
fn hour_now(s: &mut Session, hour: f64) {
    let care = s.care_at();
    let day = (care / 86_400.0).floor() * 86_400.0;
    let at = day + hour * 3600.0;
    s.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
    s.machine.sys.rtc.base_time = s.machine.sys.game_time;
    s.machine.sys.rtc_irq.tod_base = ((at as i64) % 86_400).rem_euclid(86_400);
    s.machine.sys.rtc_irq.tod_set_at = s.machine.sys.game_time;
}
fn home(s: &mut Session) {
    for _ in 0..14 {
        let t = text(s);
        if t.is_empty() || t == "あああああああ" {
            break;
        }
        tap(s, 1);
    }
    tap(s, 0);
    step(s, 1.0);
}
fn open_submenu(s: &mut Session) -> String {
    home(s);
    for _ in 0..16 {
        if text(s).contains("ツーしん") {
            break;
        }
        tap(s, 0);
    }
    tap(s, 1);
    step(s, 2.0);
    tap(s, 1);
    step(s, 2.0);
    text(s)
}
fn read_line(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut out = String::new();
    let mut b = [0u8; 1];
    loop {
        stream.read_exact(&mut b)?;
        if b[0] == b'\n' {
            return Ok(out);
        }
        out.push(b[0] as char);
    }
}
fn send_line(stream: &mut TcpStream, line: &str) {
    let _ = stream.write_all(format!("{line}\n").as_bytes());
    let _ = stream.flush();
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let role = std::env::args().nth(1).expect("host|join");
    let base = std::env::args().nth(2).expect("save");
    let port: u16 = std::env::args()
        .nth(3)
        .and_then(|v| v.parse().ok())
        .unwrap_or(47000);
    let secs: f64 = std::env::args()
        .nth(4)
        .and_then(|v| v.parse().ok())
        .unwrap_or(40.0);
    let hour: f64 = std::env::args()
        .nth(5)
        .and_then(|v| v.parse().ok())
        .unwrap_or(10.0);
    let option: usize = std::env::args()
        .nth(6)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let answer_decline = std::env::var("IRLAN_ANSWER").as_deref() == Ok("decline");

    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    hour_now(&mut s, hour);
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    s.display_awake(true);
    step(&mut s, 2.0);
    s.machine.stop_on_bluetooth = false;

    // Connect before touching the menu, so both sides have the same clock.
    let mut stream = if role == "host" {
        let listener = TcpListener::bind(("0.0.0.0", port)).expect("bind");
        eprintln!("listening on 0.0.0.0:{port}");
        let (s, peer) = listener.accept().expect("accept");
        eprintln!("accepted {peer}");
        s
    } else {
        let start = Instant::now();
        loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => break s,
                Err(_) if start.elapsed() < Duration::from_secs(20) => {
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(e) => panic!("connect: {e}"),
            }
        }
    };
    stream.set_nodelay(true).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();

    send_line(&mut stream, "TAMA-LAN 1");
    let hello = read_line(&mut stream).unwrap_or_default();
    eprintln!("peer hello: {hello:?}");

    let sub = open_submenu(&mut s);
    eprintln!("submenu: {sub:?}");
    // Host proposes when option is 2; the joining side always waits in あそぶ.
    let pick = if option == 2 && role == "host" {
        2
    } else if option == 2 {
        0
    } else {
        option
    };
    for _ in 0..pick {
        tap(&mut s, 0);
    }
    tap(&mut s, 1);
    step(&mut s, 2.0);
    eprintln!("after select: {:?}", text(&mut s));

    // Tell the peer we are at the waiting screen, then let the host open
    // the link and the guest follow 0.5 s later.
    eprintln!("{role}: send WAIT");
    send_line(&mut stream, "WAIT");
    eprintln!("{role}: read WAIT -> {:?}", read_line(&mut stream));
    if role == "host" {
        std::thread::sleep(Duration::from_millis(1000));
        eprintln!("host: send GO");
        send_line(&mut stream, "GO");
    } else {
        eprintln!("join: read GO -> {:?}", read_line(&mut stream));
        std::thread::sleep(Duration::from_millis(500));
    }
    let _ = s.key(1, true);
    step(&mut s, 0.2);
    let _ = s.key(1, false);
    if role == "host" {
        // Match the local bridge: the host's emulated clock leads by 0.5 s.
        step(&mut s, 0.5);
    }
    step(&mut s, 1.0);
    eprintln!("after start: {:?}", text(&mut s));
    // Same well-formed frame the local bridge uses to kick the firmware.
    let tab: Vec<u16> = (0..256)
        .map(|i| {
            let mut c = i as u16;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    (c >> 1) ^ 0x8408
                } else {
                    c >> 1
                };
            }
            c
        })
        .collect();
    let mut p = vec![0xFEu8];
    p.extend(std::iter::repeat(0).take(21));
    let mut crc = 0xFFFFu16;
    for &byte in &p {
        crc = (crc >> 8) ^ tab[((crc ^ byte as u16) & 0xFF) as usize];
    }
    let c = !crc;
    p.push((c & 0xFF) as u8);
    p.push((c >> 8) as u8);
    let mut f = vec![0xC0];
    f.extend(p);
    f.push(0xC0);
    if std::env::var("IRLAN_NO_KICK").is_err() {
        for byte in &f {
            s.machine.sys.uart_receive(*byte);
        }
    }
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    fn send_frame(stream: &mut TcpStream, typ: u8, data: &[u8]) {
        let mut h = [
            typ,
            (data.len() & 0xFF) as u8,
            ((data.len() >> 8) & 0xFF) as u8,
        ];
        let _ = stream.write_all(&mut h);
        if !data.is_empty() {
            let _ = stream.write_all(data);
        }
        let _ = stream.flush();
    }
    fn recv_frame(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
        let mut h = [0u8; 3];
        stream.read_exact(&mut h).ok()?;
        let len = h[1] as usize | ((h[2] as usize) << 8);
        let mut data = vec![0u8; len];
        if len > 0 {
            stream.read_exact(&mut data).ok()?;
        }
        Some((h[0], data))
    }

    let mut sent = 0usize;
    let mut recv = 0usize;
    let mut answered = false;
    let mut answer_phase = 0u32;
    let mut answer_tick = 0usize;
    let ticks = (secs / 0.02) as usize;
    let mut px = [0u32; 16_384];
    for i in 0..ticks {
        step(&mut s, 0.02);
        let out = s.machine.sys.uart_take();
        if !out.is_empty() {
            if i < 3 {
                eprintln!(
                    "i={i} now={:.2} out={} first={:02X?}",
                    s.machine.now(),
                    out.len(),
                    &out[..out.len().min(24)]
                );
            }
            send_frame(&mut stream, 0, &out);
            sent += out.len();
        }
        // Tick barrier: both sides advance exactly one 20 ms slice together.
        send_frame(&mut stream, 1, &[]);
        let mut got_tick = false;
        let mut guard = 0;
        while !got_tick && guard < 64 {
            guard += 1;
            match recv_frame(&mut stream) {
                Some((0, data)) => {
                    if i < 3 {
                        eprintln!(
                            "i={i} now={:.2} in={} first={:02X?}",
                            s.machine.now(),
                            data.len(),
                            &data[..data.len().min(24)]
                        );
                    }
                    for b in data {
                        s.machine.sys.uart_receive(b);
                        recv += 1;
                    }
                }
                Some((1, _)) => got_tick = true,
                Some(_) => {}
                None => {
                    eprintln!("i={i} peer closed or timed out");
                    got_tick = true;
                }
            }
        }
        if i % 100 == 0 {
            s.frame(&mut px);
            let t = pocket_meets::menu_text::read(&px);
            eprintln!("i={i:4} sent={sent} recv={recv} text=[{t}]");
        }
        if role == "join" && !answered && i > 30 {
            s.frame(&mut px);
            let t = pocket_meets::menu_text::read(&px);
            if t.contains("けっこん する") {
                eprintln!("i={i} answer screen: {t:?}");
                answered = true;
                answer_phase = 1;
                answer_tick = i;
            }
        }
        // Keep the answer key inside the tick barrier: hold it for 80 ms.
        if answer_phase == 1 {
            let k = if answer_decline { 0 } else { 1 };
            let _ = s.key(k, true);
            answer_tick = i;
            answer_phase = 2;
        } else if answer_phase == 2 && i - answer_tick >= 4 {
            let k = if answer_decline { 0 } else { 1 };
            let _ = s.key(k, false);
            if answer_decline {
                answer_phase = 3;
                answer_tick = i;
            } else {
                answer_phase = 0;
            }
        } else if answer_phase == 3 {
            let _ = s.key(1, true);
            answer_tick = i;
            answer_phase = 4;
        } else if answer_phase == 4 && i - answer_tick >= 4 {
            let _ = s.key(1, false);
            answer_phase = 0;
        }
    }
    eprintln!("done sent={sent} recv={recv}");
    let _ = s.save();
}
