//! Desktop entry point. All application logic lives in the `siyak_lib` crate.

// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    siyak_lib::run()
}
