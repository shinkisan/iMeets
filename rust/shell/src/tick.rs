//! Section 4.2 periods. Pure so the tick can be tested without a ROM.

pub const FRAME_NS: u64 = 16_666_667;
pub const AUTOSAVE_MS: u64 = 30_000;
pub const CATCHUP_SAVE_MS: u64 = 10_000;
pub const CATCHUP_GAP_S: f64 = 30.0;
pub const SLICE_CAP_S: f64 = 0.5;
pub const QUANTUM_VISIBLE_S: f64 = 0.02;
pub const QUANTUM_HIDDEN_S: f64 = 0.05;
pub const DEADLINE_VISIBLE_MS: u64 = 8;
pub const DEADLINE_HIDDEN_MS: u64 = 100;
/// Picture in Picture is watching, but the toy page is not. Fast enough for
/// the LCD to move, slower than a foreground frame.
pub const DEADLINE_PRESENT_MS: u64 = 33;
pub const BACKLOG_EPSILON_S: f64 = 0.000_001;
pub const CARE_SYNC_BUDGET_S: f64 = 10.0;
pub const SHELF_REPLAN_MS: u64 = 600;

pub fn wants_catch_up(dt: f64, now: f64, care_at: f64, paused: bool) -> bool {
    !paused && dt.is_finite() && (dt > CATCHUP_GAP_S || now - care_at > CATCHUP_GAP_S)
}

pub fn quantum(visible: bool) -> f64 {
    if visible {
        QUANTUM_VISIBLE_S
    } else {
        QUANTUM_HIDDEN_S
    }
}

pub fn slice_cap(dt: f64) -> f64 {
    if !dt.is_finite() || dt < 0.0 {
        0.0
    } else {
        dt.min(SLICE_CAP_S)
    }
}

pub fn deadline_ms(visible: bool, link_state: i32, present: bool) -> u64 {
    if visible || (1..=4).contains(&link_state) {
        DEADLINE_VISIBLE_MS
    } else if present {
        DEADLINE_PRESENT_MS
    } else {
        DEADLINE_HIDDEN_MS
    }
}

/// Keep the firmware idle counter down while the person asked for a lit
/// display and can still see it: the toy page in the foreground, or Picture
/// in Picture. A hidden app with PiP closed follows the toy's own sleep.
/// The default preference (`follow` false) keeps the hint false. Catch-up
/// leaves the hint off while it replays; the live tick turns it back on.
pub fn display_hint(
    toy_visible: bool,
    session_active: bool,
    care_paused: bool,
    follow_system_sleep: bool,
    phone_interactive: bool,
    present: bool,
) -> bool {
    session_active
        && !care_paused
        && follow_system_sleep
        && ((toy_visible && phone_interactive) || present)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods_match_the_spec() {
        assert!(!wants_catch_up(30.0, 100.0, 80.0, false));
        assert!(wants_catch_up(30.1, 100.0, 90.0, false));
        assert!(wants_catch_up(1.0, 100.0, 69.0, false));
        assert!(!wants_catch_up(100.0, 100.0, 0.0, true));
        assert_eq!(quantum(true), 0.02);
        assert_eq!(quantum(false), 0.05);
        assert_eq!(slice_cap(10.0), 0.5);
        assert_eq!(slice_cap(0.2), 0.2);
        assert_eq!(slice_cap(-1.0), 0.0);
        assert_eq!(deadline_ms(true, 0, false), 8);
        assert_eq!(deadline_ms(false, 1, false), 8);
        assert_eq!(deadline_ms(false, 4, true), 8);
        assert_eq!(deadline_ms(false, 0, false), 100);
        assert_eq!(deadline_ms(false, 5, false), 100);
        assert_eq!(deadline_ms(false, 0, true), DEADLINE_PRESENT_MS);
        assert_eq!(FRAME_NS, 16_666_667);
        assert!(!display_hint(true, true, false, false, true, false));
        assert!(display_hint(true, true, false, true, true, false));
        assert!(display_hint(true, true, false, true, true, true));
        assert!(!display_hint(true, true, true, true, true, false));
        assert!(!display_hint(true, true, false, true, false, false));
        assert!(display_hint(false, true, false, true, false, true));
        assert!(!display_hint(false, true, false, false, false, true));
        assert!(!display_hint(false, false, false, true, false, true));
    }
}
