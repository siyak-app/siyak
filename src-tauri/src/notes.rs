//! Bookmarks and notes in `user.db` (spec §5.5, §13.5).
//!
//! `user.db` has no `verses` table, so anything shown with a `surah:ayah` takes
//! a [`VerseIndex`] read from `content.db` beforehand: no command ever holds
//! both database locks at once.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::db::content::{VerseIndex, VerseRef};
use crate::error::{AppError, AppErrorCode, AppResult};

/// The current UTC time as stored in `user.db`.
pub(crate) const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";

/// A bookmarked verse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Bookmark {
    /// Global verse id.
    pub verse_id: u32,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
}

/// Adds the bookmark if the verse has none and removes it otherwise. Returns
/// whether the verse is now bookmarked.
pub fn toggle_bookmark(
    conn: &mut Connection,
    index: &VerseIndex,
    verse_id: u32,
) -> AppResult<bool> {
    index.check_range(verse_id, verse_id)?;
    let tx = conn.transaction()?;
    let removed = tx.execute("DELETE FROM bookmarks WHERE verse_id = ?1", [verse_id])?;
    if removed == 0 {
        tx.execute(
            &format!("INSERT INTO bookmarks (verse_id, created_at) VALUES (?1, {NOW})"),
            [verse_id],
        )?;
    }
    tx.commit()?;
    Ok(removed == 0)
}

/// Every bookmark, newest first.
pub fn list_bookmarks(conn: &Connection) -> AppResult<Vec<Bookmark>> {
    let mut stmt = conn
        .prepare("SELECT verse_id, created_at FROM bookmarks ORDER BY created_at DESC, id DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Bookmark {
            verse_id: row.get(0)?,
            created_at: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A note on a verse or a range of verses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Note {
    /// Note id.
    pub id: u32,
    /// Global id of the first verse.
    pub verse_from: u32,
    /// Global id of the last verse; the same as `verse_from` for a single verse.
    pub verse_to: u32,
    /// The first verse.
    pub from: VerseRef,
    /// The last verse.
    pub to: VerseRef,
    /// Markdown.
    pub body: String,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub updated_at: String,
}

/// A note to create (no `id`) or replace.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
pub struct NoteInput {
    /// The note to replace, or `None` for a new one.
    pub id: Option<u32>,
    /// Global id of the first verse.
    pub verse_from: u32,
    /// Global id of the last verse.
    pub verse_to: u32,
    /// Markdown; must not be blank.
    pub body: String,
}

/// A range of global verse ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
pub struct VerseIdRange {
    /// First verse.
    pub from: u32,
    /// Last verse.
    pub to: u32,
}

const NOTE_COLUMNS: &str = "id, verse_from, verse_to, body, created_at, updated_at";

/// A note row whose verses are known; a note on a verse the text does not have
/// (a hand-edited database) is left out rather than failing the whole list.
fn note_from_row(row: &rusqlite::Row<'_>, index: &VerseIndex) -> rusqlite::Result<Option<Note>> {
    let verse_from: u32 = row.get(1)?;
    let verse_to: u32 = row.get(2)?;
    let (Some(from), Some(to)) = (index.verse(verse_from), index.verse(verse_to)) else {
        return Ok(None);
    };
    Ok(Some(Note {
        id: row.get(0)?,
        verse_from,
        verse_to,
        from,
        to,
        body: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    }))
}

/// Creates or replaces a note and returns it.
pub fn upsert_note(conn: &Connection, index: &VerseIndex, input: &NoteInput) -> AppResult<Note> {
    index.check_range(input.verse_from, input.verse_to)?;
    if input.body.trim().is_empty() {
        return Err(AppError::new(AppErrorCode::InvalidInput, "empty note"));
    }
    let id = match input.id {
        Some(id) => {
            let changed = conn.execute(
                &format!(
                    "UPDATE notes SET verse_from = ?2, verse_to = ?3, body = ?4, updated_at = {NOW}
                     WHERE id = ?1"
                ),
                (id, input.verse_from, input.verse_to, &input.body),
            )?;
            if changed == 0 {
                return Err(AppError::new(AppErrorCode::NotFound, format!("note {id}")));
            }
            id
        }
        None => {
            conn.execute(
                &format!(
                    "INSERT INTO notes (verse_from, verse_to, body, created_at, updated_at)
                     VALUES (?1, ?2, ?3, {NOW}, {NOW})"
                ),
                (input.verse_from, input.verse_to, &input.body),
            )?;
            u32::try_from(conn.last_insert_rowid())
                .map_err(|err| AppError::new(AppErrorCode::QueryFailed, err.to_string()))?
        }
    };
    get_note(conn, index, id)?
        .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("note {id}")))
}

/// A note by id, if it exists.
pub fn get_note(conn: &Connection, index: &VerseIndex, id: u32) -> AppResult<Option<Note>> {
    Ok(conn
        .query_row(
            &format!("SELECT {NOTE_COLUMNS} FROM notes WHERE id = ?1"),
            [id],
            |row| note_from_row(row, index),
        )
        .optional()?
        .flatten())
}

/// Deletes a note and its place in any collection.
pub fn delete_note(conn: &mut Connection, id: u32) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM collection_items WHERE kind = 'note' AND json_extract(ref_json, '$.id') = ?1",
        [id],
    )?;
    tx.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(())
}

/// The notes touching `range`, in verse order, or all of them with the most
/// recently changed first.
pub fn list_notes(
    conn: &Connection,
    index: &VerseIndex,
    range: Option<VerseIdRange>,
) -> AppResult<Vec<Note>> {
    let notes = if let Some(VerseIdRange { from, to }) = range {
        conn.prepare(&format!(
            "SELECT {NOTE_COLUMNS} FROM notes WHERE verse_from <= ?2 AND verse_to >= ?1
             ORDER BY verse_from, verse_to, id"
        ))?
        .query_map([from, to], |row| note_from_row(row, index))?
        .collect::<Result<Vec<_>, _>>()?
    } else {
        conn.prepare(&format!(
            "SELECT {NOTE_COLUMNS} FROM notes ORDER BY updated_at DESC, id DESC"
        ))?
        .query_map([], |row| note_from_row(row, index))?
        .collect::<Result<Vec<_>, _>>()?
    };
    Ok(notes.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::test_support::{content, user};

    fn input(id: Option<u32>, from: u32, to: u32, body: &str) -> NoteInput {
        NoteInput {
            id,
            verse_from: from,
            verse_to: to,
            body: body.to_owned(),
        }
    }

    #[test]
    fn a_bookmark_toggles() {
        let index = VerseIndex::load(&content()).unwrap();
        let mut conn = user();
        assert!(toggle_bookmark(&mut conn, &index, 2).unwrap());
        assert_eq!(list_bookmarks(&conn).unwrap()[0].verse_id, 2);
        assert!(!toggle_bookmark(&mut conn, &index, 2).unwrap());
        assert!(list_bookmarks(&conn).unwrap().is_empty());
    }

    #[test]
    fn a_bookmark_needs_a_verse() {
        let index = VerseIndex::load(&content()).unwrap();
        let err = toggle_bookmark(&mut user(), &index, 9999).unwrap_err();
        assert_eq!(err.code, AppErrorCode::InvalidInput);
    }

    #[test]
    fn notes_are_created_and_replaced() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        let note = upsert_note(&conn, &index, &input(None, 1, 2, "first")).unwrap();
        assert_eq!(note.from, VerseRef { surah: 1, ayah: 1 });
        assert_eq!(note.to, VerseRef { surah: 1, ayah: 2 });

        let edited = upsert_note(&conn, &index, &input(Some(note.id), 2, 2, "second")).unwrap();
        assert_eq!(edited.id, note.id);
        assert_eq!(edited.body, "second");
        assert_eq!(list_notes(&conn, &index, None).unwrap().len(), 1);
    }

    #[test]
    fn invalid_notes_are_rejected() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        for bad in [
            input(None, 1, 1, "  \n"),
            input(None, 2, 1, "reversed"),
            input(None, 1, 9999, "no such verse"),
        ] {
            let err = upsert_note(&conn, &index, &bad).unwrap_err();
            assert_eq!(err.code, AppErrorCode::InvalidInput, "{bad:?}");
        }
        let err = upsert_note(&conn, &index, &input(Some(5), 1, 1, "gone")).unwrap_err();
        assert_eq!(err.code, AppErrorCode::NotFound);
    }

    #[test]
    fn listing_a_range_finds_the_notes_touching_it() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        upsert_note(&conn, &index, &input(None, 1, 2, "both")).unwrap();
        upsert_note(&conn, &index, &input(None, 3, 3, "later")).unwrap();
        let bodies = |from, to| -> Vec<String> {
            list_notes(&conn, &index, Some(VerseIdRange { from, to }))
                .unwrap()
                .into_iter()
                .map(|note| note.body)
                .collect()
        };
        assert_eq!(bodies(2, 2), ["both"]);
        assert_eq!(bodies(2, 3), ["both", "later"]);
        assert!(bodies(4, 4).is_empty());
    }

    #[test]
    fn deleting_a_note_removes_it_from_collections() {
        let index = VerseIndex::load(&content()).unwrap();
        let mut conn = user();
        let note = upsert_note(&conn, &index, &input(None, 1, 1, "kept?")).unwrap();
        conn.execute_batch(&format!(
            "INSERT INTO collections (id, title, created_at, updated_at) VALUES (1, 't', 'n', 'n');
             INSERT INTO collection_items (collection_id, position, kind, ref_json, created_at)
               VALUES (1, 0, 'note', '{{\"id\":{}}}', 'n'), (1, 1, 'root', '{{\"root\":\"x\"}}', 'n');",
            note.id
        ))
        .unwrap();
        delete_note(&mut conn, note.id).unwrap();
        let left: u32 = conn
            .query_row("SELECT count(*) FROM collection_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 1);
        assert!(get_note(&conn, &index, note.id).unwrap().is_none());
    }
}
