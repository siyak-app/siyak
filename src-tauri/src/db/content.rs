//! `content.db`: the Quran text and installed pack content.
//!
//! On first launch the bundled seed database is copied into place (spec §5);
//! afterwards only the app writes to this file.
//! TODO(M3): pack installation writes translation/tafsir/morphology rows here.

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::{Migration, connect, migrate};
use crate::error::{AppError, AppErrorCode, AppResult};

/// File name of the content database inside the app data directory.
pub const FILE_NAME: &str = "content.db";

/// Content schema migrations, oldest first.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/content/0001_initial.sql"),
}];

/// Opens `content.db` at `path`, copying `seed` there first if the file does
/// not exist yet, then applies pending migrations.
pub fn open(path: &Path, seed: &Path) -> AppResult<Connection> {
    if !path.exists() {
        copy_seed(seed, path)?;
    }
    let mut conn = connect(path)?;
    migrate(&mut conn, MIGRATIONS)?;
    Ok(conn)
}

/// Copies through a temporary file and renames it, so an interrupted copy
/// never leaves a half-written `content.db` that would be opened next time.
fn copy_seed(seed: &Path, dest: &Path) -> AppResult<()> {
    if !seed.is_file() {
        return Err(AppError::new(
            AppErrorCode::SeedMissing,
            format!("seed database not found at {}", seed.display()),
        ));
    }
    let io_failed = |err: std::io::Error| {
        AppError::new(
            AppErrorCode::IoFailed,
            format!("copying {} to {}: {err}", seed.display(), dest.display()),
        )
    };
    let tmp = dest.with_extension("db.tmp");
    fs::copy(seed, &tmp).map_err(io_failed)?;
    fs::rename(&tmp, dest).map_err(io_failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{schema_version, tests::assert_contiguous};

    #[test]
    fn migrations_are_contiguous() {
        assert_contiguous(MIGRATIONS);
    }

    #[test]
    fn migrations_apply_on_empty_db_and_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, MIGRATIONS).unwrap(), MIGRATIONS.len());
        assert_eq!(migrate(&mut conn, MIGRATIONS).unwrap(), 0);
        assert_eq!(schema_version(&conn).unwrap(), 1);
        for table in [
            "surahs",
            "verses",
            "packs",
            "translations",
            "tafsir",
            "tafsir_paragraphs",
            "words",
            "roots",
            "lemmas",
            "word_segments",
            "verses_fts",
            "translations_fts",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "missing table {table}");
        }
    }

    #[test]
    fn first_open_copies_seed() {
        let dir = tempfile::tempdir().unwrap();
        let seed = dir.path().join("seed.db");
        {
            let mut conn = Connection::open(&seed).unwrap();
            migrate(&mut conn, MIGRATIONS).unwrap();
            conn.execute(
                "INSERT INTO surahs VALUES (1, 'الفاتحة', 'Fâtiha', 'Al-Fatihah', 'meccan', 7)",
                [],
            )
            .unwrap();
        }
        let path = dir.path().join(FILE_NAME);
        let conn = open(&path, &seed).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM surahs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert!(!path.with_extension("db.tmp").exists());
    }

    #[test]
    fn missing_seed_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let err = open(&dir.path().join(FILE_NAME), &dir.path().join("nope.db")).unwrap_err();
        assert_eq!(err.code, AppErrorCode::SeedMissing);
    }
}
