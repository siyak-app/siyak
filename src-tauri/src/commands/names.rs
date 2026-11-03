/// Names of every command the webview may invoke.
///
/// Included by `build.rs` (to generate the app's `allow-*` permissions) and by
/// the library (to check it matches the registered commands), so the list
/// exists exactly once. Add new commands here and to `capabilities/main.json`.
pub const COMMAND_NAMES: &[&str] = &[
    "list_surahs",
    "get_surah",
    "get_verses",
    "list_translations",
    "get_core_status",
    "list_tafsirs",
    "get_tafsir",
    "get_tafsir_introduction",
    "get_compare",
    "classify_query",
    "search_text",
    "get_settings",
    "set_setting",
    "set_settings",
    "fetch_manifest",
    "install_pack",
    "install_bundle",
    "cancel_install",
    "uninstall_pack",
    "vacuum_content",
    "acknowledge_pack_terms",
    "get_network_log",
    "open_pack_source",
];
