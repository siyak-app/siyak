//! Repository automation, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! Written in Rust rather than as a Node script so it can reuse the app's own
//! embedded migrations: the development database schema is identical to the
//! one the app creates by construction.
//!
//! Tasks:
//! - `build-dev-db`: regenerate `dev-data/dev-content.db` from `dev-data/fixtures/`.
//! - `bindings`: regenerate `src/lib/bindings.ts` from the registered commands.
//! - `sync-core [--from <path>]`: check and copy `siyak-data`'s `core.db` to
//!   `src-tauri/resources/core.db` (default source `../siyak-data/dist/core.db`).

mod dev_db;
mod sync_core;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("build-dev-db") => repo_root().and_then(|root| dev_db::build(&root)),
        Some("bindings") => repo_root().and_then(|root| {
            let path = root.join("src/lib/bindings.ts");
            siyak_lib::export_bindings(&path)?;
            println!("wrote {}", path.display());
            Ok(())
        }),
        Some("sync-core") => repo_root().and_then(|root| sync_core::run(&root, &args[1..])),
        _ => {
            eprintln!("usage: cargo xtask <build-dev-db|bindings|sync-core>");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?)
}
