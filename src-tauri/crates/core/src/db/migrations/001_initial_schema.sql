-- Migration 001 — initial schema.
--
-- A transcription of PRD Appendix A (docs/PRD.md). Read it there first; this
-- file is the executable copy, not the specification. Two deliberate
-- differences from the appendix text, both explained below, because the absence
-- of something the specification lists has to read as a decision rather than as
-- an omission.
--
-- ── 1. The four PRAGMA statements at the head of Appendix A are not here. ────
--
-- Only `journal_mode = WAL` is persistent — SQLite records it in the file
-- header and every later connection inherits it. `foreign_keys`,
-- `synchronous` and `cache_size` are *per connection*: setting them once,
-- here, would look correct in this transaction and be silently off in every
-- connection opened afterwards. An unnoticed `foreign_keys = OFF` means none of
-- the `REFERENCES` and `ON DELETE CASCADE` clauses below enforce anything.
--
-- All four therefore live in `super::connection`: `apply_pragmas` sets them and
-- `verify_pragmas` reads every one of them back to prove it applied. Both run
-- inside `connection::open`, which is the only way this crate hands out a
-- connection: `db` re-exports `open` and nothing else from that module.
-- `journal_mode = WAL` additionally *cannot* run inside a transaction, and this
-- file always does (see `super::migrate`).
--
-- ── 2. `trg_sections_ai` (Appendix A, last statement) is not here. ───────────
--
-- The appendix's own implementation note calls it illustrative, and it does not
-- work as written: `'rebuild-song'` is not an FTS5 command (the recognised ones
-- are `rebuild`, `optimize`, `integrity-check`, `merge`, `delete-all`), so
-- SQLite raises `SQL logic error` and the trigger would make every single
-- INSERT into `song_sections` fail. Nor is per-row synchronisation the right
-- shape: `songs_fts.body` is the concatenation of *all* of a song's sections,
-- so the unit of reindexing is the song.
--
-- Its replacement is the `reindex_song(song_id)` routine the appendix note
-- describes, called inside the same transaction as any song, section or author
-- mutation. That routine belongs to the items that first write those tables
-- (FR-2xx); this migration only creates the tables it will maintain.
--
-- ─────────────────────────────────────────────────────────────
-- Songs
-- ─────────────────────────────────────────────────────────────
CREATE TABLE songs (
    id               TEXT PRIMARY KEY,              -- UUIDv7
    title            TEXT NOT NULL,
    alternate_title  TEXT,
    ccli_number      TEXT,
    copyright_text   TEXT,
    song_key         TEXT,
    tempo_bpm        INTEGER,
    default_arrangement_id TEXT,                    -- no FK; two triggers below
    source_provider  TEXT,                          -- FR-605 provenance
    source_url       TEXT,
    retrieved_at     TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT                           -- soft delete, FR-202
);
CREATE INDEX idx_songs_title   ON songs(title) WHERE deleted_at IS NULL;
CREATE INDEX idx_songs_updated ON songs(updated_at DESC);

CREATE TABLE authors (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,               -- stored exactly once
    created_at  TEXT NOT NULL
);

-- role is part of the key: an author may hold several roles on one song,
-- and each (song, author, role) is an independent fact.
CREATE TABLE song_authors (
    song_id    TEXT NOT NULL REFERENCES songs(id)   ON DELETE CASCADE,
    author_id  TEXT NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    role       TEXT NOT NULL
        CHECK (role IN ('words','music','arrangement','translation')),
    PRIMARY KEY (song_id, author_id, role)
);
CREATE INDEX idx_song_authors_author ON song_authors(author_id);

-- A section's text is stored exactly once, however often it is sung.
CREATE TABLE song_sections (
    id           TEXT PRIMARY KEY,
    song_id      TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    label        TEXT NOT NULL,                     -- 'Verse 1', 'Chorus', 'Bridge'
    section_type TEXT NOT NULL
        CHECK (section_type IN ('verse','chorus','bridge','pre_chorus',
                                'tag','ending','intro','interlude','other')),
    content      TEXT NOT NULL,                     -- newline-separated lines
    sort_order   INTEGER NOT NULL,                  -- authoring order only
    UNIQUE (song_id, label)
);
CREATE INDEX idx_sections_song ON song_sections(song_id, sort_order);

CREATE TABLE song_arrangements (
    id          TEXT PRIMARY KEY,
    song_id     TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,                      -- 'Default', 'Short version'
    created_at  TEXT NOT NULL,
    UNIQUE (song_id, name)
);

-- position is in the key: the same section legitimately repeats.
CREATE TABLE arrangement_items (
    arrangement_id TEXT NOT NULL
        REFERENCES song_arrangements(id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    section_id     TEXT NOT NULL
        REFERENCES song_sections(id) ON DELETE CASCADE,
    PRIMARY KEY (arrangement_id, position)
);
CREATE INDEX idx_arritems_section ON arrangement_items(section_id);

-- ── What the two triggers below enforce, and what nothing enforces. ─────────
--
-- They are not a foreign key, and this file used to say they were.
-- `songs.default_arrangement_id` has no `REFERENCES` clause at all: the
-- reference is circular (`songs` → `song_arrangements` → `songs`), so the
-- column cannot carry one at CREATE time, and SQLite cannot add one afterwards
-- without rewriting the table.
--
-- What the triggers do hold: the *write-to-`songs`* direction, on both INSERT
-- and UPDATE. A song cannot be pointed at an arrangement that does not exist,
-- or at one belonging to another song.
--
-- What nothing holds: **DELETE**. Neither trigger fires when the
-- `song_arrangements` row a song points at is deleted, and with no FK there is
-- no `ON DELETE` either — so `songs.default_arrangement_id` is left naming a
-- row that is gone. Proved against a connection rather than inferred from the
-- missing `REFERENCES`: point a song at its own arrangement (1 row changed),
-- `DELETE FROM song_arrangements` (1 row deleted, none left), and
-- `default_arrangement_id` still reads back as that arrangement's id.
--
-- Whose it is: the first item that actually **deletes a `song_arrangements`
-- row** — FR-202's hard delete of a song, or an arrangement-CRUD item if one
-- ever exists. **Not FR-204**, which this comment used to name: FR-204's
-- requirement text is "an ordered sequence of references to that song's
-- sections. A default arrangement is generated on creation" and says nothing
-- about deleting anything. What FR-204 carries is the *write* direction of
-- `default_arrangement_id`, and that direction is the one the two triggers
-- below already hold. Closing the DELETE hole means adding the FK, which means
-- rewriting the table in a *later* migration — this one is not edited
-- (ADR-0049).
CREATE TRIGGER trg_songs_default_arrangement_fk
BEFORE UPDATE OF default_arrangement_id ON songs
WHEN NEW.default_arrangement_id IS NOT NULL
     AND NOT EXISTS (SELECT 1 FROM song_arrangements
                     WHERE id = NEW.default_arrangement_id
                       AND song_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'default_arrangement_id must belong to this song');
END;

-- The same guard on INSERT, because UPDATE alone leaves a song free to be born
-- pointing at another song's arrangement. Its consequence is that an INSERT
-- with a non-NULL default_arrangement_id is *always* rejected — intended, not a
-- side effect: song_arrangements.song_id references songs(id), so an
-- arrangement of a song that does not exist yet cannot exist either, and any
-- non-NULL value at INSERT time therefore names some other song's row. The
-- legal flow is three steps, and this trigger is what makes the schema enforce
-- it: INSERT with NULL, create the arrangement, then UPDATE.
CREATE TRIGGER trg_songs_default_arrangement_fk_insert
BEFORE INSERT ON songs
WHEN NEW.default_arrangement_id IS NOT NULL
     AND NOT EXISTS (SELECT 1 FROM song_arrangements
                     WHERE id = NEW.default_arrangement_id
                       AND song_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'default_arrangement_id must belong to this song');
END;

-- ─────────────────────────────────────────────────────────────
-- Bible
-- ─────────────────────────────────────────────────────────────
CREATE TABLE bible_versions (
    id             TEXT PRIMARY KEY,
    abbreviation   TEXT NOT NULL UNIQUE,            -- 'TB', 'KJV', 'WEB'
    full_name      TEXT NOT NULL,
    language_code  TEXT NOT NULL,                   -- BCP-47
    copyright_text TEXT,
    is_public_domain INTEGER NOT NULL DEFAULT 0,    -- see R3
    imported_at    TEXT NOT NULL
);

CREATE TABLE bible_books (
    id           INTEGER PRIMARY KEY,               -- 1..66 canonical order
    testament    TEXT NOT NULL CHECK (testament IN ('OT','NT')),
    chapter_count INTEGER NOT NULL
);

-- Names and abbreviations are per language, so they are their own relation.
CREATE TABLE bible_book_names (
    book_id       INTEGER NOT NULL REFERENCES bible_books(id),
    language_code TEXT NOT NULL,
    name          TEXT NOT NULL,
    PRIMARY KEY (book_id, language_code)
);

-- Each abbreviation is an independent fact about (book, language).
CREATE TABLE bible_book_abbreviations (
    book_id       INTEGER NOT NULL REFERENCES bible_books(id),
    language_code TEXT NOT NULL,
    abbreviation  TEXT NOT NULL,                    -- 'Yoh', 'Jn', 'Joh'
    PRIMARY KEY (book_id, language_code, abbreviation)
);
CREATE INDEX idx_abbrev_lookup
    ON bible_book_abbreviations(language_code, abbreviation);

-- Natural composite key; text depends on the whole key.
CREATE TABLE bible_verses (
    version_id TEXT    NOT NULL REFERENCES bible_versions(id) ON DELETE CASCADE,
    book_id    INTEGER NOT NULL REFERENCES bible_books(id),
    chapter    INTEGER NOT NULL,
    verse      INTEGER NOT NULL,
    text       TEXT    NOT NULL,
    PRIMARY KEY (version_id, book_id, chapter, verse)
) WITHOUT ROWID;

-- ─────────────────────────────────────────────────────────────
-- Media, templates, imported decks
-- ─────────────────────────────────────────────────────────────
CREATE TABLE media_assets (
    id            TEXT PRIMARY KEY,
    relative_path TEXT NOT NULL UNIQUE,             -- relative to media root
    original_name TEXT NOT NULL,
    mime_type     TEXT NOT NULL,
    width_px      INTEGER,
    height_px     INTEGER,
    byte_size     INTEGER NOT NULL,
    content_hash  TEXT NOT NULL,                    -- BLAKE3, FR-705
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_media_hash ON media_assets(content_hash);

CREATE TABLE templates (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    is_builtin    INTEGER NOT NULL DEFAULT 0,       -- FR-410, not deletable
    schema_version INTEGER NOT NULL,
    document      TEXT NOT NULL,                    -- JSON, Appendix B
    thumbnail_path TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (name, is_builtin)
);

-- Which media a template references — extracted so relinking can find them.
CREATE TABLE template_media (
    template_id TEXT NOT NULL REFERENCES templates(id)     ON DELETE CASCADE,
    media_id    TEXT NOT NULL REFERENCES media_assets(id)  ON DELETE RESTRICT,
    PRIMARY KEY (template_id, media_id)
);

CREATE TABLE imported_decks (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    source_path      TEXT,                          -- provenance, FR-506
    source_hash      TEXT,                          -- staleness check, FR-507
    source_kind      TEXT NOT NULL
        CHECK (source_kind IN ('pptx','ppt','pdf')),
    slide_count      INTEGER NOT NULL,
    render_width_px  INTEGER NOT NULL,
    render_height_px INTEGER NOT NULL,
    imported_at      TEXT NOT NULL
);

CREATE TABLE deck_slides (
    deck_id     TEXT    NOT NULL REFERENCES imported_decks(id) ON DELETE CASCADE,
    slide_index INTEGER NOT NULL,                   -- 0-based
    image_path  TEXT    NOT NULL,                   -- relative to cache root
    thumb_path  TEXT    NOT NULL,
    PRIMARY KEY (deck_id, slide_index)
) WITHOUT ROWID;

-- ─────────────────────────────────────────────────────────────
-- Tags (polymorphic)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE tags (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    color TEXT
);

CREATE TABLE taggables (
    tag_id      TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL
        CHECK (entity_type IN ('song','deck','media','template')),
    entity_id   TEXT NOT NULL,
    PRIMARY KEY (tag_id, entity_type, entity_id)
);
CREATE INDEX idx_taggables_entity ON taggables(entity_type, entity_id);

-- ─────────────────────────────────────────────────────────────
-- Application state
-- ─────────────────────────────────────────────────────────────
CREATE TABLE recent_sessions (                      -- FR-302
    file_path    TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    opened_at    TEXT NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                             -- JSON-encoded
);

CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

-- ─────────────────────────────────────────────────────────────
-- Derived search indexes (NOT base relations — see §6.6)
-- ─────────────────────────────────────────────────────────────
CREATE VIRTUAL TABLE songs_fts USING fts5(
    song_id UNINDEXED,
    title,
    alternate_title,
    authors,
    body,                                           -- all sections concatenated
    tokenize = 'porter unicode61 remove_diacritics 2',
    prefix   = '2 3'
);

CREATE VIRTUAL TABLE verses_fts USING fts5(
    version_id UNINDEXED,
    book_id    UNINDEXED,
    chapter    UNINDEXED,
    verse      UNINDEXED,
    text,
    tokenize = 'unicode61 remove_diacritics 2',
    prefix   = '2 3'
);
