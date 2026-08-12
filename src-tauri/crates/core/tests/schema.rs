//! Whole-schema behaviour that only exists once migration 001 has run end to
//! end: FTS5 availability, the Appendix A table/index/trigger counts, the
//! absence of the illustrative `trg_sections_ai`, and both directions of
//! ADR-0026's `default_arrangement_id` INSERT guard.
//!
//! Exercised through `aeroworship_core::db`'s public surface only —
//! `open_and_migrate` plus raw SQL on the `Connection` it returns — the same
//! surface an application gets. Nothing here reaches into `pub(crate)` items,
//! so it lives as a Cargo integration test under `tests/` rather than inside
//! the crate.

use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeroworship_core::db::open_and_migrate;
use rusqlite::Connection;

/// A real file on disk, not `:memory:` — see the same note in
/// `src/db/connection.rs`'s test module for why that distinction matters
/// here. Cleaned up on drop, including the `-wal`/`-shm` siblings, so nothing
/// from this suite is ever a candidate for the `.gitignore` artefact patterns
/// SETUP-06 added; it never writes inside the repository at all.
struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_nanos();
        let path = temp_dir().join(format!(
            "aeroworship-schema-test-{tag}-{}-{nanos}-{n}.db",
            std::process::id()
        ));
        Self { path }
    }

    fn open(&self) -> Connection {
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

fn extended_code(err: &rusqlite::Error) -> Option<i32> {
    match err {
        rusqlite::Error::SqliteFailure(e, _) => Some(e.extended_code),
        _ => None,
    }
}

fn insert_song(
    conn: &Connection,
    id: &str,
    default_arrangement_id: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO songs (id, title, default_arrangement_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![id, format!("song {id}"), default_arrangement_id],
    )
}

// ─────────────────────────────────────────────────────────────
// P1.5 — FTS5
// ─────────────────────────────────────────────────────────────

/// `rusqlite` 0.40 has no `fts5` feature to gate this on (see the comment in
/// `crates/core/Cargo.toml`); the only guard that exists is that migration
/// 001's `CREATE VIRTUAL TABLE … USING fts5` runs at all, and this exercises
/// what it created past creation, into an actual insert and `MATCH` query.
#[test]
fn fts5_tables_exist_and_accept_inserts_and_match_queries() {
    let db = TempDb::new("fts5");
    let conn = db.open();

    conn.execute(
        "INSERT INTO songs_fts (song_id, title, alternate_title, authors, body) \
         VALUES ('song-1', 'Amazing Grace', NULL, 'John Newton', 'amazing grace how sweet')",
        [],
    )
    .expect("songs_fts must accept an insert");

    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM songs_fts WHERE songs_fts MATCH 'amazing'",
            [],
            |row| row.get(0),
        )
        .expect("songs_fts must be queryable with MATCH");
    assert_eq!(hits, 1);

    conn.execute(
        "INSERT INTO verses_fts (version_id, book_id, chapter, verse, text) \
         VALUES ('KJV', 43, 3, 16, 'For God so loved the world')",
        [],
    )
    .expect("verses_fts must accept an insert");

    let verse_hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM verses_fts WHERE verses_fts MATCH 'loved'",
            [],
            |row| row.get(0),
        )
        .expect("verses_fts must be queryable with MATCH");
    assert_eq!(verse_hits, 1);
}

// ─────────────────────────────────────────────────────────────
// P1.6 — schema shape
// ─────────────────────────────────────────────────────────────

/// Locks the counts a schema change should have to move deliberately.
///
/// Table count includes FTS5's own shadow tables (`_data`, `_idx`,
/// `_docsize`, `_config` per virtual table), which is why it is measured
/// rather than derived from counting `CREATE TABLE` statements in the SQL
/// text — that shadow-table count is not written anywhere in Appendix A.
/// Index count filters out SQLite's automatic `sqlite_autoindex_*` entries
/// (`sql IS NULL`) so only the 8 explicit `CREATE INDEX` statements are
/// counted. Table/index/trigger *names* are not enumerated in full — Appendix
/// A is the source of truth this migration is diffed against directly, and a
/// 21-table name list here would duplicate it without catching more than the
/// count already does. The four named anchors below are the ones a typo could
/// slip past the count check alone: joining on a misspelled FTS5 table name
/// still leaves the total unchanged.
#[test]
fn schema_shape_matches_appendix_a() {
    let db = TempDb::new("shape");
    let conn = db.open();

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        table_count, 33,
        "21 base tables + 2 × 6 FTS5-related entries (each virtual table itself plus the 5 shadow \
         tables SQLite creates for a non-external-content fts5 table: _data, _idx, _docsize, \
         _config, _content — verified against a plain build of SQLite's own fts5, not assumed) \
         — if this moves, confirm it is an intended schema change and not a drifted FTS5 \
         shadow-table shape before updating the number"
    );

    let explicit_index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND sql IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(explicit_index_count, 8);

    let trigger_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trigger_count, 2);

    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");

    for name in ["songs", "schema_migrations", "songs_fts", "verses_fts"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "expected table {name} to exist");
    }
}

// ─────────────────────────────────────────────────────────────
// P1.7 — trg_sections_ai deliberately not implemented
// ─────────────────────────────────────────────────────────────

/// Appendix A's own implementation note calls `trg_sections_ai` illustrative,
/// and `'rebuild-song'` is not a real FTS5 command — the migration's header
/// comment documents that implementing it as written would make every insert
/// into `song_sections` fail with `SQL logic error`. This checks both halves
/// of that decision: the trigger genuinely is not there, and the insert it
/// would have broken genuinely succeeds.
#[test]
fn trg_sections_ai_is_absent_and_song_sections_inserts_succeed() {
    let db = TempDb::new("sections");
    let conn = db.open();

    let trigger_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = 'trg_sections_ai'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trigger_exists, 0);

    insert_song(&conn, "song-sec", None).unwrap();
    conn.execute(
        "INSERT INTO song_sections (id, song_id, label, section_type, content, sort_order) \
         VALUES ('sec-1', 'song-sec', 'Verse 1', 'verse', 'line one', 0)",
        [],
    )
    .expect(
        "song_sections insert must succeed now that trg_sections_ai (which would have raised \
         'SQL logic error' on every insert) was not implemented",
    );
}

// ─────────────────────────────────────────────────────────────
// P0.3 — ADR-0026, both trigger directions
// ─────────────────────────────────────────────────────────────

#[test]
fn insert_with_default_arrangement_belonging_to_another_song_is_rejected() {
    let db = TempDb::new("insert-other-song");
    let conn = db.open();

    insert_song(&conn, "song-a", None).unwrap();
    conn.execute(
        "INSERT INTO song_arrangements (id, song_id, name, created_at) \
         VALUES ('arr-a', 'song-a', 'Default', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    let err = insert_song(&conn, "song-c", Some("arr-a"))
        .expect_err("song-c must not be allowed to be born pointing at song-a's arrangement");
    assert_eq!(extended_code(&err), Some(1811));
    assert!(err
        .to_string()
        .contains("default_arrangement_id must belong to this song"));
}

/// The structural reason a self-referencing INSERT is impossible:
/// `song_arrangements.song_id` references `songs(id)`, so no arrangement can
/// be created for a song that has not been inserted yet.
#[test]
fn arrangement_cannot_be_created_before_its_song_exists() {
    let db = TempDb::new("fk-order");
    let conn = db.open();

    let err = conn
        .execute(
            "INSERT INTO song_arrangements (id, song_id, name, created_at) \
             VALUES ('arr-x', 'song-not-yet-born', 'Default', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect_err("song_arrangements.song_id references songs(id)");
    assert_eq!(extended_code(&err), Some(787));
}

/// Consequence of the FK ordering above: even a `default_arrangement_id`
/// meant to name the row's own future arrangement is rejected on INSERT,
/// because no arrangement row for it can possibly exist yet. This is the
/// "also rejected" half ADR-0026 calls out explicitly — not a side effect,
/// the only possible outcome.
#[test]
fn insert_with_a_default_arrangement_id_meant_to_be_its_own_is_also_rejected() {
    let db = TempDb::new("insert-self");
    let conn = db.open();

    let err = insert_song(&conn, "song-d", Some("would-be-arr-d"))
        .expect_err("no arrangement row for song-d's own id can exist before song-d does");
    assert_eq!(extended_code(&err), Some(1811));
}

/// The legal flow ADR-0026 describes: INSERT with NULL, create the
/// arrangement, then UPDATE. The pre-existing `BEFORE UPDATE` trigger must
/// keep allowing this — zero regression from the new `BEFORE INSERT` trigger.
#[test]
fn the_legal_three_step_flow_succeeds() {
    let db = TempDb::new("legal-flow");
    let conn = db.open();

    insert_song(&conn, "song-e", None).expect("step 1: insert with NULL default_arrangement_id");
    conn.execute(
        "INSERT INTO song_arrangements (id, song_id, name, created_at) \
         VALUES ('arr-e', 'song-e', 'Default', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("step 2: create the arrangement now that the song exists");
    conn.execute(
        "UPDATE songs SET default_arrangement_id = 'arr-e' WHERE id = 'song-e'",
        [],
    )
    .expect("step 3: point the song at its own arrangement");

    let recorded: String = conn
        .query_row(
            "SELECT default_arrangement_id FROM songs WHERE id = 'song-e'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(recorded, "arr-e");
}

/// Regression guard for the older `BEFORE UPDATE` trigger, unchanged by
/// ADR-0026: a song still cannot be pointed at another song's arrangement via
/// UPDATE.
#[test]
fn update_to_another_songs_arrangement_is_still_rejected() {
    let db = TempDb::new("update-regression");
    let conn = db.open();

    insert_song(&conn, "song-a", None).unwrap();
    conn.execute(
        "INSERT INTO song_arrangements (id, song_id, name, created_at) \
         VALUES ('arr-a', 'song-a', 'Default', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    insert_song(&conn, "song-b", None).unwrap();

    let err = conn
        .execute(
            "UPDATE songs SET default_arrangement_id = 'arr-a' WHERE id = 'song-b'",
            [],
        )
        .expect_err("song-b must not be allowed to point at song-a's arrangement");
    assert_eq!(extended_code(&err), Some(1811));
}
