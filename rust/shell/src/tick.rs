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
pub const BACKLOG_EPSILON_S: f64 = 0.000_001;
pub const CARE_SYNC_BUDGET_S: f64 = 10.0;
pub const SHELF_REPLAN_MS: u64 = 600;

pub fn wants_catch_up(dt: f64, now: f64, care_at: f64, paused: bool) -> bool {
    !paused && dt.is_finite() && (dt > CATCHUP_GAP_S || now - care_at > CATCHUP_GAP_S)
}

pub fn quantum(visible: bool) -> f64 {
    if visible { QUANTUM_VISIBLE_S } else { QUANTUM_HIDDEN_S }
}

pub fn slice_cap(dt: f64) -> f64 {
    if !dt.is_finite() || dt < 0.0 { 0.0 } else { dt.min(SLICE_CAP_S) }
}

pub fn deadline_ms(visible: bool, link_state: i32) -> u64 {
    if visible || (1..=4).contains(&link_state) {
        DEADLINE_VISIBLE_MS
    } else {
        DEADLINE_HIDDEN_MS
    }
}

/// Hint is true only when every condition holds. The default preference
/// (`follow` false) keeps the hint false.
pub fn display_hint(
    toy_visible: bool,
    session_active: bool,
    catchup_done: bool,
    care_paused: bool,
    follow_system_sleep: bool,
    phone_interactive: bool,
) -> bool {
    toy_visible && session_active && catchup_done && !care_paused && follow_system_sleep && phone_interactive
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
        assert_eq!(deadline_ms(true, 0), 8);
        assert_eq!(deadline_ms(false, 1), 8);
        assert_eq!(deadline_ms(false, 4), 8);
        assert_eq!(deadline_ms(false, 0), 100);
        assert_eq!(deadline_ms(false, 5), 100);
        assert_eq!(FRAME_NS, 16_666_667);
        assert!(!display_hint(true, true, true, false, false, true));
        assert!(display_hint(true, true, true, false, true, true));
        assert!(!display_hint(true, true, false, false, true, true));
        assert!(!display_hint(true, true, true, true, true, true));
        assert!(!display_hint(true, true, true, false, true, false));
    }
}
