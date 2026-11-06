//! App-container paths, ROM import, and the current-image pointer.
//!
//! `firmware-v1.bin` is the only ROM that lives. Sibling save folders are
//! never walked and never deleted by a current-stem operation.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::copy::{self, Lang};
use crate::rom::{self, ImageClass, ROM_BYTES};
use crate::zipimp::{self, ZipError};

pub const IMPORT_CAP: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ImportReport {
    pub stems: Vec<String>,
    pub current: Option<String>,
    pub notes: Vec<String>,
}

pub struct Container {
    pub root: PathBuf,
}

impl Container {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn firmware(&self) -> PathBuf {
        self.root.join("firmware-v1.bin")
    }
    pub fn roms(&self) -> PathBuf {
        self.root.join("ROMs")
    }
    pub fn saves(&self) -> PathBuf {
        self.root.join("Saves")
    }
    pub fn care(&self) -> PathBuf {
        self.root.join("Care")
    }
    pub fn shelf_backups(&self) -> PathBuf {
        self.root.join("ShelfBackups")
    }
    pub fn shelf(&self) -> PathBuf {
        self.root.join("TamaShelf")
    }
    pub fn imports(&self) -> PathBuf {
        self.shelf().join("imports")
    }
    pub fn settings(&self) -> PathBuf {
        self.root.join("settings")
    }
    pub fn care_plan(&self) -> PathBuf {
        self.root.join("care_plan")
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn incoming(&self) -> PathBuf {
        self.cache().join("incoming.t18s")
    }
    pub fn outgoing(&self) -> PathBuf {
        self.cache().join("outgoing.t18s")
    }
    pub fn pending_shelf(&self) -> PathBuf {
        self.shelf_backups().join("before-shelf.t18s.pending")
    }
    pub fn final_shelf(&self) -> PathBuf {
        self.shelf_backups().join("before-shelf.t18s")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [
            self.roms(),
            self.saves(),
            self.care(),
            self.shelf_backups(),
            self.imports(),
            self.cache(),
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

pub fn has_current_rom(root: &Path) -> bool {
    fs::metadata(root.join("firmware-v1.bin"))
        .map(|m| m.len() == ROM_BYTES as u64)
        .unwrap_or(false)
}

pub fn empty_library_copy() -> &'static str {
    copy::text(Lang::En, "no_rom_imported")
}

pub fn missing_rom_copy() -> &'static str {
    copy::text(Lang::En, "import_rom_first")
}

/// One selected ROM becomes current. Several are stored for the person to pick.
pub fn selection_becomes_current(count: usize) -> bool {
    count == 1
}

/// Same stem, right size: keep the file already on disk.
pub fn should_keep_local(existing_len: u64, existing_stem: Option<&str>, new_stem: &str) -> bool {
    existing_len == ROM_BYTES as u64 && existing_stem == Some(new_stem)
}

pub fn accept_stem(seen: &mut HashSet<String>, stem: &str) -> bool {
    seen.insert(stem.to_string())
}

pub fn import_len_ok(len: u64) -> bool {
    len <= IMPORT_CAP
}

fn en(key: &str) -> String {
    copy::text(Lang::En, key).to_string()
}

pub fn import_file(root: &Path, path: &Path) -> Result<ImportReport, String> {
    let file = File::open(path).map_err(|_| en("import_failed_again"))?;
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    import_reader(root, file, len)
}

pub fn import_bytes(root: &Path, data: &[u8]) -> Result<ImportReport, String> {
    import_reader(root, std::io::Cursor::new(data), data.len() as u64)
}

fn import_reader(root: &Path, mut reader: impl Read, len: u64) -> Result<ImportReport, String> {
    let _ = Container::new(root).ensure();
    let mut magic = [0u8; 4];
    let n = reader
        .read(&mut magic)
        .map_err(|_| en("import_failed_again"))?;
    if n == 0 {
        return Err(en("choose_rom"));
    }
    if zipimp::is_zip(&magic[..n]) {
        if len > zipimp::ROM_MAX_BYTES {
            return Err(en("zip_too_large"));
        }
        let mut data = Vec::with_capacity(len.min(zipimp::ROM_MAX_BYTES) as usize);
        data.extend_from_slice(&magic[..n]);
        reader
            .read_to_end(&mut data)
            .map_err(|_| en("cannot_read_zip"))?;
        return import_zip(root, &data);
    }
    let chained = std::io::Cursor::new(magic[..n].to_vec()).chain(reader);
    match rom::read_candidate(chained) {
        Ok(buf) => store_one_image(root, &buf, true),
        Err(ImageClass::NotImage) => Err(en("choose_rom")),
        Err(ImageClass::Unsupported) => Err(en("choose_rom")),
        Err(_) => Err(en("choose_rom")),
    }
}

fn import_zip(root: &Path, data: &[u8]) -> Result<ImportReport, String> {
    let entries = zipimp::list_zip(data).map_err(zip_msg)?;
    zipimp::check_limits(
        &entries,
        zipimp::ROM_MAX_ENTRIES,
        zipimp::ROM_MAX_BYTES,
        Some(zipimp::ROM_MAX_BINS),
    )
    .map_err(zip_msg)?;
    let mut kept: Vec<(String, Vec<u8>)> = Vec::new();
    let mut seen = HashSet::new();
    for ent in &entries {
        if zipimp::skip_rom_junk(&ent.name) || zipimp::escapes(&ent.name) {
            continue;
        }
        if !zipimp::is_bin_name(&ent.name) || ent.uncompressed_size != ROM_BYTES as u64 {
            continue;
        }
        let bytes = zipimp::extract(data, ent).map_err(zip_msg)?;
        match rom::classify(&bytes) {
            ImageClass::Supported { stem, .. } => {
                if accept_stem(&mut seen, &stem) {
                    kept.push((stem, bytes));
                }
            }
            ImageClass::Unsupported | ImageClass::NotImage => continue,
        }
    }
    if kept.is_empty() {
        return Err(en("zip_no_rom"));
    }
    finalize(root, &kept)
}

fn zip_msg(err: ZipError) -> String {
    match err {
        ZipError::TooLarge => en("zip_too_large"),
        ZipError::CannotRead => en("cannot_read_zip"),
    }
}

fn store_one_image(root: &Path, bytes: &[u8], make_current: bool) -> Result<ImportReport, String> {
    let stem = match rom::classify(bytes) {
        ImageClass::Supported { stem, .. } => stem,
        ImageClass::NotImage => {
            return Err(rom::native_error(&ImageClass::NotImage)
                .unwrap_or("choose_rom")
                .to_string())
        }
        ImageClass::Unsupported => {
            return Err(en("choose_rom"));
        }
    };
    let mut items = vec![(stem, bytes.to_vec())];
    if !make_current {
        items.clear();
    }
    // `make_current` false is unused. One raw image always becomes current.
    let _ = items;
    finalize(
        root,
        &[(
            match rom::classify(bytes) {
                ImageClass::Supported { stem, .. } => stem,
                _ => return Err(en("choose_rom")),
            },
            bytes.to_vec(),
        )],
    )
}

fn finalize(root: &Path, items: &[(String, Vec<u8>)]) -> Result<ImportReport, String> {
    let _ = Container::new(root).ensure();
    let mut notes = Vec::new();
    let mut stems = Vec::new();
    for (stem, bytes) in items {
        let dest = root.join("ROMs").join(format!("{stem}.bin"));
        let existing_len = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        let existing_stem = if existing_len == ROM_BYTES as u64 {
            fs::read(&dest).ok().and_then(|b| match rom::classify(&b) {
                ImageClass::Supported { stem, .. } => Some(stem),
                _ => None,
            })
        } else {
            None
        };
        if should_keep_local(existing_len, existing_stem.as_deref(), stem) {
            notes.push(en("local_rom_kept"));
        } else {
            write_atomic(&dest, bytes)?;
        }
        stems.push(stem.clone());
    }
    let current = if selection_becomes_current(items.len()) {
        retain_current(root)?;
        write_atomic(&root.join("firmware-v1.bin"), &items[0].1)?;
        Some(items[0].0.clone())
    } else {
        None
    };
    Ok(ImportReport {
        stems,
        current,
        notes,
    })
}

/// If the current image has a name, copy it into `ROMs/` before replacing it.
pub fn retain_current(root: &Path) -> Result<(), String> {
    let fw = root.join("firmware-v1.bin");
    if fs::metadata(&fw)
        .map(|m| m.len() == ROM_BYTES as u64)
        .unwrap_or(false)
    {
        let bytes = fs::read(&fw).map_err(|e| e.to_string())?;
        if let ImageClass::Supported { stem, .. } = rom::classify(&bytes) {
            let dest = root.join("ROMs").join(format!("{stem}.bin"));
            if !dest.exists() {
                let _ = fs::create_dir_all(root.join("ROMs"));
                fs::copy(&fw, &dest).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Library rows: 8 MiB files under `ROMs/` with a supported name, sorted.
pub fn list_library(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let dir = root.join("ROMs");
    let Ok(rd) = fs::read_dir(&dir) else {
        return names;
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if fs::metadata(&path)
            .map(|m| m.len() == ROM_BYTES as u64)
            .unwrap_or(false)
        {
            if let Ok(bytes) = fs::read(&path) {
                if let ImageClass::Supported { stem, .. } = rom::classify(&bytes) {
                    names.push(stem);
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Delete save files that belong to one stem. Other stems stay.
pub fn remove_stem_saves(saves: &Path, stem: &str) -> std::io::Result<()> {
    let dir = saves.join(stem);
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

pub fn save_names(stem: &str) -> [&'static str; 5] {
    let _ = stem;
    [
        "autosave.t18s",
        "game.flash",
        "game.flash.json",
        "mixing-return.t18s",
        "before_import.t18s",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_inputs_do_not_replace_the_current_image() {
        let root = crate::tmpdir("lib");
        let c = Container::new(&root);
        c.ensure().unwrap();
        fs::write(c.firmware(), b"KEEP").unwrap();

        let err = import_bytes(&root, b"nope").unwrap_err();
        assert_eq!(err, "choose a supported Meets / On ROM.");
        assert_eq!(fs::read(c.firmware()).unwrap(), b"KEEP");

        let err = import_bytes(&root, b"PK\x03\x04not-a-real-zip").unwrap_err();
        assert_eq!(err, "cannot read this ZIP");
        assert_eq!(fs::read(c.firmware()).unwrap(), b"KEEP");

        let err = import_bytes(&root, &zipimp::empty_zip()).unwrap_err();
        assert_eq!(err, "ZIP contains no supported ROM");

        let mut names = Vec::new();
        for i in 0..33 {
            names.push(format!("rom{i}.bin"));
        }
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let err = import_bytes(&root, &zipimp::zip_names(&refs, 8_388_608)).unwrap_err();
        assert_eq!(err, "ZIP too large or too many files");
        assert_eq!(fs::read(c.firmware()).unwrap(), b"KEEP");

        assert!(!has_current_rom(&root));
        assert_eq!(empty_library_copy(), "no ROM imported yet");
        assert_eq!(missing_rom_copy(), "Import a ROM first.");
        assert!(selection_becomes_current(1));
        assert!(!selection_becomes_current(2));
        assert!(should_keep_local(
            ROM_BYTES as u64,
            Some("fw_tg18_en_v057_2019-05-13_fairy"),
            "fw_tg18_en_v057_2019-05-13_fairy"
        ));
        assert!(!should_keep_local(4, Some("fw"), "fw"));
        assert!(import_len_ok(IMPORT_CAP));
        assert!(!import_len_ok(IMPORT_CAP + 1));

        let saves = c.saves();
        fs::create_dir_all(saves.join("other")).unwrap();
        fs::write(saves.join("other/autosave.t18s"), b"other-rom").unwrap();
        fs::create_dir_all(saves.join("mine")).unwrap();
        fs::write(saves.join("mine/autosave.t18s"), b"mine").unwrap();
        fs::write(saves.join("mine/game.flash"), b"flash").unwrap();
        remove_stem_saves(&saves, "mine").unwrap();
        assert!(!saves.join("mine").exists());
        assert_eq!(
            fs::read(saves.join("other/autosave.t18s")).unwrap(),
            b"other-rom"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn supported_import_if_rom_present() {
        let Some(path) = std::env::var_os("POCKET_MEETS_ROM") else {
            eprintln!("skip: POCKET_MEETS_ROM is not set");
            return;
        };
        let root = crate::tmpdir("librom");
        let report = import_file(&root, Path::new(&path)).expect("import");
        assert!(report.current.is_some());
        assert!(has_current_rom(&root));
        let stem = report.current.unwrap();
        assert!(root.join("ROMs").join(format!("{stem}.bin")).exists());
        assert_eq!(list_library(&root), vec![stem]);
        let _ = fs::remove_dir_all(&root);
    }
}
