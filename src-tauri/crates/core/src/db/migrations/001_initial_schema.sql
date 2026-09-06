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

CREATE TABLE song_authors (
    song_id    TEXT NOT NULL REFERENCES songs(id)   ON DELETE CASCADE,
    author_id  TEXT NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('words','music','arrangement','translation')),
    PRIMARY KEY (song_id, author_id, role)
);
CREATE INDEX idx_song_authors_author ON song_authors(author_id);

CREATE TABLE song_sections (
    id           TEXT PRIMARY KEY,
    song_id      TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    label        TEXT NOT NULL,                     -- 'Verse 1', 'Chorus', 'Bridge'
    section_type TEXT NOT NULL CHECK (section_type IN ('verse','chorus','bridge','pre_chorus','tag','ending','intro','interlude','other')),
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

CREATE TABLE arrangement_items (
    arrangement_id TEXT NOT NULL REFERENCES song_arrangements(id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    section_id     TEXT NOT NULL REFERENCES song_sections(id) ON DELETE CASCADE,
    PRIMARY KEY (arrangement_id, position)
);
CREATE INDEX idx_arritems_section ON arrangement_items(section_id);

CREATE TRIGGER trg_songs_default_arrangement_fk
BEFORE UPDATE OF default_arrangement_id ON songs
WHEN NEW.default_arrangement_id IS NOT NULL
     AND NOT EXISTS (SELECT 1 FROM song_arrangements
                     WHERE id = NEW.default_arrangement_id
                       AND song_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'default_arrangement_id must belong to this song');
END;

CREATE TRIGGER trg_songs_default_arrangement_fk_insert
BEFORE INSERT ON songs
WHEN NEW.default_arrangement_id IS NOT NULL
     AND NOT EXISTS (SELECT 1 FROM song_arrangements
                     WHERE id = NEW.default_arrangement_id
                       AND song_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'default_arrangement_id must belong to this song');
END;

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

CREATE TABLE bible_book_names (
    book_id       INTEGER NOT NULL REFERENCES bible_books(id),
    language_code TEXT NOT NULL,
    name          TEXT NOT NULL,
    PRIMARY KEY (book_id, language_code)
);

CREATE TABLE bible_book_abbreviations (
    book_id       INTEGER NOT NULL REFERENCES bible_books(id),
    language_code TEXT NOT NULL,
    abbreviation  TEXT NOT NULL,                    -- 'Yoh', 'Jn', 'Joh'
    PRIMARY KEY (book_id, language_code, abbreviation)
);
CREATE INDEX idx_abbrev_lookup
    ON bible_book_abbreviations(language_code, abbreviation);

CREATE TABLE bible_verses (
    version_id TEXT    NOT NULL REFERENCES bible_versions(id) ON DELETE CASCADE,
    book_id    INTEGER NOT NULL REFERENCES bible_books(id),
    chapter    INTEGER NOT NULL,
    verse      INTEGER NOT NULL,
    text       TEXT    NOT NULL,
    PRIMARY KEY (version_id, book_id, chapter, verse)
) WITHOUT ROWID;

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
    source_kind      TEXT NOT NULL CHECK (source_kind IN ('pptx','ppt','pdf')),
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

CREATE TABLE tags (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    color TEXT
);

CREATE TABLE taggables (
    tag_id      TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('song','deck','media','template')),
    entity_id   TEXT NOT NULL,
    PRIMARY KEY (tag_id, entity_type, entity_id)
);
CREATE INDEX idx_taggables_entity ON taggables(entity_type, entity_id);

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
