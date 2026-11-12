//! Repair a skipped age-display refresh after a paused local midnight.
//! Only the verified Sweets code/layout is supported. See sweets-age-and-pause.md.

const SWEETS_CODE_HASH: u64 = 0x28ad_63c3_4fa9_e369;
const PARAM: usize = 0xd834;
const MAX_MINUTES: u32 = 999 * 1440;

#[derive(Clone, Copy)]
pub(crate) struct AgeProfile;

impl AgeProfile {
    pub(crate) fn for_image(image: &[u8]) -> Option<Self> {
        (tg18::is_image(image) && tg18::code_hash(image) == SWEETS_CODE_HASH).then_some(Self)
    }

    /// None means SRAM is not ready. Never advances growth or requests events.
    /// Some(true) means the two-byte display cache was raised.
    pub(crate) fn refresh(self, sram: &mut [u8]) -> Option<bool> {
        let p = sram.get_mut(PARAM..PARAM + 0x53)?;
        if !(1..=4).contains(&p[0x52]) {
            return None;
        }
        let minutes = u32::from_le_bytes(p[0x1c..0x20].try_into().unwrap());
        let old = u16::from_le_bytes(p[0x36..0x38].try_into().unwrap());
        if minutes > MAX_MINUTES || old > 999 {
            return None;
        }
        let age = minutes.div_ceil(1440) as u16;
        // This is a missed-refresh repair, not a way to reduce a saved age.
        if age <= old {
            return Some(false);
        }
        p[0x36..0x38].copy_from_slice(&age.to_le_bytes());
        Some(true)
    }
}

pub(crate) fn crossed_midnight(before: f64, after: f64) -> bool {
    before.is_finite()
        && after.is_finite()
        && before >= 0.0
        && (after / 86_400.0).floor() > (before / 86_400.0).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_changes_only_the_age_cache_and_never_reduces_it() {
        let mut ram = vec![0xa5; PARAM + 0x100];
        ram[PARAM + 0x52] = 4;
        ram[PARAM + 0x1c..PARAM + 0x20].copy_from_slice(&4513u32.to_le_bytes());
        ram[PARAM + 0x36..PARAM + 0x38].copy_from_slice(&3u16.to_le_bytes());
        let before = ram.clone();
        assert_eq!(AgeProfile.refresh(&mut ram), Some(true));
        assert_eq!(
            u16::from_le_bytes(ram[PARAM + 0x36..PARAM + 0x38].try_into().unwrap()),
            4
        );
        assert_eq!(&ram[..PARAM + 0x36], &before[..PARAM + 0x36]);
        assert_eq!(&ram[PARAM + 0x38..], &before[PARAM + 0x38..]);
        assert_eq!(AgeProfile.refresh(&mut ram), Some(false));
        ram[PARAM + 0x1c..PARAM + 0x20].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(AgeProfile.refresh(&mut ram), Some(false));
        assert_eq!(ram[PARAM + 0x36], 4);
    }

    #[test]
    fn unsupported_or_uninitialised_state_is_left_alone() {
        assert!(AgeProfile::for_image(&[]).is_none());
        let mut image = vec![0; tg18::FLASH_SIZE];
        image[..4].copy_from_slice(b"SPII");
        assert!(AgeProfile::for_image(&image).is_none());
        let mut ram = vec![0; PARAM + 0x53];
        assert_eq!(AgeProfile.refresh(&mut ram), None);
        ram[PARAM + 0x52] = 4;
        ram[PARAM + 0x1c..PARAM + 0x20].copy_from_slice(&u32::MAX.to_le_bytes());
        let before = ram.clone();
        assert_eq!(AgeProfile.refresh(&mut ram), None);
        assert_eq!(ram, before);
    }

    #[test]
    fn local_calendar_boundaries_ignore_same_day_and_backwards_changes() {
        assert!(!crossed_midnight(10.0 * 3600.0, 20.0 * 3600.0));
        assert!(crossed_midnight(86_399.0, 86_400.0));
        assert!(crossed_midnight(
            23.0 * 3600.0,
            10.0 * 86_400.0 + 10.0 * 3600.0
        ));
        assert!(!crossed_midnight(86_400.0, 1.0));
        assert!(!crossed_midnight(f64::NAN, 86_400.0));
    }
}
