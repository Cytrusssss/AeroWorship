//! SQLite storage: migrations, queries and the FTS index (PRD §6.13).
//!
//! Nothing here reaches for an application path. Where the file lives is the
//! shell's business — it resolves it through the Tauri path API (PRD §6.9) and
//! hands it in — which is what lets the whole of this layer be exercised
//! against a temporary file with no Tauri runtime anywhere near it (NFR-32,
//! GATE-G10).
//!
//! Two entry points, and there is a reason there are not more:
//!
//! - [`open`] is the only way a connection is created, so the per-connection
//!   pragmas Appendix A requires cannot be skipped.
//! - [`open_and_migrate`] is what an application start-up wants: open, bring
//!   the schema up to date, hand back a usable connection.
//!
//! `queries/` from PRD §6.13 exists as of FR-203, which opened the first write
//! path into the tables; it holds one module per aggregate and grows as the
//! FR-2xx items reach the rest of Appendix A. `fts/` is still absent, and
//! deliberately: it belongs to FR-201, together with the `reindex_song` routine
//! that replaces Appendix A's illustrative FTS trigger (see the head of
//! `migrations/001_initial_schema.sql`). Songs written before that item lands
//! are not in the index and FR-201 owes them a backfill.

mod connection;
mod error;
mod migrations;
pub mod queries;

use std::path::Path;

use rusqlite::Connection;

pub use connection::open;
pub use error::DbError;
pub use migrations::{current_version, latest_version, migrate, Migration, MIGRATIONS};

/// Opens the database at `path` and applies any migrations it is missing.
///
/// The file and its `-wal`/`-shm` siblings are created if absent; the parent
/// directory is not, since the caller is the one that knows where the data root
/// is.
pub fn open_and_migrate(path: &Path) -> Result<Connection, DbError> {
    let mut conn = open(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}
