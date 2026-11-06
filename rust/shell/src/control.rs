//! JSON control channel. Shelf uses pause, write, register, and cpsr.
//! The other commands are on the same channel.

use serde::Serialize;
use serde_json::{json, Value};

use crate::copy::{self, Lang};
use crate::session::{self, Session};

#[derive(Serialize)]
struct FailBody<'a> {
    ok: bool,
    error: &'a str,
}

#[derive(Serialize)]
struct OkBody {
    ok: bool,
    result: Value,
}

fn fail(msg: &str) -> String {
    serde_json::to_string(&FailBody {
        ok: false,
        error: msg,
    })
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":"{}"}}"#, msg.replace('"', "'")))
}

fn ok(result: Value) -> String {
    serde_json::to_string(&OkBody { ok: true, result })
        .unwrap_or_else(|_| r#"{"ok":false,"error":"invalid JSON"}"#.into())
}

pub fn control_json(session: Option<&mut Session>, request: Option<&str>) -> String {
    let Some(request) = request.map(str::trim).filter(|s| !s.is_empty()) else {
        return fail(copy::text(Lang::En, "null_control"));
    };
    let Some(session) = session else {
        return fail(copy::text(Lang::En, "null_control"));
    };
    let value: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(_) => return fail("invalid JSON"),
    };
    let cmd = value.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
    match cmd {
        "inspect" => ok(json!({
            "pc": session.machine.cpu.pc(),
            "cpsr": session.machine.cpu.cpsr,
            "asleep": session.asleep(),
            "seconds": session.seconds(),
            "link_state": session.link.state,
            "name": session.name(),
            "care_at": session.care_at(),
            "paused": session.control_paused,
            "care_paused": session.care.paused,
        })),
        "pause" => {
            let paused = value
                .get("paused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.control_paused = paused;
            ok(json!({"paused": paused}))
        }
        "read" => read(session, &value),
        "write" => write(session, &value),
        "key" => key(session, &value),
        "patch_rom" => patch_rom(session, &value),
        "export_rom" => export_rom(session, &value),
        "register" => register(session, &value),
        "cpsr" => {
            let Some(v) = value.get("value").and_then(as_u32) else {
                return fail("cpsr needs a value");
            };
            session.machine.cpu.write_cpsr(v);
            ok(json!({"cpsr": v}))
        }
        "bus_write" => bus_write(session, &value),
        "clock" => clock(session, &value),
        "clock_scale" => clock_scale(session, &value),
        "run" | "advance" => bounded_run(session, &value, cmd),
        _ => fail("unknown command"),
    }
}

fn as_u32(v: &Value) -> Option<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .or_else(|| v.as_i64().and_then(|n| u32::try_from(n).ok()))
}

fn as_u64_num(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
}

fn read(session: &mut Session, value: &Value) -> String {
    let Some(address) = value.get("address").and_then(as_u32) else {
        return fail("read needs an address");
    };
    let Some(length) = value.get("length").and_then(|v| v.as_i64()) else {
        return fail("read needs a length");
    };
    if !(0..=65536).contains(&length) {
        return fail(copy::text(Lang::En, "read_limit"));
    }
    let length = length as usize;
    if !session::span_ok(address, length) {
        return fail("address is outside RAM and SRAM");
    }
    let mut buf = vec![0u8; length];
    session.machine.sys.read_block(address, &mut buf);
    ok(json!({"bytes": buf}))
}

fn write(session: &mut Session, value: &Value) -> String {
    let Some(address) = value.get("address").and_then(as_u32) else {
        return fail("write needs an address");
    };
    let Some(bytes) = byte_list(value.get("bytes")) else {
        return fail("write needs bytes");
    };
    if !session::span_ok(address, bytes.len()) {
        return fail("address is outside RAM and SRAM");
    }
    session.machine.sys.poke(address, &bytes);
    ok(json!({"wrote": bytes.len()}))
}

fn byte_list(v: Option<&Value>) -> Option<Vec<u8>> {
    let arr = v?.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for n in arr {
        let b = n.as_u64()?;
        if b > 255 {
            return None;
        }
        out.push(b as u8);
    }
    Some(out)
}

fn key(session: &mut Session, value: &Value) -> String {
    let Some(index) = value.get("index").and_then(|v| v.as_i64()) else {
        return fail(copy::text(Lang::En, "key_limit"));
    };
    if !(0..=2).contains(&index) {
        return fail(copy::text(Lang::En, "key_limit"));
    }
    let down = value.get("down").and_then(|v| v.as_bool()).unwrap_or(false);
    session.key(index as usize, down).map_err(|e| e).ok();
    ok(json!({"index": index, "down": down}))
}

fn patch_rom(session: &mut Session, value: &Value) -> String {
    let Some(offset) = value.get("offset").and_then(as_u64_num) else {
        return fail("patch_rom needs an offset");
    };
    let Some(bytes) = byte_list(value.get("bytes")) else {
        return fail("patch_rom needs bytes");
    };
    let end = offset.saturating_add(bytes.len() as u64);
    if end > tg18::FLASH_SIZE as u64 {
        return fail("patch stays inside the 8 MiB flash image");
    }
    let addr = tg18::FLASH_BASE + offset as u32;
    session.machine.sys.poke(addr, &bytes);
    ok(json!({"patched": bytes.len()}))
}

fn export_rom(session: &Session, value: &Value) -> String {
    let Some(path) = value.get("path").and_then(|v| v.as_str()) else {
        return fail("export_rom needs a path");
    };
    if let Err(e) = std::fs::write(path, session.image.as_slice()) {
        return fail(&e.to_string());
    }
    ok(json!({"path": path, "bytes": session.image.len()}))
}

fn reg_index(v: &Value) -> Option<usize> {
    if let Some(n) = v.as_u64() {
        if n <= 15 {
            return Some(n as usize);
        }
    }
    if let Some(s) = v.as_str() {
        let s = s.trim().to_ascii_lowercase();
        let rest = s.strip_prefix('r')?;
        let n: usize = rest.parse().ok()?;
        if n <= 15 {
            return Some(n);
        }
    }
    None
}

fn register(session: &mut Session, value: &Value) -> String {
    let Some(index) = value.get("index").and_then(reg_index) else {
        return fail("register index is r0–r15");
    };
    let Some(reg) = value.get("value").and_then(as_u32) else {
        return fail("register needs a value");
    };
    session.machine.cpu.r[index] = reg;
    ok(json!({"index": index, "value": reg}))
}

fn bus_write(session: &mut Session, value: &Value) -> String {
    let Some(address) = value.get("address").and_then(as_u32) else {
        return fail("bus_write needs an address");
    };
    let Some(reg) = value.get("value").and_then(as_u32) else {
        return fail("bus_write needs a value");
    };
    if session::span_ok(address, 4) {
        session.machine.sys.poke(address, &reg.to_le_bytes());
    } else {
        session.machine.sys.regs.insert(address, reg);
    }
    ok(json!({"address": address, "value": reg}))
}

fn clock(session: &mut Session, value: &Value) -> String {
    let Some(seconds) = value.get("seconds").and_then(|v| v.as_f64()) else {
        return fail(copy::text(Lang::En, "rtc_limit"));
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return fail(copy::text(Lang::En, "rtc_limit"));
    }
    let ticks = seconds * 32768.0;
    if ticks > ((1u64 << 48) - 1) as f64 {
        return fail(copy::text(Lang::En, "rtc_limit"));
    }
    session.machine.sys.rtc.base_ticks = ticks as i64;
    session.machine.sys.rtc.base_time = session.machine.sys.game_time;
    ok(json!({"seconds": seconds}))
}

fn clock_scale(session: &mut Session, value: &Value) -> String {
    let Some(scale) = value.get("scale").and_then(|v| v.as_f64()) else {
        return fail(copy::text(Lang::En, "scale_limit"));
    };
    if !scale.is_finite() || scale <= 0.0 {
        return fail(copy::text(Lang::En, "scale_limit"));
    }
    session.machine.sys.turbo = scale;
    ok(json!({"scale": scale}))
}

fn bounded_run(session: &mut Session, value: &Value, cmd: &str) -> String {
    let Some(seconds) = value.get("seconds").and_then(|v| v.as_f64()) else {
        return fail(copy::text(Lang::En, "advance_limit"));
    };
    if !seconds.is_finite() || seconds < 0.0 || seconds > 60.0 {
        return fail(copy::text(Lang::En, "advance_limit"));
    }
    if !session.playable {
        return fail(copy::text(Lang::En, "err_unsupported"));
    }
    let code = if cmd == "advance" {
        session.step(seconds)
    } else {
        session.step(seconds)
    };
    if code < 0 {
        return fail(&session.error);
    }
    ok(json!({"code": code, "seconds": seconds}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_and_a_ram_roundtrip() {
        assert_eq!(
            control_json(None, None),
            r#"{"ok":false,"error":"null session or request"}"#
        );
        let mut s = Session::bare(vec![1, 2, 3, 4]);
        assert!(control_json(Some(&mut s), Some("")).contains("null session or request"));
        assert!(control_json(Some(&mut s), None).contains("null session or request"));

        let too_long = control_json(
            Some(&mut s),
            Some(r#"{"cmd":"read","address":536870912,"length":65537}"#),
        );
        assert!(too_long.contains("read is at most 65536 bytes"));
        let adv = control_json(Some(&mut s), Some(r#"{"cmd":"advance","seconds":60.1}"#));
        assert!(adv.contains("advance is at most 60 seconds"));
        let scale = control_json(Some(&mut s), Some(r#"{"cmd":"clock_scale","scale":0}"#));
        assert!(scale.contains("clock scale must be finite and positive"));
        let scale = control_json(Some(&mut s), Some(r#"{"cmd":"clock_scale","scale":null}"#));
        assert!(scale.contains("clock scale must be finite and positive"));
        let rtc = control_json(Some(&mut s), Some(r#"{"cmd":"clock","seconds":-1}"#));
        assert!(rtc.contains("RTC value stays in a 48-bit range"));
        let huge = (1u64 << 48) as f64;
        let rtc = control_json(
            Some(&mut s),
            Some(&format!(r#"{{"cmd":"clock","seconds":{huge}}}"#)),
        );
        assert!(rtc.contains("RTC value stays in a 48-bit range"));
        let key = control_json(Some(&mut s), Some(r#"{"cmd":"key","index":3,"down":true}"#));
        assert!(key.contains("key index is 0, 1, or 2"));

        let wrote = control_json(
            Some(&mut s),
            Some(r#"{"cmd":"write","address":536870912,"bytes":[9,8,7,6]}"#),
        );
        assert!(wrote.contains("\"ok\":true"));
        let read = control_json(
            Some(&mut s),
            Some(r#"{"cmd":"read","address":536870912,"length":4}"#),
        );
        assert!(read.contains("[9,8,7,6]"));

        let reg = control_json(
            Some(&mut s),
            Some(r#"{"cmd":"register","index":"r14","value":42}"#),
        );
        assert!(reg.contains("\"ok\":true"));
        assert_eq!(s.machine.cpu.r[14], 42);
        control_json(Some(&mut s), Some(r#"{"cmd":"cpsr","value":19}"#));
        assert_eq!(s.machine.cpu.cpsr & 0x1F, 19);
        control_json(Some(&mut s), Some(r#"{"cmd":"pause","paused":true}"#));
        assert!(s.control_paused);
        let inspected = control_json(Some(&mut s), Some(r#"{"cmd":"inspect"}"#));
        assert!(inspected.contains("\"paused\":true"));
        assert!(inspected.contains("Fairy"));

        let path = crate::tmpdir("ctl").join("rom.bin");
        let exp = control_json(
            Some(&mut s),
            Some(&format!(
                r#"{{"cmd":"export_rom","path":"{}"}}"#,
                path.display()
            )),
        );
        assert!(exp.contains("\"ok\":true"), "{exp}");
        assert_eq!(std::fs::read(&path).unwrap(), vec![1, 2, 3, 4]);

        // A non-positive run is rejected before the CPU starts.
        let before = s.machine.executed;
        let run = control_json(Some(&mut s), Some(r#"{"cmd":"run","seconds":61}"#));
        assert!(run.contains("advance is at most 60 seconds"));
        assert_eq!(s.machine.executed, before);
    }
}
