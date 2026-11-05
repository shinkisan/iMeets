//! One open toy. `openCare` is what the shell uses. `advanceClock` and
//! `careAdopt` are exported and are not called by the UI.
//!
//! TODO(pause-clock-split): SPEC §14.6. A stored pause sets the RTC to now
//! and `care_at` to now, so the away gap is not simulated. Which side of
//! the core would have done that split is not specified.
//! TODO(care-adopt): SPEC §14.7. The export does nothing.
//! TODO(link-kind-signatures): SPEC §14.1–14.3. An unclassified Bluetooth
//! or infrared stop is the camera path: restore the pre-press snapshot,
//! return 1, and do not open BLE or Mixing. A hosted route (return 2) runs
//! only when a classification is already latched inside this crate.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tg18::save;
use tg18::snapshot::{SnapError, Snapshot};
use tg18::sound::{CallDetector, ToneSynth};
use tg18::{Key, Machine, Outcome, CPU_HZ};

use crate::care::CareTimeline;
use crate::copy::{self, Lang};
use crate::keypatch;
use crate::library::{self, Container};
use crate::link::{self, Link, Route};
use crate::rom::{self, ImageClass};
use crate::shelf::{self, Edit};
use crate::tick::{self, CARE_SYNC_BUDGET_S};

pub const AUDIO_RATE: u32 = 44_100;
const REWIND_GAP: f64 = 1.0;
/// A down and an up can both land before the next quantum. Keep the key
/// down this long so the firmware debounce still sees the tap.
const MIN_HOLD: f64 = 0.05;
/// The same floor once the firmware's debounce is patched down to 2 samples.
const MIN_HOLD_FAST: f64 = 0.016;
/// Seconds after waking before the fast-key patch goes live, so the firmware
/// can boot and settle with its original timings.
const KEYPATCH_GRACE: f64 = 2.0;

#[derive(Clone, Debug)]
pub struct PumpIn {
    pub now_unix: f64,
    pub dt: f64,
    pub visible: bool,
    pub monotonic_ms: u64,
    pub last_frame_ns: u64,
    pub last_save_ms: u64,
    pub last_catchup_save_ms: u64,
    pub backlog: f64,
    pub phone_interactive: bool,
    pub follow_system_sleep: bool,
    pub muted: bool,
    pub frame_ns: u64,
}

impl Default for PumpIn {
    fn default() -> Self {
        Self {
            now_unix: 0.0,
            dt: 0.0,
            visible: false,
            monotonic_ms: 0,
            last_frame_ns: 0,
            last_save_ms: 0,
            last_catchup_save_ms: 0,
            backlog: 0.0,
            phone_interactive: true,
            follow_system_sleep: false,
            muted: false,
            frame_ns: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PumpOut {
    pub stopped: bool,
    pub error: String,
    pub notice: String,
    pub step_code: i32,
    pub route: i32,
    pub take_call: bool,
    pub play_audio: bool,
    pub volume: f32,
    pub drew_frame: bool,
    pub did_save: bool,
    pub catching_up: bool,
    pub tick: bool,
    pub next_ms: u64,
    pub backlog: f64,
    pub remain: f64,
    /// Seconds of away time being replayed (0 when not catching up).
    pub catchup_total: f64,
    pub last_frame_ns: u64,
    pub last_save_ms: u64,
    pub last_catchup_save_ms: u64,
    pub pcm: Vec<i16>,
}

pub struct Session {
    pub machine: Machine,
    pub image: Arc<Vec<u8>>,
    pub stem: String,
    pub title: String,
    pub root: PathBuf,
    pub save_dir: PathBuf,
    pub error: String,
    pub notice: String,
    pub stopped: bool,
    pub playable: bool,
    pub link: Link,
    pub care: CareTimeline,
    pub keys: [bool; 3],
    pub shelf_covers: bool,
    pub switching: bool,
    pub control_paused: bool,
    pub catching_up: bool,
    pub synth: ToneSynth,
    pub calls: CallDetector,
    pub audio_t: f64,
    /// Firmware PCM waiting for a visible tick. Hidden ticks drop it.
    pub pending_pcm: Vec<i16>,
    /// Infrared bytes drained for an app-side transport.
    pub ir_pending: VecDeque<u8>,
    pub rewind: Option<Snapshot>,
    pub rewind_at: f64,
    pub take_call: bool,
    pub awake_hint: bool,
    /// Last wall-clock re-base, in pump monotonic milliseconds.
    last_clock_sync_ms: u64,
    /// Hash of the last LCD frame handed to the UI.
    last_drawn_hash: u64,
    /// Away seconds in the current replay, for progress reporting.
    catchup_total: f64,
    /// Optional firmware patch that shortens the key debounce.
    pub fast_keys: bool,
    /// Emulated time the toy last woke, or the last time it was awake. The
    /// patch waits for the boot to finish before it goes live.
    awake_since: f64,
    /// Shortest time a tap keeps the key down, in emulated seconds.
    min_hold: f64,
}

impl Session {
    /// A machine with no supported firmware. Tests use it for memory and
    /// snapshot checks. `step` refuses to execute the CPU.
    pub fn bare(image: Vec<u8>) -> Self {
        let image = Arc::new(image);
        let machine = Machine::new(image.clone(), &image, 0.0);
        Self::from_parts(machine, image, "bare".into(), "Fairy (EN v057)".into(), false)
    }

    fn from_parts(mut machine: Machine, image: Arc<Vec<u8>>, stem: String, title: String, playable: bool) -> Self {
        machine.stop_on_bluetooth = true;
        Self {
            machine,
            image,
            stem,
            title,
            root: PathBuf::new(),
            save_dir: PathBuf::new(),
            error: String::new(),
            notice: String::new(),
            stopped: false,
            playable,
            link: Link::default(),
            care: CareTimeline::at(0.0),
            keys: [false; 3],
            shelf_covers: false,
            switching: false,
            control_paused: false,
            catching_up: false,
            synth: ToneSynth::new(AUDIO_RATE, 1.0),
            calls: CallDetector::default(),
            audio_t: 0.0,
            pending_pcm: Vec::new(),
            ir_pending: VecDeque::new(),
            rewind: None,
            rewind_at: -REWIND_GAP,
            take_call: false,
            awake_hint: false,
            last_clock_sync_ms: 0,
            last_drawn_hash: 0,
            catchup_total: 0.0,
            fast_keys: false,
            awake_since: 0.0,
            min_hold: MIN_HOLD,
        }
    }

    /// Turn the fast-key firmware patch on or off. The ROM on disk is never
    /// touched: the patch rewrites the scan routines where the firmware keeps
    /// them in SRAM, and only when every byte still matches what we expect.
    pub fn set_fast_keys(&mut self, on: bool) {
        self.fast_keys = on && self.is_sweets();
        self.min_hold = if self.fast_keys { MIN_HOLD_FAST } else { MIN_HOLD };
        if !on {
            keypatch::restore(&mut self.machine);
        }
    }

    pub fn open(rom: &Path, saves: &Path) -> Result<Self, String> {
        Self::open_care(rom, saves, false)
    }

    pub fn open_care(rom: &Path, saves: &Path, paused: bool) -> Result<Self, String> {
        let file = fs::File::open(rom).map_err(|_| copy::text(Lang::En, "import_failed_again").to_string())?;
        let bytes = rom::read_candidate(file).map_err(|_| copy::text(Lang::En, "choose_rom").to_string())?;
        let (stem, title) = match rom::classify(&bytes) {
            ImageClass::Supported { stem, title, .. } => (stem, title),
            ImageClass::NotImage => return Err(copy::text(Lang::En, "err_not_8mib").into()),
            ImageClass::Unsupported => return Err(copy::text(Lang::En, "err_unsupported").into()),
        };
        let image = Arc::new(bytes);
        let save_dir = saves.join(&stem);
        let _ = fs::create_dir_all(&save_dir);
        let mut machine = Machine::new(image.clone(), &image, tg18::rtc_seconds_now());
        let auto = save_dir.join("autosave.t18s");
        if auto.exists() {
            let snap = Snapshot::load(auto.to_str().unwrap_or("")).map_err(|e| e.to_string())?;
            snap.restore(&mut machine).map_err(|e| e.to_string())?;
        } else {
            let flash = save_dir.join("game.flash");
            if let Some(path) = flash.to_str() {
                match save::load_flash(path, &image) {
                    Ok(Some(flash)) => {
                        machine = Machine::new(image.clone(), &flash, 0.0);
                        if let Some(meta) = save::read_meta(path) {
                            save::restore_rtc(&mut machine, &meta, false);
                        }
                    }
                    Ok(None) => {}
                    Err(SnapError::Mismatch) => return Err(copy::text(Lang::En, "different_rom").into()),
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        machine.stop_on_bluetooth = true;
        let mut session = Self::from_parts(machine, image, stem, title, true);
        session.save_dir = save_dir;
        if let Some(parent) = saves.parent() {
            session.root = parent.to_path_buf();
        }
        let now = tg18::unix_now();
        let stored = CareTimeline::load(&session.save_dir);
        let had = session.save_dir.join("care.json").exists();
        session.care = if had { stored } else { CareTimeline::at(now) };
        if paused {
            session.care.set_paused(true, now);
        } else if session.care.paused {
            session.care.set_paused(false, now);
        }
        session.align_clock_for_open(now);
        let _ = session.care.store(&session.save_dir);
        if session.save_dir.join("link-pending").exists() {
            let back = session.save_dir.join("mixing-return.t18s");
            if back.exists() {
                match Snapshot::load(back.to_str().unwrap_or("")).and_then(|s| s.restore(&mut session.machine)) {
                    Ok(()) => {
                        session.machine.stop_on_bluetooth = true;
                        session.link.recovered = true;
                        session.notice = copy::text(Lang::En, "link_restored").into();
                    }
                    Err(_) => session.error = copy::text(Lang::En, "no_recovery").into(),
                }
            } else {
                session.error = copy::text(Lang::En, "no_recovery").into();
            }
            let _ = fs::remove_file(session.save_dir.join("link-pending"));
        }
        session.link.enable(true);
        Ok(session)
    }

    /// Start the RTC at the last accounted moment (wall clock minus the away
    /// gap) so the catch-up replay walks that gap minute by minute and lands
    /// on now. With nothing to replay (`due == 0`) this is simply "now".
    ///
    /// Paused care is a different thing: the whole world is supposed to be
    /// frozen, so the clock keeps whatever the save holds. Moving it to the
    /// phone's time would jump the pet's day forward even though nothing was
    /// replayed (its bedtime, its schedule) - the one part of "pause" that
    /// used to leak.
    fn align_clock_for_open(&mut self, now_unix: f64) {
        if self.care.paused {
            return;
        }
        let due = self.care.due(now_unix);
        let wall = tg18::rtc_seconds_now();
        let at = (wall - due).max(0.0);
        self.machine.sys.rtc.base_ticks = (at * 32768.0) as i64;
        self.machine.sys.rtc.base_time = self.machine.sys.game_time;
    }

    /// Re-base the RTC on the phone clock. Used while the toy is being played
    /// normally; the catch-up replay must not be re-based or it would jump.
    fn sync_clock_to_wall(&mut self) {
        let wall = tg18::rtc_seconds_now();
        self.machine.sys.rtc.base_ticks = (wall * 32768.0) as i64;
        self.machine.sys.rtc.base_time = self.machine.sys.game_time;
        // The RTC chip only feeds what is drawn; the game's own clock - the
        // one behind bedtime and daily events - is the SoC time of day. Move
        // both, or resuming a pause leaves the pet on the frozen schedule.
        self.machine.sys.rtc_irq.tod_base = (wall as i64).rem_euclid(86_400);
        self.machine.sys.rtc_irq.tod_set_at = self.machine.sys.game_time;
    }

    /// The translation table and the fast-key patch are written against the
    /// Sweets dump; other firmwares share the engine but not these bytes.
    pub fn is_sweets(&self) -> bool {
        self.stem == "fw_tg18_jp_v062_2019-09-12_sweets"
    }

    pub fn name(&self) -> &str {
        &self.title
    }

    pub fn seconds(&self) -> f64 {
        self.machine.now()
    }

    pub fn asleep(&self) -> bool {
        self.machine.sys.powered_off || self.machine.alarm_wake
    }

    pub fn care_at(&self) -> f64 {
        self.care.care_at
    }

    pub fn care_adopt(&mut self) {
        crate::care::care_adopt();
    }

    /// Moves the RTC only. Does not move `care_at`. The UI must not call this.
    pub fn advance_clock(&mut self, seconds: f64) {
        self.machine.sys.rtc.base_ticks += (seconds * 32768.0) as i64;
    }

    pub fn care_pause(&mut self, paused: bool, now: f64) {
        let was_paused = self.care.paused;
        self.care.set_paused(paused, now);
        if was_paused && !paused {
            // Resuming puts the toy back on the phone's clock. The paused gap
            // itself is still never replayed.
            self.sync_clock_to_wall();
        }
        if !self.save_dir.as_os_str().is_empty() {
            let _ = self.care.store(&self.save_dir);
        }
    }

    pub fn care_sync(&mut self, now: f64, budget: f64) -> f64 {
        if self.care.paused || self.stopped {
            return 0.0;
        }
        let due = self.care.due(now);
        if due <= 0.001 {
            return 0.0;
        }
        let slice = due.min(budget.max(0.0)).min(CARE_SYNC_BUDGET_S);
        if self.playable {
            if self.run_seconds(slice) < 0 {
                return -1.0;
            }
        }
        self.care.care_at += slice;
        (due - slice).max(0.0)
    }

    pub fn display_awake(&mut self, on: bool) {
        self.awake_hint = on;
        self.machine.never_sleep = on;
    }

    pub fn release_keys(&mut self) {
        self.keys = [false; 3];
        self.machine.sys.live_keys = [false; 3];
    }

    pub fn key(&mut self, index: usize, down: bool) -> Result<(), String> {
        if index > 2 {
            return Err(copy::text(Lang::En, "key_limit").into());
        }
        if self.shelf_covers || self.switching {
            return Ok(());
        }
        if down {
            if !crate::input::accept_down(self.keys[index], true) {
                return Ok(());
            }
            let t = self.machine.now();
            if self.playable && !self.machine.sys.powered_off && t - self.rewind_at >= REWIND_GAP {
                self.rewind = Some(Snapshot::capture(&self.machine));
                self.rewind_at = t;
            }
            self.machine.alarm_wake = false;
            self.machine.sys.live_keys[index] = true;
            self.machine.sys.hold_until[index] = t + self.min_hold;
            self.keys[index] = true;
            if self.machine.sys.powered_off {
                let key = key_of(index);
                self.machine.sys.key_script.retain(|&(at, _)| at > t - 1.0);
                self.machine.press(t, key);
            }
        } else {
            self.machine.sys.live_keys[index] = false;
            self.keys[index] = false;
        }
        Ok(())
    }

    pub fn step(&mut self, dt: f64) -> i32 {
        if self.stopped {
            return -1;
        }
        if self.control_paused || self.switching {
            return 0;
        }
        if !self.playable {
            self.stopped = true;
            self.error = copy::text(Lang::En, "emulator_stopped").into();
            return -1;
        }
        if self.fast_keys && self.is_sweets() {
            if self.machine.sys.powered_off {
                self.awake_since = self.machine.now();
            }
            // The boot and wake path need the original timings: a press that
            // is accepted two samples after power-on is dropped and the toy
            // goes straight back to sleep. Keep the patch off until the toy
            // has been awake for a moment.
            if self.machine.sys.powered_off || self.machine.now() - self.awake_since < KEYPATCH_GRACE {
                keypatch::restore(&mut self.machine);
            } else {
                keypatch::apply(&mut self.machine);
            }
        }
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.run_seconds(dt)
    }

    fn run_seconds(&mut self, seconds: f64) -> i32 {
        if !self.playable {
            self.stopped = true;
            self.error = copy::text(Lang::En, "emulator_stopped").into();
            return -1;
        }
        let limit = self.machine.executed.saturating_add((seconds * CPU_HZ) as u64);
        // A powered-off CPU stays halted. `run` would continue past the
        // power-off instruction, and the LCD would stay on the sleep color
        // because only a cold boot clears `powered_off`.
        let outcome = if self.machine.sys.powered_off {
            self.step_while_off(limit)
        } else {
            self.machine.run_with_sleep(limit)
        };
        self.ingest_pending_audio();
        match outcome {
            Outcome::Limit | Outcome::PowerOff => 0,
            other => self.on_link_stop(other),
        }
    }

    /// Desktop tick while `powered_off`: cold-boot on the next scripted press
    /// or RTC alarm, and do not execute the halted CPU. With the display-awake
    /// hint, a machine that slept anyway is given a B press.
    fn step_while_off(&mut self, limit: u64) -> Outcome {
        let now = self.machine.now();
        if self.machine.never_sleep && !self.machine.sys.key_script.iter().any(|&(at, _)| at >= now) {
            self.machine.press(now, Key::B);
        }
        if !self.machine.power_cycle(limit) {
            self.advance_to_limit(limit);
        } else if !self.machine.sys.powered_off && self.machine.executed < limit {
            // Woken by the RTC alarm or a button: run the firmware for the rest
            // of the slice instead of returning with time still unaccounted.
            let outcome = self.machine.run_with_sleep(limit);
            if outcome == Outcome::PowerOff && self.machine.executed < limit {
                self.advance_to_limit(limit);
                return Outcome::Limit;
            }
            return outcome;
        }
        if self.machine.sys.powered_off {
            Outcome::PowerOff
        } else {
            Outcome::Limit
        }
    }

    fn advance_to_limit(&mut self, limit: u64) {
        let turbo = self.machine.sys.turbo;
        let executed = self.machine.executed;
        self.machine.sys.game_time += (limit.saturating_sub(executed)) as f64 / CPU_HZ * turbo;
        self.machine.executed = limit;
        self.machine.sys.now = limit as f64 / CPU_HZ;
    }

    fn ingest_pending_audio(&mut self) {
        let events = std::mem::take(&mut self.machine.sys.sound.events);
        if self.calls.feed(&events) {
            self.take_call = true;
        }
        let t1 = self.machine.now();
        let mut span = (t1 - self.audio_t).max(0.0);
        if span > 2.0 {
            span = 2.0;
        }
        let n = (span * AUDIO_RATE as f64) as usize;
        if n > 0 {
            self.synth.render(&events, self.audio_t, self.audio_t + span, n, &mut self.pending_pcm);
            self.audio_t = t1;
            self.cap_pcm();
        }
    }

    fn cap_pcm(&mut self) {
        let cap = AUDIO_RATE as usize * 2;
        if self.pending_pcm.len() > cap {
            let drop_n = self.pending_pcm.len() - cap;
            self.pending_pcm.drain(..drop_n);
        }
    }

    /// Visible ticks play firmware PCM. Mute is volume 0 and still returns samples.
    fn take_pcm(&mut self, play: bool) -> Vec<i16> {
        let pcm = std::mem::take(&mut self.pending_pcm);
        if play { pcm } else { Vec::new() }
    }

    /// Feed buzzer events without running the CPU. Used so mute tests can
    /// see `takeCall` from the firmware tune shape.
    pub fn ingest_sound(&mut self, events: &[(f64, f64, f64)]) {
        if self.calls.feed(events) {
            self.take_call = true;
        }
        let t0 = events.first().map(|e| e.0).unwrap_or(self.audio_t);
        let t1 = events.last().map(|e| e.0).unwrap_or(t0);
        let n = ((t1 - t0).max(0.0) * AUDIO_RATE as f64) as usize;
        self.synth.render(events, t0, t1.max(t0), n.max(1).min(AUDIO_RATE as usize * 2), &mut self.pending_pcm);
        self.cap_pcm();
    }

    pub fn on_link_stop(&mut self, what: Outcome) -> i32 {
        self.release_keys();
        match what {
            Outcome::BleFail => {
                self.stopped = true;
                self.error = copy::text(Lang::En, "link_stopped_firmware").into();
                -1
            }
            Outcome::Bluetooth | Outcome::Infrared => {
                let hosted = self.link.classified && link::step_code(true, self.link.web, self.link.kind) == 2;
                if hosted {
                    self.keep_recovery();
                    2
                } else {
                    self.notice = copy::text(Lang::En, "unsupported_link").into();
                    self.error = copy::text(Lang::En, "link_native").into();
                    self.restore_rewind();
                    self.link.cancel();
                    1
                }
            }
            Outcome::Crash | Outcome::Stuck => {
                self.stopped = true;
                let fault = self.machine.fault().map(|f| format!("{f:?}")).unwrap_or_default();
                self.error = format!("{} {fault}", copy::text(Lang::En, "emulator_stopped"));
                -1
            }
            Outcome::Limit | Outcome::PowerOff => 0,
        }
    }

    fn keep_recovery(&mut self) {
        if self.save_dir.as_os_str().is_empty() {
            if self.rewind.is_none() {
                self.error = copy::text(Lang::En, "no_recovery").into();
            }
            return;
        }
        match &self.rewind {
            Some(snap) => {
                let path = self.save_dir.join("mixing-return.t18s");
                if let Some(s) = path.to_str() {
                    let _ = snap.save(s);
                }
                let _ = fs::write(self.save_dir.join("link-pending"), b"1");
            }
            None => self.error = copy::text(Lang::En, "no_recovery").into(),
        }
    }

    fn restore_rewind(&mut self) {
        let Some(snap) = self.rewind.take() else {
            self.error = copy::text(Lang::En, "no_recovery").into();
            return;
        };
        let image = self.image.clone();
        let mut machine = Machine::new(image.clone(), &image, 0.0);
        if snap.restore(&mut machine).is_ok() {
            machine.stop_on_bluetooth = true;
            machine.never_sleep = self.awake_hint;
            self.machine = machine;
        }
    }

    pub fn route(&self) -> Route {
        if !self.link.classified {
            Route::Unsupported
        } else {
            link::route(self.link.web, self.link.kind)
        }
    }

    pub fn frame(&mut self, out: &mut [u32]) {
        if out.len() < 16_384 {
            return;
        }
        if self.asleep() {
            out[..16_384].fill(0xFF29_2B2A);
            return;
        }
        self.machine.sys.lcd.argb(&mut out[..16_384]);
        for px in &mut out[..16_384] {
            *px |= 0xFF00_0000;
        }
    }

    pub fn pull_audio(&mut self) -> Vec<i16> {
        Vec::new()
    }

    pub fn save(&mut self) -> Result<(), String> {
        if self.stopped {
            return Err(copy::text(Lang::En, "export_last_good").into());
        }
        if self.save_dir.as_os_str().is_empty() {
            return Err("no save directory".into());
        }
        fs::create_dir_all(&self.save_dir).map_err(|e| e.to_string())?;
        let auto = self.save_dir.join("autosave.t18s");
        Snapshot::capture(&self.machine).save(auto.to_str().unwrap_or("")).map_err(|e| e.to_string())?;
        let flash = self.save_dir.join("game.flash");
        save::write_machine(&self.machine, flash.to_str().unwrap_or(""), false, self.asleep()).map_err(|e| e.to_string())?;
        self.care.store(&self.save_dir).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn export_save(&mut self, path: &Path) -> Result<(), String> {
        if self.stopped {
            let auto = self.save_dir.join("autosave.t18s");
            if auto.exists() {
                fs::copy(&auto, path).map_err(|e| e.to_string())?;
                return Ok(());
            }
            return Err(copy::text(Lang::En, "export_last_good").into());
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Snapshot::capture(&self.machine).save(path.to_str().unwrap_or("")).map_err(|e| e.to_string())
    }

    pub fn import_save(&mut self, path: &Path) -> Result<(), String> {
        let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if !library::import_len_ok(len) {
            return Err(format!("{} {}", copy::text(Lang::En, "import_failed"), "file is larger than 32 MiB"));
        }
        if self.stopped {
            return Err(format!("{} previous save was preserved", copy::text(Lang::En, "import_failed")));
        }
        let bytes = fs::read(path).map_err(|e| format!("{} {e}", copy::text(Lang::En, "import_failed")))?;
        if !self.save_dir.as_os_str().is_empty() {
            let _ = fs::create_dir_all(&self.save_dir);
            let backup = self.save_dir.join("before_import.t18s");
            let _ = Snapshot::capture(&self.machine).save(backup.to_str().unwrap_or(""));
        }
        let snap = match Snapshot::from_bytes(&bytes) {
            Ok(s) => s,
            Err(e) => return Err(format!("{} {e}", copy::text(Lang::En, "import_failed"))),
        };
        let image = self.image.clone();
        let mut fresh = Machine::new(image.clone(), &image, 0.0);
        match snap.restore(&mut fresh) {
            Ok(()) => {
                fresh.stop_on_bluetooth = true;
                self.machine = fresh;
                self.link.enable(true);
                Ok(())
            }
            Err(SnapError::Mismatch) => Err(format!(
                "{} {} {}",
                copy::text(Lang::En, "import_failed"),
                copy::text(Lang::En, "different_rom"),
                copy::text(Lang::En, "choose_matching_save")
            )),
            Err(e) => Err(format!("{} {e}", copy::text(Lang::En, "import_failed"))),
        }
    }

    pub fn delete_current_save(&mut self) -> Result<(), String> {
        if self.switching || link::blocks_edit(self.link.state) {
            return Err(copy::text(Lang::En, "mixing_busy").into());
        }
        self.release_keys();
        let saves = self.save_dir.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let stem = self.stem.clone();
        library::remove_stem_saves(&saves, &stem).map_err(|e| e.to_string())?;
        let image = self.image.clone();
        self.machine = Machine::new(image.clone(), &image, tg18::rtc_seconds_now());
        self.machine.stop_on_bluetooth = true;
        self.stopped = false;
        self.link = Link::default();
        self.link.enable(true);
        self.catching_up = false;
        self.catchup_total = 0.0;
        self.take_call = false;
        self.care = CareTimeline::at(tg18::unix_now());
        if !self.save_dir.as_os_str().is_empty() {
            let _ = fs::create_dir_all(&self.save_dir);
            let _ = self.care.store(&self.save_dir);
        }
        Ok(())
    }

    pub fn pump(&mut self, inp: &PumpIn) -> PumpOut {
        let mut out = PumpOut {
            stopped: self.stopped,
            error: self.error.clone(),
            notice: String::new(),
            step_code: 0,
            route: 0,
            take_call: false,
            play_audio: false,
            volume: if inp.muted { 0.0 } else { 1.0 },
            drew_frame: false,
            did_save: false,
            catching_up: false,
            tick: !self.care.paused && !self.stopped,
            next_ms: tick::deadline_ms(inp.visible, self.link.state),
            backlog: inp.backlog,
            remain: 0.0,
            catchup_total: 0.0,
            last_frame_ns: inp.last_frame_ns,
            last_save_ms: inp.last_save_ms,
            last_catchup_save_ms: inp.last_catchup_save_ms,
            pcm: Vec::new(),
        };
        if self.care.paused || self.stopped || self.control_paused {
            out.tick = !self.care.paused && !self.stopped && !self.control_paused;
            self.draw_if_due(inp, &mut out);
            return out;
        }
        self.take_call = false;
        let catch_up = tick::wants_catch_up(inp.dt, inp.now_unix, self.care.care_at, self.care.paused);
        if catch_up {
            self.catching_up = true;
            self.display_awake(false);
            if self.catchup_total <= 0.0 {
                self.catchup_total = self.care.due(inp.now_unix);
            }
            let remain = self.care_sync(inp.now_unix, CARE_SYNC_BUDGET_S);
            out.remain = remain;
            out.catchup_total = self.catchup_total;
            if remain < 0.0 {
                self.stopped = true;
                out.stopped = true;
                out.error = self.error.clone();
                out.tick = false;
                self.pending_pcm.clear();
                return out;
            }
            if remain > 0.001 {
                out.catching_up = true;
                out.next_ms = 1;
                self.pending_pcm.clear();
                if inp.monotonic_ms.saturating_sub(inp.last_catchup_save_ms) >= tick::CATCHUP_SAVE_MS {
                    if self.save().is_ok() {
                        out.did_save = true;
                        out.last_catchup_save_ms = inp.monotonic_ms;
                        out.last_save_ms = inp.monotonic_ms;
                    }
                }
                return out;
            }
            let _ = self.save();
            out.did_save = true;
            out.last_save_ms = inp.monotonic_ms;
            out.last_catchup_save_ms = inp.monotonic_ms;
            self.catching_up = false;
            self.catchup_total = 0.0;
            out.catchup_total = 0.0;
            out.remain = 0.0;
        }
        let add = tick::slice_cap(inp.dt);
        out.backlog += add;
        let mut simulated = 0.0;
        while out.backlog > tick::BACKLOG_EPSILON_S {
            let q = tick::quantum(inp.visible).min(out.backlog);
            let code = self.step(q);
            simulated += q;
            out.backlog -= q;
            out.step_code = code;
            if code < 0 {
                out.stopped = true;
                out.error = format!("{} {}", copy::text(Lang::En, "emulator_stopped"), self.error);
                out.tick = false;
                self.pending_pcm.clear();
                break;
            }
            if code == 1 {
                out.notice = copy::text(Lang::En, "unsupported_link").into();
                out.route = 0;
            }
            if code == 2 {
                out.route = match self.route() {
                    Route::WebMeets => 2,
                    Route::Mixing => 3,
                    Route::Unsupported => 1,
                };
            }
            if q <= 0.0 {
                break;
            }
        }
        self.care.care_at += simulated;
        if !self.catching_up
            && !self.care.paused
            && inp.monotonic_ms.saturating_sub(self.last_clock_sync_ms) >= 1_000
        {
            self.sync_clock_to_wall();
            self.last_clock_sync_ms = inp.monotonic_ms;
        }
        out.take_call = self.take_call;
        out.play_audio = inp.visible;
        out.pcm = self.take_pcm(inp.visible);
        let hint = tick::display_hint(
            inp.visible,
            !self.stopped,
            !self.catching_up,
            self.care.paused,
            inp.follow_system_sleep,
            inp.phone_interactive,
        );
        self.display_awake(hint);
        self.draw_if_due(inp, &mut out);
        if inp.monotonic_ms.saturating_sub(out.last_save_ms) >= tick::AUTOSAVE_MS && !self.save_dir.as_os_str().is_empty() {
            if self.save().is_ok() {
                out.did_save = true;
                out.last_save_ms = inp.monotonic_ms;
            }
        }
        out.next_ms = tick::deadline_ms(inp.visible, self.link.state);
        out.catching_up = self.catching_up;
        out
    }

    fn draw_if_due(&mut self, inp: &PumpIn, out: &mut PumpOut) {
        if !inp.visible || inp.frame_ns.saturating_sub(inp.last_frame_ns) < tick::FRAME_NS {
            return;
        }
        // Only hand over a frame the game actually changed. The old code pulled
        // every 33 ms even on a still screen, which delayed changes and did
        // needless work.
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for &b in &self.machine.sys.lcd.fb {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        if hash == self.last_drawn_hash {
            return;
        }
        self.last_drawn_hash = hash;
        out.drew_frame = true;
        out.last_frame_ns = inp.frame_ns;
    }

    pub fn link_connect(&mut self) {
        self.link.connect();
    }

    pub fn link_cancel(&mut self) {
        if self.link.state != 0 && self.link.state != 5 {
            self.restore_rewind();
        }
        self.link.cancel();
    }

    pub fn link_write(&mut self, bytes: &[u8]) -> bool {
        self.link.write(bytes).is_ok()
    }

    pub fn link_read(&mut self, out: &mut [u8]) -> i32 {
        self.link.read(out)
    }

    /// Feed bytes from an app-side infrared transport into the firmware.
    pub fn ir_write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.machine.sys.uart_receive(b);
        }
    }

    /// Drain firmware infrared output for an app-side transport.
    pub fn ir_read(&mut self, out: &mut [u8]) -> i32 {
        if self.ir_pending.is_empty() {
            self.ir_pending.extend(self.machine.sys.uart_take());
        }
        let n = out.len().min(self.ir_pending.len());
        for slot in out.iter_mut().take(n) {
            *slot = self.ir_pending.pop_front().unwrap();
        }
        n as i32
    }

    pub fn refuse_shelf(&self) -> Result<(), String> {
        if let Some(msg) = shelf::refuse(self.link.state, self.catching_up, self.care.paused) {
            Err(msg.into())
        } else {
            Ok(())
        }
    }

    pub fn apply_edits(&mut self, edits: &[Edit]) -> Result<(), String> {
        self.refuse_shelf()?;
        self.release_keys();
        let pending = if self.root.as_os_str().is_empty() {
            std::env::temp_dir().join("before-shelf.t18s.pending")
        } else {
            Container::new(&self.root).pending_shelf()
        };
        let final_path = if self.root.as_os_str().is_empty() {
            std::env::temp_dir().join("before-shelf.t18s")
        } else {
            Container::new(&self.root).final_shelf()
        };
        let snapshot = Snapshot::capture(&self.machine).to_bytes();
        let result = shelf::transact(&pending, &final_path, &snapshot, || {
            self.control_paused = true;
            for edit in edits {
                match edit {
                    Edit::Write(w) => self.poke_checked(w.address, &w.bytes)?,
                    Edit::Register { index, value } => {
                        if *index > 15 {
                            return Err(copy::text(Lang::En, "key_limit").into());
                        }
                        self.machine.cpu.r[*index as usize] = *value;
                    }
                    Edit::Cpsr(v) => self.machine.cpu.write_cpsr(*v),
                }
            }
            self.control_paused = false;
            if !self.save_dir.as_os_str().is_empty() {
                self.save()?;
            }
            Ok(())
        });
        if result.is_err() {
            if pending.exists() {
                let _ = self.import_save(&pending);
            } else if final_path.exists() && snapshot_failed_midway(&result) {
                let _ = self.import_save(&final_path);
            }
            self.control_paused = false;
            if !self.save_dir.as_os_str().is_empty() {
                let _ = self.save();
            }
        }
        result
    }

    fn poke_checked(&mut self, address: u32, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() {
            return Ok(());
        }
        if !span_ok(address, bytes.len()) {
            return Err("address is outside RAM and SRAM".into());
        }
        self.machine.sys.poke(address, bytes);
        Ok(())
    }

    pub fn send_tamago(&mut self, bytes: &[u8]) -> Result<String, String> {
        self.refuse_shelf()?;
        let parsed = crate::shelf::tamago::parse(bytes)?;
        let sigs = shelf::locate_firmware(&self.image).ok();
        if parsed.cat.occupancy() && sigs.is_none() {
            return Err(copy::text(Lang::En, "sigs_unavailable").into());
        }
        let parameter = sigs.map(|s| s.parameter);
        let (slot, writes) = {
            let ram = self.machine.sys.ram.clone();
            let sram = self.machine.sys.sram.clone();
            crate::shelf::slots::place(&ram, &sram, &parsed, parameter)?
        };
        let edits: Vec<Edit> = writes.into_iter().map(Edit::Write).collect();
        self.apply_edits(&edits)?;
        let _ = slot;
        Ok(crate::shelf::tamago::send_sentence(parsed.cat).into())
    }

    pub fn set_balance(&mut self, add: bool, amount: i64) -> Result<u32, String> {
        let Some(sigs) = shelf::locate_firmware(&self.image).ok() else {
            return Err(copy::text(Lang::En, "sigs_unavailable").into());
        };
        let off = sigs.parameter as usize + 32;
        if off + 4 > self.machine.sys.sram.len() {
            return Err(copy::text(Lang::En, "balance_range").into());
        }
        let current = u32::from_le_bytes(self.machine.sys.sram[off..off + 4].try_into().unwrap());
        let word = crate::shelf::slots::balance_word(current, add, amount).map_err(|e| e.to_string())?;
        let bytes = word.to_le_bytes();
        self.apply_edits(&[Edit::Write(crate::shelf::slots::MemWrite {
            address: crate::shelf::slots::SRAM_BASE + off as u32,
            bytes: bytes.to_vec(),
        })])?;
        Ok(word)
    }

    pub fn marry(&mut self, choices: &shelf::Choices) -> Result<shelf::MarriagePlan, String> {
        self.refuse_shelf()?;
        let sigs = shelf::locate_firmware(&self.image).ok();
        let plan = shelf::marriage::marriage_plan(&self.machine.sys.sram, &cpu_bytes(&self.machine), sigs, choices)?;
        self.apply_edits(&plan.edits)?;
        Ok(plan)
    }

    pub fn undo_shelf(&mut self) -> Result<(), String> {
        shelf::undo_allowed(&self.title).map_err(|e| e.to_string())?;
        let path = if self.root.as_os_str().is_empty() {
            return Err(copy::text(Lang::En, "import_failed").into());
        } else {
            Container::new(&self.root).final_shelf()
        };
        self.import_save(&path)
    }

    /// Switch to an already-staged image. A failed save leaves `firmware-v1.bin` alone.
    pub fn switch_to_image(&mut self, bytes: &[u8], saves: &Path) -> Result<String, String> {
        if link::blocks_edit(self.link.state) {
            return Err(copy::text(Lang::En, "mixing_busy").into());
        }
        if self.switching {
            return Err(copy::text(Lang::En, "switch_kept_prefix").into());
        }
        self.switching = true;
        self.release_keys();
        if let Err(e) = self.save() {
            self.switching = false;
            return Err(format!("{} {e}", copy::text(Lang::En, "switch_save_prefix")));
        }
        let class = rom::classify(bytes);
        let (stem, title) = match class {
            ImageClass::Supported { stem, title, .. } => (stem, title),
            _ => {
                self.switching = false;
                return Err(format!(
                    "{} {}",
                    copy::text(Lang::En, "switch_kept_prefix"),
                    copy::text(Lang::En, "choose_rom")
                ));
            }
        };
        let old_title = self.title.clone();
        let old_dir = self.save_dir.clone();
        if crate::care::outgoing_should_pause(&old_title, &title) && !old_dir.as_os_str().is_empty() {
            let _ = crate::care::pause_folder(&old_dir, tg18::unix_now());
        }
        if !self.root.as_os_str().is_empty() {
            if let Err(e) = library::retain_current(&self.root) {
                self.switching = false;
                return Err(format!("{} {e}", copy::text(Lang::En, "switch_kept_prefix")));
            }
            if let Err(e) = library::write_atomic(&self.root.join("firmware-v1.bin"), bytes) {
                self.switching = false;
                return Err(format!("{} {e}", copy::text(Lang::En, "switch_kept_prefix")));
            }
            let _ = library::write_atomic(&self.root.join("ROMs").join(format!("{stem}.bin")), bytes);
        }
        match Self::open_care_bytes(bytes, saves, self.care.paused) {
            Ok(mut fresh) => {
                fresh.root = self.root.clone();
                *self = fresh;
                self.switching = false;
                Ok(format!("{}{stem}", copy::text(Lang::En, "loaded_prefix")))
            }
            Err(e) => {
                self.switching = false;
                Err(format!("{} {e}", copy::text(Lang::En, "switch_kept_prefix")))
            }
        }
    }

    fn open_care_bytes(bytes: &[u8], saves: &Path, paused: bool) -> Result<Self, String> {
        let (stem, title) = match rom::classify(bytes) {
            ImageClass::Supported { stem, title, .. } => (stem, title),
            ImageClass::NotImage => return Err(copy::text(Lang::En, "err_not_8mib").into()),
            ImageClass::Unsupported => return Err(copy::text(Lang::En, "err_unsupported").into()),
        };
        let image = Arc::new(bytes.to_vec());
        let mut machine = Machine::new(image.clone(), &image, tg18::rtc_seconds_now());
        machine.stop_on_bluetooth = true;
        let mut session = Self::from_parts(machine, image, stem, title, true);
        session.save_dir = saves.join(&session.stem);
        let _ = fs::create_dir_all(&session.save_dir);
        let now = tg18::unix_now();
        session.care = CareTimeline::load(&session.save_dir);
        if !session.save_dir.join("care.json").exists() {
            session.care = CareTimeline::at(now);
        }
        if paused {
            session.care.set_paused(true, now);
        } else if session.care.paused {
            session.care.set_paused(false, now);
        }
        session.align_clock_for_open(now);
        let _ = session.care.store(&session.save_dir);
        session.link.enable(true);
        Ok(session)
    }
}

fn snapshot_failed_midway(result: &Result<(), String>) -> bool {
    result.is_err()
}

fn cpu_bytes(m: &Machine) -> Vec<u8> {
    let mut out = Vec::with_capacity(180);
    for w in m.cpu.state() {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

fn key_of(index: usize) -> Key {
    match index {
        0 => Key::A,
        1 => Key::B,
        _ => Key::C,
    }
}

pub fn span_ok(address: u32, len: usize) -> bool {
    let end = address as u64 + len as u64;
    let ram = (address as u64) >= 0x2000_0000 && end <= 0x2000_0000 + 0x100_0000;
    let sram = (address as u64) >= 0xF800_0000 && end <= 0xF800_0000 + 0x10_0000;
    ram || sram
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_tune() -> Vec<(f64, f64, f64)> {
        let mut ev = Vec::new();
        let mut t = 0.0;
        for (f, len, gap) in [
            (2793.0, 0.064, 0.128),
            (2350.0, 0.256, 0.192),
            (2793.0, 0.064, 0.032),
            (3136.0, 0.064, 0.032),
            (4187.0, 0.128, 0.0),
        ] {
            ev.push((t, f, 0.5));
            t += len;
            ev.push((t, 0.0, 0.5));
            t += gap;
        }
        ev
    }

    #[test]
    fn keys_frames_pause_and_unsupported_link_restores() {
        let mut s = Session::bare(vec![1, 2, 3, 4]);
        s.shelf_covers = true;
        s.key(0, true).unwrap();
        assert!(!s.keys[0]);
        s.shelf_covers = false;
        s.key(0, true).unwrap();
        s.key(0, true).unwrap();
        s.key(2, true).unwrap();
        assert!(s.keys[0] && s.keys[2]);
        assert!(s.machine.sys.live_keys[0] && s.machine.sys.live_keys[2]);
        s.release_keys();
        assert!(!s.keys[0] && !s.keys[2]);

        s.machine.sys.powered_off = true;
        let mut frame = vec![0u32; 16384];
        s.frame(&mut frame);
        assert!(frame.iter().all(|p| *p == 0xFF29_2B2A));
        s.key(1, true).unwrap();
        assert!(!s.machine.alarm_wake);

        s.care.set_paused(true, 10.0);
        let mut inp = PumpIn { now_unix: 10_000.0, dt: 100.0, visible: true, ..PumpIn::default() };
        let out = s.pump(&inp);
        assert!(!out.catching_up);
        assert_eq!(s.machine.executed, 0);
        assert!(s.care.due(10_000.0) == 0.0);

        s.care.set_paused(false, 10_000.0);
        s.stopped = false;
        s.playable = false;
        inp.dt = 0.02;
        inp.now_unix = 10_000.0;
        let out = s.pump(&inp);
        assert!(out.stopped);
        assert!(out.error.contains("emulator stopped"));
        assert_eq!(s.machine.executed, 0);

        let mut s = Session::bare(vec![9, 9, 9, 9]);
        s.machine.sys.ram[0] = 0x11;
        s.rewind = Some(Snapshot::capture(&s.machine));
        s.machine.sys.ram[0] = 0x22;
        let code = s.on_link_stop(Outcome::Bluetooth);
        assert_eq!(code, 1);
        assert_eq!(s.machine.sys.ram[0], 0x11);
        assert_eq!(s.link.state, 0);
        assert!(!link::opens_ble(s.route()));
        assert!(s.notice.contains("returned to the previous screen"));

        let mut s = Session::bare(vec![3, 3, 3, 3]);
        s.link.classified = true;
        s.link.kind = 20;
        s.machine.sys.ram[0] = 0x33;
        s.rewind = Some(Snapshot::capture(&s.machine));
        s.machine.sys.ram[0] = 0x44;
        let root = crate::tmpdir("link");
        s.save_dir = root.join("Saves/bare");
        fs::create_dir_all(&s.save_dir).unwrap();
        assert_eq!(s.on_link_stop(Outcome::Bluetooth), 2);
        assert_eq!(s.machine.sys.ram[0], 0x44);
        assert!(s.save_dir.join("mixing-return.t18s").exists());
        assert!(link::opens_mixing(link::route(false, 20)));
        assert!(!link::opens_ble(link::route(false, 20)));
        let _ = fs::remove_dir_all(&root);

        s.pending_pcm.clear();
        s.ingest_sound(&[(0.0, 440.0, 0.5), (0.05, 0.0, 0.5)]);
        assert!(!s.pending_pcm.is_empty());
        let heard = s.pump(&PumpIn { visible: true, dt: 0.0, now_unix: 0.0, ..PumpIn::default() });
        assert!(heard.play_audio);
        assert!(!heard.pcm.is_empty());
        assert_eq!(heard.volume, 1.0);
        s.ingest_sound(&call_tune());
        assert!(s.take_call);
        let muted = PumpIn { muted: true, visible: false, ..PumpIn::default() };
        assert_eq!(if muted.muted { 0.0 } else { 1.0 }, 0.0);
        assert!(s.take_call);
    }

    #[test]
    fn ir_bytes_roundtrip() {
        let mut s = Session::bare(vec![0; 8 * 1024 * 1024]);
        s.ir_pending.extend([1, 2, 3, 4]);
        let mut out = [0u8; 8];
        let n = s.ir_read(&mut out);
        assert_eq!(&out[..n as usize], &[1, 2, 3, 4]);
    }

    #[test]
    fn save_import_mismatch_keeps_the_other_rom() {
        let root = crate::tmpdir("saves");
        let saves = root.join("Saves");
        fs::create_dir_all(saves.join("other")).unwrap();
        fs::write(saves.join("other/autosave.t18s"), b"other-rom").unwrap();

        let mut a = Session::bare(vec![1, 2, 3, 4]);
        a.root = root.clone();
        a.stem = "stem-a".into();
        a.save_dir = saves.join("stem-a");
        a.playable = true;
        a.machine.sys.ram[16] = 0xAB;
        a.save().unwrap();
        let exported = root.join("cache/outgoing.t18s");
        a.export_save(&exported).unwrap();

        let mut b = Session::bare(vec![5, 6, 7, 8]);
        b.root = root.clone();
        b.stem = "stem-b".into();
        b.save_dir = saves.join("stem-b");
        b.playable = true;
        b.machine.sys.ram[16] = 0x10;
        let err = b.import_save(&exported).unwrap_err();
        assert!(err.contains("it belongs to a different ROM version"));
        assert!(err.contains("choose a .t18s that matches the current ROM version."));
        assert_eq!(b.machine.sys.ram[16], 0x10);
        assert!(b.save_dir.join("before_import.t18s").exists());
        assert_eq!(fs::read(saves.join("other/autosave.t18s")).unwrap(), b"other-rom");

        b.machine.sys.ram[16] = 0x10;
        let mut same = Session::bare(vec![5, 6, 7, 8]);
        same.save_dir = saves.join("stem-b");
        same.playable = true;
        same.machine.sys.ram[16] = 0x77;
        let own = root.join("cache/own.t18s");
        same.export_save(&own).unwrap();
        b.import_save(&own).unwrap();
        assert_eq!(b.machine.sys.ram[16], 0x77);

        b.delete_current_save().unwrap();
        assert!(!saves.join("stem-b/autosave.t18s").exists());
        assert_eq!(fs::read(saves.join("other/autosave.t18s")).unwrap(), b"other-rom");

        let mut blocked = Session::bare(b"KEEP".to_vec());
        blocked.root = root.clone();
        blocked.save_dir = root.join("not-a-dir/stem");
        fs::write(root.join("not-a-dir"), b"x").unwrap();
        fs::write(root.join("firmware-v1.bin"), b"KEEP").unwrap();
        let err = blocked.switch_to_image(&[0u8; 8], &saves).unwrap_err();
        assert!(err.contains("current progress could not be saved; switch cancelled"));
        assert_eq!(fs::read(root.join("firmware-v1.bin")).unwrap(), b"KEEP");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn shelf_send_backs_up_and_sanrio_gates_undo() {
        let root = crate::tmpdir("shelf-send");
        let mut s = Session::bare(vec![1, 1, 1, 1]);
        s.root = root.clone();
        s.save_dir = root.join("Saves/bare");
        fs::create_dir_all(&s.save_dir).unwrap();
        fs::create_dir_all(root.join("Saves/other")).unwrap();
        fs::write(root.join("Saves/other/autosave.t18s"), b"safe").unwrap();
        let base = crate::shelf::slots::table(crate::shelf::tamago::Cat::Meal).base;
        for b in &mut s.machine.sys.ram[base..base + 2048] {
            *b = 0xFF;
        }
        let meal = crate::shelf::tamago::build(-2_130_640_383, 42, 7, 516, &[]);
        let msg = s.send_tamago(&meal).unwrap();
        assert_eq!(msg, "sent to the restaurant menu to be bought");
        assert_eq!(s.machine.sys.ram[base], b'T');
        assert!(root.join("ShelfBackups/before-shelf.t18s").exists());
        assert!(!root.join("ShelfBackups/before-shelf.t18s.pending").exists());
        assert_eq!(fs::read(root.join("Saves/other/autosave.t18s")).unwrap(), b"safe");

        let err = s.undo_shelf().unwrap_err();
        assert_eq!(err, "Switch to Sanrio first");
        s.title = "Sanrio (JP v056)".into();
        // The backup is a real snapshot of this same image, so undo loads it.
        s.undo_shelf().unwrap();
        assert_eq!(s.machine.sys.ram[base], 0xFF);

        s.care.set_paused(true, 1.0);
        let err = s.send_tamago(&meal).unwrap_err();
        assert_eq!(err, "resume your pet first");
        s.care.set_paused(false, 2.0);
        s.catching_up = true;
        assert_eq!(s.refuse_shelf().unwrap_err(), "Shelf blocked.");
        s.catching_up = false;
        s.link.state = 3;
        assert_eq!(s.send_tamago(&meal).unwrap_err(), "finish or cancel Mixing Meets first");
        s.link.state = 0;

        let err = s.set_balance(false, 10).unwrap_err();
        assert!(err.contains("shelf signatures"));
        let mut choices = crate::shelf::Choices {
            parts: [0, 0, 0, 0],
            pixels: [None, None, None, None],
            colors: vec![1],
            sex: 0,
            mode: 1,
            dna_n: 0,
            random_sex: 0,
            random_dna: [0; 8],
        };
        choices.sex = 0;
        let err = s.marry(&choices).unwrap_err();
        assert_eq!(err, "Pet growth state is unavailable.");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tap_holds_briefly_and_wakes_a_powered_off_machine() {
        let mut s = Session::bare(vec![1, 2, 3, 4]);
        s.playable = true;
        s.key(1, true).unwrap();
        s.key(1, false).unwrap();
        assert!(!s.keys[1]);
        assert!(s.machine.sys.hold_until[1] >= s.machine.now() + MIN_HOLD - 1e-9);
        assert!(s.machine.sys.key_script.is_empty());

        s.machine.sys.powered_off = true;
        let asleep_at = s.machine.executed;
        assert_eq!(s.step(0.05), 0);
        assert!(s.machine.sys.powered_off);
        assert!(s.machine.executed > asleep_at);

        s.machine.alarm_wake = true;
        s.key(1, true).unwrap();
        s.key(1, false).unwrap();
        assert!(!s.keys[1]);
        assert!(!s.machine.alarm_wake);
        assert_eq!(s.machine.sys.key_script.len(), 1);
        assert_eq!(s.step(0.05), 0);
        assert!(!s.machine.sys.powered_off);
        assert!(!s.asleep());
    }

    #[test]
    fn tap_wakes_a_sleeping_fairy_if_rom() {
        let Some(path) = std::env::var_os("POCKET_MEETS_ROM") else {
            eprintln!("skip: POCKET_MEETS_ROM is not set");
            return;
        };
        let root = crate::tmpdir("wake");
        let mut s = Session::open(Path::new(&path), &root.join("Saves")).expect("open");
        let mut emu = 0.0;
        while emu < 600.0 {
            let code = s.step(0.05);
            assert!(code >= 0, "step failed at {emu}s: {}", s.error);
            emu += 0.05;
            if s.asleep() {
                break;
            }
        }
        assert!(
            s.asleep(),
            "firmware did not sleep within {emu}s powered_off={} alarm_wake={}",
            s.machine.sys.powered_off, s.machine.alarm_wake
        );
        // A simulator tap: down and up before the next quantum.
        s.key(1, true).unwrap();
        s.key(1, false).unwrap();
        assert!(!s.keys[1]);
        let mut woke = false;
        for _ in 0..40 {
            let code = s.step(0.05);
            assert!(code >= 0, "wake step failed: {}", s.error);
            if !s.asleep() {
                woke = true;
                break;
            }
        }
        assert!(
            woke,
            "B tap left the toy asleep powered_off={} alarm_wake={}",
            s.machine.sys.powered_off, s.machine.alarm_wake
        );
        for _ in 0..20 {
            let code = s.step(0.05);
            assert!(code >= 0, "post-wake step failed: {}", s.error);
        }
        assert!(!s.asleep(), "toy went back to sleep within a second of the tap");
        let mut frame = vec![0u32; 16384];
        s.frame(&mut frame);
        assert!(frame.iter().any(|p| *p != 0xFF29_2B2A));
        let _ = fs::remove_dir_all(&root);
    }
    #[test]
    fn paused_care_leaves_the_toy_clock_alone() {
        let now = tg18::unix_now();
        let mut s = Session::bare(vec![0u8; 0x1000]);
        s.machine.sys.rtc.base_ticks = 1_000_000;
        s.machine.sys.rtc.base_time = s.machine.sys.game_time;
        s.care = CareTimeline::at(now);
        s.care.set_paused(true, now);

        s.align_clock_for_open(now);
        assert_eq!(s.machine.sys.rtc.ticks(s.machine.sys.game_time), 1_000_000);

        // Resuming lines the clock back up with the phone.
        s.care.set_paused(false, now);
        s.align_clock_for_open(now);
        let seconds = s.machine.sys.rtc.ticks(s.machine.sys.game_time) as f64 / 32768.0;
        assert!((seconds - tg18::rtc_seconds_now()).abs() < 2.0);

        // Toggling pause off mid-session snaps the clock back at once, and the
        // gap is still not replayed.
        s.machine.sys.rtc.base_ticks = 1_000_000;
        s.care.set_paused(true, now);
        s.care_pause(false, now);
        let seconds = s.machine.sys.rtc.ticks(s.machine.sys.game_time) as f64 / 32768.0;
        assert!((seconds - tg18::rtc_seconds_now()).abs() < 2.0);
        assert!(s.care.due(tg18::unix_now()) < 2.0);
    }

    #[test]
    fn only_the_sweets_dump_arms_the_fast_key_patch() {
        let mut s = Session::bare(vec![0u8; 0x1000]);
        assert!(!s.is_sweets());
        s.set_fast_keys(true);
        assert!(!s.fast_keys);
        assert!((s.min_hold - MIN_HOLD).abs() < 1e-9);

        s.stem = "fw_tg18_jp_v062_2019-09-12_sweets".into();
        s.set_fast_keys(true);
        assert!(s.fast_keys);
        assert!((s.min_hold - MIN_HOLD_FAST).abs() < 1e-9);
    }

}
