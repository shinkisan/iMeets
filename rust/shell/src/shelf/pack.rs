//! Material packs. `shelf.json` is `tama-shelf` version 1.
//! Failure deletes the new directory and leaves the previous pack.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static PACK_SEQ: AtomicU64 = AtomicU64::new(1);

use serde_json::Value;

use crate::copy::{self, Lang};
use crate::settings::Settings;
use crate::zipimp::{self, ZipError};

pub const TABS: [&str; 6] = ["wallpaper", "food", "item", "accessory", "game", "station"];

pub fn tab_label(lang: Lang, tab: &str) -> &'static str {
    let key = match tab {
        "wallpaper" => "tab_wallpaper",
        "food" => "tab_food",
        "item" => "tab_item",
        "accessory" => "tab_accessory",
        "game" => "tab_game",
        "station" => "tab_station",
        _ => "",
    };
    copy::text(lang, key)
}

#[derive(Clone, Debug)]
pub struct Listed {
    pub name: String,
    pub kind: String,
    pub language: String,
    pub path: String,
    pub id: u32,
    pub price: u32,
}

fn zip_err(e: ZipError) -> String {
    match e {
        ZipError::TooLarge => copy::text(Lang::En, "zip_too_large").into(),
        ZipError::CannotRead => copy::text(Lang::En, "cannot_read_zip").into(),
    }
}

pub fn install_pack(root: &Path, zip: &[u8], settings: &mut Settings) -> Result<String, String> {
    let entries = zipimp::list_zip(zip).map_err(zip_err)?;
    zipimp::check_limits(&entries, zipimp::SHELF_MAX_ENTRIES, zipimp::SHELF_MAX_BYTES, None).map_err(zip_err)?;
    let mut names = Vec::new();
    for e in &entries {
        if zipimp::escapes(&e.name) {
            return Err(copy::text(Lang::En, "cannot_read_zip").into());
        }
        if names.iter().any(|n: &String| n == &e.name) {
            return Err(copy::text(Lang::En, "cannot_read_zip").into());
        }
        names.push(e.name.clone());
    }
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let n = PACK_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir_name = format!("pack-{nanos:x}{n:x}");
    let dest = root.join("TamaShelf").join(&dir_name);
    if let Err(err) = extract_and_check(zip, &entries, &dest) {
        let _ = fs::remove_dir_all(&dest);
        return Err(err);
    }
    let previous = settings.shelf_pack.clone();
    settings.shelf_pack = dir_name.clone();
    settings.shelf_enabled = true;
    if !previous.is_empty() && previous != dir_name {
        let old = root.join("TamaShelf").join(&previous);
        if old.starts_with(root.join("TamaShelf")) && old != dest {
            let _ = fs::remove_dir_all(old);
        }
    }
    Ok(dir_name)
}

fn extract_and_check(zip: &[u8], entries: &[zipimp::ZipMeta], dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for ent in entries {
        if ent.name.ends_with('/') {
            continue;
        }
        let bytes = zipimp::extract(zip, ent).map_err(zip_err)?;
        let rel = PathBuf::from(&ent.name);
        let path = dest.join(&rel);
        if !path.starts_with(dest) {
            return Err(copy::text(Lang::En, "cannot_read_zip").into());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    let text = fs::read_to_string(dest.join("shelf.json")).map_err(|_| "shelf.json is missing".to_string())?;
    let value: Value = serde_json::from_str(&text).map_err(|_| "shelf.json is not JSON".to_string())?;
    validate_manifest(dest, &value)
}

pub fn validate_manifest(dir: &Path, value: &Value) -> Result<(), String> {
    let obj = value.as_object().ok_or_else(|| "shelf.json is not JSON".to_string())?;
    if obj.get("format").and_then(|v| v.as_str()) != Some("tama-shelf") {
        return Err("shelf format must be tama-shelf".into());
    }
    if obj.get("version").and_then(|v| v.as_i64()) != Some(1) {
        return Err("shelf version must be 1".into());
    }
    for key in ["items", "parts"] {
        let arr = obj.get(key).and_then(|v| v.as_array()).ok_or_else(|| format!("{key} must be an array"))?;
        if arr.len() > 5000 {
            return Err(format!("{key} has more than 5000 elements"));
        }
        for el in arr {
            for field in ["path", "thumb", "offset", "table", "data", "english"] {
                if let Some(name) = el.get(field).and_then(|v| v.as_str()) {
                    if name.is_empty() || zipimp::escapes(name) || !dir.join(name).is_file() {
                        return Err(format!("{field} does not name a file in the pack"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Version 1 packs taken from the English firmware can store the name table
/// glyph codes instead of text: letters are ASCII + 0x9F, and four control
/// codes stand for a space. Without this the shelf shows Latin-1 mojibake.
fn decode_name(raw: &str) -> String {
    let high = raw.chars().filter(|c| ('\u{00C0}'..='\u{00FF}').contains(c)).count();
    let len = raw.chars().count();
    let cjk = raw.chars().any(|c| matches!(c as u32, 0x3040..=0x30FF | 0x4E00..=0x9FFF));
    // A single accent is ordinary text; the glyph dump is mostly high bytes.
    if high < 2 || high * 2 < len || cjk {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch as u32 {
            0x5E | 0x63 | 0x6C | 0x71 => out.push(' '),
            0xA0..=0xFF => out.push(char::from_u32(ch as u32 - 0x9F).unwrap_or(ch)),
            _ => out.push(ch),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn list_items(pack_dir: &Path, tab: &str, language: &str) -> Result<Vec<Listed>, String> {
    let text = fs::read_to_string(pack_dir.join("shelf.json")).map_err(|_| copy::text(Lang::En, "load_pack_first").to_string())?;
    let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let items = value.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for el in items {
        let kind = el.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let lang = el.get("language").and_then(|v| v.as_str()).unwrap_or("");
        if kind != tab {
            continue;
        }
        if lang != language && lang != "all" {
            continue;
        }
        out.push(Listed {
            name: decode_name(el.get("name").and_then(|v| v.as_str()).unwrap_or("")),
            kind: kind.to_string(),
            language: lang.to_string(),
            path: el.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            id: el.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            price: el.get("price").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_lists_food_once_and_rolls_back() {
        assert_eq!(TABS.len(), 6);
        assert_eq!(TABS[1], "food");
        assert!(!TABS.contains(&"magic"));
        assert_eq!(tab_label(Lang::Zh, "food"), "食物");
        assert_eq!(tab_label(Lang::En, "station"), "Destinations");

        let root = crate::tmpdir("pack");
        let imports = root.join("TamaShelf/imports");
        fs::create_dir_all(&imports).unwrap();
        fs::write(imports.join("keep.bin"), b"custom").unwrap();

        let manifest = br#"{"format":"tama-shelf","version":1,"items":[
            {"kind":"food","language":"jp","name":"meal","path":"meal.bin"},
            {"kind":"food","language":"en","name":"snack","path":"snack.bin"},
            {"kind":"food","language":"all","name":"both","path":"both.bin"},
            {"kind":"wallpaper","language":"jp","name":"wall","path":"wall.bin"}
        ],"parts":[]}"#;
        let zip = zipimp::zip_stored(&[
            ("shelf.json", manifest),
            ("meal.bin", b"m"),
            ("snack.bin", b"s"),
            ("both.bin", b"b"),
            ("wall.bin", b"w"),
        ]);
        let mut settings = Settings::new_user();
        let name = install_pack(&root, &zip, &mut settings).unwrap();
        assert!(settings.shelf_enabled);
        assert_eq!(settings.shelf_pack, name);
        let listed = list_items(&root.join("TamaShelf").join(&name), "food", "jp").unwrap();
        let names: Vec<_> = listed.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["meal", "both"]);
        assert!(imports.join("keep.bin").exists());

        let bad = zipimp::zip_stored(&[("shelf.json", br#"{"format":"nope","version":1,"items":[],"parts":[]}"#)]);
        let err = install_pack(&root, &bad, &mut settings).unwrap_err();
        assert!(err.contains("tama-shelf"));
        assert_eq!(settings.shelf_pack, name);
        assert!(root.join("TamaShelf").join(&name).join("shelf.json").exists());
        assert!(imports.join("keep.bin").exists());

        let zip2 = zipimp::zip_stored(&[
            ("shelf.json", manifest),
            ("meal.bin", b"m"),
            ("snack.bin", b"s"),
            ("both.bin", b"b"),
            ("wall.bin", b"w"),
        ]);
        let name2 = install_pack(&root, &zip2, &mut settings).unwrap();
        assert_ne!(name2, name);
        assert!(!root.join("TamaShelf").join(&name).exists());
        assert!(imports.join("keep.bin").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn real_shelf_gifts_parse_if_present() {
        let Some(path) = std::env::var_os("POCKET_MEETS_SHELF") else {
            eprintln!("skip: POCKET_MEETS_SHELF is not set");
            return;
        };
        let bytes = fs::read(&path).expect("shelf zip");
        let root = crate::tmpdir("shelf-real");
        let mut settings = Settings::new_user();
        let name = install_pack(&root, &bytes, &mut settings).expect("install");
        let dir = root.join("TamaShelf").join(&name);
        let text = fs::read_to_string(dir.join("shelf.json")).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        let items = value["items"].as_array().expect("items");
        let mut stations = 0usize;
        let mut bad = Vec::new();
        for el in items {
            let rel = el["path"].as_str().unwrap_or("");
            let kind = el["kind"].as_str().unwrap_or("");
            let data = fs::read(dir.join(rel)).unwrap_or_default();
            match crate::shelf::tamago::parse(&data) {
                Ok(t) => {
                    if t.cat.tab() != kind {
                        bad.push(format!("{rel}: kind {kind} parsed as {}", t.cat.tab()));
                    }
                    if kind == "station" {
                        stations += 1;
                    }
                }
                Err(e) => bad.push(format!("{rel}: {e}")),
            }
        }
        assert!(bad.is_empty(), "{} failed:\n{}", bad.len(), bad.join("\n"));
        assert_eq!(stations, 4, "parsed {} of {} items", items.len(), items.len());
        let _ = fs::remove_dir_all(&root);
    }
    #[test]
    fn legacy_english_names_are_decoded() {
        assert_eq!(decode_name("åàí"), "FAN");
        assert_eq!(decode_name("âàóläàñò"), "CAT EARS");
        assert_eq!(decode_name("âçäññèäòc"), "CHERRIES");
        assert_eq!(decode_name("たまハット"), "たまハット");
        assert_eq!(decode_name("STRAWBERRY"), "STRAWBERRY");
        // A Latin-1 name without the glyph range is left alone.
        assert_eq!(decode_name("Café"), "Café");
    }
}
