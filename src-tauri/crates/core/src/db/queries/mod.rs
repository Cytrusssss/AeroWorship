//! The SQL that reads and writes the Appendix A tables (PRD §6.13, `queries/`).
//!
//! One module per aggregate, named after the domain concept rather than after a
//! service, the same rule `lib.rs` states for the crate as a whole.
//!
//! **Three rules every module here follows**, written once rather than in each
//! of them:
//!
//! * **Bound parameters, never a formatted string.** Song titles, section
//!   labels and lyric text arrive from an operator's paste, an online lyric
//!   provider (FR-604) or an `.aero` file written by another program (FR-703).
//!   None of it is trusted (NFR-28), and there is no `format!` in this
//!   directory that produces SQL.
//! * **Anything that writes more than one row writes it in a transaction.** A
//!   song whose sections half-landed is worse than a song that failed to land.
//!   The transaction is `rusqlite`'s `Connection::transaction`, which is
//!   `BEGIN DEFERRED` — its default, and worth naming here because it decides
//!   *when* the write lock is taken. Deferred takes none until the first
//!   statement that needs one, so a transaction that reads first and writes
//!   afterwards begins on a read snapshot and has to **upgrade**. Under WAL
//!   with a second connection open, that upgrade fails with
//!   `SQLITE_BUSY_SNAPSHOT`, and **waiting does not resolve it**: the snapshot
//!   this transaction is reading is already stale, so `busy_timeout` cannot
//!   help and the only way out is to roll the whole transaction back and run
//!   it again. All three write paths in [`song`] are safe for a reason worth
//!   stating rather than by luck — each inserts a row *before* it reads
//!   anything, so the write lock is held from the first statement and no
//!   upgrade happens. The first read-then-write path is FR-202's
//!   `upsert_song`, and it owes an explicit choice: either
//!   `transaction_with_behavior(TransactionBehavior::Immediate)`, which takes
//!   the write lock up front and can therefore wait, or a retry loop written
//!   for that error.
//! * **Nothing here reads a clock or invents an id.** Both arrive as arguments.
//!   See the head of [`song`] for why that is a design decision and not an
//!   oversight.
//!
//! **Where the model types are, and why they are here rather than in
//! `models/`.** `models/` is documented as the serde types that become the
//! generated TypeScript contract (NFR-33). These are storage records: they
//! carry `id`, `created_at` and `updated_at` because the *shell* supplies
//! those, so a `Deserialize` on them would open a second entry path that lets a
//! frontend choose a song's creation time and its id — precisely the shape
//! ADR-0045 named. Appendix D's `Song` and `SongInput` are wire types belonging
//! to `get_song`/`upsert_song` (FR-202), which is the item that owns that
//! boundary and its re-validation. They are deliberately not designed here.
//!
//! **`fts/` is still not here**, and that is deliberate for FR-203: an index
//! that is written and never read is dead code. The obligation to backfill
//! `songs_fts` for songs written before it exists belongs to FR-201 and is
//! recorded in that row of `PROGRESS.md`.

pub mod song;
