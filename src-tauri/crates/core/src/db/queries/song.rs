use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::db::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionType {
    Verse,
    Chorus,
    Bridge,
    PreChorus,
    Tag,
    Ending,
    Intro,
    Interlude,
    Other,
}

impl SectionType {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSection {
    pub id: String,
    pub label: String,
    pub section_type: SectionType,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub alternate_title: Option<String>,
    pub ccli_number: Option<String>,
    pub copyright_text: Option<String>,
    pub song_key: Option<String>,
    pub tempo_bpm: Option<i64>,
    pub default_arrangement_id: Option<String>,
    pub source_provider: Option<String>,
    pub source_url: Option<String>,
    pub retrieved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub sections: Vec<SongSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangementItem {
    pub position: i64,
    pub section_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrangement {
    pub id: String,
    pub song_id: String,
    pub name: String,
    pub created_at: String,
    pub items: Vec<ArrangementItem>,
}

pub fn insert_song(conn: &mut Connection, song: &Song) -> Result<(), DbError> {
    refuse_uninsertable(song)?;

    let tx = conn.transaction()?;
    write_song(&tx, song)?;
    tx.commit()?;
    Ok(())
}

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
        name: "Default".to_owned(),
        created_at: created_at.to_owned(),
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

pub fn insert_arrangement(conn: &mut Connection, arrangement: &Arrangement) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    write_arrangement(&tx, arrangement)?;
    tx.commit()?;
    Ok(())
}

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

    for arrangement in &mut arrangements {
        arrangement.items = load_arrangement_items(conn, &arrangement.id)?;
    }
    Ok(arrangements)
}

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

pub fn expand_arrangement(
    conn: &Connection,
    song_id: &str,
    arrangement_id: &str,
) -> Result<Option<Vec<SongSection>>, DbError> {
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
        arrangement_exists = true;
        let Some(item_section_id) = item_section_id else {
            continue;
        };
        let (Some(id), Some(label), Some(section_type), Some(content), Some(owning_song)) =
            (id, label, section_type, content, owning_song)
        else {
            return Err(DbError::SectionNotInSong {
                arrangement_id: arrangement_id.to_owned(),
                song_id: song_id.to_owned(),
                section_id: item_section_id,
            });
        };
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

fn first_repeated_label(sections: &[SongSection]) -> Option<&str> {
    let mut seen = std::collections::BTreeSet::new();
    for section in sections {
        if !seen.insert(section.label.as_str()) {
            return Some(&section.label);
        }
    }
    None
}
