//! The database fixture every suite that writes to tables shares.
//!
//! `tests/schema.rs` and `tests/song_sections.rs` each carried a copy of
//! [`TempDb`]; `song_sections.rs` named the third copy as the point at which
//! duplication stops being cheaper than a shared module, and left the
//! extraction to whichever of FR-201/202/204 landed first. FR-204 is that item,
//! and this is that module.
//!
//! ── Why a subdirectory, and how `dead_code` is settled ──────────────────
//!
//! Cargo builds a separate test binary for every `tests/*.rs`, and each one is
//! its own crate. A shared helper therefore has to be `mod`-included into each
//! suite — which means it is compiled once per suite, and anything a given
//! suite does not call is `dead_code` there. Under
//! `clippy --all-targets -- -D warnings` that is a build failure, not a
//! warning, and it is the reason the extraction was deferred twice.
//!
//! Two parts settle it, and both matter:
//!
//! * **The file lives in `tests/common/`, not at `tests/common.rs`.** Cargo's
//!   integration-test autodiscovery takes `tests/*.rs` and `tests/*/main.rs`;
//!   a subdirectory holding only `mod.rs` is neither, so this file is never
//!   built as a test binary of its own. At `tests/common.rs` it would be, and
//!   cargo would report a suite with zero tests beside the real ones.
//! * **This module holds exactly one item, and all three suites use every part
//!   of it.** `TempDb::new` and `TempDb::open` are both called by
//!   `schema.rs`, `song_sections.rs` and `song_arrangements.rs`, and `Drop`
//!   is never dead. So there is no `#![allow(dead_code)]` here and none is
//!   needed. **That is a constraint on what may be added, not a happy
//!   accident**: the first helper that only one suite calls makes every other
//!   suite fail to compile under `-D warnings`. Add such a helper to the suite
//!   that wants it, or add it here only once a second suite calls it.
//!
//! ── Three properties that must survive any edit ─────────────────────────
//!
//! * **A real file under the system temp directory, never `:memory:`.**
//!   `db::open` sets `journal_mode = WAL`, which an in-memory database does
//!   not honour; a suite on `:memory:` would be exercising a connection the
//!   application never has.
//! * **`Drop` removes the file *and* its `-wal`/`-shm`/`-journal` siblings.**
//!   Nothing any suite writes is inside the repository, so no artefact of a
//!   test run is ever a candidate for the `.gitignore` patterns SETUP-06
//!   added.
//! * **`TempDb` is declared before the `Connection` in every test body.**
//!   Locals drop in reverse declaration order, so the connection must close
//!   before the guard unlinks the file. On Windows that is not a nicety:
//!   unlinking a file that still has an open handle **fails silently** here —
//!   `remove_file`'s error is discarded — and the guard would leave the file
//!   behind while reporting success. Keep the two lines in that order.

use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeroworship_core::db::open_and_migrate;
use rusqlite::Connection;

/// A migrated database in a temporary file, removed when the guard drops.
pub struct TempDb {
    path: PathBuf,
}

impl TempDb {
    /// A fresh, unused path. `tag` only makes the file recognisable while a
    /// debugger is stopped; uniqueness comes from the process id, the clock
    /// and the counter, so two suites may use the same tag.
    pub fn new(tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_nanos();
        let path = temp_dir().join(format!(
            "aeroworship-test-{tag}-{}-{nanos}-{n}.db",
            std::process::id()
        ));
        Self { path }
    }

    /// Opens the path through the application's own entry point, migrations
    /// and pragmas included.
    pub fn open(&self) -> Connection {
        open_and_migrate(&self.path).expect("a fresh temp path should migrate cleanly")
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut os = self.path.as_os_str().to_owned();
            os.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(os));
        }
    }
}
