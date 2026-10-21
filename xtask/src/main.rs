//! Repository automation, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! Written in Rust rather than as a Node script so it can reuse the app's own
//! embedded migrations: the development database schema is identical to the
//! one the app creates by construction.
//!
//! Tasks:
//! - `build-dev-db`: regenerate `dev-data/dev-content.db` from `dev-data/fixtures/`.

mod dev_db;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("build-dev-db") => repo_root().and_then(|root| dev_db::build(&root)),
        _ => {
            eprintln!("usage: cargo xtask <build-dev-db>");
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
