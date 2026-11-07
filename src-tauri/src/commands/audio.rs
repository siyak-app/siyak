//! Recitation audio (ADR 0014): which audio packs are installed. The verse files
//! themselves reach the webview through the `siyak-audio:` scheme (`crate::audio`).

use tauri::State;

use crate::AppState;
use crate::audio::{self, AudioPack};
use crate::error::AppResult;

/// Lists the installed audio packs (reciters).
#[tauri::command]
#[specta::specta]
pub async fn list_audio_packs(state: State<'_, AppState>) -> AppResult<Vec<AudioPack>> {
    audio::list_audio_packs(&*state.content()?)
}
