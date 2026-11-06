//! WebMeets peripheral limits. Payloads are 1..=20 bytes and are not decoded.
//! TODO(att-layout): SPEC §14.4.

use crate::identity::BLE_NAME;

pub const SERVICE_UUID: &str = "0000fff0-0000-1000-8000-00805f9b34fb";
pub const TX_UUID: &str = "0000fff1-0000-1000-8000-00805f9b34fb";
pub const CCCD_UUID: &str = "00002902-0000-1000-8000-00805f9b34fb";
pub const RX_UUID: &str = "0000fff2-0000-1000-8000-00805f9b34fb";

pub const CCCD_ENABLE: [u8; 2] = [0x01, 0x00];
pub const CCCD_DISABLE: [u8; 2] = [0x00, 0x00];

pub const RX_QUEUE_MAX: usize = 128;
pub const TX_QUEUE_MAX: usize = 512;
pub const IDLE_TIMEOUT_MS: i64 = 300_000;
pub const ACTIVE_TIMEOUT_MS: i64 = 120_000;
pub const DISCONNECT_GRACE_MS: i64 = 2_500;
pub const COMMIT_CLOSE_MS: i64 = 300;
pub const NAME_POLLS: i32 = 25;
pub const NAME_POLL_MS: i64 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttError {
    NotPermitted,
    FailReceive,
}

pub fn rx_write(
    owner: bool,
    subscribed: bool,
    prepared: bool,
    offset: i32,
    len: usize,
    queued: usize,
) -> Result<(), AttError> {
    if !owner || !subscribed || prepared || offset != 0 || !(1..=20).contains(&len) {
        return Err(AttError::NotPermitted);
    }
    if queued >= RX_QUEUE_MAX {
        return Err(AttError::FailReceive);
    }
    Ok(())
}

pub fn push_notify(queue_len: usize, payload_len: usize) -> Result<(), AttError> {
    if payload_len == 0 || payload_len > 20 {
        return Err(AttError::FailReceive);
    }
    if queue_len >= TX_QUEUE_MAX {
        return Err(AttError::FailReceive);
    }
    Ok(())
}

pub fn cccd_read(owner: bool, offset: i32, subscribed: bool) -> Result<[u8; 2], AttError> {
    if owner && offset == 0 {
        Ok(if subscribed {
            CCCD_ENABLE
        } else {
            CCCD_DISABLE
        })
    } else {
        Err(AttError::NotPermitted)
    }
}

pub fn cccd_write(prepared: bool, offset: i32, value: &[u8]) -> Result<bool, AttError> {
    if prepared || offset != 0 {
        return Err(AttError::NotPermitted);
    }
    if value == CCCD_ENABLE {
        Ok(true)
    } else if value == CCCD_DISABLE {
        Ok(false)
    } else {
        Err(AttError::NotPermitted)
    }
}

pub fn tx_read(owner: bool, offset: i32) -> Result<&'static [u8], AttError> {
    if owner && offset == 0 {
        Ok(&[])
    } else {
        Err(AttError::NotPermitted)
    }
}

pub fn execute_write() -> AttError {
    AttError::NotPermitted
}

#[derive(Clone, Debug)]
pub struct Deadlines {
    pub deadline_ms: i64,
}

impl Deadlines {
    pub fn start(now: i64) -> Self {
        Self {
            deadline_ms: now + IDLE_TIMEOUT_MS,
        }
    }
    pub fn touch(&mut self, now: i64) {
        self.deadline_ms = now + ACTIVE_TIMEOUT_MS;
    }
    pub fn expired(&self, now: i64) -> bool {
        now >= self.deadline_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Preparing,
    Waiting,
    Connected,
    Receiving,
    Success,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct Receive {
    pub phase: Phase,
    pub committed: bool,
    pub grace_deadline: Option<i64>,
    pub success_at: Option<i64>,
}

impl Receive {
    pub fn new() -> Self {
        Self {
            phase: Phase::Preparing,
            committed: false,
            grace_deadline: None,
            success_at: None,
        }
    }

    pub fn on_native(&mut self, state: i32, now: i64) {
        if state == 3 {
            self.phase = Phase::Receiving;
        } else if state == 5 {
            self.committed = true;
            self.success_at = Some(now + COMMIT_CLOSE_MS);
        } else if state < 0 && !self.committed {
            self.phase = Phase::Failed;
        } else if state < 0 && self.committed {
            self.success_at = Some(now + COMMIT_CLOSE_MS);
        }
    }

    pub fn on_disconnect(&mut self, now: i64) {
        if self.committed {
            self.success_at.get_or_insert(now + COMMIT_CLOSE_MS);
        } else {
            self.grace_deadline = Some(now + DISCONNECT_GRACE_MS);
        }
    }

    pub fn poll(&mut self, now: i64) -> Phase {
        if let Some(t) = self.success_at {
            if now >= t {
                self.phase = Phase::Success;
            }
        } else if let Some(t) = self.grace_deadline {
            if now >= t && !self.committed {
                self.phase = Phase::Cancelled;
            }
        }
        self.phase
    }
}

pub fn previous_name_to_store<'a>(current: &'a str) -> Option<&'a str> {
    if current == BLE_NAME {
        None
    } else {
        Some(current)
    }
}

pub fn should_restore(previous: Option<&str>, current: &str) -> bool {
    previous.is_some() && current == BLE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gatt_limits_timeouts_and_commit() {
        assert_eq!(SERVICE_UUID, "0000fff0-0000-1000-8000-00805f9b34fb");
        assert_eq!(TX_UUID, "0000fff1-0000-1000-8000-00805f9b34fb");
        assert_eq!(RX_UUID, "0000fff2-0000-1000-8000-00805f9b34fb");
        assert_eq!(CCCD_UUID, "00002902-0000-1000-8000-00805f9b34fb");
        assert!(rx_write(true, true, false, 0, 1, 0).is_ok());
        assert!(rx_write(true, true, false, 0, 20, 127).is_ok());
        assert_eq!(
            rx_write(true, true, false, 0, 0, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(true, true, false, 0, 21, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(false, true, false, 0, 4, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(true, false, false, 0, 4, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(true, true, true, 0, 4, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(true, true, false, 1, 4, 0),
            Err(AttError::NotPermitted)
        );
        assert_eq!(
            rx_write(true, true, false, 0, 4, 128),
            Err(AttError::FailReceive)
        );
        assert_eq!(push_notify(0, 21), Err(AttError::FailReceive));
        assert_eq!(push_notify(512, 1), Err(AttError::FailReceive));
        assert!(push_notify(511, 20).is_ok());
        assert_eq!(cccd_read(true, 0, true).unwrap(), CCCD_ENABLE);
        assert_eq!(cccd_read(true, 0, false).unwrap(), CCCD_DISABLE);
        assert!(cccd_read(false, 0, true).is_err());
        assert_eq!(cccd_write(false, 0, &CCCD_ENABLE).unwrap(), true);
        assert_eq!(cccd_write(false, 0, &CCCD_DISABLE).unwrap(), false);
        assert!(cccd_write(true, 0, &CCCD_ENABLE).is_err());
        assert!(cccd_write(false, 0, &[0x02, 0x00]).is_err());
        assert!(tx_read(true, 0).unwrap().is_empty());
        assert!(tx_read(true, 1).is_err());
        assert_eq!(execute_write(), AttError::NotPermitted);

        let mut d = Deadlines::start(0);
        assert_eq!(d.deadline_ms, 300_000);
        assert!(!d.expired(299_999));
        d.touch(1_000);
        assert_eq!(d.deadline_ms, 1_000 + 120_000);
        assert_eq!(NAME_POLLS, 25);
        assert_eq!(NAME_POLL_MS, 200);

        let mut r = Receive::new();
        r.phase = Phase::Waiting;
        r.on_native(3, 0);
        assert_eq!(r.phase, Phase::Receiving);
        r.on_native(5, 10);
        assert!(r.committed);
        assert_eq!(r.success_at, Some(310));
        assert_eq!(r.poll(309), Phase::Receiving);
        assert_eq!(r.poll(310), Phase::Success);

        let mut drop = Receive::new();
        drop.on_disconnect(0);
        assert_eq!(drop.poll(2_499), Phase::Preparing);
        assert_eq!(drop.poll(2_500), Phase::Cancelled);

        let mut late = Receive::new();
        late.on_native(5, 0);
        late.on_disconnect(10);
        assert_eq!(late.poll(10 + COMMIT_CLOSE_MS), Phase::Success);

        let mut neg = Receive::new();
        neg.on_native(-1, 0);
        assert_eq!(neg.phase, Phase::Failed);
        let mut after = Receive::new();
        after.on_native(5, 0);
        after.on_native(-1, 5);
        assert!(after.committed);
        assert_eq!(after.poll(5 + COMMIT_CLOSE_MS), Phase::Success);

        assert_eq!(previous_name_to_store("Phone"), Some("Phone"));
        assert_eq!(previous_name_to_store(BLE_NAME), None);
        assert!(should_restore(Some("Phone"), BLE_NAME));
        assert!(!should_restore(Some("Phone"), "Phone"));
    }
}
