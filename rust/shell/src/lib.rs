//! Pocket Meets shell. The CPU is the vendored MIT core; this crate is the
//! care, library, link, and shelf policy around it.
//!
//! The C ABI is not thread-safe. Call it from one serial queue.

pub mod appearance;
pub mod care;
pub mod control;
pub mod copy;
pub mod ffi;
pub mod forecast;
pub mod gatt;
pub mod identity;
pub mod input;
pub mod keypatch;
pub mod library;
pub mod link;
pub mod mixing;
pub mod notify;
pub mod rom;
pub mod session;
pub mod settings;
pub mod shelf;
pub mod snapshot_io;
pub mod textbuf;
pub mod menu_text;
pub mod tick;
pub mod zipimp;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Unique directory for tests. The caller may delete it.
pub fn tmpdir(name: &str) -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let p = std::env::temp_dir().join(format!("pocket-meets-{name}-{nanos}-{n}"));
    std::fs::create_dir_all(&p).expect("tmpdir");
    p
}
