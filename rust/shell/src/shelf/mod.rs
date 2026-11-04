//! Tama Shelf. Edits go through `control` pause / write / register / cpsr.
//! Magic Hut is marriage copy, not a tab.

pub mod custom;
pub mod marriage;
pub mod pack;
pub mod slots;
pub mod tamago;

use std::fs;
use std::path::Path;

use crate::copy::{self, Lang};
use crate::link;

pub use marriage::{locate_firmware, Choices, Edit, MarriagePlan, Sigs};
pub use slots::MemWrite;

pub fn refuse(link_state: i32, catching_up: bool, paused: bool) -> Option<&'static str> {
    if link::blocks_edit(link_state) {
        return Some(copy::text(Lang::En, "mixing_busy"));
    }
    if catching_up {
        return Some(copy::text(Lang::En, "shelf_blocked"));
    }
    if paused {
        return Some(copy::text(Lang::En, "resume_pet_first"));
    }
    None
}

/// `name` must contain the substring `Sanrio` (case-sensitive).
pub fn undo_allowed(name: &str) -> Result<(), &'static str> {
    if name.contains("Sanrio") {
        Ok(())
    } else {
        Err(copy::text(Lang::En, "sanrio_first"))
    }
}

/// Write the pre-edit snapshot, run `body`, rename on success.
/// On failure the pending bytes are returned so the caller can import them.
/// The pending file is removed either way.
pub fn transact(pending: &Path, final_path: &Path, snapshot: &[u8], body: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    if let Some(parent) = pending.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(pending, snapshot).map_err(|e| e.to_string())?;
    let result = body();
    if result.is_ok() {
        if let Some(parent) = final_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::rename(pending, final_path).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_file(pending);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_undo_gate_and_backup_files() {
        assert_eq!(refuse(2, false, false), Some("finish or cancel Mixing Meets first"));
        assert_eq!(refuse(0, true, false), Some("Shelf blocked."));
        assert_eq!(refuse(0, false, true), Some("resume your pet first"));
        assert_eq!(refuse(0, false, false), None);
        assert_eq!(refuse(5, false, false), None);

        assert!(undo_allowed("Sanrio (JP v056)").is_ok());
        assert_eq!(undo_allowed("sanrio"), Err("Switch to Sanrio first"));
        assert_eq!(undo_allowed("Fairy (EN v057)"), Err("Switch to Sanrio first"));

        let root = crate::tmpdir("shelf");
        let pending = root.join("ShelfBackups/before-shelf.t18s.pending");
        let final_path = root.join("ShelfBackups/before-shelf.t18s");
        let sibling = root.join("Saves/other/autosave.t18s");
        fs::create_dir_all(sibling.parent().unwrap()).unwrap();
        fs::write(&sibling, b"keep").unwrap();

        transact(&pending, &final_path, b"snap-ok", || Ok(())).unwrap();
        assert_eq!(fs::read(&final_path).unwrap(), b"snap-ok");
        assert!(!pending.exists());
        assert_eq!(fs::read(&sibling).unwrap(), b"keep");

        let err = transact(&pending, &final_path, b"snap-bad", || Err("boom".into())).unwrap_err();
        assert_eq!(err, "boom");
        assert!(!pending.exists());
        assert_eq!(fs::read(&final_path).unwrap(), b"snap-ok");
        assert_eq!(fs::read(&sibling).unwrap(), b"keep");
        let _ = fs::remove_dir_all(&root);
    }
}
