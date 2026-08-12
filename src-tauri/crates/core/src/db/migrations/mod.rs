//! Schema migrations.
//!
//! **Shape of a migration.** One numbered `.sql` file per version, next to this
//! module and pulled in with [`include_str!`]. SQL stays SQL, so it diffs
//! directly against Appendix A and reads the way the specification is written;
//! embedding it means a release carries its schema inside the binary, with no
//! file to install alongside it and nothing to go missing on a user's machine
//! (NFR-30).
//!
//! **Why the whole of Appendix A is version 1.** The appendix is a single
//! specification of a single schema; splitting it into several migrations would
//! invent boundaries the PRD does not draw and force every later reader to
//! reassemble them. Everything that changes the schema after this gets its own
//! numbered file — migration files are append-only, exactly like `decisions.md`,
//! because a database in the field has already run the old text.
//!
//! **Ordering and idempotency.** [`MIGRATIONS`] is checked at compile time to
//! start at 1 and to increase strictly, so ordering is not something the runner
//! has to trust. [`migrate`] applies only versions above the one recorded in
//! the file, which makes a second run a no-op; the statements themselves are
//! deliberately *not* `IF NOT EXISTS`, so a database whose bookkeeping says
//! "not applied" while the tables already exist fails loudly instead of being
//! quietly papered over.
//!
//! **Transactions.** One transaction per migration, containing the DDL, the
//! `schema_migrations` row and the `user_version` bump together — so the
//! recorded version and the schema it describes can never be half a step apart.
//! Not one transaction for all of them: a database that stopped at version 3 of
//! 5 is a coherent state the next run continues from. Note that
//! `PRAGMA journal_mode = WAL` cannot run inside a transaction at all, which is
//! one of the reasons Appendix A's opening pragmas live in
//! [`super::connection`] instead.

use rusqlite::Connection;

use super::connection;
use super::error::DbError;

/// One numbered schema change.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Version this migration brings the database to. Recorded in
    /// `schema_migrations.version` and in `PRAGMA user_version`.
    pub version: u32,
    /// Human-readable name, used in nothing but error messages and the file
    /// name.
    pub name: &'static str,
    /// The statements, run as one batch inside one transaction.
    pub sql: &'static str,
}

/// Every migration this build knows, in ascending order.
///
/// Append only. Never renumber, never edit an entry that has shipped.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    sql: include_str!("001_initial_schema.sql"),
}];

// Ordering is a compile error rather than a runtime check, so a badly numbered
// migration cannot reach a user's database.
const _: () = {
    assert!(
        !MIGRATIONS.is_empty(),
        "MIGRATIONS must not be empty: latest_version() would report 0 and every \
         database would look already-migrated"
    );
    assert!(
        MIGRATIONS[0].version == 1,
        "the first migration must be version 1: a fresh database starts at 0"
    );
    let mut i = 1;
    while i < MIGRATIONS.len() {
        assert!(
            MIGRATIONS[i - 1].version < MIGRATIONS[i].version,
            "MIGRATIONS must be sorted by strictly increasing version"
        );
        i += 1;
    }
};

/// Highest version this build can bring a database to.
pub const fn latest_version() -> u32 {
    // The entries are known-sorted by the assertion above, so the last one is
    // the highest.
    MIGRATIONS[MIGRATIONS.len() - 1].version
}

/// Reads the schema version recorded in `conn`.
///
/// Returns 0 for a database no migration has touched.
///
/// Two records are kept and both are consulted. `schema_migrations` is the one
/// Appendix A specifies; `PRAGMA user_version` is a copy of the same number in
/// the file header, which exists even before any table does and so can be read
/// on a brand-new file without a "no such table" error to disambiguate. They
/// are written in the same transaction and therefore cannot drift, so a
/// disagreement means something else wrote to the file and is reported rather
/// than resolved.
pub fn current_version(conn: &Connection) -> Result<u32, DbError> {
    let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    let recorded: u32 = if table_exists(conn, "schema_migrations")? {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?
    } else {
        0
    };

    if user_version != recorded {
        return Err(DbError::InconsistentVersion {
            user_version,
            recorded,
        });
    }

    Ok(user_version)
}

/// Brings `conn` up to [`latest_version`], applying whatever is missing.
///
/// Running this on an already-current database does nothing and is not an
/// error. Returns the version the database is at afterwards.
///
/// Fails with [`DbError::FutureSchema`] if the file records a version this
/// build does not know — a database touched by a newer AeroWorship. It is
/// refused with that message rather than opened and allowed to fail later as a
/// missing column, which is the stance FR-708 sets for `.aero` files; a
/// database is worth more than a session file, not less.
pub fn migrate(conn: &mut Connection) -> Result<u32, DbError> {
    // Refuse to write through a connection that did not come from
    // `super::open` — without `foreign_keys = ON` the DDL below declares
    // constraints that nothing enforces.
    connection::verify_pragmas(conn)?;

    let current = current_version(conn)?;
    let latest = latest_version();

    if current > latest {
        return Err(DbError::FutureSchema {
            found: current,
            supported: latest,
        });
    }

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        // `strftime` rather than a clock crate: the timestamp is ISO-8601 UTC
        // per Appendix A, and SQLite already knows how to produce one.
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at)
             VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [migration.version],
        )?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }

    Ok(latest)
}

/// Whether a table of this name exists. Virtual tables count as
/// `type = 'table'` for `sqlite_master`, same as ordinary ones, so
/// `songs_fts`/`verses_fts` are included; views do not — they carry
/// `type = 'view'` — and so are already excluded by the `type = 'table'`
/// filter below without needing a name check of their own.
fn table_exists(conn: &Connection, name: &str) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// See `connection::tests::TempDbPath` for why this is a real file rather
    /// than `:memory:`. Duplicated here rather than shared: the two test
    /// modules are compiled as separate `#[cfg(test)]` units with no existing
    /// shared test-support module to hang a common helper from, and one is not
    /// worth inventing for fifteen lines.
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
                "aeroworship-migrations-test-{tag}-{}-{nanos}-{n}.db",
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

    /// P0.4 — a database no migration has touched reports version 0 without
    /// tripping the "no such table" path `current_version` guards against:
    /// `schema_migrations` does not exist yet on a fresh file.
    #[test]
    fn current_version_is_zero_on_a_fresh_database() {
        let tmp = TempDbPath::new("fresh");
        let conn = connection::open(&tmp.path).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    /// P0.4 — running `migrate` twice is a no-op the second time, and the two
    /// version records (`PRAGMA user_version`, `MAX(schema_migrations.version)`)
    /// agree with each other and with `latest_version()` after each run.
    #[test]
    fn migrate_twice_is_idempotent_and_the_two_version_records_agree() {
        let tmp = TempDbPath::new("idempotent");
        let mut conn = connection::open(&tmp.path).unwrap();

        let first = migrate(&mut conn).unwrap();
        assert_eq!(first, latest_version());

        let user_version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let recorded: u32 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(user_version, recorded);
        assert_eq!(user_version, latest_version());

        let second = migrate(&mut conn).unwrap();
        assert_eq!(second, latest_version());

        let applied_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            applied_rows,
            MIGRATIONS.len() as i64,
            "a second migrate() run must not re-insert bookkeeping rows"
        );
    }

    /// P0.4 — `PRAGMA user_version` and `schema_migrations` are written in the
    /// same transaction, so they should never disagree on their own; a
    /// deliberate mismatch is reported rather than silently resolved one way
    /// or the other.
    #[test]
    fn inconsistent_version_between_the_two_records_is_reported_with_both_numbers() {
        let tmp = TempDbPath::new("inconsistent");
        let mut conn = connection::open(&tmp.path).unwrap();
        migrate(&mut conn).unwrap();

        conn.pragma_update(None, "user_version", 99u32).unwrap();

        match current_version(&conn) {
            Err(DbError::InconsistentVersion {
                user_version,
                recorded,
            }) => {
                assert_eq!(user_version, 99);
                assert_eq!(recorded, 1);
            }
            other => panic!("expected InconsistentVersion, got {other:?}"),
        }
    }

    /// P0.4 — a `user_version` higher than this build's `latest_version()` is
    /// refused explicitly (FR-708's stance, applied to the database), rather
    /// than being opened and left to fail later as a missing column.
    #[test]
    fn a_user_version_higher_than_latest_version_is_refused_as_future_schema() {
        let tmp = TempDbPath::new("future");
        let mut conn = connection::open(&tmp.path).unwrap();
        migrate(&mut conn).unwrap();

        // Bump both records together, consistently, to a version this build
        // does not know — simulating a database written by a newer
        // AeroWorship rather than one merely out of sync with itself.
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) \
             VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 2u32).unwrap();

        match migrate(&mut conn) {
            Err(DbError::FutureSchema { found, supported }) => {
                assert_eq!(found, 2);
                assert_eq!(supported, latest_version());
            }
            other => panic!("expected FutureSchema, got {other:?}"),
        }
    }

    /// P3 — regression guard for the doc comment on `table_exists`, which
    /// claims views count as `type = 'table'` in `sqlite_master`. Measured
    /// against the SQLite this build actually bundles, that is false: a
    /// `CREATE VIEW` row carries `type = 'view'`, not `'table'`. The
    /// function's own behaviour is unaffected either way — it already filters
    /// on `type = 'table'` and so already excludes views — so this is not a
    /// functional defect, only the doc comment overstating what the code has
    /// to rely on to be correct.
    #[test]
    fn table_exists_is_true_for_a_table_and_false_for_a_view() {
        let tmp = TempDbPath::new("table-exists");
        let conn = connection::open(&tmp.path).unwrap();
        conn.execute_batch(
            "CREATE TABLE probe_table (x);
             CREATE VIEW probe_view AS SELECT * FROM probe_table;",
        )
        .unwrap();

        assert!(table_exists(&conn, "probe_table").unwrap());
        assert!(!table_exists(&conn, "probe_view").unwrap());
    }
}
