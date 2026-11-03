//! Pack commands (spec §6.4, §12): the catalog, the install queue, removal,
//! terms acknowledgement, the connection log and source links.
//!
//! Job updates arrive as the `pack://progress`, `pack://done` and
//! `pack://error` events (payloads [`PackProgress`], [`PackDone`] and
//! [`PackFailed`]).

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::db::user::{self, NetworkLogEntry};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::packs::manifest::{self, Catalog, InstalledPack, Snapshot};
use crate::packs::service::{
    DONE_EVENT, ERROR_EVENT, PROGRESS_EVENT, PackDone, PackEvents, PackFailed, PackProgress,
};

/// Sends job updates to the webview as events.
pub struct TauriEvents(pub AppHandle);

impl PackEvents for TauriEvents {
    fn progress(&self, event: PackProgress) {
        let _ = self.0.emit(PROGRESS_EVENT, event);
    }
    fn done(&self, event: PackDone) {
        let _ = self.0.emit(DONE_EVENT, event);
    }
    fn failed(&self, event: PackFailed) {
        let _ = self.0.emit(ERROR_EVENT, event);
    }
}

/// Sizes of `content.db` before and after compacting, in bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct VacuumSizes {
    /// Before.
    pub before: f64,
    /// After.
    pub after: f64,
}

/// The manifest (cached for an hour unless `force`) combined with what is
/// installed and queued. Never fails for network reasons: an offline catalog
/// carries the error and the last good manifest, if any.
#[tauri::command]
#[specta::specta]
pub async fn fetch_manifest(state: State<'_, AppState>, force: bool) -> AppResult<Catalog> {
    let snapshot = state.packs()?.manifest(force).await;
    catalog(&state, &snapshot)
}

/// Queues a pack (and the packs it requires). `confirm_replace` confirms
/// that a morphology pack replaces the installed one.
#[tauri::command]
#[specta::specta]
pub async fn install_pack(
    state: State<'_, AppState>,
    id: String,
    confirm_replace: bool,
) -> AppResult<()> {
    let packs = state.packs()?;
    let snapshot = packs.manifest(false).await;
    let catalog = catalog(&state, &snapshot)?;
    packs.install(&catalog, &snapshot, &[id], confirm_replace)
}

/// Queues the members of a bundle that are not installed yet.
#[tauri::command]
#[specta::specta]
pub async fn install_bundle(
    state: State<'_, AppState>,
    id: String,
    confirm_replace: bool,
) -> AppResult<()> {
    let packs = state.packs()?;
    let snapshot = packs.manifest(false).await;
    let catalog = catalog(&state, &snapshot)?;
    let members = packs.bundle_members(&catalog, &id)?;
    packs.install(&catalog, &snapshot, &members, confirm_replace)
}

/// Cancels a queued or running job; the job reports `install_cancelled`.
#[tauri::command]
#[specta::specta]
pub async fn cancel_install(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.packs()?.cancel(&id)
}

/// Removes an installed pack (not a bundled one).
#[tauri::command]
#[specta::specta]
pub async fn uninstall_pack(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.packs()?.uninstall(&id).await
}

/// Compacts `content.db`; suggested after removing packs.
#[tauri::command]
#[specta::specta]
pub async fn vacuum_content(state: State<'_, AppState>) -> AppResult<VacuumSizes> {
    let sizes = state.packs()?.vacuum().await?;
    Ok(VacuumSizes {
        before: sizes.before as f64,
        after: sizes.after as f64,
    })
}

/// Records that the user has read the terms of `id` at `version`.
#[tauri::command]
#[specta::specta]
pub async fn acknowledge_pack_terms(
    state: State<'_, AppState>,
    id: String,
    version: String,
) -> AppResult<()> {
    user::acknowledge_terms(&*state.user()?, &id, &version)
}

/// The newest connection log entries, newest first (spec §14.1).
#[tauri::command]
#[specta::specta]
pub async fn get_network_log(
    state: State<'_, AppState>,
    limit: u32,
) -> AppResult<Vec<NetworkLogEntry>> {
    user::network_log(&*state.user()?, limit)
}

/// Opens the source page of a pack (`citation.url`) in the default browser.
/// Only addresses from the manifest or `content.db` are opened, never one
/// from the webview.
#[tauri::command]
#[specta::specta]
pub async fn open_pack_source(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let snapshot = state.packs()?.manifest(false).await;
    let catalog = catalog(&state, &snapshot)?;
    let url = catalog
        .packs
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| p.source_url.clone())
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("{id}: no source link")))?;
    tauri_plugin_opener::open_url(url, None::<&str>)
        .map_err(|err| AppError::new(AppErrorCode::IoFailed, err.to_string()))
}

/// The catalog for `snapshot`, reading installed packs and acknowledgements
/// from the shared connections (each lock held only for its query).
fn catalog(state: &AppState, snapshot: &Snapshot) -> AppResult<Catalog> {
    let installed: Vec<InstalledPack> = manifest::installed_packs(&*state.content()?)?;
    let acks = user::acknowledged_terms(&*state.user()?)?;
    Ok(state.packs()?.catalog(snapshot, &installed, &acks))
}
