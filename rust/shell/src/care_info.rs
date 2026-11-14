//! Read-only hidden care fields for the verified Sweets JP v062 code image.
//! Evidence, units and lifecycle boundaries: docs/sweets-info-fields.md.
use serde::Serialize;

const SWEETS_CODE_HASH: u64 = 0x28ad_63c3_4fa9_e369;
const PARAM: usize = 0xd834;
const MAX_GROWTH_MINUTES: u32 = 999 * 1440;
const MAX_AGE: u16 = 999;
const MAX_MONEY: u32 = 99_999;

#[derive(Clone, Copy)]
pub(crate) struct CareInfoProfile;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoStatus {
    NoSession,
    UnsupportedRom,
    CatchingUp,
    Stopped,
    Asleep,
    Initializing,
    Ready,
}

#[derive(Debug, Serialize)]
pub struct CareInfo {
    pub status: InfoStatus,
    pub paused: bool,
    pub data: Option<CareData>,
}

impl CareInfo {
    pub fn unavailable(status: InfoStatus, paused: bool) -> Self {
        Self {
            status,
            paused,
            data: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CareData {
    pub age: u16,
    pub money: u32,
    pub growth: Growth,
    pub care_misses: u8,
    pub hunger_ignore_minutes: Option<u8>,
    pub mood_ignore_minutes: Option<u8>,
    pub fullness: u8,
    pub fullness_drop_minutes: Option<u8>,
    pub mood: u8,
    pub mood_max: u8,
    pub mood_drop_minutes: Option<u8>,
    pub potty_seconds: u16,
}

#[derive(Debug, Serialize)]
pub struct Growth {
    pub state: &'static str,
    pub remaining_minutes: Option<u32>,
    pub elapsed_minutes: Option<u32>,
    pub required_minutes: Option<u32>,
}

impl CareInfoProfile {
    pub(crate) fn for_image(image: &[u8]) -> Option<Self> {
        (tg18::is_image(image) && tg18::code_hash(image) == SWEETS_CODE_HASH).then_some(Self)
    }

    pub(crate) fn read(self, sram: &[u8]) -> Option<CareData> {
        let p = sram.get(PARAM..PARAM + 0x53)?;
        let stage = p[0x52];
        let mood_max = match stage {
            1..=3 => 30,
            4 => 100,
            _ => return None,
        };
        let growth_minutes = u32::from_le_bytes(p[0x1c..0x20].try_into().ok()?);
        let money = u32::from_le_bytes(p[0x20..0x24].try_into().ok()?);
        let age = u16::from_le_bytes(p[0x36..0x38].try_into().ok()?);
        let events = u32::from_le_bytes(p[0x0c..0x10].try_into().ok()?);
        let potty_seconds = u16::from_le_bytes([p[0x26], p[0x27]]);
        if p[5] > 6
            || p[0x11] > mood_max
            || p[9] > 15
            || p[0x13] > 15
            || potty_seconds > 20
            || growth_minutes > MAX_GROWTH_MINUTES
            || age > MAX_AGE
            || money > MAX_MONEY
        {
            return None;
        }
        // A disabled growth counter must not be presented as a running ETA.
        let mut growth = growth(stage, growth_minutes, events & 0x80 != 0);
        if p[0x16] & 4 != 0 && growth.state == "growing" {
            growth.state = "blocked";
        }
        Some(CareData {
            age,
            money,
            growth,
            care_misses: p[0x14],
            hunger_ignore_minutes: (events & 1 != 0 && p[9] > 0).then_some(p[9]),
            mood_ignore_minutes: (events & 2 != 0 && p[0x13] > 0).then_some(p[0x13]),
            fullness: p[5],
            fullness_drop_minutes: (p[5] > 0 && p[8] > 0).then_some(p[8]),
            mood: p[0x11],
            mood_max,
            mood_drop_minutes: (p[0x11] > 0 && p[0x12] > 0).then_some(p[0x12]),
            potty_seconds,
        })
    }
}

fn growth(stage: u8, minutes: u32, pending: bool) -> Growth {
    let (start, end) = match stage {
        1 => (0, 60),
        2 => (60, 1500),
        3 => (1500, 2940),
        4 => {
            return Growth {
                // An alarm cold boot rebuilds SRAM. Do not label a partial or
                // inconsistent record as an adult, or hide pending evolution.
                state: if pending {
                    "pending"
                } else if minutes >= 2940 {
                    "mature"
                } else {
                    "unavailable"
                },
                remaining_minutes: None,
                elapsed_minutes: None,
                required_minutes: None,
            };
        }
        _ => {
            return Growth {
                state: "unavailable",
                remaining_minutes: None,
                elapsed_minutes: None,
                required_minutes: None,
            }
        }
    };
    let valid = (start..end).contains(&minutes);
    Growth {
        state: if pending {
            "pending"
        } else if valid {
            "growing"
        } else {
            "unavailable"
        },
        remaining_minutes: (!pending && valid).then(|| end - minutes),
        elapsed_minutes: (minutes >= start).then(|| (minutes - start).min(end - start)),
        required_minutes: Some(end - start),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ram() -> Vec<u8> {
        let mut r = vec![0; PARAM + 0x53];
        r[PARAM + 0x52] = 2;
        r[PARAM + 0x1c..PARAM + 0x20].copy_from_slice(&100u32.to_le_bytes());
        r
    }

    #[test]
    fn stage_thresholds_are_cumulative_and_do_not_invent_an_eta() {
        for (stage, minutes, expected) in [
            (1, 59, 1),
            (2, 60, 1440),
            (2, 1499, 1),
            (3, 1500, 1440),
            (3, 2939, 1),
        ] {
            assert_eq!(
                growth(stage, minutes, false).remaining_minutes,
                Some(expected)
            );
        }
        assert_eq!(growth(2, 59, false).state, "unavailable");
        assert_eq!(growth(2, 1500, false).state, "unavailable");
        assert_eq!(growth(2, 1500, true).state, "pending");
        assert_eq!(growth(4, 4513, false).state, "mature");
        assert_eq!(growth(4, 2939, false).state, "unavailable");
        assert_eq!(growth(4, 2940, false).state, "mature");
        assert_eq!(growth(4, 2940, true).state, "pending");
        for stage in [0, 5, 255] {
            assert_eq!(growth(stage, 4513, false).state, "unavailable");
        }
        assert!(growth(2, 1500, false).remaining_minutes.is_none());
    }

    #[test]
    fn inactive_calls_and_empty_needs_have_no_false_zero_deadline() {
        let mut r = ram();
        r[PARAM + 9] = 15;
        r[PARAM + 0x13] = 5;
        let before = r.clone();
        let d = CareInfoProfile.read(&r).unwrap();
        assert!(d.hunger_ignore_minutes.is_none());
        assert!(d.mood_ignore_minutes.is_none());
        assert!(d.fullness_drop_minutes.is_none());
        assert!(d.mood_drop_minutes.is_none());
        assert_eq!(r, before);
        r[PARAM + 0x0c] = 3;
        let d = CareInfoProfile.read(&r).unwrap();
        assert_eq!(d.hunger_ignore_minutes, Some(15));
        assert_eq!(d.mood_ignore_minutes, Some(5));
        r[PARAM + 9] = 0;
        assert!(CareInfoProfile
            .read(&r)
            .unwrap()
            .hunger_ignore_minutes
            .is_none());
    }

    #[test]
    fn rejects_uninitialized_and_invalid_layouts_and_marks_growth_blocked() {
        assert!(CareInfoProfile::for_image(&[]).is_none());
        assert!(CareInfoProfile.read(&[]).is_none());
        let mut r = ram();
        r[PARAM + 0x11] = 31;
        assert!(CareInfoProfile.read(&r).is_none());
        r[PARAM + 0x52] = 4;
        assert_eq!(CareInfoProfile.read(&r).unwrap().mood_max, 100);
        r[PARAM + 0x52] = 2;
        r[PARAM + 0x11] = 30;
        r[PARAM + 0x16] = 4;
        assert_eq!(CareInfoProfile.read(&r).unwrap().growth.state, "blocked");
        r[PARAM + 0x52] = 0;
        assert!(CareInfoProfile.read(&r).is_none());

        let mut r = ram();
        r[PARAM + 0x36..PARAM + 0x38].copy_from_slice(&3u16.to_le_bytes());
        r[PARAM + 0x20..PARAM + 0x24].copy_from_slice(&1250u32.to_le_bytes());
        let d = CareInfoProfile.read(&r).unwrap();
        assert_eq!(d.age, 3);
        assert_eq!(d.money, 1250);
        r[PARAM + 0x36..PARAM + 0x38].copy_from_slice(&1000u16.to_le_bytes());
        assert!(CareInfoProfile.read(&r).is_none());
        let mut r = ram();
        r[PARAM + 0x20..PARAM + 0x24].copy_from_slice(&100_000u32.to_le_bytes());
        assert!(CareInfoProfile.read(&r).is_none());
    }

    #[test]
    fn young_sleep_and_minute_alarm_never_report_mature() {
        use crate::session::Session;
        let Ok(fixtures) = std::env::var("POCKET_MEETS_POTTY_FIXTURES") else {
            return;
        };
        let image = std::fs::read(std::env::var("POCKET_MEETS_ROM").expect("ROM")).unwrap();
        for (stage, id, minutes) in [(2u8, 2u16, 100u32), (3, 6, 1600)] {
            let mut s = Session::bare(image.clone());
            s.playable = true;
            tg18::snapshot::Snapshot::load(
                std::path::Path::new(&fixtures)
                    .join("room.t18s")
                    .to_str()
                    .unwrap(),
            )
            .unwrap()
            .restore(&mut s.machine)
            .unwrap();
            // Controlled young-character fixture, not a claim of natural growth.
            let p = &mut s.machine.sys.sram[PARAM..PARAM + 0x53];
            p[..2].copy_from_slice(&id.to_le_bytes());
            p[2..4].copy_from_slice(&id.to_le_bytes());
            p[0x52] = stage;
            p[0x1c..0x20].copy_from_slice(&minutes.to_le_bytes());
            p[5] = 3;
            p[0x11] = 20;
            p[0x16] = 0;
            p[0x0c..0x10].fill(0);
            let sys = &mut s.machine.sys;
            let now = sys.rtc.ticks(sys.game_time) as f64 / 32768.;
            let night = (now / 86400.).floor() * 86400. + 21. * 3600.;
            sys.rtc.base_ticks = (night * 32768.) as i64;
            sys.rtc.base_time = sys.game_time;
            sys.rtc_irq.tod_base = (night as i64) % 86400;
            sys.rtc_irq.tod_set_at = sys.game_time;
            s.display_awake(false);
            let mut saw_alarm = false;
            for _ in 0..7000 {
                assert_eq!(s.step(0.01), 0);
                saw_alarm |= s.machine.alarm_wake;
                let info = s.care_info();
                if s.asleep() {
                    assert!(matches!(info.status, InfoStatus::Asleep));
                    assert!(info.data.is_none());
                } else if let Some(data) = info.data {
                    assert_ne!(data.growth.state, "mature");
                    assert!(data.growth.remaining_minutes.is_some());
                }
            }
            assert!(
                saw_alarm,
                "stage {stage} must cover a minute-alarm cold boot"
            );
            assert_eq!(s.machine.sys.sram[PARAM + 0x52], stage);
        }
    }

    #[test]
    fn real_rom_snapshot_is_read_only_and_availability_is_explicit() {
        use crate::{ffi, session::Session};
        use std::ffi::CStr;
        let Ok(fixtures) = std::env::var("POCKET_MEETS_POTTY_FIXTURES") else {
            return;
        };
        let image = std::fs::read(std::env::var("POCKET_MEETS_ROM").expect("ROM")).unwrap();
        let mut renamed = image.clone();
        renamed[0x400000] ^= 1; // writable save region does not change code identity
        assert!(CareInfoProfile::for_image(&renamed).is_some());
        renamed[0x100000] ^= 1;
        assert!(CareInfoProfile::for_image(&renamed).is_none());
        let mut s = Session::bare(image);
        s.playable = true;
        tg18::snapshot::Snapshot::load(
            std::path::Path::new(&fixtures)
                .join("natural/000000030000.t18s")
                .to_str()
                .unwrap(),
        )
        .unwrap()
        .restore(&mut s.machine)
        .unwrap();
        s.stem = "a renamed ROM".into();
        let before = s.machine.sys.sram.clone();
        let executed = s.machine.executed;
        let read = |s: &mut Session| {
            let ptr = ffi::pm_care_info(s);
            let value: serde_json::Value =
                serde_json::from_str(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()).unwrap();
            ffi::pm_string_free(ptr);
            value
        };
        let v = read(&mut s);
        assert_eq!(v["status"], "ready");
        assert_eq!(v["data"]["potty_seconds"], 16);
        assert_eq!(s.machine.sys.sram, before);
        assert_eq!(s.machine.executed, executed);
        s.care.paused = true;
        assert_eq!(read(&mut s)["paused"], true);
        s.catching_up = true;
        assert_eq!(read(&mut s)["status"], "catching_up");
        assert!(read(&mut s)["data"].is_null());
        s.catching_up = false;
        s.machine.sys.powered_off = true;
        assert_eq!(read(&mut s)["status"], "asleep");
        // Minute alarms run a cold boot without a user wake. Never publish
        // partially restored parameters even when they look like an adult.
        s.machine.sys.powered_off = false;
        s.machine.alarm_wake = true;
        s.machine.sys.sram[PARAM + 0x52] = 4;
        let v = read(&mut s);
        assert_eq!(v["status"], "asleep");
        assert!(v["data"].is_null());
        s.machine.alarm_wake = false;
        assert_eq!(read(&mut s)["status"], "ready");
        let mut unknown = Session::bare(vec![0; tg18::FLASH_SIZE]);
        assert_eq!(read(&mut unknown)["status"], "unsupported_rom");
        assert_eq!(read(&mut unknown)["data"], serde_json::Value::Null);
    }
}
