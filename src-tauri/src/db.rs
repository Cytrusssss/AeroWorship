//! Shell side of the storage layer: where the database file lives, and making
//! sure it exists and is migrated before anything asks it a question.
//!
//! Everything else about SQLite — the DDL, the migration runner, the
//! per-connection pragmas — is in [`aeroworship_core::db`], which cannot depend
//! on `tauri` (ADR-0008). This crate does not depend on `rusqlite` either, so
//! the only way it can obtain a connection at all is through that module's
//! `open`, and the Appendix A pragmas cannot be bypassed here even by accident.
//!
//! No path is written as a literal. The data root is resolved through the Tauri
//! path API, which is the only thing that knows where `%APPDATA%` actually is
//! on a given machine.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

/// The single data root, as named by PRD §6.9 — `%APPDATA%\AeroWorship` on
/// Windows.
///
/// Note this is deliberately *not* [`app_data_dir`](tauri::path::PathResolver::app_data_dir),
/// which would append the bundle identifier and give
/// `%APPDATA%\id.aeroworship.app`. §6.9 spells the directory out, and NFR-30
/// asks for a root a user can find, copy and restore; the product name serves
/// that better than a reverse-DNS string. The `%APPDATA%` part is still
/// resolved by Tauri — only the leaf is ours.
///
/// **[`data_root`] is the only place this is resolved, and `app_data_dir` must
/// not be called anywhere else in this crate** (ADR-0025). Being off Tauri's
/// default path means any other caller of `app_data_dir` gets a *different*
/// directory, and the two roots then coexist with no symptom whatsoever:
/// nothing errors, nothing warns, both are perfectly valid directories. What
/// the user reports is "my data is gone" while it sits in the folder next door.
/// Anything that needs a file under the data root — templates, media, exports,
/// logs — goes through [`data_root`].
const DATA_ROOT: &str = "AeroWorship";

/// Database file name (PRD §6.9). Its `-wal` and `-shm` siblings sit next to it
/// and are created by SQLite.
const DATABASE_FILE: &str = "aeroworship.db";

/// Absolute path of the data root, whether or not it exists yet.
pub fn data_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, tauri::Error> {
    Ok(app.path().data_dir()?.join(DATA_ROOT))
}

/// Absolute path of the database file, whether or not it exists yet.
pub fn database_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, tauri::Error> {
    Ok(data_root(app)?.join(DATABASE_FILE))
}

/// Creates the data root if needed, then opens and migrates the database.
///
/// Called once from the setup hook. The connection is opened and dropped: the
/// point of this call is that the file exists and its schema is current before
/// the first window appears. How connections are held afterwards — one shared
/// behind a mutex, or one per operation — is a question the first item that
/// actually queries the database gets to answer (FR-2xx), and pre-empting it
/// here would be guessing.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_root(app)?)?;
    aeroworship_core::db::open_and_migrate(&database_path(app)?)?;
    Ok(())
}
