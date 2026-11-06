use pocket_meets::session::Session;
use std::path::Path;

fn step(s: &mut Session, seconds: f64) {
    let mut left = seconds;
    while left > 0.0 {
        let q = left.min(0.05);
        s.step(q);
        left -= q;
    }
}

fn press(s: &mut Session, key: usize) {
    let _ = s.key(key, true);
    step(s, 0.25);
    let _ = s.key(key, false);
    step(s, 0.75);
}

fn capture(s: &mut Session) -> (String, String) {
    let mut pixels = [0u32; 16_384];
    s.frame(&mut pixels);
    let menu = pocket_meets::menu_text::read(&pixels);
    let buf = pocket_meets::textbuf::message_text(&s.machine.sys.sram);
    (menu, buf)
}

fn main() {
    let rom = std::env::args().nth(1).unwrap();
    let saves = std::env::args().nth(2).unwrap();

    let mut seqs: Vec<Vec<usize>> = Vec::new();
    for k in 0..10 {
        let mut q: Vec<usize> = std::iter::repeat(0).take(k).collect();
        seqs.push(q.clone());
        q.push(1);
        seqs.push(q.clone());
        for j in 0..4 {
            let mut r = q.clone();
            r.extend(std::iter::repeat(0).take(j));
            seqs.push(r.clone());
            r.push(1);
            seqs.push(r.clone());
            r.push(2);
            seqs.push(r);
        }
    }

    for (i, seq) in seqs.iter().enumerate() {
        let mut s = Session::open_care(Path::new(&rom), Path::new(&saves), false).unwrap();
        step(&mut s, 1.5);
        if s.asleep() {
            press(&mut s, 1);
        }
        for &k in seq {
            press(&mut s, k);
        }
        let (menu, buf) = capture(&mut s);
        let keys: String = seq.iter().map(|k| "ABC".chars().nth(*k).unwrap()).collect();
        println!(
            "{i}\t{keys}\t{}\t{}",
            menu.replace('\t', " "),
            buf.replace('\t', " ")
        );
        if i % 10 == 9 {
            eprintln!("{}/{}", i + 1, seqs.len());
        }
    }
}
