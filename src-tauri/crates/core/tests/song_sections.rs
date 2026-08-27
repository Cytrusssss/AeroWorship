//! FR-203: a song is an ordered set of labelled sections, and **a section's
//! text is stored exactly once regardless of how many times it is sung**
//! (PRD §4.2, FR-203's sole acceptance criterion).
//!
//! Exercised through `aeroworship_core::db`'s public surface only —
//! `open_and_migrate`, then `queries::song` — which is the surface an
//! application gets, so this lives as a Cargo integration test rather than
//! inside the crate. Raw SQL appears in two places and only two: to *count rows
//! in the tables* (the acceptance criterion is a statement about storage, not
//! about return values, so it has to be read off the tables), and to write rows
//! no public path can write, which is how the read path's defences against a
//! foreign or hand-edited database are reached at all.
//!
//! ── The database fixture ───────────────────────────────────────────
//!
//! `TempDb` used to live here, in the second of what became three copies.
//! FR-204 extracted it to `tests/common/mod.rs`, as the note it replaced said
//! whichever item landed next should; the reasons for a real temp file, for
//! the `Drop` guard's siblings and for declaring the guard **before** the
//! `Connection` are stated there and have not changed.
//!
//! What stays here is the data: `L1`…`L4` and structural labels, no real
//! lyrics anywhere — `Verse 1` and `Chorus` are the *names of a song's parts*,
//! not its words.

mod common;

use aeroworship_core::db::queries::song::{
    insert_arrangement, insert_song, load_arrangements, load_song, Arrangement, ArrangementItem,
    SectionType, Song, SongSection,
};
use aeroworship_core::db::DbError;
use common::TempDb;
use rusqlite::Connection;

/// Fixed timestamps: nothing in `queries::song` reads a clock, and a fixture
/// that did would make every assertion on a round-trip unrepeatable.
const T0: &str = "2026-01-01T00:00:00Z";

/// A song with every optional field empty, for the tests that care about
/// sections rather than about columns. [`the_whole_record_round_trips`] is the
/// one that fills them all in.
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
///
/// The acceptance criterion is about what is *stored*, so it cannot be checked
/// by looking at what a function returned; every FR-203 assertion that says
/// "exactly once" goes through here.
fn count(conn: &Connection, sql: &str, args: &[&str]) -> i64 {
    conn.query_row(sql, rusqlite::params_from_iter(args), |row| row.get(0))
        .unwrap_or_else(|err| panic!("count query failed: {sql}: {err}"))
}

fn loaded(conn: &Connection, song_id: &str) -> Song {
    load_song(conn, song_id)
        .expect("load_song must not fail on a song this test just wrote")
        .unwrap_or_else(|| panic!("song {song_id} was written and must be readable"))
}

fn labels(song: &Song) -> Vec<&str> {
    song.sections.iter().map(|s| s.label.as_str()).collect()
}

fn section_ids(song: &Song) -> Vec<&str> {
    song.sections.iter().map(|s| s.id.as_str()).collect()
}

// ─────────────────────────────────────────────────────────────
// The acceptance criterion
// ─────────────────────────────────────────────────────────────

/// FR-203: *"A section's text is stored exactly once regardless of how many
/// times it is sung."*
///
/// The chorus below is sung three times out of six positions. The assertions
/// count rows **in `song_sections`**, not entries in the `Vec` that came back:
/// a storage layer that duplicated the text per occurrence could still return a
/// tidy four-section song, and it is the duplication in the table that FR-203
/// forbids — it is what makes "editing a section's text updates every
/// occurrence" (FR-204) possible at all.
#[test]
fn a_section_sung_three_times_is_still_one_row_of_stored_text() {
    let db = TempDb::new("sung-three-times");
    let mut conn = db.open();

    let chorus_text = "L1\nL2\nL3";
    insert_song(
        &mut conn,
        &song(
            "song-1",
            "Song One",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1\nL2"),
                section("sec-ch", "Chorus", SectionType::Chorus, chorus_text),
                section("sec-v2", "Verse 2", SectionType::Verse, "L3\nL4"),
                section("sec-br", "Bridge", SectionType::Bridge, "L1"),
            ],
        ),
    )
    .expect("a four-section song must insert");

    insert_arrangement(
        &mut conn,
        &arrangement(
            "arr-1",
            "song-1",
            "Default",
            &["sec-v1", "sec-ch", "sec-v2", "sec-ch", "sec-br", "sec-ch"],
        ),
    )
    .expect("repeating a section across positions is what FR-203 exists for");

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-1"],
        ),
        4,
        "six sung positions over four sections must be four rows in song_sections",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1 AND label = ?2",
            &["song-1", "Chorus"],
        ),
        1,
        "the chorus is sung three times and stored once",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE content = ?1",
            &[chorus_text],
        ),
        1,
        "the chorus text itself must appear in exactly one row, under any id",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM arrangement_items WHERE arrangement_id = ?1",
            &["arr-1"],
        ),
        6,
        "the repetition lives in arrangement_items, which must hold all six positions",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(DISTINCT section_id) FROM arrangement_items WHERE arrangement_id = ?1",
            &["arr-1"],
        ),
        4,
        "those six positions must reference only the four stored sections",
    );

    let arrangements = load_arrangements(&conn, "song-1").expect("arrangements must read back");
    assert_eq!(arrangements.len(), 1);
    let played: Vec<&str> = arrangements[0]
        .items
        .iter()
        .map(|item| item.section_id.as_str())
        .collect();
    assert_eq!(
        played,
        ["sec-v1", "sec-ch", "sec-v2", "sec-ch", "sec-br", "sec-ch"],
        "the arrangement must read back as the sung order, repeats included",
    );
}

// ─────────────────────────────────────────────────────────────
// Round-trip and order
// ─────────────────────────────────────────────────────────────

/// Every column a song has, out exactly as it went in.
///
/// `default_arrangement_id` is the one field that cannot take part: Appendix
/// A's insert trigger rejects any non-NULL value, so `None` is the only legal
/// input and there is nothing to round-trip. See
/// [`a_default_arrangement_id_at_insert_is_refused_by_name`].
#[test]
fn the_whole_record_round_trips() {
    let db = TempDb::new("round-trip");
    let mut conn = db.open();

    let original = Song {
        id: "song-full".to_owned(),
        title: "Song Full".to_owned(),
        alternate_title: Some("Alternate Full".to_owned()),
        ccli_number: Some("1234567".to_owned()),
        copyright_text: Some("(c) 2026 Example".to_owned()),
        song_key: Some("F#m".to_owned()),
        tempo_bpm: Some(72),
        default_arrangement_id: None,
        source_provider: Some("example-provider".to_owned()),
        source_url: Some("https://example.invalid/song".to_owned()),
        retrieved_at: Some("2026-02-03T04:05:06Z".to_owned()),
        created_at: T0.to_owned(),
        updated_at: "2026-03-04T05:06:07Z".to_owned(),
        deleted_at: None,
        sections: vec![
            section("sec-a", "Verse 1", SectionType::Verse, "L1\nL2"),
            section("sec-b", "Chorus", SectionType::Chorus, "L3\nL4"),
        ],
    };

    insert_song(&mut conn, &original).expect("a fully populated song must insert");
    assert_eq!(
        loaded(&conn, "song-full"),
        original,
        "load_song must return the record insert_song was given, field for field",
    );
}

/// An id that was never written is `Ok(None)`, not an error and not an empty
/// song.
#[test]
fn an_unknown_song_id_reads_as_none() {
    let db = TempDb::new("unknown-id");
    let conn = db.open();

    assert!(load_song(&conn, "song-never-written")
        .expect("a missing song is not an error")
        .is_none());
}

/// Sections come back in the order they were authored in — the position in the
/// `Vec` handed to `insert_song` — and not in any order the ids or the labels
/// happen to have.
///
/// The fixture is built so the three orders disagree: ids descend while
/// authoring order ascends, and the labels sort differently again. An
/// implementation that ordered by `id` or by `label` would return a plausible
/// song with its verses shuffled, which is the failure an operator meets on a
/// projector rather than in a diagnostic.
#[test]
fn sections_come_back_in_authoring_order_not_in_id_or_label_order() {
    let db = TempDb::new("authoring-order");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-2",
            "Song Two",
            vec![
                section("sec-3", "Verse 1", SectionType::Verse, "L1"),
                section("sec-2", "Chorus", SectionType::Chorus, "L2"),
                section("sec-1", "Bridge", SectionType::Bridge, "L3"),
            ],
        ),
    )
    .expect("insert must accept sections whose ids descend");

    let read = loaded(&conn, "song-2");
    assert_eq!(
        section_ids(&read),
        ["sec-3", "sec-2", "sec-1"],
        "authoring order, not id order",
    );
    assert_eq!(labels(&read), ["Verse 1", "Chorus", "Bridge"]);

    let stored_order: Vec<(String, i64)> = {
        let mut stmt = conn
            .prepare("SELECT id, sort_order FROM song_sections WHERE song_id = ?1 ORDER BY id")
            .unwrap();
        let rows = stmt
            .query_map(["song-2"], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        rows.map(|row| row.unwrap()).collect()
    };
    assert_eq!(
        stored_order,
        vec![
            ("sec-1".to_owned(), 2),
            ("sec-2".to_owned(), 1),
            ("sec-3".to_owned(), 0),
        ],
        "sort_order must be filled from the position in the Vec",
    );
}

/// Sections that tie on `sort_order` still come back in one fixed order,
/// because the read orders by `(sort_order, id)`.
///
/// `insert_song` cannot produce a tie — it numbers from the `Vec` — so the tie
/// is written here in raw SQL, which is exactly how one reaches a real
/// database: an `.aero` import (FR-701), an online import (FR-604) or a build
/// older than this one may all have written `song_sections` rows directly.
///
/// **The fixture is chosen so the tie-break is observable, and it is not
/// observable by accident.** With `sec-z`, `sec-m`, `sec-a` all at
/// `sort_order = 0`, inserted in that order, `ORDER BY sort_order` alone
/// returns them in *insertion* order — `idx_sections_song` is
/// `(song_id, sort_order)`, so the tie falls through to the rowid — and the
/// expected list below is precisely the one a dropped `, id` cannot produce. If
/// you are reading this because you simplified that `ORDER BY`, the answer it
/// now gives depends on the order rows happened to be written in, which two
/// churches' databases will not agree on.
#[test]
fn sections_tied_on_sort_order_are_broken_by_id_not_by_insertion_order() {
    let db = TempDb::new("tie-break");
    let mut conn = db.open();

    insert_song(&mut conn, &song("song-3", "Song Three", Vec::new()))
        .expect("a song may be inserted with no sections");

    for (id, label, sort_order) in [
        ("sec-z", "Verse 3", 0),
        ("sec-m", "Verse 2", 0),
        ("sec-a", "Verse 1", 0),
        ("sec-b", "Chorus", 1),
    ] {
        conn.execute(
            "INSERT INTO song_sections (id, song_id, label, section_type, content, sort_order)
             VALUES (?1, 'song-3', ?2, 'verse', 'L1', ?3)",
            rusqlite::params![id, label, sort_order],
        )
        .expect("raw section insert stands in for a foreign writer");
    }

    let read = loaded(&conn, "song-3");
    assert_eq!(
        section_ids(&read),
        ["sec-a", "sec-m", "sec-z", "sec-b"],
        "a tie on sort_order must be broken by id, so the order is a fact and not a query plan",
    );
}

/// Arrangement items are ordered by `position`, which need not be contiguous
/// and need not be handed in sorted.
#[test]
fn arrangement_items_are_ordered_by_position_and_may_have_gaps() {
    let db = TempDb::new("item-order");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-4",
            "Song Four",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-ch", "Chorus", SectionType::Chorus, "L2"),
                section("sec-br", "Bridge", SectionType::Bridge, "L3"),
            ],
        ),
    )
    .unwrap();

    insert_arrangement(
        &mut conn,
        &Arrangement {
            id: "arr-4".to_owned(),
            song_id: "song-4".to_owned(),
            name: "Short version".to_owned(),
            created_at: T0.to_owned(),
            items: vec![
                ArrangementItem {
                    position: 30,
                    section_id: "sec-br".to_owned(),
                },
                ArrangementItem {
                    position: 10,
                    section_id: "sec-v1".to_owned(),
                },
                ArrangementItem {
                    position: 20,
                    section_id: "sec-ch".to_owned(),
                },
            ],
        },
    )
    .expect("positions may arrive unsorted and with gaps");

    let arrangements = load_arrangements(&conn, "song-4").unwrap();
    let items: Vec<(i64, &str)> = arrangements[0]
        .items
        .iter()
        .map(|item| (item.position, item.section_id.as_str()))
        .collect();
    assert_eq!(items, [(10, "sec-v1"), (20, "sec-ch"), (30, "sec-br")]);
}

/// Arrangements come back ordered by name, and a song with none reads as an
/// empty `Vec` rather than as an error — which is every song until FR-204
/// creates the default one.
#[test]
fn arrangements_are_ordered_by_name_and_a_song_may_have_none() {
    let db = TempDb::new("arrangement-order");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-5",
            "Song Five",
            vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();
    assert!(
        load_arrangements(&conn, "song-5").unwrap().is_empty(),
        "a song with no arrangements reads as an empty Vec",
    );

    for name in ["Short version", "Default", "Long version"] {
        let id = format!("arr-{}", name.replace(' ', "-"));
        insert_arrangement(&mut conn, &arrangement(&id, "song-5", name, &["sec-v1"])).unwrap();
    }

    let names: Vec<String> = load_arrangements(&conn, "song-5")
        .unwrap()
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(names, ["Default", "Long version", "Short version"]);
}

// ─────────────────────────────────────────────────────────────
// `content` is storage, not something this layer tidies
// ─────────────────────────────────────────────────────────────

/// `content` round-trips **byte for byte**, including line separators that are
/// not `\n`.
///
/// If you are reading this because you just made it red by normalising the
/// column — a `content.replace("\r\n", "\n")`, or a `trim`, on the way in or on
/// the way out — that is the change this test exists to stop, and the reason is
/// not tidiness:
///
/// * `models::split_slides` is the sole authority on what a line is, and it
///   counts **seven characters** — `\n`, a lone `\r`, U+000B, U+000C, U+0085,
///   U+2028 and U+2029, with `\r\n` counted as one break and not two. They are
///   the **four** UAX #14 mandatory-break classes BK, CR, LF and NL; three
///   classes would describe only six of them, and the one left over is U+0085
///   (see `models::slide`). Normalising here buys the consumer that matters
///   nothing, because it already handles all seven.
/// * It would claim an invariant this layer cannot keep. FR-202's update path,
///   FR-604's online import and FR-701's `.aero` import are all further ways a
///   row reaches this column, and rows already in a church's database were
///   never normalised at all. An invariant guarded by one of several entry
///   paths has stopped being one (ADR-0045).
/// * A U+2028 in a paste is a line break the *source document* chose. Rewriting
///   it is a repair, and a repaired value leaves through an `.aero` export
///   (FR-706) as something nobody typed.
///
/// The stored bytes are checked as well as the returned ones, so a failure says
/// which side rewrote them.
#[test]
fn content_is_stored_and_returned_byte_for_byte() {
    let db = TempDb::new("verbatim-content");
    let mut conn = db.open();

    // Every separator `split_slides` recognises, plus a trailing space and a
    // trailing break, in one string. Synthetic markers only.
    let awkward = "L1\r\nL2\rL3\u{000b}L4\u{000c}L5\u{0085}L6\u{2028}L7\u{2029}L8  \n";
    insert_song(
        &mut conn,
        &song(
            "song-6",
            "Song Six",
            vec![section("sec-odd", "Verse 1", SectionType::Verse, awkward)],
        ),
    )
    .expect("no line separator is a reason to refuse a section");

    let stored: String = conn
        .query_row(
            "SELECT content FROM song_sections WHERE id = 'sec-odd'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        stored.as_bytes(),
        awkward.as_bytes(),
        "the write path rewrote content: stored {}, was handed {}",
        stored.escape_debug(),
        awkward.escape_debug(),
    );

    let read = loaded(&conn, "song-6");
    assert_eq!(
        read.sections[0].content.as_bytes(),
        awkward.as_bytes(),
        "the read path rewrote content: returned {}",
        read.sections[0].content.escape_debug(),
    );
}

// ─────────────────────────────────────────────────────────────
// Labels
// ─────────────────────────────────────────────────────────────

/// Two sections sharing a label are refused **by name**, and nothing is
/// written.
///
/// The `UNIQUE (song_id, label)` constraint would refuse it too, as
/// `"UNIQUE constraint failed: song_sections.song_id, song_sections.label"` —
/// a message that names the columns and not the word the operator typed twice.
/// Hence the pre-check, and hence this test asserts on the variant and on the
/// label inside it, not merely that an error came back.
#[test]
fn a_repeated_label_names_the_label_and_writes_nothing() {
    let db = TempDb::new("duplicate-label");
    let mut conn = db.open();

    let err = insert_song(
        &mut conn,
        &song(
            "song-7",
            "Song Seven",
            vec![
                section("sec-1", "Chorus", SectionType::Chorus, "L1"),
                section("sec-2", "Verse 1", SectionType::Verse, "L2"),
                section("sec-3", "Chorus", SectionType::Chorus, "L3"),
            ],
        ),
    )
    .expect_err("two sections labelled Chorus must be refused");

    match &err {
        DbError::DuplicateSectionLabel { song_id, label } => {
            assert_eq!(song_id, "song-7");
            assert_eq!(
                label, "Chorus",
                "the diagnostic must name the repeated label"
            );
        }
        other => panic!("expected DuplicateSectionLabel naming the label, got {other:?}"),
    }
    assert!(
        err.to_string().contains("Chorus"),
        "the displayed message must quote the label: {err}",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-7"]
        ),
        0,
        "a refused song must leave no song row",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-7"],
        ),
        0,
        "a refused song must leave no section rows",
    );
}

/// `"Chorus"` and `"chorus"` are **two labels**, and a song may carry both.
///
/// This is the deliberately permissive direction, so it is worth saying who it
/// is addressed to: if you are here because you just made this test red by
/// "hardening" the duplicate check into a case-insensitive one, that check now
/// refuses a song the schema accepts. `UNIQUE (song_id, label)` compares under
/// SQLite's default `BINARY` collation; a case-folding pre-check invents a rule
/// this layer enforces nowhere else and cannot enforce on the other entry paths
/// — the same second-entry-path shape as ADR-0045, and the same reason FR-401
/// accepts markup metacharacters in a template name on purpose. If the
/// *product* should warn about near-duplicate labels, that belongs in the
/// editor where the operator can see both, not in the storage layer where it
/// silently diverges from the constraint.
#[test]
fn labels_are_compared_byte_for_byte_so_case_makes_two_labels() {
    let db = TempDb::new("label-case");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-8",
            "Song Eight",
            vec![
                section("sec-1", "Chorus", SectionType::Chorus, "L1"),
                section("sec-2", "chorus", SectionType::Chorus, "L2"),
            ],
        ),
    )
    .expect("BINARY collation makes Chorus and chorus two distinct labels");

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-8"],
        ),
        2,
        "both labels must be stored",
    );
    assert_eq!(labels(&loaded(&conn, "song-8")), ["Chorus", "chorus"]);
}

// ─────────────────────────────────────────────────────────────
// Section types
// ─────────────────────────────────────────────────────────────

/// All nine `section_type` values Appendix A's `CHECK` lists survive a
/// round-trip.
///
/// Two things at once, and both matter: every spelling `SectionType::as_str`
/// produces is one the `CHECK` accepts — a tenth spelling, or `prechorus` for
/// `pre_chorus`, would fail the insert — and `from_db` is its exact inverse.
#[test]
fn all_nine_section_types_survive_the_check_constraint_and_come_back_equal() {
    let db = TempDb::new("nine-types");
    let mut conn = db.open();

    let types = [
        SectionType::Verse,
        SectionType::Chorus,
        SectionType::Bridge,
        SectionType::PreChorus,
        SectionType::Tag,
        SectionType::Ending,
        SectionType::Intro,
        SectionType::Interlude,
        SectionType::Other,
    ];
    let sections: Vec<SongSection> = types
        .iter()
        .enumerate()
        .map(|(i, ty)| section(&format!("sec-{i}"), &format!("Part {i}"), *ty, "L1"))
        .collect();

    insert_song(&mut conn, &song("song-9", "Song Nine", sections))
        .expect("every SectionType spelling must satisfy the CHECK constraint");

    let read = loaded(&conn, "song-9");
    let read_types: Vec<SectionType> = read.sections.iter().map(|s| s.section_type).collect();
    assert_eq!(read_types, types, "from_db must invert as_str for all nine");
}

/// A `section_type` the build does not know is refused **by name**, not mapped
/// to `other`.
///
/// The `CHECK` makes such a row unreachable through ordinary SQLite, so the
/// fixture writes it with `PRAGMA ignore_check_constraints = ON` — standing in
/// for the row that predates the constraint or was written by something other
/// than this program, which is the only way the branch is ever reached. Mapping
/// it to [`SectionType::Other`] would put a section on a projector under a type
/// nobody chose, and would hide the corrupt row rather than report it.
#[test]
fn an_unknown_section_type_is_refused_rather_than_guessed() {
    let db = TempDb::new("unknown-type");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-10",
            "Song Ten",
            vec![section("sec-ok", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();

    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .unwrap();
    conn.execute(
        "INSERT INTO song_sections (id, song_id, label, section_type, content, sort_order)
         VALUES ('sec-bad', 'song-10', 'Refrain', 'refrain', 'L2', 1)",
        [],
    )
    .expect("the pragma must let a foreign section_type through, or this test proves nothing");
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .unwrap();

    let err = load_song(&conn, "song-10")
        .expect_err("a section_type outside the nine must not be read as a song");
    match &err {
        DbError::UnknownSectionType { section_id, value } => {
            assert_eq!(section_id, "sec-bad");
            assert_eq!(value, "refrain");
        }
        other => panic!("expected UnknownSectionType naming the row and the value, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────
// Soft delete
// ─────────────────────────────────────────────────────────────

/// `load_song` returns a soft-deleted song, tombstone and all.
///
/// FR-202 promises a 30-day recovery window, and a restore path has to be able
/// to *read* what it restores; a `WHERE deleted_at IS NULL` here would make
/// that impossible through this function and invite a second, unfiltered reader
/// beside it. The filtering belongs to the list and the search (FR-201/202),
/// which is why `idx_songs_title` is the partial index it is. If you are here
/// because you added that clause, add it in those readers instead.
#[test]
fn a_soft_deleted_song_is_still_readable_and_carries_its_tombstone() {
    let db = TempDb::new("soft-delete");
    let mut conn = db.open();

    let mut deleted = song(
        "song-11",
        "Song Eleven",
        vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
    );
    deleted.deleted_at = Some("2026-06-01T12:00:00Z".to_owned());
    insert_song(&mut conn, &deleted).unwrap();

    let read = load_song(&conn, "song-11")
        .expect("reading a soft-deleted song is not an error")
        .expect("a soft-deleted song must still be readable, or FR-202 cannot restore it");
    assert_eq!(read.deleted_at.as_deref(), Some("2026-06-01T12:00:00Z"));
    assert_eq!(
        read.sections.len(),
        1,
        "its sections must come back too, or a restore would restore an empty song",
    );
}

// ─────────────────────────────────────────────────────────────
// All-or-nothing writes
// ─────────────────────────────────────────────────────────────

/// A section write that fails partway leaves **no** trace of the song.
///
/// The failure is arranged through the public API alone: the second song reuses
/// a section id the first song already owns, so its song row and its first
/// section land and the second section trips the primary key. Without the
/// transaction the database would keep a titled song with half its lyrics —
/// worse than a song that failed to save, because it looks like a song. Hence
/// the counts across all three tables rather than a look at the returned error.
#[test]
fn a_section_write_that_fails_partway_leaves_no_song_behind() {
    let db = TempDb::new("rollback-sections");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-a",
            "Song A",
            vec![section("sec-shared", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();

    let err = insert_song(
        &mut conn,
        &song(
            "song-b",
            "Song B",
            vec![
                section("sec-b1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-shared", "Chorus", SectionType::Chorus, "L2"),
                section("sec-b3", "Bridge", SectionType::Bridge, "L3"),
            ],
        ),
    )
    .expect_err("a section id that already exists must fail the primary key");
    assert!(
        matches!(err, DbError::Sqlite(_)),
        "expected the constraint failure to surface as DbError::Sqlite, got {err:?}",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-b"]
        ),
        0,
        "the song row must be rolled back",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-b"],
        ),
        0,
        "the section that landed before the failure must be rolled back",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE song_id = ?1",
            &["song-b"],
        ),
        0,
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_sections WHERE song_id = ?1",
            &["song-a"],
        ),
        1,
        "the song that was already there must be untouched",
    );
}

/// An item write that fails partway leaves no arrangement row behind.
///
/// Two items at the same position trip `PRIMARY KEY (arrangement_id,
/// position)` after the arrangement row has already been written. An
/// arrangement with a name and half its order is the same class of half-object
/// as the song above.
#[test]
fn an_item_write_that_fails_partway_leaves_no_arrangement_behind() {
    let db = TempDb::new("rollback-items");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-12",
            "Song Twelve",
            vec![
                section("sec-v1", "Verse 1", SectionType::Verse, "L1"),
                section("sec-ch", "Chorus", SectionType::Chorus, "L2"),
            ],
        ),
    )
    .unwrap();

    let err = insert_arrangement(
        &mut conn,
        &Arrangement {
            id: "arr-12".to_owned(),
            song_id: "song-12".to_owned(),
            name: "Default".to_owned(),
            created_at: T0.to_owned(),
            items: vec![
                ArrangementItem {
                    position: 0,
                    section_id: "sec-v1".to_owned(),
                },
                ArrangementItem {
                    position: 0,
                    section_id: "sec-ch".to_owned(),
                },
            ],
        },
    )
    .expect_err("two items at one position must fail the primary key");
    assert!(
        matches!(err, DbError::Sqlite(_)),
        "expected DbError::Sqlite, got {err:?}",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE song_id = ?1",
            &["song-12"],
        ),
        0,
        "the arrangement row written before the failure must be rolled back",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM arrangement_items WHERE arrangement_id = ?1",
            &["arr-12"],
        ),
        0,
    );
}

// ─────────────────────────────────────────────────────────────
// Constraints the schema cannot express
// ─────────────────────────────────────────────────────────────

/// An arrangement may not point at another song's section.
///
/// **The schema does not catch this** — `arrangement_items.section_id`
/// references `song_sections(id)` and nothing ties it to
/// `song_arrangements.song_id`, so such a row is accepted in raw SQL with every
/// foreign key satisfied. The consequence is not abstract: editing that
/// section's lyrics would change a song nobody edited. The check therefore
/// lives in `insert_arrangement`, and the assertions below are what tell a
/// missing check apart from a working one — the error variant, *and* the
/// arrangement row rolled back with it.
#[test]
fn an_arrangement_may_not_point_at_another_songs_section() {
    let db = TempDb::new("cross-song");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-x",
            "Song X",
            vec![section("sec-x1", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();
    insert_song(
        &mut conn,
        &song(
            "song-y",
            "Song Y",
            vec![section("sec-y1", "Verse 1", SectionType::Verse, "L2")],
        ),
    )
    .unwrap();

    let err = insert_arrangement(
        &mut conn,
        &arrangement("arr-y", "song-y", "Default", &["sec-y1", "sec-x1"]),
    )
    .expect_err("song Y's arrangement must not be allowed to play song X's section");

    match &err {
        DbError::SectionNotInSong {
            arrangement_id,
            song_id,
            section_id,
        } => {
            assert_eq!(arrangement_id, "arr-y");
            assert_eq!(song_id, "song-y");
            assert_eq!(section_id, "sec-x1");
        }
        other => panic!("expected SectionNotInSong naming all three ids, got {other:?}"),
    }

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-y"],
        ),
        0,
        "the arrangement row must be rolled back with the refused item",
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM arrangement_items WHERE arrangement_id = ?1",
            &["arr-y"],
        ),
        0,
    );
}

/// A section id that exists nowhere is named, rather than reported as a foreign
/// key failure.
///
/// The foreign key would catch this one; catching it in the same check names
/// the id the caller got wrong instead of the constraint it tripped.
#[test]
fn an_arrangement_item_pointing_at_no_section_at_all_names_the_id() {
    let db = TempDb::new("dangling-section");
    let mut conn = db.open();

    insert_song(
        &mut conn,
        &song(
            "song-13",
            "Song Thirteen",
            vec![section("sec-v1", "Verse 1", SectionType::Verse, "L1")],
        ),
    )
    .unwrap();

    let err = insert_arrangement(
        &mut conn,
        &arrangement("arr-13", "song-13", "Default", &["sec-v1", "sec-nowhere"]),
    )
    .expect_err("an item must not reference a section that does not exist");

    match &err {
        DbError::SectionNotInSong { section_id, .. } => assert_eq!(section_id, "sec-nowhere"),
        other => panic!("expected SectionNotInSong naming sec-nowhere, got {other:?}"),
    }
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM song_arrangements WHERE id = ?1",
            &["arr-13"],
        ),
        0,
    );
}

/// A song offered with a `default_arrangement_id` is refused with a diagnostic
/// that names the flow to use.
///
/// Appendix A's insert trigger refuses it too — always, since an arrangement of
/// a song that does not exist yet cannot exist — so the discriminator here is
/// **which** error, not whether one arrives. The trigger says
/// `"default_arrangement_id must belong to this song"`, which sends the reader
/// hunting for the right id when no id would have worked; the module refuses it
/// up front instead. If you are here because you deleted that guard, the write
/// is still rejected and the message is now the wrong one.
#[test]
fn a_default_arrangement_id_at_insert_is_refused_by_name() {
    let db = TempDb::new("default-arrangement");
    let mut conn = db.open();

    let mut premature = song("song-14", "Song Fourteen", Vec::new());
    premature.default_arrangement_id = Some("arr-not-yet".to_owned());

    let err = insert_song(&mut conn, &premature)
        .expect_err("a song cannot be born pointing at an arrangement");
    match &err {
        DbError::DefaultArrangementAtInsert { song_id } => assert_eq!(song_id, "song-14"),
        other => panic!(
            "expected DefaultArrangementAtInsert, got {other:?} — the schema trigger's own \
             message names an id, but no id would have been accepted here"
        ),
    }
    assert!(
        !err.to_string().contains("must belong to this song"),
        "the diagnostic must not be the trigger's, which sends the reader after a valid id: {err}",
    );

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM songs WHERE id = ?1",
            &["song-14"],
        ),
        0,
    );
}
