//! API key commands (spec §14.2). Keys go into the OS keychain and never come
//! back: the webview only learns whether each provider's key is saved.

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::AppState;
use crate::error::AppResult;
use crate::secrets::ProviderId;

/// Whether a provider's key is saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ApiKeyStatus {
    /// The provider.
    pub provider: ProviderId,
    /// Whether its key is in the keychain.
    pub saved: bool,
}

/// Which providers have a saved key, in display order.
#[tauri::command]
#[specta::specta]
pub async fn get_api_keys(state: State<'_, AppState>) -> AppResult<Vec<ApiKeyStatus>> {
    let keys = state.keys();
    ProviderId::ALL
        .into_iter()
        .map(|provider| {
            Ok(ApiKeyStatus {
                provider,
                saved: keys.has_key(provider)?,
            })
        })
        .collect()
}

/// Saves (or replaces) a provider's key; surrounding spaces are dropped.
#[tauri::command]
#[specta::specta]
pub async fn set_api_key(
    state: State<'_, AppState>,
    provider: ProviderId,
    key: String,
) -> AppResult<()> {
    state.keys().set_key(provider, &key)
}

/// Removes a provider's key from the keychain.
#[tauri::command]
#[specta::specta]
pub async fn delete_api_key(state: State<'_, AppState>, provider: ProviderId) -> AppResult<()> {
    state.keys().delete_key(provider)
}
