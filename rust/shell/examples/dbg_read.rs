fn main() {
    let path = std::env::args().nth(1).unwrap();
    let raw = std::fs::read(path).unwrap();
    let pixels: Vec<u32> = raw.chunks_exact(4).map(|c| u32::from_le_bytes([c[0],c[1],c[2],c[3]])).collect();
    println!("{:?}", pocket_meets::menu_text::read(&pixels));
}
