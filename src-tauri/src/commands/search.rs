//! Search commands (spec §7.1, §7.2).

use tauri::State;

use crate::AppState;
use crate::error::AppResult;
use crate::search::{self, QueryClass, SearchFilters, SearchResults, SearchScope};

/// Classifies a query: a verse to jump to, Arabic letters, a Latin-script root
/// or other text.
#[tauri::command]
#[specta::specta]
pub async fn classify_query(state: State<'_, AppState>, query: String) -> AppResult<QueryClass> {
    search::classify_query(&*state.content()?, &query)
}

/// Searches the Arabic text or the translations, best matches first. An empty
/// `pack_ids` searches every installed translation.
#[tauri::command]
#[specta::specta]
pub async fn search_text(
    state: State<'_, AppState>,
    query: String,
    scope: SearchScope,
    pack_ids: Vec<String>,
    filters: SearchFilters,
    limit: u32,
) -> AppResult<SearchResults> {
    search::search_text(&*state.content()?, &query, scope, &pack_ids, filters, limit)
}
