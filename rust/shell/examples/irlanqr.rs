//! QR pairing prototype: host renders a QR, join decodes it and connects.
//!
//! Host (the side that shows the QR) starts the listener and writes
//! `/tmp/tama-join.png`. The join side accepts the QR payload directly or a
//! file containing it, then connects without any manual address entry.
//!
//! Usage:
//!   POCKET_MEETS_ROM=... irlanqr host <save> <port> <seconds> <hour> [option]
//!   POCKET_MEETS_ROM=... irlanqr join <payload-or-file> <save> <seconds> <hour> [option]
//!
//! option: 0 あそぶ (default), 2 プロポーズ (host proposes, join waits)
//!
//! QR_NODE_PATH points at the node_modules used by tools/qr/qrgen.cjs
//! (default `/tmp/qr/node_modules`).

use pocket_meets::session::Session;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc;
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
fn local_ipv4() -> Vec<String> {
    let mut out = vec!["127.0.0.1".to_string()];
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                let ip = addr.ip().to_string();
                if !out.contains(&ip) {
                    out.push(ip);
                }
            }
        }
    }
    out
}
fn token_hex() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}
fn parse_payload(payload: &str) -> Option<(Vec<String>, u16, String)> {
    let mut hosts = Vec::new();
    let mut port = None;
    let mut token = None;
    let query = payload.split("tama://join?").nth(1)?;
    for part in query.split('&') {
        let (k, v) = part.split_once('=')?;
        match k {
            "h" => {
                hosts = v
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .map(|x| x.to_string())
                    .collect()
            }
            "p" => port = v.parse().ok(),
            "t" => token = Some(v.to_string()),
            _ => {}
        }
    }
    Some((hosts, port?, token?))
}
fn write_qr(payload: &str) -> bool {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/qr/qrgen.cjs");
    let default =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/qr/node_modules");
    let node_path =
        std::env::var("QR_NODE_PATH").unwrap_or_else(|_| default.to_string_lossy().into_owned());
    std::process::Command::new("node")
        .arg(script)
        .arg(payload)
        .arg("/tmp/tama-join.png")
        .env("NODE_PATH", node_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let role = std::env::args().nth(1).expect("host|join");
    let answer_decline = std::env::var("IRLAN_ANSWER").as_deref() == Ok("decline");
    let native = std::env::var("IRLAN_NATIVE").as_deref() == Ok("1");
    let verbose = std::env::var("IRLAN_VERBOSE").is_ok();
    let (base, secs, hour, option, stream) = if role == "host" {
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
        let listener = TcpListener::bind(("0.0.0.0", port)).expect("bind");
        let port = listener.local_addr().unwrap().port();
        let ips = local_ipv4();
        let token = token_hex();
        let payload = format!(
            "tama://join?v=1&h={}&p={port}&t={token}&c={:06}",
            ips.join(","),
            port as u32 * 7 % 1_000_000
        );
        if write_qr(&payload) {
            eprintln!("QR /tmp/tama-join.png");
        } else {
            eprintln!("QR unavailable");
        }
        eprintln!("PAYLOAD {payload}");
        let (stream, _) = listener.accept().expect("accept");
        eprintln!("CONNECTED");
        (base, secs, hour, option, stream)
    } else {
        let payload = std::env::args().nth(2).expect("payload");
        let payload = if Path::new(&payload).is_file() {
            std::fs::read_to_string(&payload)
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            payload
        };
        let (hosts, port, _token) = parse_payload(&payload).expect("payload");
        let base = std::env::args().nth(3).expect("save");
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
        let mut stream = None;
        for host in &hosts {
            if let Ok(s) = TcpStream::connect_timeout(
                &format!("{host}:{port}").parse().unwrap(),
                Duration::from_secs(2),
            ) {
                stream = Some(s);
                eprintln!("CONNECTED {host}:{port}");
                break;
            }
        }
        let stream = stream.expect("unreachable");
        (base, secs, hour, option, stream)
    };

    let mut s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    hour_now(&mut s, hour);
    step(&mut s, 2.0);
    step(&mut s, 15.0);
    s.display_awake(true);
    step(&mut s, 2.0);
    s.machine.stop_on_bluetooth = false;
    let mut stream = stream;
    stream.set_nodelay(true).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    send_line(&mut stream, "TAMA-LAN 1");
    let hello = read_line(&mut stream).unwrap_or_default();
    if verbose {
        eprintln!("peer hello: {hello:?}");
    }

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
    if !native {
        let sub = open_submenu(&mut s);
        if verbose {
            eprintln!("submenu: {sub:?}");
        }
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
        if verbose {
            eprintln!("after select: {:?}", text(&mut s));
        }

        // Tell the peer we are at the waiting screen, then let the host open
        // the link and the guest follow 0.5 s later.
        if verbose {
            eprintln!("{role}: send WAIT");
        }
        send_line(&mut stream, "WAIT");
        let _ = read_line(&mut stream);
        if verbose {
            eprintln!("{role}: read WAIT");
        }
        if role == "host" {
            std::thread::sleep(Duration::from_millis(1000));
            if verbose {
                eprintln!("host: send GO");
            }
            send_line(&mut stream, "GO");
        } else {
            let _ = read_line(&mut stream);
            if verbose {
                eprintln!("join: read GO");
            }
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
        if verbose {
            eprintln!("after start: {:?}", text(&mut s));
        }
        if std::env::var("IRLAN_NO_KICK").is_err() {
            for byte in &f {
                s.machine.sys.uart_receive(*byte);
            }
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

    let (key_tx, key_rx) = mpsc::channel::<usize>();
    if native {
        std::thread::spawn(move || {
            let mut line = String::new();
            loop {
                line.clear();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        for c in line.trim().chars() {
                            let k = match c {
                                'a' | 'A' => 0,
                                'b' | 'B' => 1,
                                'c' | 'C' => 2,
                                _ => continue,
                            };
                            if key_tx.send(k).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });
    }
    let mut queued: VecDeque<usize> = VecDeque::new();
    let mut held: Option<(usize, usize)> = None;
    let mut next_key_at = 250usize;
    let shot_dir = std::env::var("IRLAN_SHOT_DIR").ok();
    let mut last_text = String::new();
    let mut kicked = false;
    let mut kick_at: Option<usize> = None;
    let mut sent = 0usize;
    let mut recv = 0usize;
    let mut answered = false;
    let mut answer_phase = 0u32;
    let mut answer_tick = 0usize;
    let ticks = (secs / 0.02) as usize;
    let mut px = [0u32; 16_384];
    for i in 0..ticks {
        if native {
            while let Ok(k) = key_rx.try_recv() {
                queued.push_back(k);
            }
            if held.is_none() && i >= next_key_at {
                if let Some(k) = queued.pop_front() {
                    let _ = s.key(k, true);
                    held = Some((k, i));
                }
            }
            if let Some((k, at)) = held {
                if i - at >= 10 {
                    let _ = s.key(k, false);
                    held = None;
                    next_key_at = i + 35;
                }
            }
        }
        if native && !kicked {
            if let Some(k) = kick_at {
                if i >= k {
                    for byte in &f {
                        s.machine.sys.uart_receive(*byte);
                    }
                    kicked = true;
                }
            }
        }
        step(&mut s, 0.02);
        let out = s.machine.sys.uart_take();
        if !out.is_empty() {
            if verbose && i < 3 {
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
                    if verbose && i < 3 {
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
        if i % 25 == 0 {
            s.frame(&mut px);
            if let Some(dir) = &shot_dir {
                if i % 100 == 0 {
                    let _ = std::fs::create_dir_all(dir);
                    let mut rgb = Vec::with_capacity(16_384 * 3);
                    for p in &px {
                        rgb.push(((p >> 16) & 0xff) as u8);
                        rgb.push(((p >> 8) & 0xff) as u8);
                        rgb.push((p & 0xff) as u8);
                    }
                    let _ = std::fs::write(format!("{dir}/{role}-{i:04}.rgb"), &rgb);
                }
            }
            let t = pocket_meets::menu_text::read(&px);
            if t != last_text {
                eprintln!("[{t}]");
                last_text = t.clone();
            }
            if native && !kicked && kick_at.is_none() && t.contains("つうしん ちゅう") {
                kick_at = Some(i + 50);
            }
        }
        if !native && role == "join" && !answered && i > 30 {
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
