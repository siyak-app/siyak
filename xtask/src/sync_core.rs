//! Copies the real `core.db` built by `siyak-data` into the app as
//! `src-tauri/resources/core.db`, after checking that it fits this build.
//!
//! `core.db` is the database release builds bundle and copy to `content.db` on
//! first launch (spec §5, §6.2). It is built and validated in the separate
//! `siyak-data` repository; this task only checks what the app depends on:
//! the row counts, `content_meta.core_version` and that its schema is exactly
//! what this app's content migrations create.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use siyak_lib::db::{content::MIGRATIONS, migrate, schema_shape, schema_version};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Where the bundled core database lives in the app repository.
pub const RESOURCE_PATH: &str = "src-tauri/resources/core.db";

/// Runs the task. `args` are the arguments after `sync-core`.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let from = match args {
        [] => root.join("../siyak-data/dist/core.db"),
        [flag, path] if flag == "--from" => PathBuf::from(path),
        _ => return Err("usage: cargo xtask sync-core [--from <path to core.db>]".into()),
    };
    let from = from
        .canonicalize()
        .map_err(|err| format!("{}: {err}", from.display()))?;
    let dest = root.join(RESOURCE_PATH);

    let info = check(&from)?;
    if let Some(current) = read_core_version(&dest)? {
        if info.core_version < current {
            return Err(format!(
                "core_version {} is lower than the committed {current}; it must never go down",
                info.core_version
            )
            .into());
        }
        if info.core_version == current && fs::read(&dest)? != fs::read(&from)? {
            println!(
                "note: same core_version {current} but different bytes; fine for a rebuild that \
                 changes no rows (e.g. a new build time), otherwise siyak-data must bump it"
            );
        }
    }

    let tmp = dest.with_extension("db.tmp");
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::copy(&from, &tmp)?;
    fs::rename(&tmp, &dest)?;

    println!("copied {} -> {}", from.display(), dest.display());
    println!("sha256          {}", sha256_hex(&dest)?);
    println!("core_version    {}", info.core_version);
    println!("tanzil_version  {}", info.tanzil_version);
    println!("siyak-data      {}", git_origin(&from));
    Ok(())
}

struct CoreInfo {
    core_version: u32,
    tanzil_version: String,
}

/// Fails unless `path` is a complete core database for this build.
fn check(path: &Path) -> Result<CoreInfo> {
    let core = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let count = |table: &str| -> Result<i64> {
        Ok(core.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
    };
    for (table, expected) in [("surahs", 114), ("verses", 6236)] {
        let found = count(table)?;
        if found != expected {
            return Err(format!("{table}: expected {expected} rows, found {found}").into());
        }
    }

    let expected_version = u32::try_from(MIGRATIONS.len())?;
    let version = schema_version(&core)?;
    if version != expected_version {
        return Err(format!(
            "schema version {version}, but this app's content migrations reach {expected_version}"
        )
        .into());
    }
    let mut fresh = Connection::open_in_memory()?;
    migrate(&mut fresh, MIGRATIONS)?;
    let (want, got) = (schema_shape(&fresh)?, schema_shape(&core)?);
    if want != got {
        let mut diff = String::new();
        for key in want
            .keys()
            .chain(got.keys().filter(|k| !want.contains_key(*k)))
        {
            if want.get(key) != got.get(key) {
                writeln!(diff, "  {key}")?;
                writeln!(
                    diff,
                    "    migrations: {}",
                    want.get(key).map_or("-", String::as_str)
                )?;
                writeln!(
                    diff,
                    "    core.db:    {}",
                    got.get(key).map_or("-", String::as_str)
                )?;
            }
        }
        return Err(format!("schema differs from the content migrations:\n{diff}").into());
    }

    let meta = |key: &str| -> Result<Option<String>> {
        Ok(core
            .query_row(
                "SELECT value FROM content_meta WHERE key = ?1",
                [key],
                |r| r.get(0),
            )
            .optional()?)
    };
    let core_version = meta("core_version")?
        .ok_or("content_meta has no core_version")?
        .parse::<u32>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or("content_meta.core_version is not a positive integer")?;
    let tanzil_version = meta("tanzil_version")?.ok_or("content_meta has no tanzil_version")?;
    Ok(CoreInfo {
        core_version,
        tanzil_version,
    })
}

/// `core_version` of an existing core database, or `None` if there is none yet.
fn read_core_version(path: &Path) -> Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM content_meta WHERE key = 'core_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(value.and_then(|v| v.parse().ok()))
}

fn sha256_hex(path: &Path) -> Result<String> {
    let hash = Sha256::digest(fs::read(path)?);
    Ok(hash.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
}

/// The `siyak-data` commit the file comes from, if it lies inside a git
/// checkout; uncommitted changes there are flagged, since the build may then
/// not match the commit.
fn git_origin(file: &Path) -> String {
    let Some(dir) = file.parent() else {
        return "unknown".to_owned();
    };
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    match git(&["rev-parse", "HEAD"]) {
        Some(commit) => {
            let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
            if dirty {
                format!("{commit} (working tree has uncommitted changes)")
            } else {
                commit
            }
        }
        None => "unknown (not a git checkout)".to_owned(),
    }
}
