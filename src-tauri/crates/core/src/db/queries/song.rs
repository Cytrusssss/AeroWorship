//! Songs, their sections and their arrangements (FR-203, FR-204, Appendix A).
//!
//! FR-203 is one sentence — "songs are stored as an ordered set of labelled
//! sections, not as a single text blob" — with one acceptance criterion: *a
//! section's text is stored exactly once regardless of how many times it is
//! sung*. Appendix A already has the shape that makes that true; this module is
//! the typed way in and out of it. The repetition lives in
//! `arrangement_items`, whose primary key is `(arrangement_id, position)`
//! precisely so the same `section_id` may appear at several positions, and
//! `song_sections` never grows a row because of it.
//!
//! Five decisions are taken here that the reader should not have to infer.
//!
//! ── 1. `content` is stored **exactly as handed in**. ─────────────────────
//!
//! The column is annotated `-- newline-separated lines`, and it would be easy
//! to read that as a promise this module should enforce by rewriting every line
//! separator to `\n` on the way in. It does not, and the reason is that there
//! would then be **two** authorities on what a line is.
//!
//! [`crate::models::split_slides`] is the one that matters, and it counts seven
//! separators, not one: `\n`, `\r\n` (as a single break), a lone `\r`, U+000B,
//! U+000C, U+0085, U+2028 and U+2029 — the **four** UAX #14 mandatory-break
//! classes BK, CR, LF and NL, chosen because they are what `white-space: pre`
//! actually breaks on (ADR-0047). Four and not three: BK gives U+000B, U+000C,
//! U+2028 and U+2029, CR gives U+000D, LF gives U+000A, and **U+0085 is class
//! NL by itself** — a set checked against three classes comes out one character
//! short, and U+0085 is the one that then looks like a mistake to remove. **That function is the sole authority on the meaning of "line",
//! and this column is storage.** Normalising here would buy nothing for the
//! consumer that matters — it counts all seven anyway — while claiming an
//! invariant this layer cannot keep: FR-202's update path, FR-604's online
//! import and FR-701's `.aero` import are all further ways a row reaches this
//! column, and a row already in a church's database was never normalised at
//! all. An invariant guarded by exactly one of several entry paths has stopped
//! being an invariant (ADR-0045).
//!
//! It is also the "reject, never repair" stance `models::template` already
//! takes, applied to text rather than to numbers. A U+2028 in a paste from a
//! browser is a line break the *source document* chose; silently rewriting the
//! operator's stored text is a repair, and a repaired value round-trips out of
//! an `.aero` export (FR-706) as something nobody typed.
//!
//! **The consequence binds every reader of this column**, so it is stated
//! rather than left to be discovered: `content` may hold any of those seven,
//! and `str::lines` sees only one of them. Anything that needs to count lines
//! calls `split_slides`.
//!
//! ── 2. Ids and timestamps are **arguments**, never generated here. ───────
//!
//! [`Song::id`], [`Song::created_at`], [`Song::updated_at`] and their siblings
//! are fields the caller fills. This module names no clock and no random
//! number generator, which is why the same input twice produces the same two
//! rows and a test can assert on them. It is the shape `BookIndex` already has
//! (ADR-0043) and the shape `split_slides` was given by removing a parameter it
//! would have ignored (ADR-0046): the pure part takes what it needs as data,
//! and the shell supplies the real values.
//!
//! Appendix A wants a UUIDv7 in `id` and an ISO-8601 UTC string in the
//! timestamps. Neither is checked here — the format of an id is not something
//! this module can distinguish from a legitimate id it has never seen, and
//! inventing a rule would refuse rows Appendix A allows. Whoever mints them
//! owes that.
//!
//! ── 3. Section order is the **position in the `Vec`**. ───────────────────
//!
//! [`SongSection`] has no `sort_order` field. `song_sections.sort_order` is
//! filled from the index of the section in [`Song::sections`] on the way in,
//! and the read path returns them in that order.
//!
//! The alternative — a caller-supplied `sort_order` — is refused because
//! nothing in the schema makes it unique. Two sections sharing `sort_order`
//! would come back in an order the caller never chose, with no error anywhere:
//! the silent-and-wrong class this repository keeps paying for. Making the
//! `Vec` authoritative makes that unrepresentable. The column stays what its
//! own comment says it is — "authoring order only" — and the cost is stated:
//! a caller that wants gaps in the numbering, to preserve a foreign system's
//! section numbers, cannot express it. Nothing in the PRD asks for that.
//!
//! [`ArrangementItem::position`] is the opposite call, for a reason in the
//! schema rather than in taste: `position` is *in* the primary key, so a
//! duplicate is a loud constraint failure rather than a silent reordering, and
//! FR-204 may legitimately want a numbering with gaps.
//!
//! ── 4. Reading a song **returns it even when it is soft-deleted**. ───────
//!
//! [`load_song`] does not filter on `deleted_at`; it returns the row and puts
//! the tombstone in [`Song::deleted_at`] for the caller to see. FR-202 promises
//! a 30-day recovery window, and a restore path has to be able to *read* what
//! it restores — a filter here would make that impossible through this function
//! and invite a second, unfiltered reader beside it.
//!
//! The paths that must hide deleted songs are the list and the search
//! (FR-201, FR-202), and Appendix A already leans that way for them:
//! `idx_songs_title` is a partial index on `deleted_at IS NULL`. Those items
//! own the filter. This one owns the fact.
//!
//! **[`expand_arrangement`] is the third reader, and unlike the other two it
//! has no channel to carry the tombstone in.** `Vec<SongSection>` has no field
//! for one and that function never reads `songs` at all, so a soft-deleted
//! song's words project onto a screen exactly as a live song's do, with nothing
//! in the result that says so. A stored service plan (FR-301) holding the
//! arrangement id of a song deleted last month shows it whole.
//!
//! **And its `Ok(None)` leads back here by a second route**, which is why that
//! function's doc names this decision and this decision names it: a caller
//! recovering from a stale arrangement id the way that doc prescribes — fall
//! back to [`Song::default_arrangement_id`] — reaches the tombstoned song's
//! default arrangement and expands that instead. The fallback is not a way out
//! of this decision; it is another door into it. The case that does stop the
//! caller is the *hard*-deleted song, and it stops it by leaving no default id
//! to read.
//!
//! **The position taken is that this stays**, and it is a decision rather than
//! an omission. Refusing the read would break the preview a restore has to go
//! through, which is the reason above; widening the return type to carry a
//! tombstone would make every caller handle a state most of them cannot act on,
//! and would still be advisory. **Nothing here closes it**, and the sentence
//! that used to say otherwise was wrong. The `song_id` argument
//! [`expand_arrangement`] takes is not the closure it was written to be: it
//! lets the function check that the pair the caller holds is a pair, which is
//! what its own doc claims for it, but it opens no channel for the tombstone.
//! A caller holding a song id could always call [`load_song`] and read
//! [`Song::deleted_at`] — before that argument existed and after — so the
//! recovery is exactly as available, and exactly as easy to forget, as it ever
//! was. **Whoever renders a stored
//! service plan (FR-301) owes that call**, which is a hand-off rather than a
//! fix; it is recorded on FR-301's own line so that it reaches a reader who
//! will act on it, rather than only being found on a Sunday.
//!
//! ── 5. Nothing here touches `songs_fts`. ────────────────────────────────
//!
//! See `super`. FR-201 owns the index, including backfilling the songs written
//! before it exists.
//!
//! ── Who bounds the input ────────────────────────────────────────────────
//!
//! Nothing in this module, and the claim is made over **columns** rather than
//! over a list of interesting axes: **every TEXT column this module binds is
//! unbounded here, save one**, and two counts are unbounded beside them.
//!
//! The exception is `song_sections.section_type`, bound from
//! [`SectionType::as_str`] and therefore never anything but one of Appendix A's
//! nine spellings. The rest is caller text passed straight through — `songs`'
//! thirteen (`id`, `title`, `alternate_title`, `ccli_number`,
//! `copyright_text`, `song_key`, `default_arrangement_id`, `source_provider`,
//! `source_url`, `retrieved_at`, `created_at`, `updated_at`, `deleted_at`),
//! `song_sections`' `id`, `song_id`, `label` and `content`,
//! `song_arrangements`' `id`, `song_id`, `name` and `created_at`, and
//! `arrangement_items`' `arrangement_id` and `section_id`. The two counts are
//! how many sections a song has and how many items an arrangement plays.
//!
//! **Over columns because the list of axes that stood here called itself
//! exhaustive and was not.** It named five — `title`, `label`, `content`,
//! section count, item count — and left out ten free-text `songs` columns and
//! `song_arrangements.name`. That mattered for the reason the list gave for
//! existing: anything that forwards its bounding obligation "to the head of
//! this module" forwards it *here*, so a column missing from here is a column
//! nobody was asked to bound. A reader sizing FR-202's wire types against the
//! old list would have capped three strings and left eleven open, and the
//! `SQLITE_TRANSIENT` argument made below for `content` holds identically for
//! a 20 MB `copyright_text`. `song_arrangements.name` is the sharpest of the
//! ones that were missing: it arrives as operator text through the public door
//! [`insert_arrangement`], and on that door's empty-items path it is the only
//! column written at all. A list of columns grows only with a migration; a
//! list of axes grows whenever someone notices one.
//!
//! The columns are not all the same risk, and the counts are a different risk
//! again — read as one sentence they point the reader at the cheapest of them,
//! so the ones that differ in kind are separated below.
//!
//! *How many sections* is a linear cost on the way in. The insert loop below
//! reuses a single prepared statement, so N sections are N bindings, N WAL
//! frames and time. **On [`insert_song`] that is all it is; on
//! [`insert_song_with_default_arrangement`] it is not.** That function builds a
//! `Vec<ArrangementItem>` holding one cloned section id per section *before*
//! the transaction opens, and holds it across the whole of it — so on that path
//! this axis carries a heap cost as well. The sentence corrected here, that the
//! section count means "no heap amplification", was written when the insert
//! loop was the only consumer of the axis; ADR-0049 records the correction
//! rather than hiding it.
//!
//! *How long a text column is* is the dangerous size on the way in. `params!`
//! binds a `&String` as text with `SQLITE_TRANSIENT`, so **SQLite copies the
//! buffer**: the peak is roughly twice the size of the value, on top of the WAL
//! frames held until the commit. `content` is named throughout this file
//! because it is where a long paste usually lands, not because it is the only
//! column that behaves this way — a 20 MB `copyright_text` or a 20 MB
//! `song_arrangements.name` costs exactly the same, and every column listed
//! above is bound by the same macro.
//!
//! *How many items an arrangement plays* is the dangerous size on the way
//! **out**, and no ceiling on any single column catches it.
//! [`expand_arrangement`] materialises one copy of a section's `content` for
//! every item that names it, so a single 2 MB section played by 10 000 items
//! expands to **20 GB** out of stored rows totalling about 2 MB — an `.aero`
//! file that passes every limit on `content` and on section count, and a
//! database that is small on disk and small in a backup. **File size is no
//! proxy at all for the cost of a read.** What has to be bounded is the product
//! of two numbers, and this module bounds neither of them.
//!
//! And `SQLITE_MAX_LENGTH` — 1 GB by default — is the only limit a 50 MB paste
//! would *meet*, which is true and, said alone, misleading: it is 45× the
//! whole 22 MB the Rust host process is given by NFR-01's memory budget (PRD
//! §5.1). The process is what gives way first, and long before SQLite objects.
//!
//! The bound belongs to the boundaries that have one to give — the IPC surface
//! (FR-202) and the file-size limits on the import paths (FR-701, FR-604) —
//! because the right number differs per call site and one chosen here could not
//! be raised by a caller that needed it larger. Naming a bounder does not
//! cancel a write, so the trigger is written down instead of left as an
//! intention, and it is written over **tables** rather than over a function
//! name: the first commit that registers a `#[tauri::command]` writing to
//! `songs`, `song_sections`, `song_arrangements` or `arrangement_items`
//! **through any door** owes the limit (ADR-0049).
//!
//! **All four tables, because three was already evadable.** The wording before
//! this one named `songs`, `song_sections` and `arrangement_items`, and this
//! module writes four tables. A command registering [`insert_arrangement`] for
//! a **named arrangement with no items** runs one statement — `INSERT INTO
//! song_arrangements` — and its item loop runs zero times, so it writes none
//! of the three tables that wording named while carrying an unbounded `name`
//! to disk. The empty arrangement is not hypothetical: this module blesses it
//! at [`insert_song_with_default_arrangement`], for the reasons stated there.
//!
//! Over a function name it had been evaded once before. The earliest wording
//! named [`insert_song`], and [`insert_song_with_default_arrangement`] does not
//! call it — it calls `refuse_uninsertable` and `write_song` directly — so a
//! command reaching only the newer door would meet the letter of that trigger
//! and miss its point. Both evasions are ADR-0045's own shape, a second entry
//! path, turned back on this obligation — the second of them on the wording
//! written to close the first. Tables do not grow without a migration; function
//! names grow whenever a path is convenient, and a list of *some* tables is a
//! function name wearing a table's clothes.
//!
//! **The read door owes a bound of its own, and no limit on writing implies
//! it.** The first commit that registers a `#[tauri::command]` reaching
//! [`expand_arrangement`] owes a ceiling on the item count of what it expands:
//! the arrangement that costs 20 GB to read costs 2 MB to write and satisfies
//! every limit the sentence above asks for. Stated because a function that
//! accepts untrusted input owes the reader the name of whoever bounds it
//! (ADR-0047).

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::db::error::DbError;

/// The nine values `song_sections.section_type` allows.
///
/// Listed in Appendix A's own order so the two can be diffed by eye. The
/// `CHECK` constraint in the schema and this enum have to agree; the enum is
/// what stops a caller writing a tenth spelling and only finding out at the
/// constraint, and what stops a reader treating the column as free text.
///
/// The type is a *classification*, not the label. A song may have "Verse 1",
/// "Verse 2" and "Verse 3", all [`SectionType::Verse`]; the label is what the
/// operator sees, the type is what a template or a report groups by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionType {
    /// A numbered verse.
    Verse,
    /// The refrain.
    Chorus,
    /// A bridge.
    Bridge,
    /// A pre-chorus, sometimes called a rise or a channel.
    PreChorus,
    /// A short repeated tag.
    Tag,
    /// A closing section.
    Ending,
    /// An instrumental or spoken introduction.
    Intro,
    /// An instrumental passage between sung sections.
    Interlude,
    /// Anything the eight above do not describe.
    Other,
}

impl SectionType {
    /// The spelling stored in the column, which is the spelling the `CHECK`
    /// constraint lists.
    ///
    /// This is not a display label: it is `pre_chorus`, not "Pre-Chorus". What
    /// an operator reads comes from `label`, or from the frontend.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verse => "verse",
            Self::Chorus => "chorus",
            Self::Bridge => "bridge",
            Self::PreChorus => "pre_chorus",
            Self::Tag => "tag",
            Self::Ending => "ending",
            Self::Intro => "intro",
            Self::Interlude => "interlude",
            Self::Other => "other",
        }
    }

    /// The inverse of [`SectionType::as_str`], for a value read back out of the
    /// column.
    ///
    /// Exact match, no case folding and no trimming: the schema's `CHECK` is a
    /// literal `IN` list under SQLite's default `BINARY` collation, so `Verse`
    /// and `verse ` are values that column cannot hold. Accepting them here
    /// would be a rule this module enforces on the way out and the schema does
    /// not on the way in.
    ///
    /// `None` means the row is not one Appendix A could have produced; the read
    /// path turns that into [`DbError::UnknownSectionType`] rather than
    /// guessing [`SectionType::Other`].
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "verse" => Some(Self::Verse),
            "chorus" => Some(Self::Chorus),
            "bridge" => Some(Self::Bridge),
            "pre_chorus" => Some(Self::PreChorus),
            "tag" => Some(Self::Tag),
            "ending" => Some(Self::Ending),
            "intro" => Some(Self::Intro),
            "interlude" => Some(Self::Interlude),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

/// One labelled block of a song's lyrics, stored exactly once (FR-203).
///
/// Its order within the song is its position in [`Song::sections`]; see the
/// head of this module for why there is no `sort_order` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSection {
    /// Stable id, supplied by the caller. Referenced by `arrangement_items`,
    /// so it outlives any particular arrangement.
    pub id: String,
    /// What the operator calls this section: "Verse 1", "Chorus", "Bridge".
    /// Unique within the song — `song_sections` has `UNIQUE (song_id, label)` —
    /// and compared byte for byte, so "Chorus" and "chorus" are two labels.
    pub label: String,
    /// What kind of section it is, independent of what it is called.
    pub section_type: SectionType,
    /// The lyric text, stored verbatim.
    ///
    /// **May contain any of the seven line separators `split_slides` counts,
    /// not just `\n`** — see the head of this module. Untrusted: it is whatever
    /// a paste, an import or another program's `.aero` file contained, and it
    /// must reach a DOM as a text node, never as markup (ADR-0042).
    pub content: String,
}

/// A song, with its sections in the order they were authored in.
///
/// The same type is what [`insert_song`] writes and what [`load_song`] returns,
/// so a song that goes in comes back equal to itself. That is a stronger claim
/// than two near-identical structs would allow, and it is the reason `id` and
/// the timestamps are ordinary fields: whoever mints them puts them here.
///
/// Arrangements are deliberately *not* a field. [`insert_song`] would then have
/// to either write them — it cannot, an arrangement's items reference sections
/// that do not exist until this call commits — or ignore them, and a field a
/// function provably ignores invites the caller to believe it was used
/// (ADR-0046). They are read and written through [`Arrangement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    /// Stable id, supplied by the caller. Appendix A expects a UUIDv7; this
    /// module does not check that.
    pub id: String,
    /// The title as the operator entered it.
    pub title: String,
    /// A second title the song is also known by, if any.
    pub alternate_title: Option<String>,
    /// CCLI licence number, for churches that report their usage (FR-605).
    pub ccli_number: Option<String>,
    /// The copyright line to display under the lyrics.
    pub copyright_text: Option<String>,
    /// Musical key, as free text — "G", "Bb", "F#m".
    pub song_key: Option<String>,
    /// Tempo in beats per minute.
    pub tempo_bpm: Option<i64>,
    /// The arrangement used when nothing else is chosen (FR-204).
    ///
    /// **Must be `None` at insert time.** Appendix A's insert trigger rejects
    /// any other value, and necessarily so: an arrangement of a song that does
    /// not exist yet cannot exist either. [`insert_song`] refuses it up front
    /// with [`DbError::DefaultArrangementAtInsert`] so the reason is legible.
    ///
    /// The column is filled by [`insert_song_with_default_arrangement`], which
    /// writes the arrangement and points the row at it inside the transaction
    /// that created the song. So this field is `None` on every value handed
    /// *in*; on a value read back it is whatever the row holds — `Some` for a
    /// song written by that function, `None` for one written by
    /// [`insert_song`] and never pointed at an arrangement since.
    pub default_arrangement_id: Option<String>,
    /// Where an imported song came from — the provider's name (FR-605).
    pub source_provider: Option<String>,
    /// The URL it was retrieved from (FR-605).
    pub source_url: Option<String>,
    /// When it was retrieved, ISO-8601 UTC.
    pub retrieved_at: Option<String>,
    /// When the song was created, ISO-8601 UTC. Supplied by the caller.
    pub created_at: String,
    /// When it was last modified, ISO-8601 UTC. Supplied by the caller.
    pub updated_at: String,
    /// When it was soft-deleted, ISO-8601 UTC, or `None` while it is live
    /// (FR-202).
    ///
    /// [`load_song`] returns soft-deleted songs; this field is how the caller
    /// knows. Listing and search must exclude them — see the head of this
    /// module.
    pub deleted_at: Option<String>,
    /// The song's sections, in authoring order. The position in this `Vec` is
    /// what is stored in `sort_order`.
    pub sections: Vec<SongSection>,
}

/// One entry in an arrangement: play this section next.
///
/// The same `section_id` may appear at any number of positions, and that is the
/// point of FR-203 — "Verse 1, Chorus, Verse 2, Chorus, Chorus" is five entries
/// over four sections, and the chorus's text is stored once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangementItem {
    /// Where in the arrangement this entry sits. Part of the primary key
    /// together with the arrangement, so two entries cannot share one.
    ///
    /// The read path orders by it. It need not be contiguous; nothing here
    /// requires that, because a gap changes no order.
    pub position: i64,
    /// The section to play. Must belong to the same song as the arrangement —
    /// a constraint the schema does not express, so [`insert_arrangement`]
    /// checks it.
    pub section_id: String,
}

/// A named order in which a song's sections are sung (FR-204).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrangement {
    /// Stable id, supplied by the caller.
    pub id: String,
    /// The song this arrangement belongs to.
    ///
    /// Redundant when the arrangement was obtained from
    /// [`load_arrangements`], which was already given the song. It is carried
    /// anyway so one type serves both directions: [`insert_arrangement`] needs
    /// it, and it is what every `section_id` is checked against.
    pub song_id: String,
    /// What the operator calls it: "Default", "Short version". Unique within
    /// the song.
    pub name: String,
    /// When it was created, ISO-8601 UTC. Supplied by the caller.
    pub created_at: String,
    /// The sections to play, in order.
    pub items: Vec<ArrangementItem>,
}

/// Writes a song and all of its sections, in one transaction.
///
/// Either the song row and every section row land, or none of them do: a
/// half-written song is worse than a song that failed to write, because it
/// looks like a song.
///
/// Two refusals happen **before** the transaction opens, so a rejected song
/// costs no database work and the diagnostic names the thing to fix rather than
/// the constraint that tripped:
///
/// * a `default_arrangement_id` that is not `None`
///   ([`DbError::DefaultArrangementAtInsert`]),
/// * two sections sharing a label ([`DbError::DuplicateSectionLabel`]).
///
/// Both refusals live in `refuse_uninsertable`, so
/// [`insert_song_with_default_arrangement`] makes exactly the same two and
/// neither can drift from the other.
///
/// **Insert, not upsert.** A song whose id already exists fails on the primary
/// key. Update — and with it the question of what happens to a section that
/// disappeared from the list — is FR-202's, which owns `upsert_song`.
///
/// **The duplicate-label check covers this function's input and nothing
/// else.** It compares the sections handed in against each other; it cannot see
/// rows already in the table, which is sound here only because the song is new.
/// Any later path that adds a section to an existing song must check against
/// the table or map the `UNIQUE` failure itself — the second-entry-path shape
/// ADR-0045 named.
///
/// **A song may be inserted already tombstoned.** [`Song::deleted_at`] is
/// bound exactly as handed in, so `Some(…)` writes a song that was never
/// visible. That is deliberate, and it is for one kind of caller: `.aero`
/// import (FR-701, FR-703) and any restore-from-backup path have to reproduce
/// a library as it was, and a song whose tombstone is dropped on the way in
/// reappears in somebody's set list. The price is stated because it compounds
/// with decision 4 at the head of this module: such a song is in no list
/// (FR-201 and FR-202 filter on `deleted_at IS NULL`) and no operator deleted
/// it, so no operator will restore it either — it is reachable only through
/// [`load_song`] by id. Whoever mints the record owes that decision; this
/// module does not second-guess it, for the same reason it does not validate
/// an id.
pub fn insert_song(conn: &mut Connection, song: &Song) -> Result<(), DbError> {
    refuse_uninsertable(song)?;

    let tx = conn.transaction()?;
    write_song(&tx, song)?;
    tx.commit()?;
    Ok(())
}

/// Writes a song, its sections **and its default arrangement**, in one
/// transaction (FR-204).
///
/// FR-204 says the default arrangement is "generated on creation", and that is
/// atomic or it is nothing. A song that landed without its default arrangement
/// is the same half-written state [`insert_song`] refuses to leave behind for
/// sections — it *looks* like a song — except that no constraint marks it, so
/// every later reader carries a case for it forever. The three statements
/// [`insert_song`] and [`insert_arrangement`] would run as two calls therefore
/// run here as one transaction, with the `UPDATE` that points
/// `songs.default_arrangement_id` at the result as the third.
///
/// Their order is the only one the schema allows, and it is the order
/// [`DbError::DefaultArrangementAtInsert`] spells out: insert the song with
/// `NULL`, create the arrangement, then update. So `song` must still carry
/// `default_arrangement_id: None` and is refused with that same error if it
/// does not — this function chooses the value, and a caller that supplied one
/// meant something this function does not do.
///
/// **`arrangement_id` and `created_at` are arguments**, for the reason
/// decision 2 at the head of this module gives: nothing here reads a clock or
/// invents an id. What is *generated* is the arrangement's contents, not its
/// identity.
///
/// **One item per section, in the song's own order, numbered from 0.** Zero
/// rather than one because `song_sections.sort_order` is already the index in
/// [`Song::sections`], so for the default arrangement `position` and
/// `sort_order` are the same number for the same section, and a reader diffing
/// the two tables sees identity rather than an off-by-one to explain. Nothing
/// depends on the choice — `position` only has to order, and it is allowed to
/// have gaps (see [`ArrangementItem::position`]).
///
/// **A song with no sections still gets its default arrangement, empty.** The
/// trigger allows it: it checks that the arrangement exists and belongs to this
/// song, not that it plays anything. The alternative — skipping the write —
/// costs more than it saves, because `default_arrangement_id IS NULL` would
/// then mean either "written by a path that predates FR-204" or "had no
/// sections at the time", two facts no reader can tell apart; and the first
/// path that adds a section to such a song would have to create the arrangement
/// as well, which is the second entry path ADR-0045 names. It would also
/// quietly ignore the `arrangement_id` it was handed.
///
/// **Neither trigger on `songs` can fire on this path, which is why no error
/// variant maps their message.** They `RAISE(ABORT, …)`, and that reaches a
/// caller raw as `SqliteFailure(1811, "default_arrangement_id must belong to
/// this song")` — the message [`DbError::DefaultArrangementAtInsert`] exists to
/// keep out of sight. The `BEFORE INSERT` trigger's `WHEN` is false because the
/// column is bound `NULL`, which `refuse_uninsertable` has already proved; the
/// `BEFORE UPDATE OF default_arrangement_id` trigger's is false because the row
/// it looks for — this id, this song — was inserted earlier in this same
/// transaction. An `arrangement_id` that already names another song's
/// arrangement never reaches the update either: it collides with
/// `song_arrangements`' primary key first. A variant for a branch no input can
/// reach is a case every reader has to rule out again.
///
/// The cross-song check inside `write_arrangement` is redundant here — every
/// section was written to this song two statements earlier — and it is paid
/// anyway, one primary-key lookup per section inside the open transaction. The
/// alternative is a second writer for `arrangement_items`, which would have to
/// be kept in step with the first.
pub fn insert_song_with_default_arrangement(
    conn: &mut Connection,
    song: &Song,
    arrangement_id: &str,
    created_at: &str,
) -> Result<(), DbError> {
    refuse_uninsertable(song)?;

    let arrangement = Arrangement {
        id: arrangement_id.to_owned(),
        song_id: song.id.clone(),
        // The spelling `song_arrangements.name` gives as its own example.
        // Written out here rather than named as a constant a test could import:
        // a test that asserts against the constant asserts nothing about the
        // string.
        name: "Default".to_owned(),
        created_at: created_at.to_owned(),
        // Counted in `i64` the same way `sort_order` is, and from the same
        // zero, so the two columns agree section by section.
        items: (0i64..)
            .zip(&song.sections)
            .map(|(position, section)| ArrangementItem {
                position,
                section_id: section.id.clone(),
            })
            .collect(),
    };

    let tx = conn.transaction()?;
    write_song(&tx, song)?;
    write_arrangement(&tx, &arrangement)?;
    tx.execute(
        "UPDATE songs SET default_arrangement_id = ?1 WHERE id = ?2",
        params![arrangement.id, song.id],
    )?;
    tx.commit()?;
    Ok(())
}

/// The two refusals both insert paths make **before** opening a transaction.
///
/// Extracted rather than repeated so the pair cannot drift, and kept out of
/// [`write_song`] so that it stays true of both callers that a rejected song
/// costs no database work at all.
fn refuse_uninsertable(song: &Song) -> Result<(), DbError> {
    if song.default_arrangement_id.is_some() {
        return Err(DbError::DefaultArrangementAtInsert {
            song_id: song.id.clone(),
        });
    }
    if let Some(label) = first_repeated_label(&song.sections) {
        return Err(DbError::DuplicateSectionLabel {
            song_id: song.id.clone(),
            label: label.to_owned(),
        });
    }
    Ok(())
}

/// Writes the `songs` row and its `song_sections` rows into an open
/// transaction.
///
/// Split out of [`insert_song`] so that
/// [`insert_song_with_default_arrangement`] reuses these statements instead of
/// holding a second copy of them — a second copy is a second place a column has
/// to be added. It takes a [`Transaction`] rather than a `Connection` so that
/// it *cannot* commit: which writes belong together is the caller's decision,
/// and both callers make it in one visible place.
///
/// It assumes [`refuse_uninsertable`] has already returned `Ok`. The `NULL`
/// bound into `default_arrangement_id` below is only true because of that call.
fn write_song(tx: &Transaction<'_>, song: &Song) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO songs (
             id, title, alternate_title, ccli_number, copyright_text, song_key,
             tempo_bpm, default_arrangement_id, source_provider, source_url,
             retrieved_at, created_at, updated_at, deleted_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            song.id,
            song.title,
            song.alternate_title,
            song.ccli_number,
            song.copyright_text,
            song.song_key,
            song.tempo_bpm,
            // Proved `None` above. Bound rather than written as a literal
            // `NULL`, so this statement stores what the record says and the
            // schema's insert trigger stays the backstop it was written to be.
            song.default_arrangement_id,
            song.source_provider,
            song.source_url,
            song.retrieved_at,
            song.created_at,
            song.updated_at,
            song.deleted_at,
        ],
    )?;

    {
        let mut insert = tx.prepare(
            "INSERT INTO song_sections (id, song_id, label, section_type, content, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        // Counted in `i64` — the column's own type — by zipping an ascending
        // range rather than casting `enumerate`'s `usize`. There is no cast to
        // reason about and no unreachable overflow arm to write.
        for (sort_order, section) in (0i64..).zip(&song.sections) {
            insert.execute(params![
                section.id,
                song.id,
                section.label,
                section.section_type.as_str(),
                section.content,
                sort_order,
            ])?;
        }
    }

    Ok(())
}

/// Reads a song and its sections, in `sort_order`.
///
/// `Ok(None)` means no song has that id. **A soft-deleted song is returned**,
/// with [`Song::deleted_at`] set — see the head of this module for why that is
/// the right answer here and whose job the filtering is.
///
/// [`Song::default_arrangement_id`] is whatever the row holds; a song read back
/// after FR-204 has pointed it at an arrangement therefore cannot be handed
/// straight to [`insert_song`], which refuses a non-`None` value. That is not a
/// round-trip this function promises: a song that already exists is an update,
/// and updates are FR-202's.
pub fn load_song(conn: &Connection, song_id: &str) -> Result<Option<Song>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, alternate_title, ccli_number, copyright_text, song_key,
                tempo_bpm, default_arrangement_id, source_provider, source_url,
                retrieved_at, created_at, updated_at, deleted_at
         FROM songs WHERE id = ?1",
    )?;
    let song = stmt
        .query_row([song_id], |row| {
            Ok(Song {
                id: row.get(0)?,
                title: row.get(1)?,
                alternate_title: row.get(2)?,
                ccli_number: row.get(3)?,
                copyright_text: row.get(4)?,
                song_key: row.get(5)?,
                tempo_bpm: row.get(6)?,
                default_arrangement_id: row.get(7)?,
                source_provider: row.get(8)?,
                source_url: row.get(9)?,
                retrieved_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                deleted_at: row.get(13)?,
                sections: Vec::new(),
            })
        })
        .optional()?;

    match song {
        None => Ok(None),
        Some(mut song) => {
            song.sections = load_sections(conn, song_id)?;
            Ok(Some(song))
        }
    }
}

/// Reads one song's sections, in `sort_order`.
///
/// Ordered by `(sort_order, id)` rather than by `sort_order` alone. Written
/// through [`insert_song`] the two can never tie — `sort_order` is the position
/// in the `Vec` — but a row written by any other path can, and an order that
/// depends on SQLite's choice of plan is an order two machines may disagree
/// about. The tie-break costs nothing and makes the answer a fact rather than a
/// coincidence.
fn load_sections(conn: &Connection, song_id: &str) -> Result<Vec<SongSection>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, label, section_type, content
         FROM song_sections WHERE song_id = ?1
         ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([song_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut sections = Vec::new();
    for row in rows {
        let (id, label, section_type, content) = row?;
        let Some(section_type) = SectionType::from_db(&section_type) else {
            return Err(DbError::UnknownSectionType {
                section_id: id,
                value: section_type,
            });
        };
        sections.push(SongSection {
            id,
            label,
            section_type,
            content,
        });
    }
    Ok(sections)
}

/// Writes an arrangement and its items, in one transaction.
///
/// Every `section_id` is checked to belong to [`Arrangement::song_id`] first,
/// inside the same transaction, and a failure aborts the whole write with
/// [`DbError::SectionNotInSong`]. The schema cannot express that constraint:
/// `arrangement_items.section_id` references `song_sections(id)` and nothing
/// ties it to the arrangement's song, so an arrangement of one song may point
/// at another song's section with every foreign key satisfied — and editing
/// that section's lyrics would then change a song nobody edited.
///
/// The check also covers a section id that exists nowhere, which the foreign
/// key would catch as well; catching it here names the id instead of the
/// constraint.
///
/// Repeating a `section_id` across positions is **not** an error. It is what
/// FR-203 exists for.
///
/// Insert, not upsert, for the same reason as [`insert_song`]. Generating the
/// default arrangement, and pointing `songs.default_arrangement_id` at it,
/// belong to [`insert_song_with_default_arrangement`], which reuses the
/// statements below rather than repeating them.
pub fn insert_arrangement(conn: &mut Connection, arrangement: &Arrangement) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    write_arrangement(&tx, arrangement)?;
    tx.commit()?;
    Ok(())
}

/// Writes the `song_arrangements` row and its `arrangement_items` rows into an
/// open transaction, checking each section's owner as it goes.
///
/// Split out of [`insert_arrangement`] for the reason [`write_song`] is split
/// out of [`insert_song`], and with the same consequence: it cannot commit, so
/// [`insert_song_with_default_arrangement`] can put this write and that one
/// inside one transaction without either function knowing about the other.
///
/// The `song_arrangements` row goes in **before** the first section is read.
/// That is what keeps this write lock-safe under the `BEGIN DEFERRED` `super`
/// describes — the write lock is taken by the first statement, so the read that
/// follows never has to upgrade a snapshot.
fn write_arrangement(tx: &Transaction<'_>, arrangement: &Arrangement) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO song_arrangements (id, song_id, name, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            arrangement.id,
            arrangement.song_id,
            arrangement.name,
            arrangement.created_at,
        ],
    )?;

    {
        let mut owner = tx.prepare("SELECT song_id FROM song_sections WHERE id = ?1")?;
        let mut insert = tx.prepare(
            "INSERT INTO arrangement_items (arrangement_id, position, section_id)
             VALUES (?1, ?2, ?3)",
        )?;
        for item in &arrangement.items {
            let owning_song: Option<String> = owner
                .query_row([&item.section_id], |row| row.get(0))
                .optional()?;
            if owning_song.as_deref() != Some(arrangement.song_id.as_str()) {
                // Returning here leaves the caller's `tx` un-committed, and
                // dropping it rolls back — the arrangement row inserted above
                // included, and on the `insert_song_with_default_arrangement`
                // path the song and its sections with it.
                return Err(DbError::SectionNotInSong {
                    arrangement_id: arrangement.id.clone(),
                    song_id: arrangement.song_id.clone(),
                    section_id: item.section_id.clone(),
                });
            }
            insert.execute(params![arrangement.id, item.position, item.section_id])?;
        }
    }

    Ok(())
}

/// Reads one song's arrangements, each with its items in `position` order.
///
/// Arrangements come back ordered by `name`, which `UNIQUE (song_id, name)`
/// makes a total order — so the answer does not depend on the query plan. An
/// empty `Vec` means the song has no arrangements at all. [`insert_song`]
/// leaves a song in that state and [`insert_song_with_default_arrangement`]
/// never does, but the state is not a property of how the song was written:
/// [`insert_arrangement`] is a public door that takes any song out of it, and
/// nothing puts a song back in, since no path here deletes an arrangement.
///
/// **This path does not re-check that each item's section belongs to
/// `song_id`.** It returns whatever `arrangement_items` holds.
/// [`insert_arrangement`] closes that hole for its own door and only for its
/// own door; the schema cannot express the constraint at all.
///
/// **Why that differs from `load_sections`, which does check on read** and
/// refuses a foreign `section_type` with [`DbError::UnknownSectionType`]: the
/// two are not the same purchase. The `section_type` check is a comparison
/// against a fixed list of nine strings on a row already in hand. This one
/// would be **one extra query per item** *here*, because this function reads
/// `arrangement_items` and nothing else; and the real owner is the *writer* — a
/// cross-song item can only get into the table through a writer that skipped
/// the check, and `.aero` import (FR-703) is the first such writer (ADR-0049).
/// A read-side check would also be the wrong remedy: it would fail the whole
/// read of an arrangement rather than fix the row.
///
/// **[`expand_arrangement`] does check, and that is not a contradiction of
/// either half.** It joins `song_sections` for the text, so the owner arrives
/// in the row it already fetched and the first reason costs it nothing. The
/// second reason is what decides *which* of the two functions refuses: a read
/// that fails cannot be the read a repair goes through, so this one keeps
/// returning the table exactly as it stands — the same reason [`load_song`]
/// returns a song someone soft-deleted.
///
/// **The consequence goes further than editing.** `songs.id` cascades:
/// `songs → song_sections → arrangement_items`. So deleting song B removes one
/// *position* from an arrangement of song A that pointed at B's section, and
/// the service order comes back one item shorter than the operator left it,
/// with nothing edited and nothing raised. Whoever hard-deletes a song
/// (FR-202) inherits that.
pub fn load_arrangements(conn: &Connection, song_id: &str) -> Result<Vec<Arrangement>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, created_at
         FROM song_arrangements WHERE song_id = ?1
         ORDER BY name",
    )?;
    let rows = stmt.query_map([song_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut arrangements = Vec::new();
    for row in rows {
        let (id, name, created_at) = row?;
        arrangements.push(Arrangement {
            id,
            song_id: song_id.to_owned(),
            name,
            created_at,
            items: Vec::new(),
        });
    }

    // Filled in a second pass rather than inside the loop above. The reason
    // first written here was that a nested prepare would be a second live
    // statement over the borrow `stmt` holds, and that is **wrong**:
    // `Connection::prepare` takes `&self`, so two `Statement`s may be alive
    // over one `&Connection`, and calling `load_arrangement_items(conn, …)`
    // inside the loop compiles. It was checked, not reasoned about.
    //
    // What two passes actually buy is that each arrangement's items are one
    // ordered read and the outer scan is finished before any of them runs.
    // What they cost is that the answer is stitched out of N+1 statements and
    // therefore N+1 snapshots, so a concurrent writer can be visible to some
    // and not to others. This function claims nothing that forbids it — it
    // reports what the tables hold. `expand_arrangement` does make a claim of
    // that kind, and is a single statement for exactly that reason.
    for arrangement in &mut arrangements {
        arrangement.items = load_arrangement_items(conn, &arrangement.id)?;
    }
    Ok(arrangements)
}

/// Reads one arrangement's items, in `position` order.
///
/// No tie-break is needed: `position` is part of the primary key, so it is
/// unique within an arrangement and the order is total.
fn load_arrangement_items(
    conn: &Connection,
    arrangement_id: &str,
) -> Result<Vec<ArrangementItem>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT position, section_id
         FROM arrangement_items WHERE arrangement_id = ?1
         ORDER BY position",
    )?;
    let rows = stmt.query_map([arrangement_id], |row| {
        Ok(ArrangementItem {
            position: row.get(0)?,
            section_id: row.get(1)?,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

/// Expands one of a song's arrangements into the sequence of sections it plays,
/// in `position` order and with every repetition materialised (FR-204).
///
/// **`song_id` is an argument rather than something this function derives, and
/// that is the whole of what it can promise about whose words come back.** The
/// arrangement row carries its own `song_id`, so deriving it would make every
/// answer internally consistent and none of them checked — a Control Panel
/// holding `songId` and `arrangementId` as two pieces of UI state, and sending
/// a stale `arrangementId` belonging to song B while song A's title is on
/// screen, would get `Ok(Some(…))` full of song B's lyrics with nothing raised
/// anywhere. The caller's belief about which song is being shown has to enter
/// the function or it cannot be compared with anything, so it enters as this
/// argument and `WHERE a.id = ?1 AND a.song_id = ?2` is where it is spent.
///
/// `Ok(None)` means **this song has no arrangement with that id**, and three
/// states produce it, not two: no arrangement anywhere carries that id; one
/// does and belongs to another song; or `song_id` names nothing at all, because
/// that song never existed or was hard-deleted — `song_arrangements.song_id`
/// references `songs(id)` `ON DELETE CASCADE`, so its arrangements went with
/// it. The three are deliberately one answer, since each means the pair the
/// caller holds is not a pair.
///
/// **The recovery differs between them, though, so prescribing one is not
/// enough.** For the first two, falling back to
/// [`Song::default_arrangement_id`] is right. For the third it has no source:
/// [`load_song`] returns `None` for a song that is not there, so there is no
/// default id to fall back *to*, and the caller's next honest move is to report
/// the song gone rather than to expand anything. For a song that is merely
/// **soft-deleted** the fallback succeeds instead, and lands the caller in
/// decision 4 at the head of this module — the default arrangement of a
/// tombstoned song expands like any other, and nothing in the result says so.
/// Decision 4 names this paragraph and this one names it back, because they are
/// one fact seen from its two ends.
///
/// `Ok(Some(vec![]))` is the answer none of the three gets: an arrangement of
/// *this* song exists and plays nothing, which is what a song with no sections
/// gets from [`insert_song_with_default_arrangement`]. Collapsing it into
/// `None` would answer a pair that does not exist with an arrangement that is
/// merely empty.
///
/// **A claim of that kind cannot be held by two statements outside one
/// transaction, so it is held by one statement.** The shape this replaced —
/// read `song_arrangements` to learn whether the arrangement exists, then read
/// `arrangement_items` for its items — took a fresh snapshot for each. Under
/// WAL with a second connection open (the configuration `super` describes at
/// length), a `DELETE FROM song_arrangements` landing between the two leaves
/// the first saying yes and the second returning no rows, which that shape
/// reported as `Ok(Some(vec![]))` — an arrangement that no longer exists,
/// answered with the value this doc reserves for one that exists and is empty.
/// The `LEFT JOIN` below is rooted at `song_arrangements` instead, and a single
/// `SELECT` outside an explicit transaction runs in one implicit read
/// transaction, so existence and contents come from one snapshot.
///
/// **That is a rule, and it is written as a rule because no test in the suite
/// holds it.** The window is only open while the `DELETE` lands *between* two
/// statements, and from outside a one-statement function that instant is not
/// reachable: there is no hook to suspend the call in the middle and this crate
/// starts no threads. The test named for this paragraph,
/// `an_arrangement_deleted_by_another_connection_is_none_not_empty`, writes the
/// replaced shape's three statements out by hand — read the owner, delete from
/// a second connection, count the items — and shows the pair that shape would
/// have answered `Some([])` to. It is an **artefact of the old shape, not a
/// guard on this one**: it calls this function only after the delete has
/// committed, and there a two-statement implementation finds no arrangement in
/// its own first statement, returns `None` as well, and leaves the test green.
/// So the rule carries itself — any edit that splits this `SELECT` in two
/// reopens a window nothing in the suite will report. An earlier wording here
/// called that sequence "the test this claim is owed"; it measures a weaker
/// thing than the sentence it was attached to, and saying so is cheaper than
/// building a harness to make the sentence true.
///
/// [`load_arrangements`] keeps the two-statement shape and makes no claim of
/// this kind; the difference between the two functions is deliberate in both
/// directions.
///
/// **Who owns the distinction, and what would end it.** A distinction no call
/// site acts on is a case every reader must rule out for nothing, and this one
/// may well become that: [`SongSection`] is not a wire type, so the first call
/// site has to map this result whatever it does, and the consumer it is
/// destined for — [`crate::models::split_slides`] — produces zero slides from
/// `None` and zero from `Some(vec![])` alike. So the trigger is written down in
/// the shape the head of this module uses for the size limits: **the first
/// commit that registers a `#[tauri::command]` returning this result** either
/// gives `None` a wire representation of its own — "that arrangement is not
/// this song's; fall back to the default" is a different message from "nothing
/// to show" — or collapses the two, at which point this returns
/// `Result<Vec<SongSection>, DbError>` and this paragraph goes with it.
///
/// **This stops at sections. It does not produce slides**, although FR-204's
/// acceptance criterion says "slide sequence". Where a section breaks into
/// slides is [`crate::models::split_slides`]'s alone to say (ADR-0046,
/// ADR-0047), and it decides it from the template's text box — geometry this
/// layer does not have and must not acquire to keep a phrase. Mapping this
/// result through it is a composition the caller makes, one section at a time.
///
/// **"Editing a section's text updates every occurrence" is inherited here, not
/// enforced.** Every occurrence's text is read from its `song_sections` row at
/// the moment of this call, so a section played five times is five reads of one
/// row and there is no copy for an update to miss. That is FR-203's
/// decomposition doing the work; all this function owes the criterion is not to
/// cache around it.
///
/// The price of materialising the repetitions is stated rather than left to be
/// met: a section played N times is N copies of its `content` in the returned
/// `Vec`. Of this module's five read functions it is the one that materialises
/// a single stored row's text more than once — [`load_song`], `load_sections`,
/// [`load_arrangements`] and `load_arrangement_items` each return one value per
/// row they read — and it does not bound N. **The head of this module names
/// that count**, as one of the two counts it lists beside the columns. It was
/// added there after this function had forwarded its obligation to a list that
/// did not contain it, which is the same failure the head now avoids by stating
/// its columns rather than choosing its axes. Nothing about the size of the
/// database bounds N either: 10 000 items naming one 2 MB section are about
/// 2 MB stored and 20 GB expanded.
///
/// **Four things can be wrong with an arrangement's items. Two are refused
/// here; the third is invisible to any query, and the fourth is dropped in
/// silence.**
///
/// *A section belonging to another song* is refused with
/// [`DbError::SectionNotInSong`]. The join already fetches
/// `song_sections.song_id` to reach the text, so the check costs no extra
/// query — which is the *cost* reason [`load_arrangements`] gives for not
/// making it, and it does not carry over. Its other reason does, and it is what
/// decides which of the two functions refuses: failing a read is no way to
/// repair a row, so the read a repair would go through stays unfiltered, while
/// this one — the projection onto a screen of the song the caller named —
/// refuses to put another song's words under that song's title. That is
/// FR-203's acceptance criterion inverted, arriving through the read door; the
/// writer that let the row in owes the real fix (FR-703, ADR-0049).
///
/// *A `section_id` that names no row at all* is refused with the same variant,
/// whose field documentation already reads "belongs elsewhere or nowhere".
/// **That is a choice between two defensible answers, so it is written as one.**
/// With `foreign_keys = ON` a dangling id cannot arise: `ON DELETE CASCADE`
/// removes the item row rather than orphaning it, which is the case below, and
/// under that pragma an inner join and this outer one return the same rows. But
/// this module already refuses a stored `section_type` outside Appendix A's
/// nine ([`DbError::UnknownSectionType`]) on the stated grounds that such a row
/// "predates the constraint or was written by something other than SQLite", and
/// a dangling `section_id` is reachable under that same threat model — the one
/// this module has already decided to believe in. An inner join drops such a
/// row without a trace, which would leave `load_arrangement_items` and this
/// function answering the same arrangement differently with no error on either
/// side. The answer taken is the one that matches the neighbouring variant:
/// detect it, name the id, and let the two functions differ loudly rather than
/// silently.
///
/// *An item removed by a cascade* is **not** visible, and this function does
/// not pretend otherwise. `arrangement_items.section_id` references
/// `song_sections(id)` `ON DELETE CASCADE`, so hard-deleting song B deletes the
/// *item rows* of song A's arrangement that pointed into B; it leaves no
/// dangling id to detect, only a gap in `position` — and gaps are legal (see
/// [`ArrangementItem::position`]), so a service order that comes back one item
/// short is indistinguishable from one written that way. Proved against a
/// connection rather than inferred from the DDL: positions 0, 1, 2, 3 with 2
/// pointing into song B read back as 0, 1, 3 after `DELETE FROM songs`, with no
/// row left behind and no error raised. Whoever hard-deletes a song (FR-202)
/// inherits that.
///
/// *An item row whose `section_id` is itself NULL* is the fourth, and it is
/// the one this function handles worst: it takes the arm written for the
/// all-NULL row an outer join produces for an arrangement with no items, and is
/// skipped without a word. Through SQLite it cannot occur — the column is `NOT
/// NULL` — but that is precisely the sentence the paragraph above refuses to
/// rest on for a dangling id, so it is not rested on here either: under the
/// same foreign-database model, an arrangement of one such item reads back as
/// `Ok(Some(vec![]))`, the answer reserved for an arrangement that exists and
/// plays nothing. Refusing it needs a discriminator this `SELECT` does not
/// fetch and an error variant that can name no id; the comment at that arm
/// says which, and both belong with the item that repairs such rows (FR-703).
/// Named here rather than left for the next reader to notice, because the
/// asymmetry is the one this function charges the inner join with.
pub fn expand_arrangement(
    conn: &Connection,
    song_id: &str,
    arrangement_id: &str,
) -> Result<Option<Vec<SongSection>>, DbError> {
    // Rooted at `song_arrangements` and joined outwards, so that one statement
    // answers both questions: whether this song has such an arrangement, and
    // what it plays. Both joins are `LEFT` for different reasons — the first so
    // that an arrangement with no items still produces a row, the second so
    // that an item whose section is missing produces one too instead of
    // vanishing.
    let mut stmt = conn.prepare(
        "SELECT i.section_id, s.id, s.label, s.section_type, s.content, s.song_id
         FROM song_arrangements a
         LEFT JOIN arrangement_items i ON i.arrangement_id = a.id
         LEFT JOIN song_sections s ON s.id = i.section_id
         WHERE a.id = ?1 AND a.song_id = ?2
         ORDER BY i.position",
    )?;
    let rows = stmt.query_map(params![arrangement_id, song_id], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let mut arrangement_exists = false;
    let mut sections = Vec::new();
    for row in rows {
        let (item_section_id, id, label, section_type, content, owning_song) = row?;
        // Any row at all is the existence proof, because the join starts at the
        // arrangement: zero rows means no arrangement of this song has this id.
        arrangement_exists = true;
        let Some(item_section_id) = item_section_id else {
            // The all-NULL row an outer join produces for an arrangement with
            // no items. Through SQLite that is the only way this column arrives
            // empty, `arrangement_items.section_id` being `NOT NULL` — but the
            // dangling-id paragraph above believes in rows SQLite did not
            // write, and under that same model a real item row with a NULL
            // `section_id` is dropped here without a word. A one-item
            // arrangement of that shape then reads as `Ok(Some(vec![]))`, the
            // value the doc reserves for "exists and plays nothing": the
            // aliasing that doc refuses for a deleted arrangement, tolerated
            // here. Telling the two apart needs a discriminator this `SELECT`
            // does not fetch (`i.arrangement_id`, which any joined item row has
            // non-NULL because `NULL = a.id` never matches) and an error that
            // can name no id, since there is none. Both belong with the item
            // that repairs such rows (FR-703); written down so the gap is known
            // rather than discovered.
            continue;
        };
        // Every `song_sections` column selected above is `NOT NULL`, so from a
        // database SQLite wrote, a `None` among them means the outer join found
        // no section row: the id dangles. Taken as one pattern rather than
        // column by column, because that is what any of them means on that
        // path. Under the foreign-database model the paragraph above uses, a
        // section row that does exist with a NULL `label` or `content` lands
        // here too, and the error then names an id that is really there and
        // calls it "belongs elsewhere or nowhere" — a wrong diagnosis with the
        // right refusal, since nothing is projected either way. Column-by-
        // column arms would buy precision only about rows SQLite cannot write.
        let (Some(id), Some(label), Some(section_type), Some(content), Some(owning_song)) =
            (id, label, section_type, content, owning_song)
        else {
            return Err(DbError::SectionNotInSong {
                arrangement_id: arrangement_id.to_owned(),
                song_id: song_id.to_owned(),
                section_id: item_section_id,
            });
        };
        // Checked before the type is parsed: which song's words these are is
        // the graver fact about the row, and it is true of it whatever the
        // type says.
        if owning_song != song_id {
            return Err(DbError::SectionNotInSong {
                arrangement_id: arrangement_id.to_owned(),
                song_id: song_id.to_owned(),
                section_id: id,
            });
        }
        let Some(section_type) = SectionType::from_db(&section_type) else {
            return Err(DbError::UnknownSectionType {
                section_id: id,
                value: section_type,
            });
        };
        sections.push(SongSection {
            id,
            label,
            section_type,
            content,
        });
    }

    if !arrangement_exists {
        return Ok(None);
    }
    Ok(Some(sections))
}

/// The first label that appears twice in `sections`, or `None`.
///
/// "First" is in the order the sections were handed in, so the same input
/// always names the same label — a diagnostic that moves is a diagnostic
/// nothing can assert on.
///
/// Compared byte for byte, because that is what the schema compares:
/// `UNIQUE (song_id, label)` uses SQLite's default `BINARY` collation, so
/// "Chorus" and "chorus" are two labels and this must not pretend otherwise.
/// Refusing more than the schema refuses would be this module inventing a rule.
fn first_repeated_label(sections: &[SongSection]) -> Option<&str> {
    let mut seen = std::collections::BTreeSet::new();
    for section in sections {
        if !seen.insert(section.label.as_str()) {
            return Some(&section.label);
        }
    }
    None
}
