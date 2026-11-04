//! Bookmark and note commands (spec §5.5, §12).

use tauri::State;

use crate::AppState;
use crate::db::content::{self, VerseIndex, VerseWithTranslations};
use crate::error::AppResult;
use crate::notes::{self, Note, NoteInput, VerseIdRange};

/// A bookmark with its verse.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
pub struct BookmarkedVerse {
    /// The verse with the requested translations.
    pub verse: VerseWithTranslations,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
}

/// Bookmarks the verse, or removes its bookmark. Returns whether it is now bookmarked.
#[tauri::command]
#[specta::specta]
pub async fn toggle_bookmark(state: State<'_, AppState>, verse_id: u32) -> AppResult<bool> {
    let index = VerseIndex::load(&*state.content()?)?;
    notes::toggle_bookmark(&mut *state.user()?, &index, verse_id)
}

/// Every bookmark with its verse, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_bookmarks(
    state: State<'_, AppState>,
    translation_ids: Vec<String>,
) -> AppResult<Vec<BookmarkedVerse>> {
    let bookmarks = notes::list_bookmarks(&*state.user()?)?;
    let conn = state.content()?;
    bookmarks
        .into_iter()
        .map(|bookmark| {
            let verses = content::get_verse_range(
                &conn,
                bookmark.verse_id,
                bookmark.verse_id,
                &translation_ids,
            )?;
            Ok(verses.into_iter().next().map(|verse| BookmarkedVerse {
                verse,
                created_at: bookmark.created_at,
            }))
        })
        .filter_map(Result::transpose)
        .collect()
}

/// Creates or replaces a note and returns it.
#[tauri::command]
#[specta::specta]
pub async fn upsert_note(state: State<'_, AppState>, note: NoteInput) -> AppResult<Note> {
    let index = VerseIndex::load(&*state.content()?)?;
    notes::upsert_note(&*state.user()?, &index, &note)
}

/// Deletes a note and its place in any collection.
#[tauri::command]
#[specta::specta]
pub async fn delete_note(state: State<'_, AppState>, id: u32) -> AppResult<()> {
    notes::delete_note(&mut *state.user()?, id)
}

/// The notes touching `range`, or all of them with the most recently changed first.
#[tauri::command]
#[specta::specta]
pub async fn list_notes(
    state: State<'_, AppState>,
    range: Option<VerseIdRange>,
) -> AppResult<Vec<Note>> {
    let index = VerseIndex::load(&*state.content()?)?;
    notes::list_notes(&*state.user()?, &index, range)
}
