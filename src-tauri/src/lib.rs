//! Siyak application core: Tauri setup and the Rust side of the IPC boundary.
//!
//! The webview is only the user interface; database, file system and network
//! access live entirely in this crate (spec §3). See `ARCHITECTURE.md` for the
//! module map.

pub mod commands;
pub mod db;
pub mod embed;
pub mod error;
pub mod export;
pub mod llm;
pub mod morphology;
pub mod net;
pub mod packs;
pub mod related;
pub mod search;
pub mod secrets;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri::path::BaseDirectory;

use crate::db::Databases;
use crate::error::{AppError, AppErrorCode, AppResult};

/// First line of the generated bindings file.
const BINDINGS_HEADER: &str =
    "// Regenerate with `cargo xtask bindings`; CI fails if this file is out of date.";

/// State managed by Tauri and shared by all commands.
///
/// A failed database setup is stored instead of aborting startup, so the
/// window still opens and every command reports the stable error code, which
/// the frontend shows as a translated message.
pub struct AppState {
    databases: AppResult<Databases>,
}

impl AppState {
    /// Returns the open databases, or the error that prevented opening them.
    pub fn databases(&self) -> AppResult<&Databases> {
        self.databases.as_ref().map_err(Clone::clone)
    }

    /// Locks the `content.db` connection.
    pub fn content(&self) -> AppResult<MutexGuard<'_, Connection>> {
        lock(&self.databases()?.content)
    }

    /// Locks the `user.db` connection.
    pub fn user(&self) -> AppResult<MutexGuard<'_, Connection>> {
        lock(&self.databases()?.user)
    }
}

/// A poisoned lock means a command panicked mid-query; SQLite has already
/// rolled back any open transaction, so the connection is still usable.
fn lock(conn: &Mutex<Connection>) -> AppResult<MutexGuard<'_, Connection>> {
    Ok(conn
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// Writes the TypeScript bindings for all commands and their types to `path`.
pub fn export_bindings(path: &Path) -> Result<(), specta_typescript::Error> {
    commands::builder().export(Typescript::default().header(BINDINGS_HEADER), path)
}

/// Builds and runs the Tauri application. Blocks until the app exits.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[expect(
    clippy::expect_used,
    reason = "the event loop failing to start is unrecoverable"
)]
pub fn run() {
    // Keep `src/lib/bindings.ts` current while developing; CI checks it with
    // `cargo xtask bindings` and `git diff`.
    if cfg!(debug_assertions) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
        if let Err(err) = export_bindings(&path) {
            eprintln!("exporting TypeScript bindings failed: {err}");
        }
    }

    let builder = commands::builder();
    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(|app| {
            let databases = open_databases(app);
            if let Err(err) = &databases {
                eprintln!("database setup failed: {err}");
            }
            app.manage(AppState { databases });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Tauri application");
}

fn open_databases(app: &tauri::App) -> AppResult<Databases> {
    let data_dir = app.path().app_data_dir().map_err(|err| {
        AppError::new(
            AppErrorCode::IoFailed,
            format!("resolving app data dir: {err}"),
        )
    })?;
    fs::create_dir_all(&data_dir).map_err(|err| {
        AppError::new(
            AppErrorCode::IoFailed,
            format!("creating {}: {err}", data_dir.display()),
        )
    })?;
    Databases::open(&data_dir, &seed_path(app)?)
}

/// The database copied to `content.db` on first launch. Debug builds use the
/// small committed development database so contributors never need the data
/// pipeline (spec §16.3); release builds use the bundled `core.db`.
///
/// Chosen with `cfg!` rather than `#[cfg]` so both branches are always compiled.
fn seed_path(app: &tauri::App) -> AppResult<PathBuf> {
    if cfg!(debug_assertions) {
        return Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-data/dev-content.db"));
    }
    app.path()
        .resolve("resources/core.db", BaseDirectory::Resource)
        .map_err(|err| AppError::new(AppErrorCode::SeedMissing, err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `COMMAND_NAMES` (used by `build.rs` for permissions) must list exactly
    /// the registered commands, or the webview would be denied a command.
    #[test]
    fn command_names_match_registered_commands() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bindings.ts");
        export_bindings(&path).unwrap();
        let bindings = fs::read_to_string(path).unwrap();

        assert_eq!(
            bindings.matches("TAURI_INVOKE(").count(),
            commands::COMMAND_NAMES.len()
        );
        for name in commands::COMMAND_NAMES {
            assert!(
                bindings.contains(&format!("TAURI_INVOKE(\"{name}\"")),
                "{name} is not registered"
            );
        }
    }
}
