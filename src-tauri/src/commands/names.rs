/// Names of every command the webview may invoke.
///
/// Included by `build.rs` (to generate the app's `allow-*` permissions) and by
/// the library (to check it matches the registered commands), so the list
/// exists exactly once. Add new commands here and to `capabilities/main.json`.
pub const COMMAND_NAMES: &[&str] = &[
    "list_surahs",
    "get_verses",
    "get_core_status",
    "get_settings",
    "set_setting",
];
