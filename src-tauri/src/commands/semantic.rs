//! Semantic layer commands (spec §7.4).

use tauri::State;

use crate::AppState;
use crate::db::user;
use crate::embed::service::SemanticStatus;
use crate::error::AppResult;
use crate::packs::manifest::installed_packs;

/// Whether semantic search and cross-references are available, and if not,
/// what the user can do about it (install a pack, store a key).
#[tauri::command]
#[specta::specta]
pub async fn get_semantic_status(state: State<'_, AppState>) -> AppResult<SemanticStatus> {
    let packs = installed_packs(&*state.content()?)?;
    let settings = user::get_settings(&*state.user()?)?;
    state.semantic().status(&packs, &settings)
}
