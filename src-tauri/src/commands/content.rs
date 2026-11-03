//! Content commands: surahs, verses and tafsir from `content.db`.

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::AppState;
use crate::db::content::{
    self, Compare, Surah, TafsirInfo, TafsirPassage, TranslationInfo, VerseWithTranslations,
};
use crate::error::{AppError, AppResult};

/// The core data in use and whether refreshing it failed at startup (spec §6.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct CoreStatus {
    /// `content_meta.core_version` of the content database.
    pub core_version: u32,
    /// Which dataset a debug build loaded (`dev` or `full`); `None` in release builds.
    pub seed: Option<String>,
    /// Why the core refresh failed, if it did; the previous data is still in use.
    pub refresh_error: Option<AppError>,
}

/// Reports the core data version and any core refresh error.
#[tauri::command]
#[specta::specta]
pub async fn get_core_status(state: State<'_, AppState>) -> AppResult<CoreStatus> {
    let refresh_error = state.databases()?.core_refresh_error.clone();
    let conn = state.content()?;
    Ok(CoreStatus {
        core_version: content::core_version(&conn, "main")?,
        seed: content::seed_marker(&conn)?,
        refresh_error,
    })
}

/// Lists all 114 surahs in Mushaf order.
#[tauri::command]
#[specta::specta]
pub async fn list_surahs(state: State<'_, AppState>) -> AppResult<Vec<Surah>> {
    content::list_surahs(&*state.content()?)
}

/// Returns one surah, including its basmala header (`None` for surahs 1 and 9).
#[tauri::command]
#[specta::specta]
pub async fn get_surah(state: State<'_, AppState>, number: u16) -> AppResult<Surah> {
    content::get_surah(&*state.content()?, number)
}

/// Lists the installed translation packs, bundled ones first.
#[tauri::command]
#[specta::specta]
pub async fn list_translations(state: State<'_, AppState>) -> AppResult<Vec<TranslationInfo>> {
    content::list_translations(&*state.content()?)
}

/// Returns the verses of a surah with the requested translations, in the
/// requested order. Unknown translation ids are ignored.
#[tauri::command]
#[specta::specta]
pub async fn get_verses(
    state: State<'_, AppState>,
    surah: u16,
    translation_ids: Vec<String>,
) -> AppResult<Vec<VerseWithTranslations>> {
    content::get_verses(&*state.content()?, surah, &translation_ids)
}

/// Lists the installed tafsir packs.
#[tauri::command]
#[specta::specta]
pub async fn list_tafsirs(state: State<'_, AppState>) -> AppResult<Vec<TafsirInfo>> {
    content::list_tafsirs(&*state.content()?)
}

/// Returns the commentary covering a verse (it may cover several), or `None`
/// if this tafsir has no commentary for it.
#[tauri::command]
#[specta::specta]
pub async fn get_tafsir(
    state: State<'_, AppState>,
    verse_id: u32,
    pack_id: String,
) -> AppResult<Option<TafsirPassage>> {
    content::get_tafsir(&*state.content()?, verse_id, &pack_id)
}

/// Returns the author's introduction of a tafsir, if it has one.
#[tauri::command]
#[specta::specta]
pub async fn get_tafsir_introduction(
    state: State<'_, AppState>,
    pack_id: String,
) -> AppResult<Option<TafsirPassage>> {
    content::get_tafsir_introduction(&*state.content()?, &pack_id)
}

/// Returns a range of verses (at most 50) with the given translations and the
/// commentary of the given tafsirs, for the compare screen.
#[tauri::command]
#[specta::specta]
pub async fn get_compare(
    state: State<'_, AppState>,
    from: u32,
    to: u32,
    translation_ids: Vec<String>,
    tafsir_ids: Vec<String>,
) -> AppResult<Compare> {
    content::get_compare(&*state.content()?, from, to, &translation_ids, &tafsir_ids)
}
