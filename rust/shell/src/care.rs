//! Care timeline. Paused gaps are excluded. Sibling ROMs are not walked.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CareTimeline {
    pub care_at: f64,
    pub paused: bool,
    #[serde(default)]
    pub pause_started: Option<f64>,
    /// Persist until a resumed Sweets session has live SRAM to repair its age cache.
    #[serde(default)]
    pub age_refresh_pending: bool,
}

impl CareTimeline {
    pub fn at(now: f64) -> Self {
        Self {
            care_at: now,
            paused: false,
            pause_started: None,
            age_refresh_pending: false,
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
