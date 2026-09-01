//! Errors raised by the storage layer.
//!
//! Spelled out by hand rather than derived: a `thiserror` dependency would buy
//! a `Display` impl for eight variants at the cost of two more crates in the
//! installer budget (NFR-16).
//!
//! **Every value in a message that came from outside this crate is printed
//! with `{:?}`, never `{}`.** An id is exactly as untrusted as a label: on the
//! `.aero` import path a song id, a section id and a section's label all
//! arrive out of the same foreign file (FR-703, NFR-28), and `queries::song`
//! says so in as many words — Appendix A wants a UUIDv7 in `id`, and nothing
//! here checks it. A `section_id` carrying a newline forges a whole log line;
//! one carrying `U+001B` writes terminal escapes. `{:?}` escapes both.
//!
//! There is no sink for that today — this crate logs nothing, and no
//! `#[tauri::command]` returns a `DbError` yet — so this is prevention rather
//! than a live leak being closed. It is also **not** an escape: a `{:?}`
//! string is Rust source syntax, not HTML and not a terminal. Whoever displays
//! one of these still owes their own escaping and their own truncation, the
//! same caution `models::TemplateError::Syntax` carries.
//!
//! Three kinds of value keep `{}`, listed so the exception reads as a decision
//! rather than as the oversight this rule exists to catch:
//!
//! * [`DbError::PragmaNotApplied`]'s three fields. `pragma` is a
//!   `&'static str` written in `super::connection`; `expected` is formatted
//!   from this crate's own constants; `actual` is either an integer rendered
//!   by `i64::to_string` or SQLite's own `journal_mode` vocabulary (`wal`,
//!   `memory`, `delete`, …). None of the three can carry a byte from an
//!   operator's paste, a lyric provider or an `.aero` file.
//! * The `u32`s in [`DbError::FutureSchema`] and
//!   [`DbError::InconsistentVersion`]. `found`, `user_version` and `recorded`
//!   do come out of the file, but they are integers: `{}` and `{:?}` render
//!   them identically, and neither has a byte to inject.
//! * [`DbError::Sqlite`]'s inner error, which is an error and not a string
//!   this crate holds. Its `Display` is SQLite's own message about SQL written
//!   as literals throughout `queries/` — there is no `format!` in that
//!   directory that produces SQL — and `Error::source` hands that same error
//!   to any chain printer, which prints it unquoted whatever this line does.
//!   The first path that will put untrusted bytes inside it is FTS5 (FR-201),
//!   whose parse errors quote the operator's query back; that item owes this
//!   decision a second look.

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

    /// Two of a song's sections carry the same label.
    ///
    /// `song_sections` has `UNIQUE (song_id, label)`, so SQLite refuses this
    /// anyway — but it refuses it as `SqliteFailure(…, "UNIQUE constraint
    /// failed: song_sections.song_id, song_sections.label")`, which names the
    /// *columns* and not the label. An operator who typed "Chorus" twice needs
    /// to be told which word to change, so the write path checks its own input
    /// first and raises this instead.
    ///
    /// **Both fields are operator or import text**, so both are printed with
    /// `{:?}` — `song_id` reaches this module from the same place `label`
    /// does, and neither of the two is validated. See this module's head for
    /// why that is still not an escape and who owes the real one.
    DuplicateSectionLabel {
        /// Song the two sections belong to.
        song_id: String,
        /// The label they share, exactly as it was handed in.
        label: String,
    },

    /// A stored `song_sections.section_type` is not one of the nine values
    /// Appendix A's `CHECK` allows.
    ///
    /// The `CHECK` makes this unreachable through SQLite, so it means the row
    /// predates the constraint or was written by something other than SQLite.
    /// Refused rather than mapped to `other`, which would put a section on a
    /// projector under a type nobody chose.
    ///
    /// **Both fields came out of the database**, which on this path is the
    /// least trustworthy source there is — this variant exists precisely
    /// because the row may have been written by something other than SQLite —
    /// so both are printed with `{:?}`. Same caution as
    /// [`DbError::DuplicateSectionLabel`].
    UnknownSectionType {
        /// `song_sections.id` of the offending row.
        section_id: String,
        /// The value found in the column.
        value: String,
    },

    /// A song was handed to the insert path with a `default_arrangement_id`
    /// already set.
    ///
    /// Appendix A rejects this too, through `trg_songs_default_arrangement_fk_insert`,
    /// and *always* — at INSERT time no arrangement of a song that does not
    /// exist yet can exist, so any non-NULL value necessarily names some other
    /// song's arrangement. But the trigger's own message reads
    /// "default_arrangement_id must belong to this song", which sends the
    /// reader looking for the right id when no id would have worked. The legal
    /// flow is three steps: insert with `None`, create the arrangement, then
    /// update (FR-204).
    DefaultArrangementAtInsert {
        /// Song that carried the value.
        song_id: String,
    },

    /// An arrangement item points at a section that is not one of its song's.
    ///
    /// This one the schema genuinely does **not** catch:
    /// `arrangement_items.section_id` references `song_sections(id)` with no
    /// clause tying it to `song_arrangements.song_id`, so an arrangement of one
    /// song may reference another song's section and every constraint is
    /// satisfied. Editing that section's text would then change a song whose
    /// lyrics nobody touched.
    ///
    /// **Raised on both a write path and a read path, and the fields mean
    /// slightly different things on each.** `insert_arrangement` raises it to
    /// refuse a row; `expand_arrangement` (FR-204) is the first *reader* to
    /// raise it, and it refuses to project the row onto a screen.
    ///
    /// **The "or nowhere" half of `section_id` below — an id naming no
    /// `song_sections` row at all — belongs mostly to the write path**, where
    /// `insert_arrangement` looks each id up before inserting its item and
    /// finds no owning song; that is the easy way to reach it and it is covered
    /// there. On the **read** path it is not an ordinary state: `db::open` sets
    /// `foreign_keys = ON` on every connection this crate hands out, and under
    /// that pragma `ON DELETE CASCADE` takes the item row rather than orphaning
    /// it, so a read reaches a dangling id only in a database written by
    /// something other than SQLite or before the constraint existed — the same
    /// threat model `UnknownSectionType` above is written for. An earlier
    /// wording called it "reachable in practice", which reads as a normal state
    /// and would stop a reader checking. `expand_arrangement` uses an outer
    /// join for it regardless, so that such a row is refused by name instead of
    /// dropped silently.
    SectionNotInSong {
        /// The arrangement the item belongs to: the one being written on the
        /// write path, the one being expanded on the read path.
        arrangement_id: String,
        /// Song that arrangement belongs to. On the read path this is the song
        /// the *caller* named, which the arrangement was already checked
        /// against.
        song_id: String,
        /// Section it referred to, which belongs elsewhere or nowhere.
        section_id: String,
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
            Self::DuplicateSectionLabel { song_id, label } => write!(
                f,
                "song {song_id:?} has two sections labelled {label:?}; \
                 a section's label identifies it within its song, so it must be unique"
            ),
            Self::UnknownSectionType { section_id, value } => write!(
                f,
                "section {section_id:?} has section_type {value:?}, which is not one of the \
                 nine values this build understands"
            ),
            Self::DefaultArrangementAtInsert { song_id } => write!(
                f,
                "song {song_id:?} cannot be inserted with a default_arrangement_id: the \
                 arrangement has to exist first, so insert the song with none, create the \
                 arrangement, then point the song at it"
            ),
            Self::SectionNotInSong {
                arrangement_id,
                song_id,
                section_id,
            } => write!(
                f,
                "arrangement {arrangement_id:?} of song {song_id:?} refers to section \
                 {section_id:?}, which is not a section of that song"
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
