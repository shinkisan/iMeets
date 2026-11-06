//! Call notifications. An ordinary wake does not notify. Mute changes
//! the channel, not detection. Five minutes collapse into one banner.

/// Ongoing residence notification. Android `startForeground` uses this id.
pub const FOREGROUND_NOTIFICATION_ID: i32 = 1;
pub const CALL_NOTIFICATION_ID: i32 = 2;
pub const CARE_FAIL_NOTIFICATION_ID: i32 = 3;
/// Background-settings self-test. Not the residence banner.
pub const TEST_NOTIFICATION_ID: i32 = 4;
pub const COLLAPSE_MS: i64 = 300_000;
pub const CHANNEL_RUNNING: &str = "pet_running";
pub const CHANNEL_CALLS: &str = "pet_calls";
pub const CHANNEL_CALLS_SILENT: &str = "pet_calls_silent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallDecision {
    pub detected: bool,
    pub banner: bool,
    pub channel: &'static str,
    pub not_before_ms: i64,
    pub last_ms: i64,
}

pub fn on_call(
    take_call: bool,
    visible: bool,
    muted: bool,
    now_ms: i64,
    last_ms: i64,
    not_before_ms: i64,
) -> CallDecision {
    if !take_call {
        return CallDecision {
            detected: false,
            banner: false,
            channel: "",
            not_before_ms,
            last_ms,
        };
    }
    let collapsed = now_ms < not_before_ms || (last_ms > 0 && now_ms < last_ms + COLLAPSE_MS);
    let banner = !visible && !collapsed;
    let channel = if !banner {
        ""
    } else if muted {
        CHANNEL_CALLS_SILENT
    } else {
        CHANNEL_CALLS
    };
    let (not_before_ms, last_ms) = if collapsed {
        (not_before_ms, last_ms)
    } else {
        (
            now_ms.saturating_add(COLLAPSE_MS).max(not_before_ms),
            now_ms,
        )
    };
    CallDecision {
        detected: true,
        banner,
        channel,
        not_before_ms,
        last_ms,
    }
}

pub struct AlarmFire {
    pub post: bool,
    pub not_before_ms: i64,
}

/// A forecast alarm. Visible play skips the banner. A missed event from
/// a force-quit gap is not posted.
pub fn on_alarm(event_ms: i64, now_ms: i64, visible: bool, missed_gap: bool) -> AlarmFire {
    let not_before_ms = event_ms.saturating_add(COLLAPSE_MS).max(now_ms);
    let post = !missed_gap && !visible && event_ms > 0 && now_ms >= event_ms;
    AlarmFire {
        post,
        not_before_ms,
    }
}

pub fn missed_while_stopped(event_ms: i64, now_ms: i64) -> bool {
    event_ms > 0 && event_ms < now_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_rules() {
        let quiet = on_call(false, false, false, 1_000, 0, 0);
        assert!(!quiet.detected);
        assert!(!quiet.banner);

        let shown = on_call(true, false, false, 1_000, 0, 0);
        assert!(shown.banner);
        assert_eq!(shown.channel, CHANNEL_CALLS);
        assert_eq!(shown.not_before_ms, 1_000 + COLLAPSE_MS);

        let silent = on_call(true, false, true, 1_000, 0, 0);
        assert!(silent.detected);
        assert_eq!(silent.channel, CHANNEL_CALLS_SILENT);

        let front = on_call(true, true, false, 1_000, 0, 0);
        assert!(front.detected);
        assert!(!front.banner);

        let again = on_call(
            true,
            false,
            false,
            50_000,
            shown.last_ms,
            shown.not_before_ms,
        );
        assert!(again.detected);
        assert!(!again.banner);

        let later = on_call(
            true,
            false,
            false,
            shown.not_before_ms,
            shown.last_ms,
            shown.not_before_ms,
        );
        assert!(later.banner);

        let due = on_alarm(5_000, 5_000, false, false);
        assert!(due.post);
        assert_eq!(due.not_before_ms, 5_000 + COLLAPSE_MS);
        assert!(!on_alarm(5_000, 5_000, true, false).post);
        assert!(missed_while_stopped(5_000, 9_000));
        assert!(!on_alarm(5_000, 9_000, false, true).post);
        assert_eq!(FOREGROUND_NOTIFICATION_ID, 1);
        assert_eq!(CALL_NOTIFICATION_ID, 2);
        assert_eq!(CARE_FAIL_NOTIFICATION_ID, 3);
        assert_eq!(TEST_NOTIFICATION_ID, 4);
    }
}
