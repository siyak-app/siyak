//! Content commands: surahs and verses from `content.db`.

use tauri::State;

use crate::AppState;
use crate::db::content::{self, Surah, VerseWithTranslations};
use crate::error::AppResult;

/// Lists all 114 surahs in Mushaf order.
#[tauri::command]
#[specta::specta]
pub async fn list_surahs(state: State<'_, AppState>) -> AppResult<Vec<Surah>> {
    content::list_surahs(&*state.content()?)
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
