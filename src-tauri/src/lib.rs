//! Siyak application core: Tauri setup and the Rust side of the IPC boundary.
//!
//! The webview is only the user interface; database, file system and network
//! access live entirely in this crate (spec §3). See `ARCHITECTURE.md` for the
//! module map.

pub mod collections;
pub mod commands;
pub mod db;
pub mod embed;
pub mod error;
pub mod export;
pub mod llm;
pub mod morphology;
pub mod net;
pub mod notes;
pub mod packs;
pub mod related;
pub mod search;
pub mod secrets;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri::path::BaseDirectory;

use crate::commands::packs::TauriEvents;
use crate::db::Databases;
use crate::db::content::Seed;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, UserDbLog};
use crate::packs::manifest::{ManifestStore, PackSource};
use crate::packs::service::{Packs, PacksConfig};

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
    packs: AppResult<Packs>,
}

impl AppState {
    /// Returns the open databases, or the error that prevented opening them.
    pub fn databases(&self) -> AppResult<&Databases> {
        self.databases.as_ref().map_err(Clone::clone)
    }

    /// Returns the pack service, or the error that prevented setting it up.
    pub fn packs(&self) -> AppResult<&Packs> {
        self.packs.as_ref().map_err(Clone::clone)
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
        // The dialogs are opened from Rust; the webview has no permission for them (spec §14.3).
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .setup(|app| {
            let state = open_state(app);
            if let Err(err) = &state.databases {
                eprintln!("database setup failed: {err}");
            } else if let Err(err) = &state.packs {
                eprintln!("pack setup failed: {err}");
            }
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Tauri application");
}

/// Opens the databases, cleans up after interrupted pack jobs and sets up the
/// pack service.
fn open_state(app: &tauri::App) -> AppState {
    let opened = data_dir(app).and_then(|dir| {
        let seed = seed(app)?;
        let databases = Databases::open(&dir, &seed)?;
        Ok((dir, seed, databases))
    });
    let (data_dir, seed, databases) = match opened {
        Ok(opened) => opened,
        Err(err) => {
            return AppState {
                databases: Err(err.clone()),
                packs: Err(err),
            };
        }
    };
    let cleaned =
        lock(&databases.content).and_then(|conn| packs::install::cleanup(&conn, &data_dir));
    if let Err(err) = cleaned {
        eprintln!("cleaning up pack files failed: {err}");
    }
    let packs = open_packs(app, data_dir.clone(), data_dir.join(seed.file_name));
    AppState {
        databases: Ok(databases),
        packs,
    }
}

/// Environment variable that points debug builds at a local pack directory
/// (e.g. `../siyak-data/dist`) instead of the published manifest.
const PACK_SOURCE_VAR: &str = "SIYAK_PACK_SOURCE";

fn open_packs(app: &tauri::App, data_dir: PathBuf, content_path: PathBuf) -> AppResult<Packs> {
    let log = UserDbLog::spawn(&data_dir.join(db::user::FILE_NAME))?;
    let net = Net::new(Arc::new(log))?;
    let source = match std::env::var_os(PACK_SOURCE_VAR) {
        Some(dir) if cfg!(debug_assertions) && !dir.is_empty() => PackSource::Local(dir.into()),
        _ => PackSource::Remote,
    };
    Ok(Packs::new(
        net,
        ManifestStore::new(source, &data_dir),
        PacksConfig {
            data_dir,
            content_path,
            disk: packs::service::available_space,
        },
        Arc::new(TauriEvents(app.handle().clone())),
    ))
}

fn data_dir(app: &tauri::App) -> AppResult<PathBuf> {
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
    Ok(data_dir)
}

/// Environment variable that picks the dataset of debug builds.
const DEV_DATA_VAR: &str = "SIYAK_DEV_DATA";

/// The database the content database is created from. Release builds use the
/// bundled `core.db`. Debug builds use it too, so published packs install, or
/// the small development database with `SIYAK_DEV_DATA=dev` (spec §16.3); each
/// gets its own content file. Both are committed, so neither needs the pipeline.
///
/// Chosen with `cfg!` rather than `#[cfg]` so both branches are always compiled.
fn seed(app: &tauri::App) -> AppResult<Seed> {
    if cfg!(debug_assertions) {
        return dev_seed(std::env::var(DEV_DATA_VAR).ok().as_deref());
    }
    let path = app
        .path()
        .resolve("resources/core.db", BaseDirectory::Resource)
        .map_err(|err| AppError::new(AppErrorCode::SeedMissing, err.to_string()))?;
    Ok(Seed::bundled(path))
}

/// The seed of a debug build for a `SIYAK_DEV_DATA` value.
fn dev_seed(value: Option<&str>) -> AppResult<Seed> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    match value.unwrap_or("full") {
        "full" | "" => Ok(Seed {
            path: root.join("resources/core.db"),
            file_name: "content.full.db",
            marker: Some("full"),
        }),
        "dev" => Ok(Seed {
            path: root.join("../dev-data/dev-content.db"),
            file_name: db::content::FILE_NAME,
            marker: Some("dev"),
        }),
        other => Err(AppError::new(
            AppErrorCode::SeedMissing,
            format!("{DEV_DATA_VAR} must be 'dev' or 'full', not {other:?}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `COMMAND_NAMES` (used by `build.rs` for permissions) must list exactly
    /// the registered commands, or the webview would be denied a command.
    #[test]
    fn dev_seeds_use_separate_content_files() {
        let dev = dev_seed(Some("dev")).unwrap();
        assert_eq!(dev.file_name, "content.db");
        assert_eq!(dev.marker, Some("dev"));
        assert!(dev.path.is_file());
        for value in [None, Some(""), Some("full")] {
            let full = dev_seed(value).unwrap();
            assert_eq!(full.file_name, "content.full.db");
            assert_eq!(full.marker, Some("full"));
            assert!(full.path.is_file());
        }
        let err = dev_seed(Some("everything")).unwrap_err();
        assert_eq!(err.code, AppErrorCode::SeedMissing);
    }

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
