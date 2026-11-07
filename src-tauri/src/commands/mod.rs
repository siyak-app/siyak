//! The IPC surface: every command the webview can call (spec §12).
//!
//! Commands are thin: they take the shared state, lock a connection and call
//! into [`crate::db`]. Types are exported to `src/lib/bindings.ts` with
//! `tauri-specta`, so the frontend never uses raw `invoke` strings.
//!
//! Commands are `async` so Tauri runs them off the main (UI) thread.

pub mod ai;
pub mod audio;
pub mod chat;
pub mod collections;
pub mod content;
pub mod export;
pub mod keys;
pub mod morphology;
pub mod notes;
pub mod packs;
pub mod search;
pub mod semantic;
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
            audio::list_audio_packs,
            morphology::get_morphology_pack,
            morphology::get_verse_words,
            morphology::get_word,
            morphology::concordance,
            morphology::search_roots,
            search::classify_query,
            search::search_text,
            semantic::get_semantic_status,
            semantic::search_meaning,
            semantic::find_verses,
            semantic::related_by_meaning,
            semantic::related_by_roots,
            keys::get_api_keys,
            keys::set_api_key,
            keys::delete_api_key,
            ai::get_ai_status,
            ai::list_models,
            ai::cancel_generation,
            ai::get_usage_summary,
            ai::list_tafsir_translations,
            ai::estimate_tafsir_translation,
            ai::translate_tafsir,
            chat::chat_send,
            chat::estimate_chat,
            chat::list_chat_sessions,
            chat::get_chat_session,
            chat::delete_chat_session,
            chat::rename_chat_session,
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
            notes::toggle_bookmark,
            notes::list_bookmarks,
            notes::upsert_note,
            notes::delete_note,
            notes::list_notes,
            collections::list_collections,
            collections::get_collection,
            collections::upsert_collection,
            collections::delete_collection,
            collections::add_collection_item,
            collections::update_collection_item,
            collections::move_collection_item,
            collections::remove_collection_item,
            export::list_citation_styles,
            export::format_citation,
            export::export_collection_markdown,
            export::export_user_data,
            export::import_user_data,
        ])
        // Event payloads and names: emitted with `Emitter::emit`, not commands.
        .typ::<PackProgress>()
        .typ::<PackDone>()
        .typ::<PackFailed>()
        .constant("PACK_PROGRESS_EVENT", PROGRESS_EVENT)
        .constant("PACK_DONE_EVENT", DONE_EVENT)
        .constant("PACK_ERROR_EVENT", ERROR_EVENT)
}
