//! Care timeline. Ordinary paused gaps are excluded; Session may grant a
//! bounded verified sleep replay. Sibling ROMs are not walked.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A receipt captured only when a verified pet is actually sleeping.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SleepPause {
    pub rtc: f64,
    pub remaining_seconds: f64,
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
}

impl SleepPause {
    pub fn replay(&self, elapsed: f64) -> Option<SleepReplay> {
        if !self.rtc.is_finite()
            || self.rtc < 0.0
            || !self.remaining_seconds.is_finite()
            || !(0.0..=86_400.0).contains(&self.remaining_seconds)
            || !elapsed.is_finite()
            || elapsed <= 0.001
        {
            return None;
        }
        let total = elapsed.min(self.remaining_seconds);
        (total > 0.001).then_some(SleepReplay {
            start_rtc: self.rtc,
            end_rtc: self.rtc + total,
            total_seconds: total,
            completed: false,
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
