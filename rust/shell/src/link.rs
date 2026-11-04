//! Link router. Kinds 20, 21, and 23 open Mixing. The WebMeets flag opens
//! the GATT receiver. Every other kind, including the camera, is cancelled.
//!
//! TODO(link-kind-signatures): SPEC §14.1–14.3. The public core reports
//! Bluetooth, Infrared, or BleFail only. It does not name the camera kind
//! or which of 20/21/23 is Visit, Present, or the third door. State names
//! for 1, 2, and 4 are also unknown. This module only applies the shell's
//! reactions. It does not invent those numbers.

use std::collections::VecDeque;

pub const MIXING_KINDS: [i32; 3] = [20, 21, 23];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Unsupported = 0,
    WebMeets = 1,
    Mixing = 2,
}

pub fn route(web_meets: bool, kind: i32) -> Route {
    if web_meets {
        Route::WebMeets
    } else if MIXING_KINDS.contains(&kind) {
        Route::Mixing
    } else {
        Route::Unsupported
    }
}

pub fn opens_ble(route: Route) -> bool {
    route == Route::WebMeets
}

pub fn opens_mixing(route: Route) -> bool {
    route == Route::Mixing
}

/// Step code: 1 unsupported, 2 the shell should route.
pub fn step_code(classified: bool, web_meets: bool, kind: i32) -> i32 {
    if !classified {
        return 1;
    }
    match route(web_meets, kind) {
        Route::Unsupported => 1,
        _ => 2,
    }
}

pub fn blocks_edit(state: i32) -> bool {
    (1..=4).contains(&state)
}

/// Unknown positive values outside 1..=5 are idle for navigation.
pub fn mixing_should_close(state: i32) -> bool {
    state == 0 || state == 5 || state > 5
}

pub fn gift_failed(state: i32) -> bool {
    state < 0
}

#[derive(Clone, Debug)]
pub struct Link {
    pub enabled: bool,
    pub state: i32,
    pub kind: i32,
    pub web: bool,
    pub recovered: bool,
    pub classified: bool,
    pub connected: bool,
    pub rx: VecDeque<Vec<u8>>,
    pub tx: VecDeque<Vec<u8>>,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            enabled: false,
            state: 0,
            kind: 0,
            web: false,
            recovered: false,
            classified: false,
            connected: false,
            rx: VecDeque::new(),
            tx: VecDeque::new(),
        }
    }
}

impl Link {
    pub fn enable(&mut self, on: bool) {
        self.enabled = on;
    }

    pub fn connect(&mut self) {
        // TODO(att-layout): SPEC §14.4. Accepted writes are queued. The
        // ATT opcodes inside the 1..20 byte frames are not interpreted.
        self.connected = true;
        if self.state == 0 || self.state == 5 {
            self.state = 3;
        }
    }

    pub fn cancel(&mut self) {
        self.state = 0;
        self.connected = false;
        self.rx.clear();
        self.tx.clear();
        self.kind = 0;
        self.web = false;
        self.classified = false;
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if !(1..=20).contains(&bytes.len()) || self.rx.len() >= 128 {
            return Err(());
        }
        self.rx.push_back(bytes.to_vec());
        Ok(())
    }

    pub fn read(&mut self, out: &mut [u8]) -> i32 {
        let Some(pkt) = self.tx.pop_front() else {
            return 0;
        };
        let n = pkt.len().min(out.len());
        out[..n].copy_from_slice(&pkt[..n]);
        n as i32
    }

    pub fn push_tx(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.is_empty() || bytes.len() > 20 || self.tx.len() >= 512 {
            return Err(());
        }
        self.tx.push_back(bytes.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_camera_mixing_and_states() {
        assert_eq!(route(true, 99), Route::WebMeets);
        assert!(opens_ble(route(true, 1)));
        assert!(!opens_mixing(route(true, 20)));
        for k in MIXING_KINDS {
            assert_eq!(route(false, k), Route::Mixing);
            assert!(opens_mixing(route(false, k)));
            assert!(!opens_ble(route(false, k)));
            assert_eq!(step_code(true, false, k), 2);
        }
        // Camera and infrared are not these kinds and are not WebMeets.
        for k in [0, 1, 19, 22, 24, 100] {
            assert_eq!(route(false, k), Route::Unsupported);
            assert!(!opens_ble(route(false, k)));
            assert!(!opens_mixing(route(false, k)));
            assert_eq!(step_code(true, false, k), 1);
        }
        assert_eq!(step_code(false, false, 20), 1);
        assert!(blocks_edit(1) && blocks_edit(4));
        assert!(!blocks_edit(0) && !blocks_edit(5) && !blocks_edit(6));
        assert!(mixing_should_close(0));
        assert!(mixing_should_close(5));
        assert!(mixing_should_close(6));
        assert!(!mixing_should_close(2));
        assert!(gift_failed(-1));
        assert!(!gift_failed(3));
    }

    #[test]
    fn packet_pipe_limits() {
        let mut l = Link::default();
        assert!(l.write(&[]).is_err());
        assert!(l.write(&[1]).is_ok());
        assert!(l.write(&[0; 20]).is_ok());
        assert!(l.write(&[0; 21]).is_err());
        l.connect();
        assert_eq!(l.state, 3);
        l.cancel();
        assert_eq!(l.state, 0);
        assert!(l.rx.is_empty());
        assert!(l.push_tx(&[0x0A]).is_ok());
        let mut buf = [0u8; 8];
        assert_eq!(l.read(&mut buf), 1);
        assert_eq!(buf[0], 0x0A);
    }
}
