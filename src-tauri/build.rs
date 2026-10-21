//! Build script: generates the Tauri context (config, capabilities, icons) and
//! the `allow-*` permissions for the app's own commands.

include!("src/commands/names.rs");

fn main() {
    // Listing the commands makes Tauri deny any command not granted by a
    // capability, instead of allowing every registered command by default.
    let manifest = tauri_build::AppManifest::new().commands(COMMAND_NAMES);
    if let Err(err) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("tauri-build failed: {err:#}");
    }
}
