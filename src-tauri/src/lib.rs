//! Siyak application core: Tauri setup and the Rust side of the IPC boundary.
//!
//! The webview is only the user interface; database, file system and network
//! access live entirely in this crate (spec §3).

/// Builds and runs the Tauri application. Blocks until the app exits.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[expect(
    clippy::expect_used,
    reason = "the event loop failing to start is unrecoverable"
)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the Tauri application");
}
