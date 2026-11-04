//! Export commands (spec §11.2, §12). The file dialogs are opened from here:
//! the webview has no file system or dialog permission (spec §14.3).

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, FilePath};

use crate::AppState;
use crate::commands::collections::read_collection;
use crate::db;
use crate::db::content::VerseIndex;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::export::backup::{self, ImportMode, ImportReport};
use crate::export::citation::{self, CitationRef, CitationStyle};
use crate::export::markdown::{self, MarkdownLabels};
use crate::notes;

/// The largest backup file read: far beyond what a person writes, so a wrong
/// file is refused before it fills the memory.
const MAX_BACKUP_BYTES: u64 = 256 * 1024 * 1024;

/// Name of the copy of `user.db` kept from just before a replacing import.
const BEFORE_IMPORT_FILE_NAME: &str = "user.db.before-import";

/// Waits for a dialog off the async workers, returning its path if one was chosen.
async fn dialog_path(
    show: impl FnOnce() -> Option<FilePath> + Send + 'static,
) -> AppResult<Option<PathBuf>> {
    let chosen = tauri::async_runtime::spawn_blocking(show)
        .await
        .map_err(|err| AppError::new(AppErrorCode::IoFailed, err.to_string()))?;
    chosen
        .map(|path| {
            path.into_path()
                .map_err(|err| AppError::new(AppErrorCode::IoFailed, err.to_string()))
        })
        .transpose()
}

fn save_dialog(
    app: &AppHandle,
    filter: &str,
    extension: &str,
    name: &str,
) -> FileDialogBuilder<tauri::Wry> {
    app.dialog()
        .file()
        .add_filter(filter, &[extension])
        .set_file_name(format!("{name}.{extension}"))
}

fn write_file(path: &Path, text: &str) -> AppResult<()> {
    std::fs::write(path, text)
        .map_err(|err| AppError::new(AppErrorCode::IoFailed, format!("{}: {err}", path.display())))
}

/// A file name from a title: what file systems refuse becomes a space.
fn file_stem(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim();
    if cleaned.is_empty() {
        "siyak".to_owned()
    } else {
        cleaned.to_owned()
    }
}

/// The available citation styles.
#[tauri::command]
#[specta::specta]
pub async fn list_citation_styles() -> AppResult<Vec<CitationStyle>> {
    Ok(citation::list_styles())
}

/// Citation text for verses or a tafsir passage, one source per line.
#[tauri::command]
#[specta::specta]
pub async fn format_citation(
    state: State<'_, AppState>,
    reference: CitationRef,
    style: String,
) -> AppResult<String> {
    let conn = state.content()?;
    let index = VerseIndex::load(&conn)?;
    citation::format_citation(&conn, &index, &reference, &style)
}

/// The Markdown of a collection, in the given translations and citation style.
pub fn collection_markdown(
    state: &AppState,
    id: u32,
    translation_ids: &[String],
    labels: &MarkdownLabels,
    style: &str,
) -> AppResult<(String, String)> {
    let collection = read_collection(state, id, translation_ids)?;
    let index = VerseIndex::load(&*state.content()?)?;
    let notes = notes::list_notes(&*state.user()?, &index, None)?;
    let text = markdown::render(&*state.content()?, &collection, &notes, labels, style)?;
    Ok((collection.summary.title, text))
}

/// Saves a collection as Markdown where the user chooses. Returns the path, or
/// `None` if the user cancelled.
#[tauri::command]
#[specta::specta]
pub async fn export_collection_markdown(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u32,
    translation_ids: Vec<String>,
    labels: MarkdownLabels,
    style: String,
) -> AppResult<Option<String>> {
    let (title, text) = collection_markdown(&state, id, &translation_ids, &labels, &style)?;
    let dialog = save_dialog(&app, "Markdown", "md", &file_stem(&title));
    let Some(path) = dialog_path(move || dialog.blocking_save_file()).await? else {
        return Ok(None);
    };
    write_file(&path, &text)?;
    Ok(Some(path.display().to_string()))
}

/// The backup file's text.
pub fn user_data_json(state: &AppState) -> AppResult<String> {
    let collected = backup::collect(&*state.user()?)?;
    let content = state.content()?;
    let index = VerseIndex::load(&content)?;
    backup::to_json(&backup::portable(collected, &content, &index)?)
}

/// Saves the backup where the user chooses. Returns the path, or `None` if the
/// user cancelled.
#[tauri::command]
#[specta::specta]
pub async fn export_user_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<String>> {
    let text = user_data_json(&state)?;
    let dialog = save_dialog(&app, "JSON", "json", "siyak-backup");
    let Some(path) = dialog_path(move || dialog.blocking_save_file()).await? else {
        return Ok(None);
    };
    write_file(&path, &text)?;
    Ok(Some(path.display().to_string()))
}

/// Imports a backup's text. A replacing import first keeps a copy of `user.db`
/// next to it, as it cannot be undone otherwise.
pub fn import_user_data_text(
    state: &AppState,
    text: &str,
    mode: ImportMode,
) -> AppResult<ImportReport> {
    let data = backup::parse(text)?;
    let prepared = {
        let content = state.content()?;
        let index = VerseIndex::load(&content)?;
        backup::prepare(data, &content, &index)?
    };
    let mut conn = state.user()?;
    if mode == ImportMode::Replace
        && let Some(path) = conn.path().filter(|path| !path.is_empty())
    {
        let copy = Path::new(path).with_file_name(BEFORE_IMPORT_FILE_NAME);
        db::backup(&conn, &copy)?;
    }
    backup::apply(&mut conn, prepared, mode)
}

/// Asks for a backup file and imports it. Returns the report, or `None` if the
/// user cancelled.
#[tauri::command]
#[specta::specta]
pub async fn import_user_data(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: ImportMode,
) -> AppResult<Option<ImportReport>> {
    let dialog = app.dialog().file().add_filter("JSON", &["json"]);
    let Some(path) = dialog_path(move || dialog.blocking_pick_file()).await? else {
        return Ok(None);
    };
    let io = |err: std::io::Error| {
        AppError::new(AppErrorCode::IoFailed, format!("{}: {err}", path.display()))
    };
    if std::fs::metadata(&path).map_err(io)?.len() > MAX_BACKUP_BYTES {
        return Err(AppError::new(AppErrorCode::BackupInvalid, "file too large"));
    }
    // Not UTF-8 means it is not a backup, whatever it is.
    let text = std::fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::InvalidData {
            AppError::new(AppErrorCode::BackupInvalid, err.to_string())
        } else {
            io(err)
        }
    })?;
    import_user_data_text(&state, &text, mode).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_lose_what_file_systems_refuse() {
        assert_eq!(file_stem("Sabır: 2/153"), "Sabır  2 153");
        assert_eq!(file_stem("..."), "siyak");
        assert_eq!(file_stem(" a. "), "a");
    }
}
