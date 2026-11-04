//! User-imported wallpapers and characters. They live in `TamaShelf/imports`
//! so replacing a pack does not delete them.
//!
//! TODO(wallpaper-pixels): SPEC §14.14. A wallpaper TAMAGO gets a random id
//! and a valid checksum. Image bytes are placed in the body only when they
//! fit. That is not the in-game pixel encoding.
//! TODO(part-pixels): pose PNGs are stored and not decoded.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::copy::{self, Lang};
use crate::shelf::marriage::{self, CHARACTER_LEN, SEX_OFFSET};
use crate::shelf::tamago::{self, Cat};
use crate::zipimp;

pub fn wallpaper_id(rng: u32) -> u16 {
    50_000 + (rng % 15_000) as u16
}

pub fn make_wallpaper(root: &Path, image: &[u8], rng: u32, name: &str) -> Result<(String, String), String> {
    let id = wallpaper_id(rng);
    // Slot size is 16384. Most photos will not fit; the body is truncated
    // and the result is only a valid container.
    let body_room = 16_384 - 122;
    let body = if image.len() <= body_room { image } else { &[] };
    let bytes = tamago::build(-2_130_239_744, id, 0, 16_384, body);
    let parsed = tamago::parse(&bytes)?;
    assert_eq_kind(&parsed);
    store_import(root, &bytes, "wallpaper", name)
}

fn assert_eq_kind(t: &tamago::Tamago) {
    debug_assert_eq!(t.cat, Cat::Wallpaper);
}

pub fn import_character_file(root: &Path, bytes: &[u8], name: &str) -> Result<String, String> {
    marriage::validate_character(bytes)?;
    let (_id, msg) = store_import(root, bytes, "character", name)?;
    Ok(msg)
}

pub fn import_character_zip(root: &Path, zip: &[u8]) -> Result<String, String> {
    let entries = zipimp::list_zip(zip).map_err(|_| copy::text(Lang::En, "cannot_read_zip").to_string())?;
    let mut json_bytes = None;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for ent in &entries {
        if ent.name.ends_with('/') || zipimp::escapes(&ent.name) {
            continue;
        }
        let data = zipimp::extract(zip, ent).map_err(|_| copy::text(Lang::En, "cannot_read_zip").to_string())?;
        if ent.name.rsplit('/').next() == Some("character.json") {
            json_bytes = Some(data);
        } else {
            files.push((ent.name.clone(), data));
        }
    }
    let Some(json_bytes) = json_bytes else {
        return Err(copy::text(Lang::En, "pose_unknown").into());
    };
    let value: Value = serde_json::from_slice(&json_bytes).map_err(|_| copy::text(Lang::En, "pose_unknown").to_string())?;
    let blob_name = value.get("blob").and_then(|v| v.as_str()).unwrap_or("");
    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("character");
    let blob = files.iter().find(|(n, _)| n == blob_name || n.ends_with(blob_name)).map(|(_, b)| b.clone());
    let Some(blob) = blob else {
        return Err(copy::text(Lang::En, "pose_unknown").into());
    };
    if blob.len() != CHARACTER_LEN || blob.get(SEX_OFFSET).copied().unwrap_or(9) > 1 {
        return Err(copy::text(Lang::En, "pose_unknown").into());
    }
    let dir = root.join("TamaShelf/imports");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = fresh_id();
    fs::write(dir.join(format!("{id}.bin")), &blob).map_err(|e| e.to_string())?;
    for (n, data) in &files {
        if n.to_ascii_lowercase().ends_with(".png") {
            let base = n.rsplit('/').next().unwrap_or(n);
            fs::write(dir.join(format!("{id}-{base}")), data).map_err(|e| e.to_string())?;
        }
    }
    let side = json!({"custom": true, "kind": "character", "name": name, "path": format!("{id}.bin")});
    fs::write(dir.join(format!("{id}.json")), side.to_string()).map_err(|e| e.to_string())?;
    Ok(copy::text(Lang::En, "chest_sentence").into())
}

pub fn import_tamago_file(root: &Path, bytes: &[u8], name: &str) -> Result<tamago::Tamago, String> {
    let t = tamago::parse(bytes)?;
    store_import(root, bytes, t.cat.tab(), name)?;
    Ok(t)
}

fn store_import(root: &Path, bytes: &[u8], kind: &str, name: &str) -> Result<(String, String), String> {
    let dir = root.join("TamaShelf/imports");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = fresh_id();
    let bin = format!("{id}.bin");
    fs::write(dir.join(&bin), bytes).map_err(|e| e.to_string())?;
    let side = json!({"custom": true, "kind": kind, "name": name, "path": bin});
    fs::write(dir.join(format!("{id}.json")), side.to_string()).map_err(|e| e.to_string())?;
    let note = if kind == "wallpaper" {
        copy::text(Lang::En, "wallpaper_encode")
    } else {
        copy::text(Lang::En, "chest_sentence")
    };
    Ok((id, note.to_string()))
}

fn fresh_id() -> String {
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{n:x}")
}

pub fn character_prefs(parts: [u8; 4], colors: &[u8], sex: u8, mode: u8, original: bool, name: &str) -> String {
    json!({
        "parts": parts,
        "colors": colors,
        "sex": sex,
        "mode": mode,
        "original": original,
        "name": name,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_ids_and_imports_are_outside_the_pack() {
        assert_eq!(wallpaper_id(0), 50_000);
        assert_eq!(wallpaper_id(14_999), 64_999);
        assert_eq!(wallpaper_id(15_000), 50_000);
        assert!(wallpaper_id(u32::MAX) <= 64_999);

        let root = crate::tmpdir("custom");
        let (_id, msg) = make_wallpaper(&root, b"not-a-real-photo", 10, "pic").unwrap();
        assert!(msg.contains("pixel encoding is unknown"));
        let bin = fs::read_dir(root.join("TamaShelf/imports")).unwrap().find_map(|e| {
            let p = e.ok()?.path();
            if p.extension().map(|x| x == "bin").unwrap_or(false) { Some(p) } else { None }
        }).unwrap();
        let parsed = tamago::parse(&fs::read(&bin).unwrap()).unwrap();
        assert_eq!(parsed.cat, Cat::Wallpaper);
        assert_eq!(parsed.id, 50_010);

        let mut blob = vec![0u8; CHARACTER_LEN];
        blob[SEX_OFFSET] = 1;
        import_character_file(&root, &blob, "ori").unwrap();

        let side = br#"{"name":"zip","blob":"c.bin"}"#;
        let zip = zipimp::zip_stored(&[("character.json", side), ("c.bin", blob.as_slice()), ("pose.png", b"\x89PNG")]);
        import_character_zip(&root, &zip).unwrap();
        let png_only = zipimp::zip_stored(&[("pose.png", b"\x89PNG")]);
        let err = import_character_zip(&root, &png_only).unwrap_err();
        assert!(err.contains("pose PNG"));

        let meal = tamago::build(-2_130_640_383, 4, 8, 516, &[]);
        let t = import_tamago_file(&root, &meal, "rice").unwrap();
        assert_eq!(t.price, 8);
        assert_eq!(tamago::preview_len(t.cat), Some(516));
        let _ = fs::remove_dir_all(&root);
    }
}
