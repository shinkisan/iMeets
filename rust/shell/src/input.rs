//! Button map. A and C together are two keys. Auto-repeat is not a new down.

use crate::copy::{self, Lang};

/// Keyboard and d-pad names the shell accepts. Unknown names are ignored.
pub fn key_index(name: &str) -> Option<u8> {
    match name {
        "A" | "a" | "Left" | "ArrowLeft" => Some(0),
        "B" | "b" | "S" | "s" | "Return" | "Enter" | "DpadCenter" => Some(1),
        "C" | "c" | "D" | "d" | "Right" | "ArrowRight" => Some(2),
        _ => None,
    }
}

/// A second down while the key is already held is keyboard auto-repeat.
pub fn accept_down(already_down: bool, down: bool) -> bool {
    down && !already_down
}

pub fn alarm_copy(exact_allowed: bool) -> &'static str {
    copy::text(Lang::En, if exact_allowed { "alarm_exact" } else { "alarm_inexact" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_level_triggered_and_not_a_chord() {
        assert_eq!(key_index("A"), Some(0));
        assert_eq!(key_index("Left"), Some(0));
        assert_eq!(key_index("S"), Some(1));
        assert_eq!(key_index("Return"), Some(1));
        assert_eq!(key_index("DpadCenter"), Some(1));
        assert_eq!(key_index("D"), Some(2));
        assert_eq!(key_index("Right"), Some(2));
        assert_eq!(key_index("M"), None);
        assert!(accept_down(false, true));
        assert!(!accept_down(true, true));
        assert!(!accept_down(true, false));
        // A and C stay distinct indices.
        assert_ne!(key_index("A"), key_index("C"));
        assert_eq!(alarm_copy(true), "notifications use an exact alarm");
        assert_eq!(alarm_copy(false), "notifications may be late");
        assert_eq!(copy::text(Lang::En, "confirm_wake"), "confirm / wake");
    }
}
