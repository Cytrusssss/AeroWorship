//! Errors raised by the storage layer.
//!
//! Spelled out by hand rather than derived: a `thiserror` dependency would buy
//! a `Display` impl for four variants at the cost of two more crates in the
//! installer budget (NFR-16).

use std::error::Error;
use std::fmt;

/// Anything that can go wrong opening or migrating the database.
#[derive(Debug)]
pub enum DbError {
    /// SQLite itself refused.
    Sqlite(rusqlite::Error),

    /// A `PRAGMA` was set but did not read back with the expected value.
    ///
    /// This is not paranoia. `foreign_keys`, `synchronous` and `cache_size` are
    /// per-connection settings, and SQLite answers a `PRAGMA` it cannot honour
    /// by reporting the value still in force rather than by raising an error —
    /// so the only way to know one applied is to read it back. A connection
    /// running with `foreign_keys = OFF` enforces none of the `REFERENCES` or
    /// `ON DELETE CASCADE` clauses in the schema while looking perfectly
    /// healthy.
    PragmaNotApplied {
        /// Name of the pragma, as it appears in Appendix A.
        pragma: &'static str,
        /// What Appendix A requires.
        expected: String,
        /// What the connection actually reports.
        actual: String,
    },

    /// The database was written by a build that knows more migrations than
    /// this one does.
    ///
    /// Refused explicitly instead of being allowed to surface later as a
    /// missing column or a constraint failure — the stance FR-708 sets for
    /// `.aero` files, applied to the database for the same reason.
    FutureSchema {
        /// Version recorded in the file.
        found: u32,
        /// Highest version this build can apply.
        supported: u32,
    },

    /// `PRAGMA user_version` and `schema_migrations` disagree.
    ///
    /// Both are written inside the same transaction by [`super::migrate`], so
    /// they cannot drift on their own. A mismatch means the file was modified
    /// by something other than this migration runner, and guessing which of
    /// the two to believe would be the wrong kind of resilience.
    InconsistentVersion {
        /// Value of `PRAGMA user_version`.
        user_version: u32,
        /// Highest version recorded in `schema_migrations`.
        recorded: u32,
    },
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "sqlite error: {err}"),
            Self::PragmaNotApplied {
                pragma,
                expected,
                actual,
            } => write!(
                f,
                "PRAGMA {pragma} did not take effect: expected {expected}, connection reports {actual}"
            ),
            Self::FutureSchema { found, supported } => write!(
                f,
                "this database was created by a newer version of AeroWorship \
                 (schema version {found}; this build understands up to {supported}). \
                 Update AeroWorship to open it — it has not been modified."
            ),
            Self::InconsistentVersion {
                user_version,
                recorded,
            } => write!(
                f,
                "database version bookkeeping is inconsistent: \
                 PRAGMA user_version is {user_version} but schema_migrations records {recorded}"
            ),
        }
    }
}

impl Error for DbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(err) => Some(err),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}
