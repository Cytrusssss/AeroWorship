use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),

    PragmaNotApplied {
        pragma: &'static str,
        expected: String,
        actual: String,
    },

    FutureSchema {
        found: u32,
        supported: u32,
    },

    InconsistentVersion {
        user_version: u32,
        recorded: u32,
    },

    DuplicateSectionLabel {
        song_id: String,
        label: String,
    },

    UnknownSectionType {
        section_id: String,
        value: String,
    },

    DefaultArrangementAtInsert {
        song_id: String,
    },

    SectionNotInSong {
        arrangement_id: String,
        song_id: String,
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
