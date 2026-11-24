//! When a saved care rule will become true, in wall seconds.
//!
//! Fullness loses one heart each time its minute timer expires, and that
//! same drop increments the digestion counter. The second such drop starts
//! the 20-second toilet urge. Mood loses up to 3 per cycle. Sleep hours do
//! not run those minute timers. An already-true hunger or mood rule is not
//! an edge, so it does not schedule another notification.

use crate::settings::EventRule;

const PARAM: usize = 0xd834;
const POTTY_GATE: usize = 0xbf20;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub fullness: u8,
    pub fullness_minutes: u8,
    pub mood: u8,
    pub mood_max: u8,
    pub mood_minutes: u8,
    pub digestion: u8,
    pub potty_seconds: u16,
    pub blocks_minutes: bool,
    pub potty_gated: bool,
    pub fullness_period: Option<u8>,
    pub mood_period: Option<u8>,
    pub tod_seconds: Option<u32>,
    pub wake_hour: Option<u8>,
    pub bed_hour: Option<u8>,
}

pub fn read_sram(sram: &[u8]) -> Option<Snapshot> {
    let p = sram.get(PARAM..PARAM + 0x53)?;
    let stage = p[0x52];
    let mood_max = match stage {
        1..=3 => 30,
        4 => 100,
        _ => return None,
    };
    let potty = u16::from_le_bytes([p[0x26], p[0x27]]);
    if p[5] > 6 || p[0x11] > mood_max || potty > 20 {
        return None;
    }
    Some(Snapshot {
        fullness: p[5],
        fullness_minutes: p[8],
        mood: p[0x11],
        mood_max,
        mood_minutes: p[0x12],
        digestion: p[0x40],
        potty_seconds: potty,
        blocks_minutes: p[0x16] & 0x1c != 0,
        potty_gated: sram.get(POTTY_GATE).copied().unwrap_or(0) != 0,
        fullness_period: None,
        mood_period: None,
        tod_seconds: None,
        wake_hour: None,
        bed_hour: None,
    })
}

/// `Some(0)` means the condition is already inside its window. `None` means
/// it will not happen while the pet is left alone.
pub fn eta_seconds(snap: &Snapshot, rule: &EventRule) -> Option<u64> {
    match rule.metric.as_str() {
        "hunger" => level_eta(hunger_minutes(snap, clamp(rule.below, 1, 6)?), snap),
        "mood" => level_eta(mood_minutes(snap, clamp(rule.below, 1, 100)?), snap),
        "potty" => potty_eta(snap, u64::from(clamp(rule.below, 1, 30)?)),
        _ => None,
    }
}

fn clamp(value: i32, low: i32, high: i32) -> Option<u8> {
    u8::try_from(value.clamp(low, high)).ok()
}

fn level_eta(minutes: Option<u32>, snap: &Snapshot) -> Option<u64> {
    let minutes = minutes?;
    if minutes == 0 {
        return None;
    }
    let seconds = wall_seconds(minutes, snap)?;
    (seconds > 0).then_some(seconds)
}

fn hunger_minutes(snap: &Snapshot, below: u8) -> Option<u32> {
    if snap.blocks_minutes || snap.fullness < below {
        return None;
    }
    let drops = u32::from(snap.fullness - (below - 1));
    minute_ticks(snap.fullness_minutes, snap.fullness_period, drops)
}

fn mood_minutes(snap: &Snapshot, below: u8) -> Option<u32> {
    if snap.blocks_minutes || snap.mood < below || snap.mood_minutes == 0 {
        return None;
    }
    let mut mood = snap.mood;
    let mut ticks = 0u32;
    let mut remaining = snap.mood_minutes;
    for _ in 0..40 {
        ticks = ticks.saturating_add(u32::from(remaining));
        mood = mood.saturating_sub(mood.min(3));
        if mood < below {
            return Some(ticks);
        }
        remaining = snap.mood_period.filter(|period| *period > 0)?;
    }
    None
}

/// `drops` fullness-loss events. The first uses the live countdown. Later
/// ones each wait one full reload period.
fn minute_ticks(first: u8, period: Option<u8>, drops: u32) -> Option<u32> {
    if drops == 0 || first == 0 {
        return None;
    }
    let mut ticks = u32::from(first);
    if drops == 1 {
        return Some(ticks);
    }
    let period = u32::from(period.filter(|value| *value > 0)?);
    ticks = ticks.saturating_add(period.saturating_mul(drops - 1));
    Some(ticks)
}

fn potty_eta(snap: &Snapshot, lead_minutes: u64) -> Option<u64> {
    let lead = lead_minutes.saturating_mul(60);
    if snap.potty_seconds > 0 {
        return Some(0);
    }
    if snap.blocks_minutes || snap.potty_gated || snap.digestion > 1 {
        return None;
    }
    let ticks = if snap.digestion == 1 && snap.fullness >= 1 {
        minute_ticks(snap.fullness_minutes, None, 1)?
    } else if snap.digestion == 0 && snap.fullness >= 2 {
        minute_ticks(snap.fullness_minutes, snap.fullness_period, 2)?
    } else {
        return None;
    };
    let until_urge = wall_seconds(ticks, snap)?;
    Some(until_urge.saturating_sub(lead))
}

fn wall_seconds(awake_minutes: u32, snap: &Snapshot) -> Option<u64> {
    let mut left = u64::from(awake_minutes).checked_mul(60)?;
    let Some(tod) = snap.tod_seconds else {
        return Some(left);
    };
    let Some(wake) = snap.wake_hour else {
        return Some(left);
    };
    let Some(bed) = snap.bed_hour else {
        return Some(left);
    };
    if wake == 0 || bed <= wake || bed >= 24 {
        return Some(left);
    }
    let wake_s = u64::from(wake) * 3600;
    let bed_s = u64::from(bed) * 3600;
    let mut tod = u64::from(tod % 86_400);
    let mut elapsed = 0u64;
    while left > 0 && elapsed < 14 * 86_400 {
        if tod < wake_s || tod >= bed_s {
            let add = if tod < wake_s {
                wake_s - tod
            } else {
                86_400 - tod + wake_s
            };
            tod = (tod + add) % 86_400;
            elapsed = elapsed.saturating_add(add);
        } else {
            let take = (bed_s - tod).min(left);
            tod = (tod + take) % 86_400;
            elapsed = elapsed.saturating_add(take);
            left -= take;
        }
    }
    (left == 0).then_some(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            fullness: 4,
            fullness_minutes: 10,
            mood: 47,
            mood_max: 100,
            mood_minutes: 5,
            digestion: 0,
            potty_seconds: 0,
            blocks_minutes: false,
            potty_gated: false,
            fullness_period: Some(80),
            mood_period: Some(35),
            tod_seconds: None,
            wake_hour: None,
            bed_hour: None,
        }
    }

    fn rule(metric: &str, below: i32) -> EventRule {
        EventRule {
            metric: metric.into(),
            below,
        }
    }

    #[test]
    fn hunger_crosses_on_the_next_drops_and_skips_an_already_true_rule() {
        let mut s = snap();
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), Some(10 * 60));
        assert_eq!(eta_seconds(&s, &rule("hunger", 3)), Some((10 + 80) * 60));
        s.fullness = 2;
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), None);
        s.fullness_minutes = 0;
        s.fullness = 4;
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), None);
        s.fullness_minutes = 10;
        s.blocks_minutes = true;
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), None);
    }

    #[test]
    fn mood_loses_three_per_cycle() {
        let s = snap();
        // 47 → 44 → 41 → 38. The third cycle is the first one under 40.
        assert_eq!(eta_seconds(&s, &rule("mood", 40)), Some((5 + 35 + 35) * 60));
        assert_eq!(eta_seconds(&s, &rule("mood", 46)), Some(5 * 60));
        let mut low = s;
        low.mood = 10;
        assert_eq!(eta_seconds(&low, &rule("mood", 20)), None);
    }

    #[test]
    fn potty_follows_the_second_fullness_loss() {
        let mut s = snap();
        s.fullness = 4;
        s.digestion = 0;
        s.fullness_minutes = 10;
        assert_eq!(eta_seconds(&s, &rule("potty", 1)), Some((10 + 80 - 1) * 60));
        s.digestion = 1;
        s.fullness = 1;
        assert_eq!(eta_seconds(&s, &rule("potty", 1)), Some((10 - 1) * 60));
        s.digestion = 0;
        s.fullness = 1;
        assert_eq!(eta_seconds(&s, &rule("potty", 1)), None);
        s.fullness = 0;
        s.digestion = 1;
        assert_eq!(eta_seconds(&s, &rule("potty", 1)), None);
        s.potty_seconds = 12;
        assert_eq!(eta_seconds(&s, &rule("potty", 1)), Some(0));
    }

    #[test]
    fn sleep_pushes_the_remaining_minutes_to_morning() {
        let mut s = snap();
        s.fullness = 4;
        s.fullness_minutes = 30;
        s.tod_seconds = Some(19 * 3600 + 50 * 60);
        s.wake_hour = Some(7);
        s.bed_hour = Some(20);
        // One drop, 30 awake minutes: 10 before bed, 11 hours asleep, 20 after waking.
        assert_eq!(
            eta_seconds(&s, &rule("hunger", 4)),
            Some(10 * 60 + 11 * 3600 + 20 * 60)
        );
        s.fullness_minutes = 10;
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), Some(10 * 60));
        s.tod_seconds = Some(21 * 3600);
        assert_eq!(eta_seconds(&s, &rule("hunger", 4)), Some(10 * 3600 + 10 * 60));
    }

    #[test]
    fn sram_layout_rejects_an_uninitialized_block() {
        let mut ram = vec![0u8; PARAM + 0x53];
        assert!(read_sram(&ram).is_none());
        ram[PARAM + 0x52] = 4;
        ram[PARAM + 5] = 3;
        ram[PARAM + 8] = 12;
        ram[PARAM + 0x11] = 40;
        ram[PARAM + 0x12] = 6;
        ram[PARAM + 0x40] = 1;
        ram[PARAM + 0x26] = 8;
        let snap = read_sram(&ram).unwrap();
        assert_eq!(snap.fullness, 3);
        assert_eq!(snap.mood, 40);
        assert_eq!(snap.digestion, 1);
        assert_eq!(snap.potty_seconds, 8);
        ram[PARAM + 0x26] = 21;
        assert!(read_sram(&ram).is_none());
    }
}
