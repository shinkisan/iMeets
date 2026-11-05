//! Dump the clock, pet and status SRAM areas plus the RTC state.
//!
//! Usage: POCKET_MEETS_ROM=... irdump <save-dir>

use pocket_meets::session::Session;
use std::path::Path;
fn main() {
    let rom = std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM");
    let base = std::env::args().nth(1).expect("save");
    let s = Session::open_care(Path::new(&rom), Path::new(&base), false).unwrap();
    for (name, off) in [("clock", 0xBE20usize), ("pet", 0x0D840usize), ("status", 0x0D7C0usize)] {
        print!("{name:6} {off:05X}: ");
        for b in &s.machine.sys.sram[off..off + 0x30] {
            print!("{b:02X} ");
        }
        println!();
    }
    println!("rtc.base_ticks={} base_time={:.1} tod_base={} tod_set_at={:.1}",
        s.machine.sys.rtc.base_ticks, s.machine.sys.rtc.base_time,
        s.machine.sys.rtc_irq.tod_base, s.machine.sys.rtc_irq.tod_set_at);
    println!("game_time={:.1} care_at={:.0}", s.machine.sys.game_time, s.care_at());
}
