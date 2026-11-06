//! Mixing Meets host pipe. The radio is fake. No IR, no firmware update,
//! no marriage check in the shell.
//!
//! TODO(mixing-api-codes): SPEC §14.10. The literals 15, 18, 46, 17, 36, 48
//! sit beside the debug text `exec api c` and are not mapped to operations.
//! A POST of `c=15` to the public host on 2026-10-31 was echoed, not executed.
//! TODO(mixing-marriage-server): SPEC §14.11. The shell does not require a
//! server field before the firmware marriage bytes are written.

use crate::identity::{APP_KEY, FAKE_DEVICE_JSON, MIXING_URL, MIXING_VERSION};

pub const SCAN_DELAY_MS: u64 = 200;
pub const WRITE_SLICE: usize = 20;
pub const FRAME_MS: u64 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleView {
    Idle,
    Waiting,
    Connected,
    Disconnected,
}

#[derive(Clone, Debug)]
pub struct MixingBle {
    pub connect_started: bool,
    pub view: BleView,
}

impl MixingBle {
    pub fn new() -> Self {
        Self {
            connect_started: false,
            view: BleView::Idle,
        }
    }

    pub fn init() -> &'static str {
        "Success"
    }

    pub fn scan_complete() -> &'static str {
        "Success"
    }

    pub fn device_list() -> &'static str {
        FAKE_DEVICE_JSON
    }

    pub fn connect(&mut self) {
        self.connect_started = true;
        self.view = BleView::Waiting;
    }

    pub fn on_state(&mut self, state: i32) -> BleView {
        if !self.connect_started {
            return self.view;
        }
        self.view = if state == 3 {
            BleView::Connected
        } else if state == 4 || state == 5 || state <= 0 {
            self.connect_started = false;
            BleView::Disconnected
        } else {
            // States 1 and 2 have no shell name. Stay waiting.
            BleView::Waiting
        };
        self.view
    }
}

pub fn notify_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        s.push_str(&format!("{b:02X} "));
    }
    s
}

pub fn slice_write(data: &[u8]) -> Vec<&[u8]> {
    if data.is_empty() {
        Vec::new()
    } else {
        data.chunks(WRITE_SLICE).collect()
    }
}

pub fn delay_ok(delay_ms: i64) -> bool {
    (0..=1000).contains(&delay_ms)
}

pub fn login_form() -> String {
    format!("AppVersion={MIXING_VERSION}&AppKey={APP_KEY}")
}

pub fn primary_url() -> &'static str {
    MIXING_URL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_pipe_and_version() {
        assert_eq!(MixingBle::init(), "Success");
        assert_eq!(MixingBle::scan_complete(), "Success");
        assert_eq!(SCAN_DELAY_MS, 200);
        assert_eq!(FRAME_MS, 100);
        let list = MixingBle::device_list();
        assert!(list.contains("TMGC_meets"));
        assert!(list.contains("02:00:00:00:00:18"));
        let mut b = MixingBle::new();
        b.connect();
        assert_eq!(b.on_state(1), BleView::Waiting);
        assert_eq!(b.on_state(3), BleView::Connected);
        assert_eq!(b.on_state(4), BleView::Disconnected);
        let mut b = MixingBle::new();
        b.connect();
        assert_eq!(b.on_state(5), BleView::Disconnected);
        let mut b = MixingBle::new();
        b.connect();
        assert_eq!(b.on_state(0), BleView::Disconnected);
        assert_eq!(notify_hex(&[0x01, 0x0A]), "01 0A ");
        assert_eq!(slice_write(&[0; 45]).len(), 3);
        assert_eq!(slice_write(&[0; 45])[2].len(), 5);
        assert!(delay_ok(0) && delay_ok(1000) && !delay_ok(1001) && !delay_ok(-1));
        let form = login_form();
        assert!(form.contains("AppVersion=1.4.3"));
        assert!(!form.contains("1.0.1"));
        assert!(form.contains(APP_KEY));
        assert!(primary_url().contains("com.daiban.tamagotchionapi_en/meets.php"));
        assert_eq!(
            crate::copy::text(crate::copy::Lang::En, "fw_update_log"),
            "the virtual controller does not accept firmware updates"
        );
    }
}
