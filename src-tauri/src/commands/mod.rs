//! The IPC surface: every command the webview can call (spec §12).
//!
//! Commands are thin: they take the shared state, lock a connection and call
//! into [`crate::db`]. Types are exported to `src/lib/bindings.ts` with
//! `tauri-specta`, so the frontend never uses raw `invoke` strings.
//!
//! Commands are `async` so Tauri runs them off the main (UI) thread.

pub mod content;
pub mod settings;

include!("names.rs");

use tauri_specta::{Builder, collect_commands};

/// Builds the `tauri-specta` builder with every command registered.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        content::list_surahs,
        content::get_surah,
        content::get_verses,
        content::list_translations,
        content::get_core_status,
        settings::get_settings,
        settings::set_setting,
        settings::set_settings,
    ])
}
