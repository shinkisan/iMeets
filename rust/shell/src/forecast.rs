//! Offline call forecast. Codes: 0 budget, 1 event, 2 horizon, negative stop.

use std::fs;
use std::path::Path;

pub const HORIZON_S: f64 = 86_400.0;
pub const WALL_BUDGET_MS: u64 = 4_000;
pub const RUN_BUDGET: i32 = 200;
pub const RETRY_NONE_MS: i64 = 30_000;
pub const RETRY_HORIZON_MS: i64 = 3_600_000;
pub const RETRY_THROWN_MS: i64 = 60_000;
pub const AGAIN_MS: i64 = 50;

#[derive(Clone, Debug)]
pub struct Decision {
    pub event_ms: Option<i64>,
    pub retry_in_ms: Option<i64>,
    pub again_in_ms: Option<i64>,
    pub clear_retry: bool,
    pub error: Option<&'static str>,
}

impl Decision {
    fn empty() -> Self {
        Self { event_ms: None, retry_in_ms: None, again_in_ms: None, clear_retry: false, error: None }
    }
}

pub fn seed_not_before(now_unix: f64, last_call_ms: i64) -> f64 {
    let floor = last_call_ms as f64 / 1000.0 + 300.0;
    if now_unix > floor { now_unix } else { floor }
}

pub fn reached_horizon(forecast_at: f64, now_unix: f64) -> bool {
    forecast_at >= now_unix + HORIZON_S
}

/// `code` is what `forecastRun` returned. Unknown positive codes defer
/// like 0. TODO(forecast-other-code): only 0, 1, 2, and negative are named.
pub fn decide(code: i32, event_unix: f64, visible_or_resident: bool) -> Decision {
    let stopped = crate::copy::text(crate::copy::Lang::En, "forecast_stopped");
    match code {
        1 => Decision {
            event_ms: Some((event_unix * 1000.0).ceil() as i64),
            retry_in_ms: None,
            again_in_ms: None,
            clear_retry: true,
            error: None,
        },
        2 => Decision { retry_in_ms: Some(RETRY_HORIZON_MS), ..Decision::empty() },
        0 => Decision {
            retry_in_ms: Some(RETRY_NONE_MS),
            again_in_ms: if visible_or_resident { Some(AGAIN_MS) } else { None },
            ..Decision::empty()
        },
        n if n < 0 => Decision { error: Some(stopped), ..Decision::empty() },
        _ => Decision { retry_in_ms: Some(RETRY_NONE_MS), ..Decision::empty() },
    }
}

pub fn thrown_retry_ms() -> i64 {
    RETRY_THROWN_MS
}

/// Delete `Care/{epoch}-*` except the names in `keep`.
pub fn retain_epoch(dir: &Path, epoch: u64, keep: &[String]) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let prefix = format!("{epoch}-");
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && !keep.iter().any(|k| k == &name) {
            let _ = fs::remove_file(ent.path());
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Forecast {
    pub at: f64,
    pub origin: f64,
    pub not_before: f64,
    pub event: Option<f64>,
    pub stopped: bool,
    pub rom: String,
    pub scripted_event: Option<f64>,
    pub snap: Vec<u8>,
}

impl Forecast {
    pub fn new(rom: &str, not_before: f64) -> Self {
        Self {
            at: not_before,
            origin: not_before,
            not_before,
            event: None,
            stopped: false,
            rom: rom.to_string(),
            scripted_event: None,
            snap: Vec::new(),
        }
    }

    /// Each budget unit searches one second. A scripted event is how tests
    /// observe code 1 without a firmware image.
    pub fn run(&mut self, budget: i32) -> i32 {
        if self.stopped {
            return -1;
        }
        if budget < 0 {
            self.stopped = true;
            return -1;
        }
        for _ in 0..budget {
            if let Some(ev) = self.scripted_event {
                let next = self.at + 1.0;
                if ev >= self.not_before && next >= ev {
                    self.at = ev;
                    self.event = Some(ev);
                    return 1;
                }
            }
            self.at += 1.0;
            if self.at >= self.origin + HORIZON_S {
                return 2;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_codes_seed_and_files() {
        let seed = seed_not_before(1_000.0, 500_000);
        assert!((seed - 1_000.0).abs() < 1e-9);
        let seed = seed_not_before(100.0, 0);
        assert!((seed - 300.0).abs() < 1e-9);

        let ev = decide(1, 10.2, false);
        assert_eq!(ev.event_ms, Some(10_200));
        assert!(ev.clear_retry);
        assert!(ev.retry_in_ms.is_none());

        let h = decide(2, 0.0, true);
        assert!(h.event_ms.is_none());
        assert_eq!(h.retry_in_ms, Some(3_600_000));

        let z = decide(0, 0.0, true);
        assert_eq!(z.retry_in_ms, Some(30_000));
        assert_eq!(z.again_in_ms, Some(50));
        assert!(decide(0, 0.0, false).again_in_ms.is_none());

        let bad = decide(-1, 0.0, false);
        assert_eq!(bad.error, Some("Firmware forecast stopped before its next call."));
        assert_eq!(thrown_retry_ms(), 60_000);
        assert!(reached_horizon(100.0 + 86_400.0, 100.0));
        assert!(!reached_horizon(100.0 + 86_399.0, 100.0));

        let mut f = Forecast::new("fw", 0.0);
        assert_eq!(f.run(10), 0);
        assert!((f.at - 10.0).abs() < 1e-9);
        f.scripted_event = Some(15.0);
        assert_eq!(f.run(10), 1);
        assert_eq!(f.event, Some(15.0));

        let mut far = Forecast::new("fw", 0.0);
        assert_eq!(far.run(200), 0);
        // Drive to the horizon. 86400 seconds, 200 per call.
        let mut code = 0;
        for _ in 0..500 {
            code = far.run(200);
            if code != 0 {
                break;
            }
        }
        assert_eq!(code, 2);

        let dir = crate::tmpdir("forecast");
        fs::write(dir.join("7-seed.t18s"), b"a").unwrap();
        fs::write(dir.join("7-old.t18s"), b"b").unwrap();
        fs::write(dir.join("8-keep.t18s"), b"c").unwrap();
        let keep = vec!["7-seed.t18s".into()];
        retain_epoch(&dir, 7, &keep).unwrap();
        assert!(dir.join("7-seed.t18s").exists());
        assert!(!dir.join("7-old.t18s").exists());
        assert!(dir.join("8-keep.t18s").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
