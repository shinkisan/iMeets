//! Preference files `settings` and `care_plan`. Missing keys use defaults.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::appearance::{self, GUIDE_COMPLETE};
use crate::copy::Lang;
use crate::identity::BUILD;

fn zh() -> String { "zh".into() }
fn jp() -> String { "jp".into() }
fn scale() -> i32 { 100 }
fn yes() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default = "zh")]
    pub language: String,
    #[serde(default)]
    pub screenshots: bool,
    #[serde(default)]
    pub dot_matrix: bool,
    /// Shrink the firmware's key debounce (see `keypatch`).
    #[serde(default)]
    pub fast_keys: bool,
    /// Fire the taptic engine when a shell key is pressed.
    #[serde(default = "yes")]
    pub key_haptics: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub follow_system_sleep: bool,
    #[serde(default = "yes")]
    pub background_care: bool,
    #[serde(default)]
    pub care_paused: bool,
    #[serde(default)]
    pub care_auto_resume: bool,
    #[serde(default)]
    pub care_fallback_notice: bool,
    #[serde(default)]
    pub care_resumed_notice: bool,
    #[serde(default)]
    pub care_intro_version: i32,
    #[serde(default)]
    pub notifications_requested: bool,
    #[serde(default)]
    pub last_call_notification: i64,
    #[serde(default)]
    pub appearance: i32,
    /// Let the shell/night look follow the system light/dark setting.
    #[serde(default)]
    pub appearance_auto: bool,
    #[serde(default = "scale")]
    pub small_screen_scale: i32,
    #[serde(default)]
    pub small_screen_size_configured: bool,
    #[serde(default)]
    pub small_screen_guide_version: i32,
    #[serde(default)]
    pub small_screen_capture_guide_seen: bool,
    #[serde(default)]
    pub shelf_enabled: bool,
    #[serde(default)]
    pub shelf_pack: String,
    #[serde(default = "jp")]
    pub shelf_language: String,
    #[serde(default)]
    pub shelf_character: String,
    #[serde(default)]
    pub ble_previous_name: String,
}

impl Settings {
    pub fn new_user() -> Self {
        Self {
            language: "zh".into(),
            screenshots: false,
            dot_matrix: false,
            fast_keys: false,
            key_haptics: true,
            muted: false,
            follow_system_sleep: false,
            background_care: true,
            care_paused: false,
            care_auto_resume: false,
            care_fallback_notice: false,
            care_resumed_notice: false,
            care_intro_version: 0,
            notifications_requested: false,
            last_call_notification: 0,
            appearance: 0,
            appearance_auto: false,
            small_screen_scale: 100,
            small_screen_size_configured: false,
            small_screen_guide_version: 0,
            small_screen_capture_guide_seen: false,
            shelf_enabled: false,
            shelf_pack: String::new(),
            shelf_language: "jp".into(),
            shelf_character: String::new(),
            ble_previous_name: String::new(),
        }
    }

    pub fn normalize(&mut self) {
        if Lang::parse(&self.language).is_none() {
            self.language = "zh".into();
        }
        if self.shelf_language != "jp" && self.shelf_language != "en" {
            self.shelf_language = "jp".into();
        }
        if !(0..=3).contains(&self.appearance) {
            self.appearance = 0;
        }
        self.small_screen_scale = appearance::clamp_scale(self.small_screen_scale);
        if self.care_intro_version < 0 {
            self.care_intro_version = 0;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarePlan {
    #[serde(default)]
    pub heartbeat: i64,
    #[serde(default)]
    pub epoch: i64,
    #[serde(default)]
    pub cursor: String,
    #[serde(default)]
    pub past: String,
    #[serde(default)]
    pub event: i64,
    #[serde(default)]
    pub retry: i64,
    #[serde(default)]
    pub not_before: i64,
}

impl CarePlan {
    pub fn new() -> Self {
        Self { heartbeat: 0, epoch: 0, cursor: String::new(), past: String::new(), event: 0, retry: 0, not_before: 0 }
    }

    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.cursor.clear();
        self.past.clear();
        self.event = 0;
        self.retry = 0;
    }
}

impl Default for CarePlan {
    fn default() -> Self { Self::new() }
}

pub fn load_settings(path: &Path) -> Settings {
    let text = fs::read_to_string(path).unwrap_or_default();
    if text.trim().is_empty() {
        return Settings::new_user();
    }
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let mut s = serde_json::from_value::<Settings>(value.clone()).unwrap_or_else(|_| Settings::new_user());
    if let Some(obj) = value.as_object() {
        if !obj.contains_key("appearance") && obj.get("minimal_ui").and_then(|v| v.as_bool()) == Some(true) {
            s.appearance = 1;
        }
        if obj.contains_key("small_screen_scale") {
            s.small_screen_size_configured = true;
        }
        if !obj.contains_key("background_care") {
            s.background_care = true;
        }
        if !obj.contains_key("language") {
            s.language = "zh".into();
        }
        if !obj.contains_key("shelf_language") {
            s.shelf_language = "jp".into();
        }
        if !obj.contains_key("small_screen_scale") {
            s.small_screen_scale = 100;
        }
    }
    s.normalize();
    s
}

pub fn store_settings(path: &Path, incoming: &str) -> Result<Settings, String> {
    let value: Value = serde_json::from_str(incoming).map_err(|_| "invalid JSON".to_string())?;
    let obj = value.as_object().ok_or_else(|| "invalid JSON".to_string())?;
    if let Some(lang) = obj.get("language").and_then(|v| v.as_str()) {
        if Lang::parse(lang).is_none() {
            return Err("language must be zh or en".into());
        }
    }
    if let Some(lang) = obj.get("shelf_language").and_then(|v| v.as_str()) {
        if lang != "jp" && lang != "en" {
            return Err("shelf language must be jp or en".into());
        }
    }
    let mut s = load_settings(path);
    // Merge known keys from the object onto the current file.
    let mut base = serde_json::to_value(&s).unwrap_or(json!({}));
    if let Some(map) = base.as_object_mut() {
        for (k, v) in obj {
            if k == "minimal_ui" {
                continue;
            }
            map.insert(k.clone(), v.clone());
        }
    }
    s = serde_json::from_value(base).map_err(|e| e.to_string())?;
    if obj.contains_key("small_screen_scale") {
        s.small_screen_size_configured = true;
    }
    s.normalize();
    write_json(path, &s)?;
    Ok(s)
}

pub fn load_plan(path: &Path) -> CarePlan {
    fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(CarePlan::new)
}

pub fn store_plan(path: &Path, plan: &CarePlan) -> Result<(), String> {
    write_json(path, plan)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_notes_shown(s: &mut Settings) {
    s.care_intro_version = BUILD;
}

pub fn finish_guide(s: &mut Settings, screenshots: bool) {
    s.small_screen_guide_version = GUIDE_COMPLETE;
    if screenshots {
        s.small_screen_capture_guide_seen = true;
    }
}

pub fn set_scale(s: &mut Settings, percent: i32) {
    s.small_screen_scale = appearance::clamp_scale(percent);
    s.small_screen_size_configured = true;
}

/// Choosing a care mode by hand clears the watchdog memory.
pub fn set_background_care(s: &mut Settings, on: bool) {
    s.background_care = on;
    s.care_auto_resume = false;
    s.care_fallback_notice = false;
    s.care_resumed_notice = false;
}

pub fn set_paused(s: &mut Settings, plan: &mut CarePlan, paused: bool) {
    s.care_paused = paused;
    plan.invalidate();
}

pub fn note_heartbeat(plan: &mut CarePlan, now_ms: i64) {
    plan.heartbeat = now_ms;
}

pub fn watchdog_due(plan: &CarePlan, now_ms: i64) -> bool {
    plan.heartbeat > 0 && now_ms.saturating_sub(plan.heartbeat) > 90_000
}

pub fn apply_fallback(s: &mut Settings) {
    s.background_care = false;
    s.care_auto_resume = true;
    s.care_fallback_notice = true;
}

/// Returns the sentence to show, if any, and updates the flags.
pub fn foreground_notice(s: &mut Settings) -> Option<&'static str> {
    let lang = Lang::parse(&s.language).unwrap_or(Lang::Zh);
    if s.care_auto_resume {
        s.background_care = true;
        s.care_auto_resume = false;
        s.care_resumed_notice = true;
        s.care_fallback_notice = false;
        return Some(crate::copy::text(lang, "care_restored"));
    }
    if s.care_fallback_notice {
        s.care_fallback_notice = false;
        return Some(crate::copy::text(lang, "care_fallback"));
    }
    if s.care_resumed_notice {
        s.care_resumed_notice = false;
        return Some(crate::copy::text(lang, "care_restored"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_legacy_language_scale_and_watchdog() {
        let dir = crate::tmpdir("settings");
        let path = dir.join("settings");
        let s = load_settings(&path);
        assert_eq!(s.language, "zh");
        assert!(s.background_care);
        assert!(!s.care_paused);
        assert_eq!(s.appearance, 0);
        assert!(!s.dot_matrix);
        assert!(!s.screenshots);
        assert!(!s.muted);
        assert!(!s.follow_system_sleep);
        assert!(!s.shelf_enabled);
        assert_eq!(s.shelf_language, "jp");
        assert_eq!(s.small_screen_scale, 100);
        assert_eq!(s.care_intro_version, 0);
        assert!(!s.notifications_requested);

        fs::write(&path, r#"{"minimal_ui":true,"background_care":false}"#).unwrap();
        let s = load_settings(&path);
        assert_eq!(s.appearance, 1);
        assert!(!s.background_care);
        assert_eq!(s.language, "zh");

        fs::write(&path, r#"{"small_screen_scale":180,"language":"en"}"#).unwrap();
        let s = load_settings(&path);
        assert_eq!(s.small_screen_scale, 150);
        assert!(s.small_screen_size_configured);
        assert_eq!(s.language, "en");

        let err = store_settings(&path, r#"{"language":"fr"}"#).unwrap_err();
        assert!(err.contains("zh or en"));
        let err = store_settings(&path, r#"{"shelf_language":"zh"}"#).unwrap_err();
        assert!(err.contains("jp or en"));
        let s = store_settings(&path, r#"{"language":"zh","shelf_language":"en","screenshots":true}"#).unwrap();
        assert_eq!(s.language, "zh");
        assert_eq!(s.shelf_language, "en");
        assert!(s.screenshots);
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("minimal_ui"));

        let mut s = Settings::new_user();
        s.language = "en".into();
        let mut plan = CarePlan::new();
        set_paused(&mut s, &mut plan, true);
        assert!(s.care_paused);
        assert_eq!(plan.epoch, 1);
        assert!(plan.cursor.is_empty());
        set_paused(&mut s, &mut plan, false);
        assert_eq!(plan.epoch, 2);

        note_heartbeat(&mut plan, 1_000);
        assert!(!watchdog_due(&plan, 91_000));
        assert!(watchdog_due(&plan, 91_001));
        apply_fallback(&mut s);
        assert!(!s.background_care);
        assert!(s.care_auto_resume);
        assert!(s.care_fallback_notice);
        let msg = foreground_notice(&mut s).unwrap();
        assert_eq!(msg, "enhanced mode restored; if you had paused, pause stays");
        assert!(s.background_care);
        assert!(!s.care_auto_resume);

        apply_fallback(&mut s);
        set_background_care(&mut s, false);
        assert!(!s.care_auto_resume);
        assert!(!s.care_fallback_notice);
        assert!(foreground_notice(&mut s).is_none());

        let mut s = Settings::new_user();
        s.language = "en".into();
        s.care_fallback_notice = true;
        s.care_auto_resume = false;
        let msg = foreground_notice(&mut s).unwrap();
        assert_eq!(msg, "background care was interrupted, switched to normal mode, time still syncs");

        let _ = fs::remove_dir_all(&dir);
    }
}
