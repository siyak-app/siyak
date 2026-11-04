//! The IPC surface: every command the webview can call (spec §12).
//!
//! Commands are thin: they take the shared state, lock a connection and call
//! into [`crate::db`]. Types are exported to `src/lib/bindings.ts` with
//! `tauri-specta`, so the frontend never uses raw `invoke` strings.
//!
//! Commands are `async` so Tauri runs them off the main (UI) thread.

pub mod content;
pub mod morphology;
pub mod packs;
pub mod search;
pub mod settings;

include!("names.rs");

use tauri_specta::{Builder, collect_commands};

use crate::packs::service::{
    DONE_EVENT, ERROR_EVENT, PROGRESS_EVENT, PackDone, PackFailed, PackProgress,
};

/// Builds the `tauri-specta` builder with every command registered.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            content::list_surahs,
            content::get_surah,
            content::get_verses,
            content::list_translations,
            content::get_core_status,
            content::list_tafsirs,
            content::get_tafsir,
            content::get_tafsir_introduction,
            content::get_compare,
            morphology::get_morphology_pack,
            morphology::get_verse_words,
            morphology::get_word,
            morphology::concordance,
            morphology::search_roots,
            search::classify_query,
            search::search_text,
            settings::get_settings,
            settings::set_setting,
            settings::set_settings,
            packs::fetch_manifest,
            packs::install_pack,
            packs::install_bundle,
            packs::cancel_install,
            packs::uninstall_pack,
            packs::vacuum_content,
            packs::acknowledge_pack_terms,
            packs::get_network_log,
            packs::open_pack_source,
        ])
        // Event payloads and names: emitted with `Emitter::emit`, not commands.
        .typ::<PackProgress>()
        .typ::<PackDone>()
        .typ::<PackFailed>()
        .constant("PACK_PROGRESS_EVENT", PROGRESS_EVENT)
        .constant("PACK_DONE_EVENT", DONE_EVENT)
        .constant("PACK_ERROR_EVENT", ERROR_EVENT)
}
