//! C ABI for the iOS app. One serial queue on the Swift side.
//! Strings from `pm_string_free` are malloc'd C strings.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_last(msg: &str) {
    LAST_ERROR.with(|e| *e.borrow_mut() = msg.to_string());
}

use serde_json::{json, Value};

use crate::appearance;
use crate::copy::{self, Lang};
use crate::forecast::Forecast;
use crate::gatt;
use crate::identity;
use crate::library::{self, Container};
use crate::session::{PumpIn, Session};
use crate::settings::{self, CarePlan, Settings};
use crate::shelf::{self, Choices};

fn c_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p).to_str().ok().map(|s| s.to_string()) }
}

fn owned(s: impl AsRef<str>) -> *mut c_char {
    CString::new(s.as_ref().replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap()).into_raw()
}

fn session<'a>(p: *mut Session) -> Option<&'a mut Session> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { &mut *p })
    }
}

#[no_mangle]
pub extern "C" fn pm_string_free(p: *mut c_char) {
    if !p.is_null() {
        unsafe { drop(CString::from_raw(p)) }
    }
}

#[no_mangle]
pub extern "C" fn pm_text(lang: i32, key: *const c_char) -> *mut c_char {
    let key = c_str(key).unwrap_or_default();
    owned(copy::text(Lang::from_ffi(lang), &key))
}

#[no_mangle]
pub extern "C" fn pm_localize(text: *const c_char, lang: i32) -> *mut c_char {
    let text = c_str(text).unwrap_or_default();
    owned(copy::localize(Lang::from_ffi(lang), &text))
}

#[no_mangle]
pub extern "C" fn pm_mixing_version() -> *mut c_char {
    owned(identity::MIXING_VERSION)
}

#[no_mangle]
pub extern "C" fn pm_release() -> *mut c_char {
    owned(identity::RELEASE)
}

#[no_mangle]
pub extern "C" fn pm_build() -> i32 {
    identity::BUILD
}

#[no_mangle]
pub extern "C" fn pm_rom_kind(path: *const c_char) -> i32 {
    let Some(path) = c_str(path) else { return 0 };
    let Ok(bytes) = std::fs::read(path) else { return -1 };
    match crate::rom::classify(&bytes) {
        crate::rom::ImageClass::Supported { version, .. } => version as i32,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn pm_rom_name(path: *const c_char) -> *mut c_char {
    let Some(path) = c_str(path) else { return ptr::null_mut() };
    let Ok(bytes) = std::fs::read(path) else { return ptr::null_mut() };
    match crate::rom::classify(&bytes) {
        crate::rom::ImageClass::Supported { stem, .. } => owned(stem),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pm_has_current(root: *const c_char) -> i32 {
    let Some(root) = c_str(root) else { return 0 };
    i32::from(library::has_current_rom(Path::new(&root)))
}

#[no_mangle]
pub extern "C" fn pm_container_ensure(root: *const c_char) -> i32 {
    let Some(root) = c_str(root) else { return -1 };
    match Container::new(root).ensure() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_import_rom(root: *const c_char, path: *const c_char) -> *mut c_char {
    let (Some(root), Some(path)) = (c_str(root), c_str(path)) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    match library::import_file(Path::new(&root), Path::new(&path)) {
        Ok(r) => owned(json!({"ok": true, "result": {"stems": r.stems, "current": r.current, "notes": r.notes}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_library_json(root: *const c_char) -> *mut c_char {
    let Some(root) = c_str(root) else { return owned("[]"); };
    owned(json!(library::list_library(Path::new(&root))).to_string())
}

#[no_mangle]
pub extern "C" fn pm_settings_json(path: *const c_char) -> *mut c_char {
    let Some(path) = c_str(path) else {
        return owned(serde_json::to_string(&Settings::new_user()).unwrap_or_else(|_| "{}".into()));
    };
    let s = settings::load_settings(Path::new(&path));
    owned(serde_json::to_string(&s).unwrap_or_else(|_| "{}".into()))
}

#[no_mangle]
pub extern "C" fn pm_settings_write(path: *const c_char, json_text: *const c_char) -> *mut c_char {
    let (Some(path), Some(text)) = (c_str(path), c_str(json_text)) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    match settings::store_settings(Path::new(&path), &text) {
        Ok(s) => owned(json!({"ok": true, "result": s}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_plan_json(path: *const c_char) -> *mut c_char {
    let Some(path) = c_str(path) else { return owned("{}"); };
    let plan = settings::load_plan(Path::new(&path));
    owned(serde_json::to_string(&plan).unwrap_or_else(|_| "{}".into()))
}

#[no_mangle]
pub extern "C" fn pm_plan_store(path: *const c_char, json_text: *const c_char) -> i32 {
    let (Some(path), Some(text)) = (c_str(path), c_str(json_text)) else { return -1 };
    let Ok(plan) = serde_json::from_str::<CarePlan>(&text) else { return -1 };
    match settings::store_plan(Path::new(&path), &plan) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn open_common(rom: *const c_char, saves: *const c_char, paused: Option<bool>) -> *mut Session {
    let (Some(rom), Some(saves)) = (c_str(rom), c_str(saves)) else { return ptr::null_mut() };
    let result = match paused {
        Some(p) => Session::open_care(Path::new(&rom), Path::new(&saves), p),
        None => Session::open(Path::new(&rom), Path::new(&saves)),
    };
    match result {
        Ok(s) => {
            set_last("");
            Box::into_raw(Box::new(s))
        }
        Err(e) => {
            set_last(&e);
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn pm_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| owned(e.borrow().clone()))
}

#[no_mangle]
pub extern "C" fn pm_call_decision(
    take_call: i32,
    visible: i32,
    muted: i32,
    now_ms: i64,
    last_ms: i64,
    not_before_ms: i64,
) -> *mut c_char {
    let d = crate::notify::on_call(take_call != 0, visible != 0, muted != 0, now_ms, last_ms, not_before_ms);
    owned(json!({
        "detected": d.detected,
        "banner": d.banner,
        "channel": d.channel,
        "not_before_ms": d.not_before_ms,
        "last_ms": d.last_ms,
    }).to_string())
}

#[no_mangle]
pub extern "C" fn pm_alarm_post(event_ms: i64, now_ms: i64, visible: i32, missed_gap: i32) -> i32 {
    i32::from(crate::notify::on_alarm(event_ms, now_ms, visible != 0, missed_gap != 0).post)
}

#[no_mangle]
pub extern "C" fn pm_shelf_list(root: *const c_char, tab: *const c_char, language: *const c_char) -> *mut c_char {
    let (Some(root), Some(tab), Some(language)) = (c_str(root), c_str(tab), c_str(language)) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    let root_path = Path::new(&root);
    let settings = settings::load_settings(&root_path.join("settings"));
    if settings.shelf_pack.is_empty() {
        return owned(json!({"ok": false, "error": copy::text(Lang::En, "load_pack_first")}).to_string());
    }
    let dir = root_path.join("TamaShelf").join(&settings.shelf_pack);
    match shelf::pack::list_items(&dir, &tab, &language) {
        Ok(items) => {
            let rows: Vec<Value> = items.into_iter().map(|it| {
                let abs = if Path::new(&it.path).is_absolute() {
                    it.path
                } else {
                    dir.join(&it.path).to_string_lossy().into_owned()
                };
                json!({"name": it.name, "kind": it.kind, "language": it.language, "path": abs, "id": it.id, "price": it.price})
            }).collect();
            owned(json!({"ok": true, "result": rows}).to_string())
        }
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_set_background_care(root: *const c_char, on: i32) -> *mut c_char {
    let Some(root) = c_str(root) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    let path = Path::new(&root).join("settings");
    let mut s = settings::load_settings(&path);
    settings::set_background_care(&mut s, on != 0);
    let text = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
    match settings::store_settings(&path, &text) {
        Ok(stored) => owned(serde_json::to_string(&stored).unwrap_or_else(|_| "{}".into())),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_set_care_paused(root: *const c_char, paused: i32) -> i32 {
    let Some(root) = c_str(root) else { return -1 };
    let root = Path::new(&root);
    let mut s = settings::load_settings(&root.join("settings"));
    let mut plan = settings::load_plan(&root.join("care_plan"));
    settings::set_paused(&mut s, &mut plan, paused != 0);
    let text = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
    if settings::store_settings(&root.join("settings"), &text).is_err() {
        return -1;
    }
    if settings::store_plan(&root.join("care_plan"), &plan).is_err() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn pm_open(rom: *const c_char, saves: *const c_char) -> *mut Session {
    open_common(rom, saves, None)
}

#[no_mangle]
pub extern "C" fn pm_open_care(rom: *const c_char, saves: *const c_char, paused: i32) -> *mut Session {
    open_common(rom, saves, Some(paused != 0))
}

#[no_mangle]
pub extern "C" fn pm_close(p: *mut Session) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) }
    }
}

#[no_mangle]
pub extern "C" fn pm_name(p: *mut Session) -> *mut c_char {
    session(p).map(|s| owned(s.name())).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn pm_error(p: *mut Session) -> *mut c_char {
    session(p).map(|s| owned(&s.error)).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn pm_notice(p: *mut Session) -> *mut c_char {
    session(p).map(|s| owned(&s.notice)).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn pm_dump_window(p: *mut Session, start: u32, len: u32, path: *const c_char) -> i32 {
    let (Some(s), Some(path)) = (session(p), c_str(path)) else {
        return -1;
    };
    let sram = &s.machine.sys.sram;
    if start < tg18::SRAM_BASE {
        return -3;
    }
    let off = (start - tg18::SRAM_BASE) as usize;
    let Some(end) = off.checked_add(len as usize) else { return -3 };
    if end > sram.len() {
        return -3;
    }
    match std::fs::write(&path, &sram[off..end]) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

#[no_mangle]
pub extern "C" fn pm_dump_sram(p: *mut Session, path: *const c_char) -> i32 {
    let (Some(s), Some(path)) = (session(p), c_str(path)) else {
        return -1;
    };
    match std::fs::write(&path, &s.machine.sys.sram) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Whether the current session uses the Sweets font samples.
#[no_mangle]
pub extern "C" fn pm_current_is_sweets(p: *mut Session) -> i32 {
    i32::from(session(p).is_some_and(|s| s.is_sweets()))
}

/// Match text in a caller-provided 128×128 ARGB LCD frame. The buffer is
/// borrowed only for this call; release the returned string with pm_string_free.
#[no_mangle]
pub unsafe extern "C" fn pm_menu_text(pixels: *const u32, len: usize) -> *mut c_char {
    if pixels.is_null() || len != 128 * 128 {
        return owned("");
    }
    owned(crate::menu_text::read(std::slice::from_raw_parts(pixels, len)))
}

#[no_mangle]
pub extern "C" fn pm_player_name(p: *mut Session) -> *mut c_char {
    session(p)
        .map(|s| owned(crate::textbuf::player_name(&s.machine.sys.sram)))
        .unwrap_or_else(|| owned(""))
}

#[no_mangle]
pub extern "C" fn pm_message_text(p: *mut Session) -> *mut c_char {
    session(p)
        .map(|s| owned(crate::textbuf::message_text(&s.machine.sys.sram)))
        .unwrap_or_else(|| owned(""))
}

#[no_mangle]
pub extern "C" fn pm_seconds(p: *mut Session) -> f64 {
    session(p).map(|s| s.seconds()).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn pm_asleep(p: *mut Session) -> i32 {
    session(p).map(|s| i32::from(s.asleep())).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_step(p: *mut Session, dt: f64) -> i32 {
    session(p).map(|s| s.step(dt)).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn pm_key(p: *mut Session, index: i32, down: i32) -> i32 {
    match session(p) {
        Some(s) => match s.key(index as usize, down != 0) {
            Ok(()) => 0,
            Err(_) => -1,
        },
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_release_keys(p: *mut Session) {
    if let Some(s) = session(p) {
        s.release_keys();
    }
}

#[no_mangle]
pub extern "C" fn pm_set_shelf_open(p: *mut Session, open: i32) {
    if let Some(s) = session(p) {
        s.shelf_covers = open != 0;
        if open != 0 {
            s.release_keys();
        }
    }
}

#[no_mangle]
pub extern "C" fn pm_set_fast_keys(p: *mut Session, on: i32) {
    if let Some(s) = session(p) {
        s.set_fast_keys(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn pm_display_awake(p: *mut Session, on: i32) {
    if let Some(s) = session(p) {
        s.display_awake(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn pm_display_hint(visible: i32, active: i32, catchup_done: i32, paused: i32, follow: i32, interactive: i32) -> i32 {
    i32::from(crate::tick::display_hint(visible != 0, active != 0, catchup_done != 0, paused != 0, follow != 0, interactive != 0))
}

#[no_mangle]
pub extern "C" fn pm_frame(p: *mut Session, out: *mut u32) {
    if out.is_null() {
        return;
    }
    if let Some(s) = session(p) {
        let buf = unsafe { std::slice::from_raw_parts_mut(out, 16384) };
        s.frame(buf);
    }
}

#[no_mangle]
pub extern "C" fn pm_take_call(p: *mut Session) -> i32 {
    session(p).map(|s| i32::from(s.take_call)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_save(p: *mut Session) -> i32 {
    match session(p) {
        Some(s) => match s.save() {
            Ok(()) => 0,
            Err(_) => -1,
        },
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_delete_current_save(p: *mut Session) -> i32 {
    match session(p) {
        Some(s) => i32::from(s.delete_current_save().is_err()),
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_import_save(p: *mut Session, path: *const c_char) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let Some(path) = c_str(path) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    match s.import_save(Path::new(&path)) {
        Ok(()) => owned(r#"{"ok":true,"result":"save imported, you can continue"}"#),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_export_save(p: *mut Session, path: *const c_char) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let Some(path) = c_str(path) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    match s.export_save(Path::new(&path)) {
        Ok(()) => owned(r#"{"ok":true,"result":"save exported"}"#),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_link_enable(p: *mut Session, on: i32) {
    if let Some(s) = session(p) {
        s.link.enable(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn pm_link_state(p: *mut Session) -> i32 {
    session(p).map(|s| s.link.state).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_link_kind(p: *mut Session) -> i32 {
    session(p).map(|s| s.link.kind).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_link_web_meets(p: *mut Session) -> i32 {
    session(p).map(|s| i32::from(s.link.web)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_link_recovered(p: *mut Session) -> i32 {
    session(p).map(|s| i32::from(s.link.recovered)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_link_connect(p: *mut Session) {
    if let Some(s) = session(p) {
        s.link_connect();
    }
}

#[no_mangle]
pub extern "C" fn pm_link_cancel(p: *mut Session) {
    if let Some(s) = session(p) {
        s.link_cancel();
    }
}

#[no_mangle]
pub extern "C" fn pm_link_write(p: *mut Session, bytes: *const u8, len: i32) -> i32 {
    let Some(s) = session(p) else { return -1 };
    if bytes.is_null() || len <= 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    i32::from(!s.link_write(slice))
}

#[no_mangle]
pub extern "C" fn pm_link_read(p: *mut Session, out: *mut u8, cap: i32) -> i32 {
    let Some(s) = session(p) else { return -1 };
    if out.is_null() || cap <= 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(out, cap as usize) };
    s.link_read(slice)
}

#[no_mangle]
pub extern "C" fn pm_care_sync(p: *mut Session, now: f64, budget: f64) -> f64 {
    session(p).map(|s| s.care_sync(now, budget)).unwrap_or(-1.0)
}

#[no_mangle]
pub extern "C" fn pm_care_at(p: *mut Session) -> f64 {
    session(p).map(|s| s.care_at()).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn pm_care_pause(p: *mut Session, paused: i32, now: f64) {
    if let Some(s) = session(p) {
        s.care_pause(paused != 0, now);
    }
}

#[no_mangle]
pub extern "C" fn pm_care_adopt(p: *mut Session) {
    if let Some(s) = session(p) {
        s.care_adopt();
    }
}

#[no_mangle]
pub extern "C" fn pm_advance_clock(p: *mut Session, seconds: f64) {
    if let Some(s) = session(p) {
        s.advance_clock(seconds);
    }
}

#[repr(C)]
pub struct PmPumpIn {
    pub now_unix: f64,
    pub dt: f64,
    pub visible: i32,
    pub monotonic_ms: u64,
    pub last_frame_ns: u64,
    pub last_save_ms: u64,
    pub last_catchup_save_ms: u64,
    pub backlog: f64,
    pub phone_interactive: i32,
    pub follow_system_sleep: i32,
    pub muted: i32,
    pub frame_ns: u64,
}

#[repr(C)]
pub struct PmPumpOut {
    pub stopped: i32,
    pub step_code: i32,
    pub route: i32,
    pub take_call: i32,
    pub play_audio: i32,
    pub volume: f32,
    pub drew_frame: i32,
    pub did_save: i32,
    pub catching_up: i32,
    pub tick: i32,
    pub next_ms: u64,
    pub backlog: f64,
    pub remain: f64,
    pub catchup_total: f64,
    pub last_frame_ns: u64,
    pub last_save_ms: u64,
    pub last_catchup_save_ms: u64,
    pub pcm_count: i32,
}

#[no_mangle]
pub extern "C" fn pm_pump(p: *mut Session, inp: *const PmPumpIn, out: *mut PmPumpOut, pcm: *mut i16, pcm_cap: i32) -> i32 {
    let (Some(s), Some(inp), Some(outp)) = (session(p), unsafe { inp.as_ref() }, unsafe { out.as_mut() }) else {
        return -1;
    };
    let got = s.pump(&PumpIn {
        now_unix: inp.now_unix,
        dt: inp.dt,
        visible: inp.visible != 0,
        monotonic_ms: inp.monotonic_ms,
        last_frame_ns: inp.last_frame_ns,
        last_save_ms: inp.last_save_ms,
        last_catchup_save_ms: inp.last_catchup_save_ms,
        backlog: inp.backlog,
        phone_interactive: inp.phone_interactive != 0,
        follow_system_sleep: inp.follow_system_sleep != 0,
        muted: inp.muted != 0,
        frame_ns: inp.frame_ns,
    });
    let n = if pcm.is_null() || pcm_cap <= 0 {
        0
    } else {
        let take = got.pcm.len().min(pcm_cap as usize);
        unsafe { ptr::copy_nonoverlapping(got.pcm.as_ptr(), pcm, take) };
        take
    };
    *outp = PmPumpOut {
        stopped: i32::from(got.stopped),
        step_code: got.step_code,
        route: got.route,
        take_call: i32::from(got.take_call),
        play_audio: i32::from(got.play_audio),
        volume: got.volume,
        drew_frame: i32::from(got.drew_frame),
        did_save: i32::from(got.did_save),
        catching_up: i32::from(got.catching_up),
        tick: i32::from(got.tick),
        next_ms: got.next_ms,
        backlog: got.backlog,
        remain: got.remain,
        catchup_total: got.catchup_total,
        last_frame_ns: got.last_frame_ns,
        last_save_ms: got.last_save_ms,
        last_catchup_save_ms: got.last_catchup_save_ms,
        pcm_count: n as i32,
    };
    if !got.notice.is_empty() {
        s.notice = got.notice;
    }
    if !got.error.is_empty() {
        s.error = got.error;
    }
    0
}

#[no_mangle]
pub extern "C" fn pm_control(p: *mut Session, request: *const c_char) -> *mut c_char {
    let req = c_str(request);
    owned(crate::control::control_json(session(p), req.as_deref()))
}

#[no_mangle]
pub extern "C" fn pm_forecast_create(p: *mut Session, not_before: f64) -> *mut Forecast {
    let Some(s) = session(p) else { return ptr::null_mut() };
    let mut f = Forecast::new(&s.stem, not_before);
    if s.playable {
        f.snap = tg18::snapshot::Snapshot::capture(&s.machine).to_bytes();
    }
    Box::into_raw(Box::new(f))
}

#[no_mangle]
pub extern "C" fn pm_forecast_open(rom: *const c_char, cursor: *const c_char, not_before: f64) -> *mut Forecast {
    let rom = c_str(rom).unwrap_or_default();
    let mut f = Forecast::new(&rom, not_before);
    if let Some(path) = c_str(cursor) {
        if let Ok(bytes) = std::fs::read(path) {
            f.snap = bytes;
        }
    }
    Box::into_raw(Box::new(f))
}

#[no_mangle]
pub extern "C" fn pm_forecast_run(p: *mut Forecast, budget: i32) -> i32 {
    if p.is_null() {
        return -1;
    }
    unsafe { (*p).run(budget) }
}

#[no_mangle]
pub extern "C" fn pm_forecast_at(p: *mut Forecast) -> f64 {
    if p.is_null() { 0.0 } else { unsafe { (*p).at } }
}

#[no_mangle]
pub extern "C" fn pm_forecast_event(p: *mut Forecast) -> f64 {
    if p.is_null() { 0.0 } else { unsafe { (*p).event.unwrap_or(0.0) } }
}

#[no_mangle]
pub extern "C" fn pm_forecast_save(p: *mut Forecast, path: *const c_char) -> i32 {
    if p.is_null() {
        return -1;
    }
    let Some(path) = c_str(path) else { return -1 };
    let snap = unsafe { &(*p).snap };
    let bytes: &[u8] = if snap.is_empty() { b"pocket-meets-forecast" } else { snap };
    match std::fs::write(path, bytes) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_forecast_close(p: *mut Forecast) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)) }
    }
}

#[no_mangle]
pub extern "C" fn pm_forecast_seed(now_unix: f64, last_call_ms: i64) -> f64 {
    crate::forecast::seed_not_before(now_unix, last_call_ms)
}

#[no_mangle]
pub extern "C" fn pm_forecast_decide(code: i32, event_unix: f64, visible: i32) -> *mut c_char {
    let d = crate::forecast::decide(code, event_unix, visible != 0);
    owned(json!({
        "event_ms": d.event_ms,
        "retry_in_ms": d.retry_in_ms,
        "again_in_ms": d.again_in_ms,
        "clear_retry": d.clear_retry,
        "error": d.error,
    }).to_string())
}

#[no_mangle]
pub extern "C" fn pm_forecast_retain(dir: *const c_char, epoch: i64, keep_json: *const c_char) -> i32 {
    let (Some(dir), Some(keep_json)) = (c_str(dir), c_str(keep_json)) else { return -1 };
    let keep: Vec<String> = serde_json::from_str(&keep_json).unwrap_or_default();
    match crate::forecast::retain_epoch(Path::new(&dir), epoch.max(0) as u64, &keep) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_zone(x: f32, y: f32, w: f32, h: f32, screenshots: i32) -> i32 {
    appearance::zone(x, y, w, h, screenshots != 0).map(|z| z as i32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pm_dot_strength(pixel_points: f32) -> f32 {
    appearance::dot_strength(pixel_points)
}

#[no_mangle]
pub extern "C" fn pm_guide_visible(appearance_id: i32, ready: i32, focused: i32, shelf: i32, guide: i32, shots: i32, seen: i32) -> i32 {
    i32::from(appearance::guide_visible(appearance_id, ready != 0, focused != 0, shelf != 0, guide, shots != 0, seen != 0))
}

#[no_mangle]
pub extern "C" fn pm_exit_allowed(ready: i32, shelf: i32, guide: i32) -> i32 {
    i32::from(appearance::exit_allowed(ready != 0, shelf != 0, guide != 0))
}

#[no_mangle]
pub extern "C" fn pm_png_name(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, millis: u32, hex8: *const c_char) -> *mut c_char {
    let hex8 = c_str(hex8).unwrap_or_else(|| "00000000".into());
    owned(appearance::png_name(y, mo, d, h, mi, s, millis, &hex8))
}

#[no_mangle]
pub extern "C" fn pm_export_name(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> *mut c_char {
    owned(appearance::export_name(y, mo, d, h, mi, s))
}

#[no_mangle]
pub extern "C" fn pm_fake_device_json() -> *mut c_char {
    owned(identity::FAKE_DEVICE_JSON)
}

#[no_mangle]
pub extern "C" fn pm_mixing_url() -> *mut c_char {
    owned(identity::MIXING_URL)
}

#[no_mangle]
pub extern "C" fn pm_support_url() -> *mut c_char {
    owned(identity::SUPPORT_URL)
}

#[no_mangle]
pub extern "C" fn pm_login_form() -> *mut c_char {
    owned(crate::mixing::login_form())
}

#[no_mangle]
pub extern "C" fn pm_notify_hex(bytes: *const u8, len: i32) -> *mut c_char {
    if bytes.is_null() || len <= 0 {
        return owned("");
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    owned(crate::mixing::notify_hex(slice))
}

#[no_mangle]
pub extern "C" fn pm_gatt_rx(owner: i32, subscribed: i32, prepared: i32, offset: i32, len: i32, queued: i32) -> i32 {
    match gatt::rx_write(owner != 0, subscribed != 0, prepared != 0, offset, len.max(0) as usize, queued.max(0) as usize) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[no_mangle]
pub extern "C" fn pm_shelf_install(root: *const c_char, zip_path: *const c_char, settings_path: *const c_char) -> *mut c_char {
    let (Some(root), Some(zip_path), Some(settings_path)) = (c_str(root), c_str(zip_path), c_str(settings_path)) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    let bytes = match std::fs::read(&zip_path) {
        Ok(b) => b,
        Err(_) => return owned(json!({"ok": false, "error": copy::text(Lang::En, "cannot_read_zip")}).to_string()),
    };
    let mut s = settings::load_settings(Path::new(&settings_path));
    match shelf::pack::install_pack(Path::new(&root), &bytes, &mut s) {
        Ok(name) => {
            let stored = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
            let _ = settings::store_settings(Path::new(&settings_path), &stored);
            owned(json!({"ok": true, "result": {"pack": name}}).to_string())
        }
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_shelf_send(p: *mut Session, path: *const c_char) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let Some(path) = c_str(path) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return owned(json!({"ok": false, "error": e.to_string()}).to_string()),
    };
    match s.send_tamago(&bytes) {
        Ok(msg) => owned(json!({"ok": true, "result": {"message": msg}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_shelf_balance(p: *mut Session, add: i32, amount: i32) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    match s.set_balance(add != 0, amount as i64) {
        Ok(v) => owned(json!({"ok": true, "result": {"balance": v}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_shelf_undo(p: *mut Session) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    match s.undo_shelf() {
        Ok(()) => owned(r#"{"ok":true,"result":{"message":"progress saved"}}"#),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_shelf_marry(p: *mut Session, spec: *const c_char) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let Some(text) = c_str(spec) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let value: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return owned(json!({"ok": false, "error": e.to_string()}).to_string()),
    };
    let mut parts = [0u8; 4];
    if let Some(arr) = value.get("parts").and_then(|v| v.as_array()) {
        for (i, n) in arr.iter().take(4).enumerate() {
            parts[i] = n.as_u64().unwrap_or(0) as u8;
        }
    }
    let colors = value.get("colors").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|n| n.as_u64().map(|x| x as u8)).collect()).unwrap_or_default();
    let choices = Choices {
        parts,
        pixels: [None, None, None, None],
        colors,
        sex: value.get("sex").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        mode: value.get("mode").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        dna_n: value.get("dna_n").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        random_sex: value.get("random_sex").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        random_dna: [0; 8],
    };
    match s.marry(&choices) {
        Ok(plan) => owned(json!({"ok": true, "result": {"sex": plan.sex, "dna": plan.dna}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_custom_import(root: *const c_char, path: *const c_char, kind: i32) -> *mut c_char {
    let (Some(root), Some(path)) = (c_str(root), c_str(path)) else {
        return owned(r#"{"ok":false,"error":"null session or request"}"#);
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => return owned(json!({"ok": false, "error": e.to_string()}).to_string()),
    };
    let root = Path::new(&root);
    let result = match kind {
        0 => crate::shelf::custom::make_wallpaper(root, &bytes, bytes.len() as u32, "image").map(|(_, m)| m),
        1 => crate::shelf::custom::import_character_file(root, &bytes, "character"),
        2 => crate::shelf::custom::import_tamago_file(root, &bytes, "gift").map(|t| crate::shelf::tamago::send_sentence(t.cat).to_string()),
        _ => crate::shelf::custom::import_character_zip(root, &bytes),
    };
    match result {
        Ok(msg) => owned(json!({"ok": true, "result": {"message": msg}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_set_root(p: *mut Session, root: *const c_char) {
    if let (Some(s), Some(root)) = (session(p), c_str(root)) {
        s.root = Path::new(&root).to_path_buf();
        if s.save_dir.as_os_str().is_empty() {
            s.save_dir = s.root.join("Saves").join(&s.stem);
        }
    }
}

#[no_mangle]
pub extern "C" fn pm_switch_image(p: *mut Session, rom_path: *const c_char) -> *mut c_char {
    let Some(s) = session(p) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let Some(path) = c_str(rom_path) else { return owned(r#"{"ok":false,"error":"null session or request"}"#) };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return owned(json!({"ok": false, "error": copy::text(Lang::En, "import_failed_again")}).to_string()),
    };
    let saves = if s.save_dir.as_os_str().is_empty() {
        s.root.join("Saves")
    } else {
        s.save_dir.parent().unwrap_or(Path::new("")).to_path_buf()
    };
    match s.switch_to_image(&bytes, &saves) {
        Ok(msg) => owned(json!({"ok": true, "result": {"message": msg}}).to_string()),
        Err(e) => owned(json!({"ok": false, "error": e}).to_string()),
    }
}

#[no_mangle]
pub extern "C" fn pm_care_foreground(root: *const c_char) -> *mut c_char {
    let Some(root) = c_str(root) else { return owned(""); };
    let settings_path = Path::new(&root).join("settings");
    let mut s = settings::load_settings(&settings_path);
    let notice = settings::foreground_notice(&mut s).unwrap_or("");
    let text = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
    let _ = settings::store_settings(&settings_path, &text);
    owned(notice)
}

#[no_mangle]
pub extern "C" fn pm_care_watch(root: *const c_char, now_ms: i64) -> i32 {
    let Some(root) = c_str(root) else { return 0 };
    let root = Path::new(&root);
    let settings_path = root.join("settings");
    let plan_path = root.join("care_plan");
    let mut s = settings::load_settings(&settings_path);
    let plan = settings::load_plan(&plan_path);
    if settings::watchdog_due(&plan, now_ms) && s.background_care {
        settings::apply_fallback(&mut s);
        let text = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
        let _ = settings::store_settings(&settings_path, &text);
        return 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn pm_note_heartbeat(root: *const c_char, now_ms: i64) -> i32 {
    let Some(root) = c_str(root) else { return -1 };
    let path = Path::new(&root).join("care_plan");
    let mut plan = settings::load_plan(&path);
    settings::note_heartbeat(&mut plan, now_ms);
    match settings::store_plan(&path, &plan) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pm_alarm_exact(allowed: i32) -> *mut c_char {
    owned(crate::input::alarm_copy(allowed != 0))
}

#[cfg(test)]
mod tests {
    #[test]
    fn header_lists_the_gatt_numbers() {
        let header = include_str!("../include/pocket_meets.h");
        assert!(header.contains("PM_GATT_IDLE_MS 300000"));
        assert!(header.contains("PM_GATT_ACTIVE_MS 120000"));
        assert!(header.contains("PM_GATT_GRACE_MS 2500"));
        assert!(header.contains("PM_GATT_COMMIT_MS 300"));
        assert!(header.contains("PM_RX_QUEUE 128"));
        assert!(header.contains("PM_TX_QUEUE 512"));
        assert!(header.contains("TMGC_meets"));
        assert!(header.contains("1.4.3"));
        assert!(header.contains("libpocket_meets"));
    }
}
