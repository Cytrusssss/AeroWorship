//! The one way a SQLite connection is opened.
//!
//! Appendix A opens with four `PRAGMA` statements, and only the first of them
//! is persistent:
//!
//! | Pragma | Scope |
//! | --- | --- |
//! | `journal_mode = WAL` | written into the file header, inherited by every later connection |
//! | `foreign_keys = ON` | **per connection** |
//! | `synchronous = NORMAL` | **per connection** |
//! | `cache_size = -8000` | **per connection** |
//!
//! Running the four once, in the migration, would look right there and be
//! silently off everywhere else. So they live here instead, in the function
//! every connection is created by, and each one is read back afterwards —
//! SQLite answers a pragma it cannot honour by reporting the value still in
//! force, not by raising an error, so setting without checking proves nothing.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::error::DbError;

/// Page-cache ceiling. Negative means KiB rather than pages, so this is 8 MB —
/// the figure Appendix A ties to the memory budget in NFR-01.
const CACHE_SIZE: i64 = -8000;

/// `PRAGMA synchronous = NORMAL` as SQLite reports it back (NFR-11).
const SYNCHRONOUS_NORMAL: i64 = 1;

/// Flags [`open`] passes to `Connection::open_with_flags`, in place of
/// `Connection::open`'s default set.
///
/// `READ_WRITE`, `CREATE` and `NO_MUTEX` are the three flags this crate
/// actually relies on: a missing file must get created, and rusqlite's
/// compile-time thread-safety guarantees are documented as depending on
/// `NO_MUTEX` (see `Connection::open`'s own docs, which this mirrors).
///
/// `SQLITE_OPEN_URI` — the fourth flag in `Connection::open`'s default — is
/// deliberately left out. With it set, a path string beginning with `file:`
/// is parsed as a URI and query parameters such as `?mode=ro`,
/// `?immutable=1` or `?vfs=…` become live syntax rather than characters in a
/// filename. Nothing in this crate builds a path that way today — [`open`]'s
/// caller always passes a compile-time constant — so this changes no
/// observable behaviour right now (measured, not assumed: see the doc on
/// [`open`] for what was checked). It closes the route before any future
/// caller assembles a path from something less fixed, rather than waiting
/// for that to happen and become a bug report. Do not switch this back to
/// `Connection::open` to "simplify" it without re-adding `SQLITE_OPEN_URI`
/// on purpose and re-reading this comment first.
const OPEN_FLAGS: OpenFlags = OpenFlags::from_bits_truncate(
    OpenFlags::SQLITE_OPEN_READ_WRITE.bits()
        | OpenFlags::SQLITE_OPEN_CREATE.bits()
        | OpenFlags::SQLITE_OPEN_NO_MUTEX.bits(),
);

/// Opens `path`, creating the file if it does not exist, and applies the
/// Appendix A pragmas to the new connection.
///
/// The parent directory is *not* created: where the data root lives is the
/// shell's business, resolved through the Tauri path API (PRD §6.9). This
/// module never guesses a path.
///
/// This is deliberately the only public constructor. The shell crate does not
/// depend on `rusqlite` at all, so it has no way to obtain a `Connection`
/// except through here — the same structural trick as the crate split itself
/// (ADR-0008), rather than a rule someone has to remember.
///
/// SQLite's usual special filenames still apply: `":memory:"` yields a private
/// in-memory database and an empty path a temporary one. Neither supports WAL,
/// and both report `journal_mode = memory`, which [`verify_pragmas`] accepts
/// for that reason. Neither depends on `SQLITE_OPEN_URI` either: opened with
/// [`OPEN_FLAGS`] below, which omits that flag, `":memory:"` still reports
/// `journal_mode = memory` and an empty path still accepts a `CREATE TABLE`.
/// Checked directly against a real connection rather than assumed from the
/// SQLite docs alone.
pub fn open(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open_with_flags(path, OPEN_FLAGS)?;
    apply_pragmas(&conn)?;
    verify_pragmas(&conn)?;
    Ok(conn)
}

/// Sets the four Appendix A pragmas.
fn apply_pragmas(conn: &Connection) -> Result<(), DbError> {
    // `journal_mode` returns the resulting mode as a row, so it cannot go
    // through `pragma_update`, which expects no result. It also cannot run
    // inside a transaction — which is why it is here and not in a migration.
    let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", CACHE_SIZE)?;

    Ok(())
}

/// Reads the four pragmas back and fails unless every one of them is in force.
///
/// Called on every freshly opened connection, and again by
/// [`super::migrate`] — so a connection built by some other route cannot be
/// migrated through.
pub(crate) fn verify_pragmas(conn: &Connection) -> Result<(), DbError> {
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    // `memory` is the honest answer for an in-memory or temporary database,
    // where WAL does not exist. A file-backed database that could not be moved
    // to WAL reports the mode it kept (`delete`, `truncate`, …), never
    // `memory`, so accepting it here does not weaken the check.
    if !journal_mode.eq_ignore_ascii_case("wal") && !journal_mode.eq_ignore_ascii_case("memory") {
        return Err(DbError::PragmaNotApplied {
            pragma: "journal_mode",
            expected: "wal".to_owned(),
            actual: journal_mode,
        });
    }

    let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys != 1 {
        return Err(DbError::PragmaNotApplied {
            pragma: "foreign_keys",
            expected: "1".to_owned(),
            actual: foreign_keys.to_string(),
        });
    }

    let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
    if synchronous != SYNCHRONOUS_NORMAL {
        return Err(DbError::PragmaNotApplied {
            pragma: "synchronous",
            expected: SYNCHRONOUS_NORMAL.to_string(),
            actual: synchronous.to_string(),
        });
    }

    let cache_size: i64 = conn.query_row("PRAGMA cache_size", [], |row| row.get(0))?;
    if cache_size != CACHE_SIZE {
        return Err(DbError::PragmaNotApplied {
            pragma: "cache_size",
            expected: CACHE_SIZE.to_string(),
            actual: cache_size.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real file on disk, not `:memory:`. `verify_pragmas` deliberately
    /// accepts `journal_mode = memory` for an in-memory or temporary database,
    /// so a suite that only ever opened `:memory:` connections would never
    /// exercise the WAL assertion Appendix A actually requires. Cleaned up on
    /// drop, including SQLite's `-wal`/`-shm` siblings, so nothing here can
    /// become the leaked artefact SETUP-06's `.gitignore` guard has to catch —
    /// it never needs to, because this never writes inside the repo.
    struct TempDbPath {
        path: std::path::PathBuf,
    }

    impl TempDbPath {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before 1970")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "aeroworship-connection-test-{tag}-{}-{nanos}-{n}.db",
                std::process::id()
            ));
            Self { path }
        }
    }

    impl Drop for TempDbPath {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm", "-journal"] {
                let mut os = self.path.as_os_str().to_owned();
                os.push(suffix);
                let _ = std::fs::remove_file(std::path::PathBuf::from(os));
            }
        }
    }

    /// P0.1 — the four Appendix A pragmas as [`open`] actually leaves them,
    /// read back on the connection it returns.
    #[test]
    fn open_applies_all_four_appendix_a_pragmas() {
        let tmp = TempDbPath::new("pragmas");
        let conn = open(&tmp.path).expect("open() verifies its own pragmas before returning");

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_eq!(synchronous, SYNCHRONOUS_NORMAL);

        let cache_size: i64 = conn
            .query_row("PRAGMA cache_size", [], |row| row.get(0))
            .unwrap();
        assert_eq!(cache_size, CACHE_SIZE);
    }

    /// P0.2 — the guard carries real weight for three of the four pragmas, and
    /// is measured (not assumed) to be a no-op for the fourth. A connection
    /// opened through raw `rusqlite::Connection::open`, bypassing this
    /// module's `open` entirely, does *not* pick up `synchronous = NORMAL` or
    /// `cache_size = -8000` — those settings really do depend on this guard
    /// running. `foreign_keys` happens to read back `1` anyway, because
    /// `libsqlite3-sys`'s bundled SQLite amalgamation is compiled with
    /// `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`. That is not this module's doing; if
    /// this assertion ever fails, the bundled default changed and the pragma
    /// guard has quietly become the only thing keeping `foreign_keys` on too.
    #[test]
    fn raw_connection_misses_two_pragmas_and_only_coincidentally_keeps_foreign_keys() {
        let tmp = TempDbPath::new("raw");
        let conn = Connection::open(&tmp.path).expect("raw rusqlite open");

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            journal_mode.to_ascii_lowercase(),
            "delete",
            "SQLite's own default rollback mode; if this changed, re-check whether WAL is now the \
             library default and this test's premise needs revisiting"
        );

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_ne!(
            synchronous, SYNCHRONOUS_NORMAL,
            "a raw connection must not already sit at synchronous=NORMAL — otherwise this test \
             cannot tell the guard apart from doing nothing"
        );

        let cache_size: i64 = conn
            .query_row("PRAGMA cache_size", [], |row| row.get(0))
            .unwrap();
        assert_ne!(
            cache_size, CACHE_SIZE,
            "a raw connection must not already sit at the 8 MB ceiling — otherwise this test \
             cannot tell the guard apart from doing nothing"
        );

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            foreign_keys, 1,
            "libsqlite3-sys's bundled build is expected to default this to ON; if it reads 0, the \
             build flag changed and every REFERENCES/ON DELETE CASCADE clause in Appendix A is \
             unenforced on any connection that skips super::connection::open"
        );
    }

    /// P0.2 (continued) — `verify_pragmas` is not decorative: each of the four
    /// checks actually rejects a connection whose pragma was turned off after
    /// the fact, with the specific pragma named in the error.
    #[test]
    fn verify_pragmas_rejects_foreign_keys_turned_off() {
        let tmp = TempDbPath::new("reject-fk");
        let conn = open(&tmp.path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();

        match verify_pragmas(&conn) {
            Err(DbError::PragmaNotApplied {
                pragma,
                expected,
                actual,
            }) => {
                assert_eq!(pragma, "foreign_keys");
                assert_eq!(expected, "1");
                assert_eq!(actual, "0");
            }
            other => panic!("expected PragmaNotApplied, got {other:?}"),
        }
    }

    #[test]
    fn verify_pragmas_rejects_synchronous_turned_off() {
        let tmp = TempDbPath::new("reject-sync");
        let conn = open(&tmp.path).unwrap();
        conn.execute_batch("PRAGMA synchronous = OFF").unwrap();

        match verify_pragmas(&conn) {
            Err(DbError::PragmaNotApplied {
                pragma,
                expected,
                actual,
            }) => {
                assert_eq!(pragma, "synchronous");
                assert_eq!(expected, SYNCHRONOUS_NORMAL.to_string());
                assert_eq!(actual, "0");
            }
            other => panic!("expected PragmaNotApplied, got {other:?}"),
        }
    }

    #[test]
    fn verify_pragmas_rejects_a_cache_size_that_drifted() {
        let tmp = TempDbPath::new("reject-cache");
        let conn = open(&tmp.path).unwrap();
        conn.execute_batch("PRAGMA cache_size = -2000").unwrap();

        match verify_pragmas(&conn) {
            Err(DbError::PragmaNotApplied {
                pragma,
                expected,
                actual,
            }) => {
                assert_eq!(pragma, "cache_size");
                assert_eq!(expected, CACHE_SIZE.to_string());
                assert_eq!(actual, "-2000");
            }
            other => panic!("expected PragmaNotApplied, got {other:?}"),
        }
    }

    #[test]
    fn verify_pragmas_rejects_a_journal_mode_moved_off_wal() {
        let tmp = TempDbPath::new("reject-journal");
        let conn = open(&tmp.path).unwrap();
        let _: String = conn
            .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
            .unwrap();

        match verify_pragmas(&conn) {
            Err(DbError::PragmaNotApplied {
                pragma,
                expected,
                actual,
            }) => {
                assert_eq!(pragma, "journal_mode");
                assert_eq!(expected, "wal");
                assert_eq!(actual.to_ascii_lowercase(), "delete");
            }
            other => panic!("expected PragmaNotApplied, got {other:?}"),
        }
    }
}
