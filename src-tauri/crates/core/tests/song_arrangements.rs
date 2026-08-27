//! FR-204: *"Songs support named **arrangements**: an ordered sequence of
//! references to that song's sections. A default arrangement is generated on
//! creation."* (PRD §4.2, line 337.)
//!
//! Its acceptance criterion is two clauses, and they are tested by two
//! different means:
//!
//! * *"Selecting an arrangement expands to the correct slide sequence"* —
//!   `expand_arrangement`, asserted on the sequence it returns. It stops at
//!   sections rather than slides on purpose; `models::split_slides` is the sole
//!   authority on where a section breaks (ADR-0046, ADR-0047) and it decides
//!   that from a template's geometry, which the storage layer does not have.
//!   The composition is the caller's, so this suite asserts the sequence of
//!   *sections* and says so rather than quietly redefining the phrase.
//! * *"editing a section's text updates every occurrence"* — asserted by
//!   **decomposition**, not through an update path, because no update path
//!   exists yet: `upsert_song` is FR-202's. See
//!   [`editing_a_sections_text_updates_every_occurrence_and_copies_nothing`]
//!   for the shape and for why its second assertion is the load-bearing one.
//!
//! ── Where the assertions are read from ──────────────────────────────────
//!
//! *"A default arrangement is generated on creation"* is a statement about
//! **storage**, so a return value cannot witness it —
//! `insert_song_with_default_arrangement` returns `()`. Every assertion about
//! what creation wrote therefore goes through raw `SELECT`s against `songs`,
//! `song_arrangements`, `arrangement_items` and `song_sections`. Raw SQL is
//! also how rows no public path can write are put there, which is the only way
//! the read path's defences against a foreign or hand-edited database are
//! reachable at all.
//!
//! ── One measured fact that shapes the ordering tests ────────────────────
//!
//! **Dropping `ORDER BY i.position` from `expand_arrangement` changes nothing
//! observable.** `arrangement_items`' primary key is
//! `(arrangement_id, position)`, so SQLite serves the scan from
//! `sqlite_autoindex_arrangement_items_1` and the rows arrive in `position`
//! order whether or not the clause is written. Measured, not assumed: items
//! inserted in the order 2, 0, 4, 1, 3 come back 0, 1, 2, 3, 4 with the clause
//! deleted.
//!
//! The consequence for anyone adding a test here: **an ordering test built by
//! inserting items in ascending order proves nothing** — it is green under
//! every mutant, including the one with no ordering at all. What the fixtures
//! below do discriminate is the *direction* and the *key*: `ORDER BY
//! i.position DESC` and `ORDER BY s.sort_order` both change the answer, and
//! that is what
//! [`expanding_an_arrangement_materialises_every_repetition_in_position_order`]
//! is built to catch.
//!
//! ── Fixture data ───────────────────────────────────────────────────────
//!
//! `L1`…`L12` and structural labels, no real lyrics: `Verse 1` and `Chorus`
//! are the *names of a song's parts*, not its words. The `TempDb` guard lives
//! in `tests/common/mod.rs`; the rule that it must be declared **before** the
//! `Connection` is stated there and holds in every test body below.

mod common;

use aeroworship_core::db::queries::song::{
    expand_arrangement, insert_arrangement, insert_song, insert_song_with_default_arrangement,
    load_arrangements, load_song, Arrangement, ArrangementItem, SectionType, Song, SongSection,
};
use aeroworship_core::db::DbError;
use common::TempDb;
use rusqlite::Connection;

// ─────────────────────────────────────────────────────────────
// Fixture
// ─────────────────────────────────────────────────────────────

/// Fixed timestamps: nothing in `queries::song` reads a clock, and a fixture
/// that did would make every assertion unrepeatable.
const T0: &str = "2026-01-01T00:00:00Z";

fn song(id: &str, title: &str, sections: Vec<SongSection>) -> Song {
    Song {
        id: id.to_owned(),
        title: title.to_owned(),
        alternate_title: None,
        ccli_number: None,
        copyright_text: None,
        song_key: None,
        tempo_bpm: None,
        default_arrangement_id: None,
        source_provider: None,
        source_url: None,
        retrieved_at: None,
        created_at: T0.to_owned(),
        updated_at: T0.to_owned(),
        deleted_at: None,
        sections,
    }
}

fn section(id: &str, label: &str, section_type: SectionType, content: &str) -> SongSection {
    SongSection {
        id: id.to_owned(),
        label: label.to_owned(),
        section_type,
        content: content.to_owned(),
    }
}

/// An arrangement whose items are the given section ids at positions
/// 0, 1, 2, … — repetitions included, which is the whole point of FR-204.
fn arrangement(id: &str, song_id: &str, name: &str, section_ids: &[&str]) -> Arrangement {
    Arrangement {
        id: id.to_owned(),
        song_id: song_id.to_owned(),
        name: name.to_owned(),
        created_at: T0.to_owned(),
        items: (0i64..)
            .zip(section_ids)
            .map(|(position, section_id)| ArrangementItem {
                position,
                section_id: (*section_id).to_owned(),
            })
            .collect(),
    }
}

/// Reads a single count straight out of the tables.
fn count(conn: &Connection, sql: &str, args: &[&str]) -> i64 {
    conn.query_row(sql, rusqlite::params_from_iter(args), |row| row.get(0))
        .unwrap_or_else(|err| panic!("count query failed: {sql}: {err}"))
}

/// `(position, section_id)` for one arrangement, read from the table rather
/// than from anything the crate returned.
fn stored_items(conn: &Connection, arrangement_id: &str) -> Vec<(i64, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT position, section_id FROM arrangement_items
             WHERE arrangement_id = ?1 ORDER BY position",
        )
        .expect("the items query must prepare");
    let rows = stmt
        .query_map([arrangement_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("the items query must run");
    rows.map(|r| r.expect("an arrangement_items row must read"))
        .collect()
}

fn expanded(conn: &Connection, arrangement_id: &str) -> Vec<SongSection> {
    expand_arrangement(conn, arrangement_id)
        .expect("expand_arrangement must not fail on an arrangement this test just wrote")
        .unwrap_or_else(|| panic!("arrangement {arrangement_id} was written and must expand"))
}

fn labels(sections: &[SongSection]) -> Vec<&str> {
    sections.iter().map(|s| s.label.as_str()).collect()
}

fn contents(sections: &[SongSection]) -> Vec<&str> {
    sections.iter().map(|s| s.content.as_str()).collect()
}

fn ids(sections: &[SongSection]) -> Vec<&str> {
    sections.iter().map(|s| s.id.as_str()).collect()
}

/// SQLite's extended result code, for the tests that assert *which* constraint
/// refused a write rather than merely that one did.
fn sqlite_extended_code(err: &rusqlite::Error) -> Option<i32> {
    match err {
        rusqlite::Error::SqliteFailure(e, _) => Some(e.extended_code),
        _ => None,
    }
}

/// The same, for a refusal that arrived wrapped in [`DbError::Sqlite`].
fn extended_code(err: &DbError) -> Option<i32> {
    match err {
        DbError::Sqlite(inner) => sqlite_extended_code(inner),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────
// "A default arrangement is generated on creation"
// ─────────────────────────────────────────────────────────────

/// FR-204: *"A default arrangement is generated on creation."*
///
/// Every assertion below is a `SELECT`, because the claim is about what is in
/// the database and `insert_song_with_default_arrangement` returns `()`. Four
/// separate facts, each of which a plausible implementation can get wrong on
/// its own:
///
/// * the `song_arrangements` row exists, and is named `Default`;
/// * `songs.default_arrangement_id` **points at it** — the third statement of
///   the three-step flow Appendix A's triggers force, and the one an
///   implementation can silently omit while still writing a perfectly good
///   arrangement that nothing references;
/// * there is one item per section, numbered **from 0**, not from 1 — the
///   module documents that choice as the one that makes `position` and
///   `sort_order` the same number for the same section;
/// * the items are in the song's **authoring order**.
///
/// The three sections are deliberately arranged so that authoring order, id
/// order, label order and reverse order are four *different* sequences: the ids
/// ascend as `sec-1, sec-2, sec-3` while authoring order is
/// `sec-3, sec-1, sec-2`. An implementation that sorted the default arrangement
/// by label, by id, or that built it backwards, produces a different
/// `section_id` column here — which is what a fixture already in sorted order
/// would not have caught.
#[test]
fn creation_writes_a_default_arrangement_named_default_and_points_the_song_at_it() {
    let db = TempDb::new("default-created");
    let mut conn = db.open();

    let s = song(
        "song-1",
        "Song One",
        vec![
            section("sec-3", "Verse 1", SectionType::Verse, "L1"),
            section("sec-1", "Chorus", SectionType::Chorus, "L2"),
            section("sec-2", "Verse 2", SectionType::Verse, "L3"),
        ],
    );
    insert_song_with_default_arrangement(&mut conn, &s, "arr-1", T0)
        .expect("a fresh song and a fresh arrangement id must write");

    assert_eq!(
        stored_items(&conn, "arr-1"),
        vec![
            (0, "sec-3".to_owned()),
            (1, "sec-1".to_owned()),
            (2, "sec-2".to_owned()),
        ],
        "the default arrangement is one item per section, in authoring order, numbered from 0",
    );

    let name: String = conn
        .query_row(
            "SELECT name FROM song_arrangements WHERE id = ?1",
            ["arr-1"],
            |row| row.get(0),
        )
        .expect("the arrangement row must exist");
    assert_eq!(name, "Default");

    let default_id: Option<String> = conn
        .query_row(
            "SELECT default_arrangement_id FROM songs WHERE id = ?1",
            ["song-1"],
            |row| row.get(0),
        )
        .expect("the song row must exist");
    assert_eq!(
        default_id.as_deref(),
        Some("arr-1"),
        "an arrangement nothing points at is not a *default* arrangement",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-1"],
        ),
        3,
        "generating the arrangement must not have duplicated a single section row",
    );
}

/// A song with **no** sections still gets its default arrangement, and it
/// expands to an empty sequence rather than to `None`.
///
/// This is the pair of [`an_unknown_arrangement_id_expands_to_none`], and the
/// pair is the point: `Ok(Some(vec![]))` and `Ok(None)` are two different
/// answers and only one of them means "there is no such arrangement". An
/// implementation that inferred existence from the join — zero item rows means
/// no arrangement — passes the other test and fails this one, having just told
/// its caller that a song it wrote itself has no default arrangement.
///
/// Skipping the write for a sectionless song is the other tempting
/// simplification, and the module says why it is not free: a NULL
/// `default_arrangement_id` would then mean either "written before FR-204" or
/// "had no sections at the time", two facts no later reader can separate.
#[test]
fn a_song_with_no_sections_still_gets_an_empty_default_arrangement() {
    let db = TempDb::new("empty-default");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song("song-2", "Song Two", Vec::new()),
        "arr-2",
        T0,
    )
    .expect("a song with no sections must still be creatable");

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-2"],
        ),
        1,
        "the arrangement row must be written even when it plays nothing",
    );
    assert_eq!(stored_items(&conn, "arr-2"), Vec::<(i64, String)>::new());

    let default_id: Option<String> = conn
        .query_row(
            "SELECT default_arrangement_id FROM songs WHERE id = ?1",
            ["song-2"],
            |row| row.get(0),
        )
        .expect("the song row must exist");
    assert_eq!(default_id.as_deref(), Some("arr-2"));

    assert_eq!(
        expand_arrangement(&conn, "arr-2").expect("expanding an empty arrangement is not an error"),
        Some(Vec::new()),
        "an arrangement that plays nothing is Some([]), never None",
    );
}

/// An arrangement id that was never written is `Ok(None)`.
///
/// Read together with
/// [`a_song_with_no_sections_still_gets_an_empty_default_arrangement`]: alone,
/// either test is satisfied by an implementation that collapses the two cases
/// in the direction the other test forbids.
#[test]
fn an_unknown_arrangement_id_expands_to_none() {
    let db = TempDb::new("unknown-arrangement");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-3",
            "Song Three",
            vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-3",
        T0,
    )
    .unwrap();

    assert_eq!(
        expand_arrangement(&conn, "arr-nowhere")
            .expect("an unknown id is not an error, it is an absence"),
        None,
    );
}

// ─────────────────────────────────────────────────────────────
// "Selecting an arrangement expands to the correct slide sequence"
// ─────────────────────────────────────────────────────────────

/// FR-204: *"Selecting an arrangement expands to the correct slide
/// sequence."*
///
/// Five positions over three sections — Verse 1, Chorus, Verse 2, Chorus,
/// Chorus — so the expansion is longer than the song and every repetition has
/// to be materialised. The assertion is on the whole sequence of labels, of
/// ids *and* of texts, so a result of the right length in the wrong order still
/// fails.
///
/// **This fixture is chosen to discriminate the ordering key and direction,
/// which an ascending fixture cannot.** Removing `ORDER BY i.position`
/// altogether changes nothing — the rows already arrive in that order from the
/// `(arrangement_id, position)` primary-key index (measured; see the head of
/// this file), so no test can catch that edit and none here claims to. What
/// this one does catch:
///
/// * `ORDER BY i.position DESC` — the sequence reverses to
///   Chorus, Chorus, Verse 2, Chorus, Verse 1.
/// * `ORDER BY s.sort_order` — the sections' authoring order is Verse 1 (0),
///   Chorus (1), Verse 2 (2), so ordering by it groups the three choruses
///   together and puts Verse 2 last, whatever SQLite does with the ties. Verse
///   2 moving from the middle to the end is the difference, and it does not
///   depend on the sorter being stable.
///
/// **This stops at sections, not slides**, although the criterion says
/// "slides": where a section breaks is `models::split_slides`'s alone to
/// decide and it needs a template's text-box geometry, which this layer does
/// not have. The sequence asserted here is what the caller then maps through
/// that function, one section at a time.
#[test]
fn expanding_an_arrangement_materialises_every_repetition_in_position_order() {
    let db = TempDb::new("expand-order");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-4",
            "Song Four",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-c", "Chorus", SectionType::Chorus, "L2"),
                section("sec-v2", "Verse 2", SectionType::Verse, "L3"),
            ],
        ),
        "arr-default",
        T0,
    )
    .unwrap();

    insert_arrangement(
        &mut conn,
        &arrangement(
            "arr-full",
            "song-4",
            "Full version",
            &["sec-v1", "sec-c", "sec-v2", "sec-c", "sec-c"],
        ),
    )
    .expect("repeating a section id across positions is what FR-204 is for");

    let sequence = expanded(&conn, "arr-full");
    assert_eq!(
        labels(&sequence),
        vec!["Verse 1", "Chorus", "Verse 2", "Chorus", "Chorus"],
    );
    assert_eq!(
        ids(&sequence),
        vec!["sec-v1", "sec-c", "sec-v2", "sec-c", "sec-c"]
    );
    assert_eq!(contents(&sequence), vec!["L1", "L2", "L3", "L2", "L2"]);

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-4"],
        ),
        3,
        "five positions are still three stored sections (FR-203)",
    );
}

/// The default arrangement expands to the song's sections, in authoring order,
/// exactly once each.
///
/// The two halves of FR-204 meet here: what creation *wrote* — asserted from
/// the tables in
/// [`creation_writes_a_default_arrangement_named_default_and_points_the_song_at_it`]
/// — is what selection *plays*. The song's `default_arrangement_id` is read
/// back from the row rather than hard-coded, so this follows the same pointer
/// an application would.
#[test]
fn the_default_arrangement_expands_to_the_songs_sections_in_authoring_order() {
    let db = TempDb::new("default-expands");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-5",
            "Song Five",
            vec![
                section("sec-3", "Verse 1", SectionType::Verse, "L1"),
                section("sec-1", "Chorus", SectionType::Chorus, "L2"),
                section("sec-2", "Verse 2", SectionType::Verse, "L3"),
            ],
        ),
        "arr-5",
        T0,
    )
    .unwrap();

    let stored = load_song(&conn, "song-5")
        .expect("the song must read back")
        .expect("the song must exist");
    let default_id = stored
        .default_arrangement_id
        .as_deref()
        .expect("creation must have pointed the song at its default arrangement");

    let sequence = expanded(&conn, default_id);
    assert_eq!(labels(&sequence), vec!["Verse 1", "Chorus", "Verse 2"]);
    assert_eq!(
        sequence, stored.sections,
        "the default arrangement plays the song's sections, unchanged and unreordered",
    );
}

// ─────────────────────────────────────────────────────────────
// "Editing a section's text updates every occurrence"
// ─────────────────────────────────────────────────────────────

/// FR-204: *"editing a section's text updates every occurrence."*
///
/// **Proved by decomposition, not through an update path, and deliberately
/// so.** There is no production update path yet — `upsert_song` belongs to
/// FR-202 — and adding one here to make the test look end-to-end would be this
/// suite inventing the code it grades. What the criterion actually asserts is
/// that no occurrence carries a *copy*: change the one stored row and every
/// occurrence must change with it, because every occurrence was only ever a
/// reference. So the edit below is a raw `UPDATE` of that single row, the
/// weakest possible stand-in for whatever FR-202 eventually writes, and the
/// claim survives any update path that does not copy.
///
/// Two assertions, and **the second is the load-bearing one.** The first —
/// every occurrence of the chorus reads back with the new text — is also true
/// of an implementation that stored the chorus three times and dutifully
/// updated all three. The second pins it: after the edit, exactly **one** row
/// in the whole table holds the new text. Zero bytes were copied, so there is
/// no fourth place a future edit could miss. Without it this test proves that
/// `SELECT` returns what was `UPDATE`d, which nobody doubted.
///
/// If you are here because you made this red by adding a cache — a
/// `HashMap<section_id, String>` in front of the join, say — the failure is the
/// point: the module's contract is that expansion re-reads `song_sections` at
/// the moment of the call, and a cache is exactly the copy this criterion
/// forbids, just held in memory instead of on disk.
#[test]
fn editing_a_sections_text_updates_every_occurrence_and_copies_nothing() {
    let db = TempDb::new("edit-propagates");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-6",
            "Song Six",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-c", "Chorus", SectionType::Chorus, "L2"),
                section("sec-v2", "Verse 2", SectionType::Verse, "L3"),
            ],
        ),
        "arr-6-default",
        T0,
    )
    .unwrap();

    // Five positions over three sections: the chorus is sung three times.
    insert_arrangement(
        &mut conn,
        &arrangement(
            "arr-6",
            "song-6",
            "Full version",
            &["sec-v1", "sec-c", "sec-v2", "sec-c", "sec-c"],
        ),
    )
    .unwrap();

    let before = expanded(&conn, "arr-6");
    assert_eq!(
        before.iter().filter(|s| s.id == "sec-c").count(),
        3,
        "the fixture must actually repeat the section it is about to edit",
    );

    const EDITED: &str = "L11\nL12";
    conn.execute(
        "UPDATE song_sections SET content = ?1 WHERE id = ?2",
        rusqlite::params![EDITED, "sec-c"],
    )
    .expect("the edit must apply to the single stored row");

    let after = expanded(&conn, "arr-6");
    assert_eq!(
        contents(&after),
        vec!["L1", EDITED, "L3", EDITED, EDITED],
        "every occurrence of the edited section must carry the new text",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE content = ?1",
            &[EDITED],
        ),
        1,
        "three occurrences, one stored row: the edit reached them all because none was a copy",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-6"],
        ),
        3,
    );

    // The song's *other* arrangement — the default one — sees the same edit,
    // because it references the same row.
    assert_eq!(
        contents(&expanded(&conn, "arr-6-default")),
        vec!["L1", EDITED, "L3"],
        "every arrangement of the song sees the edit, not only the one expanded first",
    );
}

// ─────────────────────────────────────────────────────────────
// Atomicity of the combined path
// ─────────────────────────────────────────────────────────────

/// The song, its sections and its default arrangement land **together or not
/// at all**.
///
/// The failure is arranged through the public API alone: the second song is
/// offered an `arrangement_id` the first song's default arrangement already
/// owns, so its `songs` row and its section rows are written and the
/// `song_arrangements` insert then trips the primary key.
///
/// **`load_song` returning `None` is the assertion that discriminates**, and it
/// is worth saying why the error itself does not. An implementation that wrote
/// the song in one transaction and the arrangement in a second returns exactly
/// the same `Err` here — the arrangement genuinely failed — while leaving
/// behind a titled song with all its lyrics and no default arrangement. That is
/// the state FR-204 cannot describe: nothing marks it,
/// `default_arrangement_id IS NULL` is legal for songs written by
/// `insert_song`, and every later reader carries a case for it forever.
#[test]
fn a_failed_arrangement_write_leaves_no_song_behind() {
    let db = TempDb::new("combined-atomicity");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-7a",
            "Song Seven A",
            vec![section("sec-a1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-shared",
        T0,
    )
    .unwrap();

    let err = insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-7b",
            "Song Seven B",
            vec![
                section("sec-b1", "Verse 1", SectionType::Verse, "L4"),
                section("sec-b2", "Chorus", SectionType::Chorus, "L5"),
            ],
        ),
        "arr-shared",
        T0,
    )
    .expect_err("an arrangement id that already exists must fail the write");
    assert!(
        matches!(err, DbError::Sqlite(_)),
        "a colliding arrangement id is the primary key's refusal, not a mapped variant: {err:?}",
    );

    assert_eq!(
        load_song(&conn, "song-7b").expect("the read itself must not fail"),
        None,
        "a song without its default arrangement must not be left behind",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-7b"],
        ),
        0,
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-7b"],
        ),
        0,
    );

    // The first song is untouched by the second's failure.
    assert_eq!(
        stored_items(&conn, "arr-shared"),
        vec![(0, "sec-a1".to_owned())],
    );
    let default_id: Option<String> = conn
        .query_row(
            "SELECT default_arrangement_id FROM songs WHERE id = ?1",
            ["song-7a"],
            |row| row.get(0),
        )
        .expect("the first song must still be there");
    assert_eq!(default_id.as_deref(), Some("arr-shared"));
}

// ─────────────────────────────────────────────────────────────
// The refusals the combined path shares with `insert_song`
// ─────────────────────────────────────────────────────────────

/// Two sections sharing a label are refused **by name** on the combined path
/// too, and nothing is written — not the song, not the arrangement.
///
/// `insert_song` already refuses this, and `tests/song_sections.rs` covers it
/// there. The reason it is tested again on the other door is that the two doors
/// are separate entry paths into the same tables, and a check that guards one
/// of several entry paths has stopped being a check (ADR-0045). Both call
/// `refuse_uninsertable`; this is the test that says so.
///
/// The `UNIQUE (song_id, label)` constraint would refuse the write anyway, as
/// `"UNIQUE constraint failed: song_sections.song_id, song_sections.label"` —
/// so the discriminator is **which** error arrives, not whether one does. If
/// you are here because you dropped the call on this path, the write is still
/// rejected and the message now names two columns instead of the word the
/// operator typed twice.
#[test]
fn a_repeated_label_is_refused_by_name_on_the_combined_path_too() {
    let db = TempDb::new("combined-duplicate-label");
    let mut conn = db.open();

    let err = insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-8",
            "Song Eight",
            vec![
                section("sec-1", "Chorus", SectionType::Chorus, "L1"),
                section("sec-2", "Chorus", SectionType::Chorus, "L2"),
            ],
        ),
        "arr-8",
        T0,
    )
    .expect_err("two sections may not share a label");

    match &err {
        DbError::DuplicateSectionLabel { song_id, label } => {
            assert_eq!(song_id, "song-8");
            assert_eq!(label, "Chorus");
        }
        other => panic!(
            "expected DuplicateSectionLabel naming Chorus, got {other:?} — the UNIQUE constraint \
             would also have refused this, naming columns instead of the label"
        ),
    }

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-8"],
        ),
        0,
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-8"],
        ),
        0,
        "a refused song must not leave an arrangement behind either",
    );
}

/// A song offered with a `default_arrangement_id` already set is refused by
/// name on the combined path.
///
/// The refusal is sharper here than on `insert_song`, because this is precisely
/// the function that *chooses* the value: a caller who supplied one meant
/// something this function does not do, and silently overwriting it would obey
/// neither reading. Appendix A's insert trigger would refuse the row too, with
/// `"default_arrangement_id must belong to this song"` — a message that sends
/// the reader hunting for the right id when no id would have been accepted.
#[test]
fn a_supplied_default_arrangement_id_is_refused_by_name_on_the_combined_path() {
    let db = TempDb::new("combined-premature-default");
    let mut conn = db.open();

    let mut premature = song(
        "song-9",
        "Song Nine",
        vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
    );
    premature.default_arrangement_id = Some("arr-not-mine".to_owned());

    let err = insert_song_with_default_arrangement(&mut conn, &premature, "arr-9", T0)
        .expect_err("this function chooses the default arrangement; a caller may not");

    match &err {
        DbError::DefaultArrangementAtInsert { song_id } => assert_eq!(song_id, "song-9"),
        other => panic!("expected DefaultArrangementAtInsert, got {other:?}"),
    }
    assert!(
        !err.to_string().contains("must belong to this song"),
        "the diagnostic must not be the trigger's, which sends the reader after a valid id: {err}",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-9"],
        ),
        0,
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-9"],
        ),
        0,
    );
}

// ─────────────────────────────────────────────────────────────
// What `expand_arrangement` refuses to put on a screen
// ─────────────────────────────────────────────────────────────

/// Expanding an arrangement that plays **another song's** section is refused by
/// name, and the offending row is left in the table.
///
/// The schema cannot express this constraint — `arrangement_items.section_id`
/// references `song_sections(id)` and nothing ties it to the arrangement's song
/// — so the row below is written in raw SQL with every foreign key satisfied.
/// That is not a contrived state: `insert_arrangement` refuses it, but `.aero`
/// import (FR-703) and any older build are further writers, and this is what
/// such a row does when it reaches a projector.
///
/// **Refusing on this read while `load_arrangements` returns the same row
/// unchanged is deliberate, not an inconsistency**, and both halves are
/// asserted here so the pair is visible in one place. A read that fails is no
/// read for a repair to go through, so the path a repair would use stays
/// unfiltered; this path — the projection of a song onto a screen — refuses to
/// put another song's words under this song's title, which is FR-203's
/// criterion arriving through the read door.
#[test]
fn expanding_an_item_that_belongs_to_another_song_is_refused_by_name() {
    let db = TempDb::new("expand-cross-song");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-a1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-a",
        T0,
    )
    .unwrap();
    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![section("sec-b1", "Verse 1", SectionType::Verse, "L4")],
        ),
        "arr-b",
        T0,
    )
    .unwrap();

    conn.execute(
        "INSERT INTO arrangement_items (arrangement_id, position, section_id)
         VALUES ('arr-a', 1, 'sec-b1')",
        [],
    )
    .expect("the schema permits this row; that is the whole problem");

    let err = expand_arrangement(&conn, "arr-a")
        .expect_err("song A's arrangement may not play song B's words");
    match &err {
        DbError::SectionNotInSong {
            arrangement_id,
            song_id,
            section_id,
        } => {
            assert_eq!(arrangement_id, "arr-a");
            assert_eq!(song_id, "song-a");
            assert_eq!(section_id, "sec-b1");
        }
        other => panic!("expected SectionNotInSong naming sec-b1, got {other:?}"),
    }

    let arrangements =
        load_arrangements(&conn, "song-a").expect("the unfiltered read must still succeed");
    assert_eq!(
        arrangements
            .iter()
            .flat_map(|a| a.items.iter().map(|i| i.section_id.as_str()))
            .collect::<Vec<_>>(),
        vec!["sec-a1", "sec-b1"],
        "the read a repair would go through must return the row exactly as it stands",
    );
}

/// A row that is **both** another song's and of an unknown `section_type` is
/// refused as `SectionNotInSong`, not as `UnknownSectionType`.
///
/// The order of the two checks inside `expand_arrangement` is a deliberate
/// choice with a reason written beside it, and only a row that trips both can
/// tell the two orders apart — which is why this fixture goes to the trouble of
/// building one. Which song's words these are is the graver fact about the row,
/// and it is true of the row whatever the type says; reporting the type first
/// sends whoever is repairing the database after a `section_type` when the
/// section belongs to a different song entirely.
///
/// The unknown type is written with `PRAGMA ignore_check_constraints = ON`,
/// because Appendix A's `CHECK` makes such a row unreachable through ordinary
/// SQLite — which is exactly the point: it stands in for a row written before
/// the constraint, or by something other than this program.
#[test]
fn a_cross_song_item_is_refused_before_its_section_type_is_parsed() {
    let db = TempDb::new("cross-song-before-type");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-a1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-a",
        T0,
    )
    .unwrap();
    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![section("sec-b1", "Verse 1", SectionType::Verse, "L4")],
        ),
        "arr-b",
        T0,
    )
    .unwrap();

    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .expect("the pragma must apply");
    conn.execute(
        "UPDATE song_sections SET section_type = 'refrain' WHERE id = 'sec-b1'",
        [],
    )
    .expect("with the CHECK suspended, a foreign section_type is writable");
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .expect("the pragma must come back off");

    conn.execute(
        "INSERT INTO arrangement_items (arrangement_id, position, section_id)
         VALUES ('arr-a', 1, 'sec-b1')",
        [],
    )
    .unwrap();

    let err = expand_arrangement(&conn, "arr-a").expect_err("the row is wrong in two ways");
    match &err {
        DbError::SectionNotInSong { section_id, .. } => assert_eq!(section_id, "sec-b1"),
        other => panic!(
            "expected SectionNotInSong: which song's words these are is the graver fact, and it \
             is true of the row whatever its section_type says — got {other:?}"
        ),
    }
}

/// A `section_type` this build does not know is refused by name when it is
/// reached through an arrangement, not mapped to `other`.
///
/// `load_song` already refuses it on its own path; `expand_arrangement` reads
/// `song_sections` through a different query and so carries its own copy of the
/// decision. Guessing [`SectionType::Other`] would put a section on a projector
/// under a type nobody chose, and would hide the corrupt row rather than report
/// it.
#[test]
fn an_unknown_section_type_reached_through_an_arrangement_is_refused_by_name() {
    let db = TempDb::new("expand-unknown-type");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-10",
            "Song Ten",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-x", "Chorus", SectionType::Chorus, "L2"),
            ],
        ),
        "arr-10",
        T0,
    )
    .unwrap();

    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .expect("the pragma must apply");
    conn.execute(
        "UPDATE song_sections SET section_type = 'refrain' WHERE id = 'sec-x'",
        [],
    )
    .expect("with the CHECK suspended, a foreign section_type is writable");
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .expect("the pragma must come back off");

    let err = expand_arrangement(&conn, "arr-10")
        .expect_err("a type this build cannot name must not be guessed");
    match &err {
        DbError::UnknownSectionType { section_id, value } => {
            assert_eq!(section_id, "sec-x");
            assert_eq!(value, "refrain");
        }
        other => panic!("expected UnknownSectionType naming sec-x, got {other:?}"),
    }
}

/// A song written by `insert_song` alone has **no** arrangements and no
/// default.
///
/// The two entry points stay distinct: FR-204's generation happens on the door
/// that promises it, and the older door is not quietly changed to do it too —
/// which would make `insert_song`'s own contract untrue, that
/// `default_arrangement_id` is `None` at insert and stays so.
#[test]
fn the_plain_insert_path_generates_no_arrangement() {
    let db = TempDb::new("plain-insert");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-11",
            "Song Eleven",
            vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();

    let stored = load_song(&conn, "song-11").unwrap().unwrap();
    assert_eq!(stored.default_arrangement_id, None);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE song_id = ?1",
            &["song-11"],
        ),
        0,
    );
}

// ─────────────────────────────────────────────────────────────
// Claims `queries::song`'s doc comments make about the schema
//
// Each of the four tests below pins a sentence the module states as measured
// fact. The module is right about all four — that is why they are green — but
// a documented behaviour with no artefact holding it is a claim that outlives
// the code it describes, and documentation whose claim exceeds its code is the
// failure class this repository keeps paying for. These are the artefacts.
// ─────────────────────────────────────────────────────────────

/// Hard-deleting one song **silently shortens another song's arrangement**,
/// and nothing anywhere raises.
///
/// `expand_arrangement`'s doc states this as a measured fact — *"positions 0,
/// 1, 2, 3 with 2 pointing into song B read back as 0, 1, 3 after `DELETE FROM
/// songs`"* — and this is that measurement, reproduced in the exact shape the
/// sentence describes. Until this test the repository contained no `DELETE` at
/// all, so the claim rested on an experiment nobody could re-run.
///
/// The chain is `songs → song_sections → arrangement_items`, every link
/// `ON DELETE CASCADE`, and what it gives is the *opposite* of what the name
/// suggests here: it does not leave a broken reference for anyone to detect,
/// it removes the **item row**. What is left is a gap in `position`, and gaps
/// are legal — nothing distinguishes this arrangement from one an operator
/// wrote that way. Whoever opens the hard-delete path (FR-202) inherits that;
/// this test is what will tell them the behaviour is still what the comment
/// says.
///
/// **It also settles why `JOIN` → `LEFT JOIN` in `expand_arrangement` cannot
/// be caught by any test.** A dangling `section_id` is what would make the two
/// joins differ, and the cascade means one cannot exist: the row that would
/// have dangled is gone with it. The zero-dangling assertion below is that
/// premise written down as an assertion instead of inferred from the DDL.
#[test]
fn hard_deleting_a_song_silently_removes_it_from_another_songs_arrangement() {
    let db = TempDb::new("cascade-shortens");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![
                section("sec-a1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-a2", "Chorus", SectionType::Chorus, "L2"),
                section("sec-a3", "Verse 2", SectionType::Verse, "L3"),
            ],
        ),
        "arr-a-default",
        T0,
    )
    .unwrap();
    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![section("sec-b1", "Verse 1", SectionType::Verse, "L4")],
        ),
        "arr-b",
        T0,
    )
    .unwrap();

    // Positions 0, 1 and 3 are song A's own; position 2 is written by hand,
    // because `insert_arrangement` refuses a cross-song item and only a writer
    // that skipped that check — FR-703's import, an older build — can produce
    // one.
    insert_arrangement(
        &mut conn,
        &Arrangement {
            id: "arr-a".to_owned(),
            song_id: "song-a".to_owned(),
            name: "Full version".to_owned(),
            created_at: T0.to_owned(),
            items: vec![
                ArrangementItem {
                    position: 0,
                    section_id: "sec-a1".to_owned(),
                },
                ArrangementItem {
                    position: 1,
                    section_id: "sec-a2".to_owned(),
                },
                ArrangementItem {
                    position: 3,
                    section_id: "sec-a3".to_owned(),
                },
            ],
        },
    )
    .unwrap();
    conn.execute(
        "INSERT INTO arrangement_items (arrangement_id, position, section_id)
         VALUES ('arr-a', 2, 'sec-b1')",
        [],
    )
    .expect("the schema permits a cross-song item; that is the premise");

    assert_eq!(
        stored_items(&conn, "arr-a")
            .iter()
            .map(|(position, _)| *position)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3],
        "the fixture must start with all four positions filled",
    );

    let deleted = conn
        .execute("DELETE FROM songs WHERE id = ?1", ["song-b"])
        .expect("the delete itself must succeed");
    assert_eq!(deleted, 1);

    assert_eq!(
        stored_items(&conn, "arr-a"),
        vec![
            (0, "sec-a1".to_owned()),
            (1, "sec-a2".to_owned()),
            (3, "sec-a3".to_owned()),
        ],
        "position 2 is gone entirely: the cascade removed the item row, not just its target",
    );

    let dangling: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM arrangement_items i
             LEFT JOIN song_sections s ON s.id = i.section_id
             WHERE s.id IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("the dangling-id query must run");
    assert_eq!(
        dangling, 0,
        "no dangling section_id is left behind, which is why a LEFT JOIN in \
         expand_arrangement cannot behave differently from a JOIN",
    );

    assert_eq!(
        labels(&expanded(&conn, "arr-a")),
        vec!["Verse 1", "Chorus", "Verse 2"],
        "the service order comes back one item shorter, with no error raised",
    );
}

/// Neither trigger on `songs` fires on the combined path — **and both are
/// armed on the connection while it runs**.
///
/// `insert_song_with_default_arrangement` documents this as the reason no
/// `DbError` variant maps `RAISE(ABORT, 'default_arrangement_id must belong to
/// this song')`: the `BEFORE INSERT` trigger's `WHEN` is false because the
/// column is bound `NULL`, and the `BEFORE UPDATE` trigger's is false because
/// the arrangement row it looks for was inserted earlier in the same
/// transaction.
///
/// **The positive controls are what make this a test.** Asserting only that
/// the happy path succeeds would be exactly as green on a database where
/// migration 001 never created the triggers — the claim is that the guards are
/// armed and are nevertheless not tripped, so both halves have to be shown on
/// one connection. The two raw statements below arm them visibly, each
/// returning `SQLITE_CONSTRAINT_TRIGGER` (1811) with the trigger's own
/// message; the combined path runs between them and meets neither.
#[test]
fn neither_songs_trigger_fires_on_the_combined_path_though_both_are_armed() {
    let db = TempDb::new("triggers-not-fired");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-a1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-a",
        T0,
    )
    .expect("the combined path must succeed with both triggers armed");

    // Positive control 1 — the BEFORE INSERT trigger.
    let insert_err = conn
        .execute(
            "INSERT INTO songs (id, title, default_arrangement_id, created_at, updated_at)
             VALUES ('song-x', 'Song X', 'arr-a', ?1, ?1)",
            [T0],
        )
        .expect_err("a song may not be born pointing at another song's arrangement");
    assert_eq!(sqlite_extended_code(&insert_err), Some(1811));
    assert!(insert_err
        .to_string()
        .contains("default_arrangement_id must belong to this song"));

    // Positive control 2 — the BEFORE UPDATE trigger.
    insert_song(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![section("sec-b1", "Verse 1", SectionType::Verse, "L4")],
        ),
    )
    .unwrap();
    let update_err = conn
        .execute(
            "UPDATE songs SET default_arrangement_id = 'arr-a' WHERE id = 'song-b'",
            [],
        )
        .expect_err("a song may not be pointed at another song's arrangement");
    assert_eq!(sqlite_extended_code(&update_err), Some(1811));
    assert!(update_err
        .to_string()
        .contains("default_arrangement_id must belong to this song"));

    // The combined path, run again on the same connection, still meets
    // neither.
    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-c",
            "Song C",
            vec![section("sec-c1", "Verse 1", SectionType::Verse, "L5")],
        ),
        "arr-c",
        T0,
    )
    .expect("it binds NULL on insert and updates to an arrangement of its own song");

    let default_id: Option<String> = conn
        .query_row(
            "SELECT default_arrangement_id FROM songs WHERE id = 'song-c'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(default_id.as_deref(), Some("arr-c"));
}

/// An `arrangement_id` that already names another song's arrangement is
/// refused by `song_arrangements`' primary key, before the `UPDATE` — and so
/// before the `BEFORE UPDATE` trigger — is reached at all.
///
/// This is the other half of the module's argument that no error variant needs
/// to map the trigger's message. That the refusal arrives from the key is what
/// matters: had the collision been allowed, the `UPDATE` would have run and
/// the caller would have received `"default_arrangement_id must belong to this
/// song"`, which names a fixable id when the real problem is a duplicated one.
///
/// **The extended code and the message disagree, and both are asserted on
/// purpose.** SQLite reports `1555` — `SQLITE_CONSTRAINT_PRIMARYKEY` — while
/// the message it attaches reads `UNIQUE constraint failed:
/// song_arrangements.id`. The module's prose says "primary key", which matches
/// the code and not the text an operator would be shown; both are pinned here
/// so that a reader who finds either surprising can see the other was known.
#[test]
fn a_colliding_arrangement_id_is_refused_by_the_primary_key_not_by_a_trigger() {
    let db = TempDb::new("arrangement-id-collision");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-a1", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-shared",
        T0,
    )
    .unwrap();

    let err = insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![section("sec-b1", "Verse 1", SectionType::Verse, "L4")],
        ),
        "arr-shared",
        T0,
    )
    .expect_err("two songs may not share one arrangement id");

    assert_eq!(
        extended_code(&err),
        Some(1555),
        "SQLITE_CONSTRAINT_PRIMARYKEY: the refusal comes from the key, not from a trigger",
    );
    assert!(
        err.to_string()
            .contains("UNIQUE constraint failed: song_arrangements.id"),
        "the attached message says UNIQUE even though the code says PRIMARYKEY: {err}",
    );
    assert!(
        !err.to_string().contains("must belong to this song"),
        "the UPDATE, and with it the trigger, must never have been reached: {err}",
    );
}

/// A **section** write that fails partway on the combined path leaves neither
/// the song nor the arrangement behind.
///
/// [`a_failed_arrangement_write_leaves_no_song_behind`] fails at the last of
/// the three statements; this one fails at the second, with the `songs` row
/// already written and one section already in. Both points inside the
/// transaction are worth an artefact, because a rollback that covered only the
/// tail would still look correct from the arrangement's side.
///
/// **The third failure that transaction could in principle carry —
/// `DbError::SectionNotInSong`, raised by the Rust check inside
/// `write_arrangement` — is unreachable on this path, and no test here can
/// reach it.** The items are built from `song.sections`, every one of which
/// `write_song` inserted with this song's id two statements earlier, so the
/// owner lookup cannot disagree; a section id repeated within the list trips
/// `song_sections`' primary key first, which is the failure this test uses.
/// The rollback-on-a-Rust-error property is covered where it *is* reachable:
/// `tests/song_sections.rs`'s
/// `an_arrangement_may_not_point_at_another_songs_section`, on
/// `insert_arrangement`.
#[test]
fn a_section_write_that_fails_partway_leaves_no_arrangement_either() {
    let db = TempDb::new("combined-section-rollback");
    let mut conn = db.open();

    insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-shared", "Verse 1", SectionType::Verse, "L1")],
        ),
        "arr-a",
        T0,
    )
    .unwrap();

    let err = insert_song_with_default_arrangement(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![
                section("sec-b1", "Verse 1", SectionType::Verse, "L4"),
                // Already owned by song A, so this insert — the second
                // section — trips the primary key with the songs row and one
                // section row already written.
                section("sec-shared", "Chorus", SectionType::Chorus, "L5"),
            ],
        ),
        "arr-b",
        T0,
    )
    .expect_err("a section id that already exists must fail the write");
    assert_eq!(extended_code(&err), Some(1555));

    assert_eq!(
        load_song(&conn, "song-b").expect("the read must not fail"),
        None,
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE id = ?1",
            &["sec-b1"],
        ),
        0,
        "the first section, written before the failure, must be gone too",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-b"],
        ),
        0,
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE id = ?1",
            &["sec-shared"],
        ),
        1,
        "song A's section is untouched by song B's failure",
    );
}
