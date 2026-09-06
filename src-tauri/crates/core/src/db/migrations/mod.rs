use rusqlite::Connection;

use super::connection;
use super::error::DbError;

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    sql: include_str!("001_initial_schema.sql"),
}];

const _: () = {
    assert!(
        !MIGRATIONS.is_empty(),
        "MIGRATIONS must not be empty: latest_version() would report 0 and every \
         database would look already-migrated"
    );
    assert!(
        MIGRATIONS[0].version == 1,
    MIGRATIONS[MIGRATIONS.len() - 1].version
}

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

pub fn migrate(conn: &mut Connection) -> Result<u32, DbError> {
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

    #[test]
    fn current_version_is_zero_on_a_fresh_database() {
        let tmp = TempDbPath::new("fresh");
        let conn = connection::open(&tmp.path).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

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

    #[test]
    fn inconsistent_version_between_the_two_records_is_reported_with_both_numbers() {
        let tmp = TempDbPath::new("inconsistent");
        let mut conn = connection::open(&tmp.path).unwrap();
        migrate(&mut conn).unwrap();

        conn.pragma_update(None, "user_version", 99u32).unwrap();

        match current_version(&conn) {
            Err(DbError::InconsistentVersion {
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

    #[test]
    fn a_user_version_higher_than_latest_version_is_refused_as_future_schema() {
        let tmp = TempDbPath::new("future");
        let mut conn = connection::open(&tmp.path).unwrap();
        migrate(&mut conn).unwrap();

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
