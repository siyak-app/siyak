//! Settings commands backed by `user.db.settings`.

use tauri::State;

use crate::AppState;
use crate::db::user::{self, Settings};
use crate::error::AppResult;

/// Returns all stored settings as `key → JSON text`.
#[tauri::command]
#[specta::specta]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    user::get_settings(&*state.user()?)
}

/// Stores a setting. `value` must be JSON text, e.g. `"\"tr\""` or `"{\"size\":20}"`.
#[tauri::command]
#[specta::specta]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    user::set_setting(&*state.user()?, &key, &value)
}
