use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::error::DbError;

const CACHE_SIZE: i64 = -8000;

const SYNCHRONOUS_NORMAL: i64 = 1;

const OPEN_FLAGS: OpenFlags = OpenFlags::from_bits_truncate(
    OpenFlags::SQLITE_OPEN_READ_WRITE.bits()
        | OpenFlags::SQLITE_OPEN_CREATE.bits()
        | OpenFlags::SQLITE_OPEN_NO_MUTEX.bits(),
);

pub fn open(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open_with_flags(path, OPEN_FLAGS)?;
    apply_pragmas(&conn)?;
    verify_pragmas(&conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> Result<(), DbError> {
    let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", CACHE_SIZE)?;

    Ok(())
}

pub(crate) fn verify_pragmas(conn: &Connection) -> Result<(), DbError> {
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
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
