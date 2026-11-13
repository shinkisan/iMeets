//! Read-only sleep bounds for the verified Sweets v062 firmware.
//!
//! A dark LCD / halted CPU is not a sleeping pet. The firmware's room state
//! is 3 for confirmed sleep. Ordinary awake home states can plan a latest
//! night, but entry is tested on a disposable copy. Clock and character
//! schedule must be verified. Infant naps and alternate
//! downloaded-resource branches are deliberately unsupported.

use tg18::Machine;

const SWEETS_CODE_HASH: u64 = 0x28ad_63c3_4fa9_e369;
const PARAM: usize = 0xd834;
const ROOM_STATE: usize = 0xd7d4;
const CHARACTER_TABLE: usize = 0x1c2914;
const CHARACTER_BYTES: usize = 0xe4;
const NATIVE_CHARACTERS: usize = 39;
const ASSET_BANK: usize = 0x7a0000;
const ASSET_BANK_SIZE: usize = 0x8000;
const DAY: f64 = 86_400.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct SleepProfile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SleepWindow {
    /// Emulated clock seconds remaining until this night's nominal wake time.
    pub remaining_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SleepOpportunity {
    pub wake_hour: u8,
    pub bed_hour: u8,
}

impl SleepProfile {
    pub(crate) fn for_image(image: &[u8]) -> Option<Self> {
        (tg18::is_image(image) && tg18::code_hash(image) == SWEETS_CODE_HASH).then_some(Self)
    }

    pub(crate) fn window(self, machine: &Machine) -> Option<SleepWindow> {
        let state = observe(machine)?;
        if !state.sleeping {
            return None;
        }
        let remaining_seconds = (f64::from(state.wake_hour) * 3600.0 - state.tod).rem_euclid(DAY);
        // The observed native schedules all wake at 07:00 and sleep at 20..22.
        // Reject malformed / ambiguous zero or full-day windows.
        if !(0.0..=11.0 * 3600.0).contains(&remaining_seconds) || remaining_seconds == 0.0 {
            return None;
        }
        Some(SleepWindow { remaining_seconds })
    }

    /// Schedule used to select the latest night at resume. Being awake here
    /// is allowed only in the ordinary home state; night entry is verified on
    /// a disposable copy before any replay is adopted.
    pub(crate) fn opportunity(self, machine: &Machine) -> Option<SleepOpportunity> {
        let state = observe(machine)?;
        let room = *machine.sys.sram.get(ROOM_STATE)?;
        if !state.sleeping && room != 0 {
            return None;
        }
        Some(SleepOpportunity {
            wake_hour: state.wake_hour,
            bed_hour: state.bed_hour,
        })
    }

    /// None means the SRAM/clock/resource is temporarily unavailable, including
    /// cold-boot rebuilding. A valid awake observation is Some(false), even if
    /// the CPU is powered off: minute-alarm wakes can keep the LCD dark.
    pub(crate) fn still_sleeping(self, machine: &Machine) -> Option<bool> {
        observe(machine).map(|state| state.sleeping)
    }
}

struct Observation {
    sleeping: bool,
    wake_hour: u8,
    bed_hour: u8,
    tod: f64,
}

fn observe(machine: &Machine) -> Option<Observation> {
    let sram = &machine.sys.sram;
    let p = sram.get(PARAM..PARAM + 0x38f)?;
    let stage = p[0x52];
    if !(2..=4).contains(&stage)
        || p[5] > 6
        || p[0x11] > if stage == 4 { 100 } else { 30 }
        || u32::from_le_bytes(p[0x1c..0x20].try_into().ok()?) > 999 * 1440
        || u16::from_le_bytes(p[0x36..0x38].try_into().ok()?) > 999
    {
        return None;
    }
    // Only the ordinary, healthy home state is covered by the replay evidence.
    // Other contexts can have different clock/care semantics.
    if p[0x16] != 0
        || *sram.get(0xbf04)? != 0
        || *sram.get(0xbf20)? != 0
        || *sram.get(0xbee4)? != 0
        || *sram.get(0xdc4d)? != 0
    {
        return None;
    }
    let room = *sram.get(ROOM_STATE)?;
    if room > 3 {
        return None;
    }
    let (wake_hour, bed_hour) = schedule(machine, p, stage)?;
    let tod = coherent_clock(machine, p)?;
    let hour = (tod / 3600.0).floor() as u8;
    // 0x20157454 returns 3 only outside [wakeHour, bedHour). Actual room state
    // 3 is established at 0x201448F8; neither condition alone is sufficient.
    Some(Observation {
        sleeping: room == 3 && (hour < wake_hour || hour >= bed_hour),
        wake_hour,
        bed_hour,
        tod,
    })
}

fn schedule(machine: &Machine, p: &[u8], stage: u8) -> Option<(u8, u8)> {
    // Alternate character/DLC records use additional firmware branches in
    // 0x201288F2. Do not guess their pointer layout or sleeping behavior.
    if *machine.sys.sram.get(0xd22f)? != 0 || p[0x241] != 0 || p[0x372] != 0 || p[0x38e] != 0 {
        return None;
    }
    let index = usize::from(u16::from_le_bytes([p[2], p[3]]));
    if index >= NATIVE_CHARACTERS {
        return None;
    }
    let offset = CHARACTER_TABLE.checked_add(index.checked_mul(CHARACTER_BYTES)?)?;
    let native = machine.image.get(offset..offset.checked_add(0x19)?)?;
    if native[2..4] != p[..2] || native[7] != stage {
        return None;
    }
    let record = if stage <= 3 {
        // 0x20157486..90 calls 0x201288F2(1, 0): native character table.
        native
    } else {
        // 0x201574DA..E4 calls (0, 0). For the normal resource branch,
        // 0x2012897C..86 resolves the first relative pointer at 0x207A0000.
        let ram = &machine.sys.ram;
        let relative =
            u32::from_le_bytes(ram.get(ASSET_BANK..ASSET_BANK + 4)?.try_into().ok()?) as usize;
        if relative < 0x100 || relative % 4 != 0 || relative.checked_add(0x19)? > ASSET_BANK_SIZE {
            return None;
        }
        let start = ASSET_BANK.checked_add(relative)?;
        let active = ram.get(start..start.checked_add(0x19)?)?;
        // Native resources copy this metadata prefix. Their later graphics
        // pointers are relocated, so comparing the entire 0xE4 row is wrong.
        // A changed identity/stat/schedule prefix is not a verified resource.
        if active != native {
            return None;
        }
        active
    };
    let wake = record[0x17];
    let bed = record[0x18];
    // Exhaustive table read of this hash: stages 2..4 wake at 7; bed 20..22.
    (wake == 7 && (20..=22).contains(&bed)).then_some((wake, bed))
}

fn coherent_clock(machine: &Machine, p: &[u8]) -> Option<f64> {
    let sys = &machine.sys;
    if !sys.game_time.is_finite()
        || !sys.rtc.base_time.is_finite()
        || !sys.rtc_irq.tod_set_at.is_finite()
        || p[0x8f] >= 24
        || p[0x90] >= 60
        || p[0x79] >= 60
    {
        return None;
    }
    // Read clock bases directly; do not run firmware or side-effectful MMIO.
    let rtc = sys.rtc.base_ticks as f64 / 32768.0 + sys.game_time - sys.rtc.base_time;
    let soc = sys.rtc_irq.tod_base as f64 + sys.game_time - sys.rtc_irq.tod_set_at;
    if !rtc.is_finite() || !soc.is_finite() || rtc < 0.0 || soc < 0.0 {
        return None;
    }
    let rtc = rtc.rem_euclid(DAY);
    let soc = soc.rem_euclid(DAY);
    let skew = (rtc - soc).abs();
    if skew.min(DAY - skew) > 2.0 {
        return None;
    }
    let internal = f64::from(p[0x8f]) * 3600.0 + f64::from(p[0x90]) * 60.0 + f64::from(p[0x79]);
    // While the CPU sleeps it refreshes SRAM on a once-a-minute RTC alarm.
    // Permit that bounded lag, including across midnight; reject RTC jumps
    // which the firmware has not processed, and clocks ahead of the SoC.
    if (soc - internal).rem_euclid(DAY) > 65.0 {
        return None;
    }
    Some(rtc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn machine(stage: u8, hour: u8, minute: u8, second: u8) -> Machine {
        let mut image = vec![0; tg18::FLASH_SIZE];
        image[..4].copy_from_slice(b"SPII");
        let offset = CHARACTER_TABLE + 2 * CHARACTER_BYTES;
        image[offset + 2..offset + 4].copy_from_slice(&2u16.to_le_bytes());
        image[offset + 7] = stage;
        image[offset + 0x17] = 7;
        image[offset + 0x18] = 20;
        let image = Arc::new(image);
        let mut m = Machine::new(image.clone(), &image, 0.0);
        m.sys.ram[ASSET_BANK..ASSET_BANK + 4].copy_from_slice(&0x100u32.to_le_bytes());
        m.sys.ram[ASSET_BANK + 0x100..ASSET_BANK + 0x119]
            .copy_from_slice(&image[offset..offset + 0x19]);
        let p = &mut m.sys.sram[PARAM..PARAM + 0x38f];
        p[..2].copy_from_slice(&2u16.to_le_bytes());
        p[2..4].copy_from_slice(&2u16.to_le_bytes());
        p[0x52] = stage;
        p[0x8f] = hour;
        p[0x90] = minute;
        p[0x79] = second;
        m.sys.sram[ROOM_STATE] = 3;
        let tod = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
        m.sys.rtc.base_ticks = (86_400 + tod) * 32768;
        m.sys.rtc_irq.tod_base = tod;
        m
    }

    #[test]
    fn current_night_is_bounded_at_wake_and_not_lcd_power() {
        for stage in [2, 3, 4] {
            for (h, min, sec, expected) in [
                (21, 0, 0, 36000.0),
                (23, 59, 59, 25201.0),
                (0, 0, 0, 25200.0),
                (6, 59, 59, 1.0),
            ] {
                let mut m = machine(stage, h, min, sec);
                assert_eq!(SleepProfile.window(&m).unwrap().remaining_seconds, expected);
                m.sys.powered_off = true;
                assert_eq!(SleepProfile.still_sleeping(&m), Some(true));
            }
            let mut awake = machine(stage, 7, 0, 0);
            awake.sys.powered_off = true;
            assert_eq!(SleepProfile.still_sleeping(&awake), Some(false));
            assert!(SleepProfile.window(&awake).is_none());
            let mut not_yet_asleep = machine(stage, 21, 0, 0);
            not_yet_asleep.sys.sram[ROOM_STATE] = 0;
            assert_eq!(SleepProfile.still_sleeping(&not_yet_asleep), Some(false));
            assert!(SleepProfile.window(&not_yet_asleep).is_none());
        }
    }

    #[test]
    fn temporary_rebuild_clock_jumps_and_unverified_resources_are_not_sleep() {
        let mut m = machine(4, 21, 0, 0);
        m.sys.game_time += 60.0;
        assert!(SleepProfile.window(&m).is_some());
        m.sys.game_time += 10.0;
        assert_eq!(SleepProfile.still_sleeping(&m), None);
        let mut m = machine(4, 21, 0, 0);
        m.sys.rtc.base_ticks += 3600 * 32768;
        assert!(SleepProfile.window(&m).is_none());
        let mut m = machine(4, 21, 0, 0);
        m.sys.sram[PARAM + 0x52] = 0; // cold-boot SRAM rebuilding
        assert_eq!(SleepProfile.still_sleeping(&m), None);
        let mut m = machine(4, 21, 0, 0);
        m.sys.ram[ASSET_BANK + 0x100 + 0x17] = 8;
        assert_eq!(SleepProfile.still_sleeping(&m), None);
        let mut m = machine(4, 21, 0, 0);
        m.sys.ram[ASSET_BANK..ASSET_BANK + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(SleepProfile.window(&m).is_none());
        let mut m = machine(4, 21, 0, 0);
        m.sys.sram[PARAM + 0x241] = 1;
        assert!(SleepProfile.window(&m).is_none());
        assert!(SleepProfile.window(&machine(1, 21, 0, 0)).is_none());
        assert!(SleepProfile::for_image(&[]).is_none());
        assert!(SleepProfile::for_image(&machine(4, 21, 0, 0).image).is_none());
    }

    #[test]
    fn real_sweets_sleeping_snapshot_is_read_without_running_the_cpu() {
        let Ok(snapshot) = std::env::var("POCKET_MEETS_SLEEP_SNAPSHOT") else {
            return;
        };
        let image = Arc::new(
            std::fs::read(std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM")).unwrap(),
        );
        let profile =
            SleepProfile::for_image(&image).expect("wrong ROM for the private night-21 snapshot");
        let mut m = Machine::new(image.clone(), &image, 0.0);
        tg18::snapshot::Snapshot::load(&snapshot)
            .unwrap()
            .restore(&mut m)
            .unwrap();
        let before = (m.executed, m.sys.game_time, m.sys.flash.writes);
        assert_eq!(profile.still_sleeping(&m), Some(true));
        assert!((profile.window(&m).unwrap().remaining_seconds - 35990.0).abs() < 2.0);
        assert_eq!(before, (m.executed, m.sys.game_time, m.sys.flash.writes));
    }
}
