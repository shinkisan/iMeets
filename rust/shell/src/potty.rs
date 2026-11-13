//! Read-only care signal for the verified JP v062 Sweets firmware.
//! See docs/potty-sweets.md for ROM instructions and runtime validation.

use tg18::Machine;

pub const SWEETS_CODE_HASH: u64 = 0x28ad_63c3_4fa9_e369;
const PARAM: usize = 0xd834;

#[derive(Clone, Copy, Debug)]
pub struct PottyProfile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PottyState {
    /// Firmware countdown, in seconds at normal speed. Zero means no urge.
    pub seconds_left: u16,
    pub floor_count: u8,
}

impl PottyProfile {
    pub fn for_image(image: &[u8]) -> Option<Self> {
        (tg18::is_image(image) && tg18::code_hash(image) == SWEETS_CODE_HASH).then_some(Self)
    }

    pub fn read(self, machine: &Machine) -> Option<PottyState> {
        // Deep sleep clears/rebuilds SRAM. Its old contents aren't a live signal.
        if machine.sys.powered_off {
            return None;
        }
        read_sram(&machine.sys.sram)
    }
}

fn read_sram(sram: &[u8]) -> Option<PottyState> {
    let param = sram.get(PARAM..PARAM + 0x53)?;
    let seconds_left = u16::from_le_bytes([param[0x26], param[0x27]]);
    // Firmware initializes this timer to 20. Reject uninitialized/invalid data.
    if seconds_left > 20 || !(1..=5).contains(&param[0x52]) {
        return None;
    }
    Some(PottyState {
        seconds_left,
        floor_count: param[0x24],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urge_is_the_countdown_not_floor_poop() {
        let mut ram = vec![0; PARAM + 0x53];
        ram[PARAM + 0x52] = 4;
        ram[PARAM + 0x26] = 20;
        assert_eq!(
            read_sram(&ram),
            Some(PottyState {
                seconds_left: 20,
                floor_count: 0
            })
        );
        ram[PARAM + 0x26] = 0;
        ram[PARAM + 0x24] = 1;
        assert_eq!(
            read_sram(&ram),
            Some(PottyState {
                seconds_left: 0,
                floor_count: 1
            })
        );
        // A later urge can coexist with a previous pile on the floor.
        ram[PARAM + 0x26] = 7;
        assert_eq!(read_sram(&ram).unwrap().seconds_left, 7);
        ram[PARAM + 0x27] = 1;
        assert!(read_sram(&ram).is_none());
        ram[PARAM + 0x27] = 0;
        ram[PARAM + 0x52] = 0;
        assert!(read_sram(&ram).is_none());
        assert!(read_sram(&[]).is_none());
    }

    #[test]
    fn unknown_firmware_is_not_supported() {
        assert!(PottyProfile::for_image(&[]).is_none());
        let mut image = vec![0; tg18::FLASH_SIZE];
        image[..4].copy_from_slice(b"SPII");
        assert!(PottyProfile::for_image(&image).is_none());
    }

    /// Local, private snapshots are deliberately not committed. This replays the
    /// real firmware through the public Session/FFI path, without editing RAM.
    #[test]
    fn sweets_firmware_urge_expires_or_is_caught() {
        let Ok(fixtures) = std::env::var("POCKET_MEETS_POTTY_FIXTURES") else {
            return;
        };
        let image =
            std::fs::read(std::env::var("POCKET_MEETS_ROM").expect("POCKET_MEETS_ROM")).unwrap();
        assert!(
            PottyProfile::for_image(&image).is_some(),
            "wrong ROM for these fixtures"
        );
        let load = |name: &str| {
            let mut s = crate::session::Session::bare(image.clone());
            s.playable = true;
            let path = std::path::Path::new(&fixtures).join(name);
            tg18::snapshot::Snapshot::load(path.to_str().unwrap())
                .unwrap()
                .restore(&mut s.machine)
                .unwrap();
            s.display_awake(true);
            s
        };
        let step = |s: &mut crate::session::Session, seconds: f64| {
            for _ in 0..(seconds * 50.0).round() as usize {
                assert_eq!(s.step(0.02), 0);
            }
        };
        let mut missed = load("natural/000000026000.t18s");
        assert_eq!(crate::ffi::pm_potty_seconds(&mut missed), 0);
        step(&mut missed, 2.0);
        assert!((1..=20).contains(&crate::ffi::pm_potty_seconds(&mut missed)));
        assert_eq!(missed.potty_state().unwrap().floor_count, 0);
        step(&mut missed, 23.0);
        assert_eq!(crate::ffi::pm_potty_seconds(&mut missed), 0);
        assert_eq!(missed.potty_state().unwrap().floor_count, 1);

        let mut caught = load("natural/000000030000.t18s");
        assert!(crate::ffi::pm_potty_seconds(&mut caught) > 0);
        let start = caught.machine.now();
        for (at, key) in [
            (0.1, tg18::Key::A),
            (0.6, tg18::Key::A),
            (1.1, tg18::Key::A),
            (1.6, tg18::Key::B),
            (2.4, tg18::Key::B),
        ] {
            caught.machine.press(start + at, key);
        }
        step(&mut caught, 25.0);
        assert_eq!(crate::ffi::pm_potty_seconds(&mut caught), 0);
        assert_eq!(caught.potty_state().unwrap().floor_count, 0);
        // Offline replay must not present historical urges as current ones.
        let mut active = load("natural/000000030000.t18s");
        assert!(crate::ffi::pm_potty_seconds(&mut active) > 0);
        active.catching_up = true;
        assert_eq!(crate::ffi::pm_potty_seconds(&mut active), -1);
    }
}
