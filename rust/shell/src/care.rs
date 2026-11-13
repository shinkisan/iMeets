//! Care timeline. Ordinary paused gaps are excluded; Session may grant a
//! bounded verified sleep replay. Sibling ROMs are not walked.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Frozen machine clock and verified schedule for the paused pet.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SleepPause {
    pub rtc: f64,
    pub remaining_seconds: f64,
    #[serde(default)]
    pub wake_hour: Option<u8>,
    #[serde(default)]
    pub bed_hour: Option<u8>,
}

/// The endpoint is immutable. Progress is recovered from the saved machine RTC,
/// so a snapshot saved before care.json cannot grant the same minutes twice.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SleepReplay {
    pub start_rtc: f64,
    pub end_rtc: f64,
    pub total_seconds: f64,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub entering_sleep: bool,
}

impl SleepPause {
    pub fn replay(&self, elapsed: f64) -> Option<SleepReplay> {
        self.replay_at(elapsed, self.rtc + elapsed)
    }

    /// Use the resume phone clock for the latest night and the real elapsed
    /// pause duration for its interval. Older nights never accumulate.
    pub fn replay_at(&self, elapsed: f64, wall: f64) -> Option<SleepReplay> {
        if !self.rtc.is_finite()
            || self.rtc < 0.0
            || !elapsed.is_finite()
            || elapsed <= 0.001
            || !wall.is_finite()
            || wall < 0.0
        {
            return None;
        }
        let (start, end) = match (self.wake_hour, self.bed_hour) {
            (Some(wake), Some(bed)) if wake == 7 && (20..=22).contains(&bed) => {
                let day = (wall / 86_400.0).floor() * 86_400.0;
                let mut night = day + f64::from(bed) * 3600.0;
                // At bedtime exactly, the new night has no elapsed overlap.
                if night >= wall {
                    night -= 86_400.0;
                }
                let morning = night + 86_400.0 - f64::from(bed) * 3600.0 + f64::from(wake) * 3600.0;
                (night.max(wall - elapsed), morning.min(wall))
            }
            (None, None) => {
                // v1.3 metadata is upgraded from a verified live schedule by
                // Session. This fallback also reads old standalone receipts.
                if !self.remaining_seconds.is_finite()
                    || !(0.0..=86_400.0).contains(&self.remaining_seconds)
                {
                    return None;
                }
                (self.rtc, self.rtc + elapsed.min(self.remaining_seconds))
            }
            _ => return None,
        };
        let total = end - start;
        (start >= 0.0 && total > 0.001 && total <= 11.0 * 3600.0).then_some(SleepReplay {
            start_rtc: start,
            end_rtc: end,
            total_seconds: total,
            completed: false,
            entering_sleep: true,
        })
    }
}

impl SleepReplay {
    pub fn remaining(&self, rtc: f64) -> Option<f64> {
        if !rtc.is_finite()
            || !self.start_rtc.is_finite()
            || !self.end_rtc.is_finite()
            || !self.total_seconds.is_finite()
            || self.start_rtc < 0.0
            || !(0.001..=86_400.0).contains(&self.total_seconds)
            || (self.end_rtc - self.start_rtc - self.total_seconds).abs() > 0.01
            || rtc < self.start_rtc - 0.01
        {
            return None;
        }
        Some((self.end_rtc - rtc).max(0.0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CareTimeline {
    pub care_at: f64,
    pub paused: bool,
    #[serde(default)]
    pub pause_started: Option<f64>,
    /// Persist until a resumed Sweets session has live SRAM to repair its age cache.
    #[serde(default)]
    pub age_refresh_pending: bool,
    #[serde(default)]
    pub sleep_pause: Option<SleepPause>,
    #[serde(default)]
    pub sleep_replay: Option<SleepReplay>,
}

impl CareTimeline {
    pub fn at(now: f64) -> Self {
        Self {
            care_at: now,
            paused: false,
            pause_started: None,
            age_refresh_pending: false,
            sleep_pause: None,
            sleep_replay: None,
        }
    }

    pub fn due(&self, now: f64) -> f64 {
        if self.paused {
            0.0
        } else {
            (now - self.care_at).max(0.0)
        }
    }

    /// `paused` true freezes care. False jumps `care_at` to `now` so the
    /// away gap is not replayed as hunger or growth.
    pub fn set_paused(&mut self, paused: bool, now: f64) {
        if paused {
            if !self.paused {
                self.pause_started = Some(now);
            }
            self.paused = true;
            return;
        }
        if self.paused {
            self.care_at = now;
            self.pause_started = None;
        }
        self.paused = false;
    }

    pub fn load(folder: &Path) -> Self {
        let path = folder.join("care.json");
        fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| Self::at(0.0))
    }

    pub fn store(&self, folder: &Path) -> std::io::Result<()> {
        fs::create_dir_all(folder)?;
        let path = folder.join("care.json");
        let tmp = folder.join("care.json.tmp");
        fs::write(
            &tmp,
            serde_json::to_string(self).unwrap_or_else(|_| "{}".into()),
        )?;
        fs::rename(tmp, path)
    }
}

/// Only a stem that is actually being left gets a pause record.
pub fn outgoing_should_pause(old_name: &str, new_name: &str) -> bool {
    !old_name.is_empty() && old_name != new_name
}

pub fn pause_folder(folder: &Path, now: f64) -> std::io::Result<()> {
    let mut c = CareTimeline::load(folder);
    if c.care_at == 0.0 && !folder.join("care.json").exists() {
        c.care_at = now;
    }
    c.set_paused(true, now);
    c.store(folder)
}

/// TODO(care-adopt): SPEC §14.7. The shell never calls this. No effect
/// was specified, so the export is a no-op.
pub fn care_adopt() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_gap_is_not_replayed() {
        let mut c = CareTimeline::at(1_000.0);
        c.set_paused(true, 1_000.0);
        assert_eq!(c.due(9_000.0), 0.0);
        assert_eq!(c.care_at, 1_000.0);
        c.set_paused(false, 5_000.0);
        assert!((c.care_at - 5_000.0).abs() < 1e-9);
        assert!(c.due(5_000.0) <= 0.001);
        assert!((c.due(5_001.0) - 1.0).abs() < 1e-9);
        assert!(!outgoing_should_pause("Fairy (EN v057)", "Fairy (EN v057)"));
        assert!(outgoing_should_pause("Fairy (EN v057)", "Magic (EN v058)"));
        assert!(!outgoing_should_pause("", "Magic (EN v058)"));
    }

    #[test]
    fn pause_file_roundtrip_does_not_touch_siblings() {
        let root = crate::tmpdir("care");
        let a = root.join("Saves/a");
        let b = root.join("Saves/b");
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join("keep.txt"), b"safe").unwrap();
        let c = CareTimeline::at(10.0);
        c.store(&a).unwrap();
        pause_folder(&a, 40.0).unwrap();
        let loaded = CareTimeline::load(&a);
        assert!(loaded.paused);
        assert_eq!(loaded.care_at, 10.0);
        assert!(b.join("keep.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod sleep_receipt_tests {
    use super::*;

    #[test]
    fn credits_elapsed_time_only_and_caps_one_night() {
        let anchor = SleepPause {
            rtc: 100_000.0,
            remaining_seconds: 8.0 * 3600.0,
            wake_hour: None,
            bed_hour: None,
        };
        assert_eq!(anchor.replay(600.0).unwrap().total_seconds, 600.0);
        assert_eq!(
            anchor.replay(4.0 * 86400.0).unwrap().total_seconds,
            8.0 * 3600.0
        );
        assert!(anchor.replay(-10.0).is_none());
        assert!(anchor.replay(f64::NAN).is_none());
        let plan = anchor.replay(600.0).unwrap();
        assert_eq!(plan.remaining(100_300.0), Some(300.0));
        assert_eq!(plan.remaining(100_700.0), Some(0.0));
        assert_eq!(plan.remaining(99_999.0), None);
        let old: CareTimeline = serde_json::from_str(r#"{"care_at":10,"paused":true}"#).unwrap();
        assert!(old.sleep_pause.is_none());
        assert!(old.sleep_replay.is_none());
    }
}

#[cfg(test)]
mod latest_night_tests {
    use super::*;

    #[test]
    fn latest_sleep_intersection_does_not_sum_previous_nights() {
        let day = 100.0 * 86400.0;
        let make = |pause: f64, resume: f64| {
            SleepPause {
                rtc: pause,
                remaining_seconds: 0.0,
                wake_hour: Some(7),
                bed_hour: Some(20),
            }
            .replay_at(resume - pause, resume)
        };
        assert!(make(day + 18.0 * 3600.0, day + 19.0 * 3600.0).is_none());
        let partial = make(day + 18.0 * 3600.0, day + 23.0 * 3600.0).unwrap();
        assert_eq!(partial.start_rtc, day + 20.0 * 3600.0);
        assert_eq!(partial.total_seconds, 3.0 * 3600.0);
        let whole = make(day + 18.0 * 3600.0, day + 86400.0 + 12.0 * 3600.0).unwrap();
        assert_eq!(whole.total_seconds, 11.0 * 3600.0);
        let many = make(day + 18.0 * 3600.0, day + 6.0 * 86400.0 + 12.0 * 3600.0).unwrap();
        assert_eq!(many.start_rtc, day + 5.0 * 86400.0 + 20.0 * 3600.0);
        assert_eq!(many.total_seconds, 11.0 * 3600.0);
        let current = make(day + 18.0 * 3600.0, day + 6.0 * 86400.0 + 22.0 * 3600.0).unwrap();
        assert_eq!(current.start_rtc, day + 6.0 * 86400.0 + 20.0 * 3600.0);
        assert_eq!(current.total_seconds, 2.0 * 3600.0);
        assert_eq!(
            make(day + 23.0 * 3600.0, day + 86400.0 + 2.0 * 3600.0)
                .unwrap()
                .total_seconds,
            3.0 * 3600.0
        );
        assert!(make(day + 86400.0 + 8.0 * 3600.0, day + 86400.0 + 19.0 * 3600.0).is_none());
        assert!(make(day + 23.0 * 3600.0, day + 22.0 * 3600.0).is_none());
    }
}
