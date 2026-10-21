//! `user.db`: bookmarks, notes, collections, settings and other user data.
//!
//! This file is the only place user data lives, so it is backed up to
//! `user.db.bak` before any migration touches it (spec §5.6).

use std::path::Path;

use rusqlite::{Connection, MAIN_DB};

use super::{Migration, connect, migrate, pending};
use crate::error::{AppError, AppErrorCode, AppResult};

/// File name of the user database inside the app data directory.
pub const FILE_NAME: &str = "user.db";

/// File name of the pre-migration backup, next to [`FILE_NAME`].
pub const BACKUP_FILE_NAME: &str = "user.db.bak";

/// User schema migrations, oldest first.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/user/0001_initial.sql"),
}];

/// Opens (or creates) `user.db` at `path` and applies pending migrations,
/// backing up an existing database first.
pub fn open(path: &Path) -> AppResult<Connection> {
    let existed = path.exists();
    let mut conn = connect(path)?;
    if existed && !pending(&conn, MIGRATIONS)?.is_empty() {
        backup(&conn, &path.with_file_name(BACKUP_FILE_NAME))?;
    }
    migrate(&mut conn, MIGRATIONS)?;
    Ok(conn)
}

/// Uses SQLite's online backup API rather than a file copy, so the backup also
/// contains changes still sitting in the write-ahead log.
fn backup(conn: &Connection, dest: &Path) -> AppResult<()> {
    conn.backup(MAIN_DB, dest, None).map_err(|err| {
        AppError::new(
            AppErrorCode::IoFailed,
            format!("backing up to {}: {err}", dest.display()),
        )
    })
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
    }

    #[test]
    fn fresh_database_gets_no_backup() {
        let dir = tempfile::tempdir().unwrap();
        open(&dir.path().join(FILE_NAME)).unwrap();
        assert!(!dir.path().join(BACKUP_FILE_NAME).exists());
    }

    #[test]
    fn existing_database_is_backed_up_before_migrating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE marker (x TEXT); INSERT INTO marker VALUES ('old');")
                .unwrap();
        }
        open(&path).unwrap();

        let bak = Connection::open(dir.path().join(BACKUP_FILE_NAME)).unwrap();
        assert_eq!(schema_version(&bak).unwrap(), 0);
        let marker: String = bak
            .query_row("SELECT x FROM marker", [], |r| r.get(0))
            .unwrap();
        assert_eq!(marker, "old");
    }

    #[test]
    fn up_to_date_database_is_not_backed_up_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        open(&path).unwrap();
        open(&path).unwrap();
        assert!(!dir.path().join(BACKUP_FILE_NAME).exists());
    }

    /// Regression test for the spec §5.5 fix: a whole-passage summary uses
    /// `paragraph_idx = -1`, so a second summary for the same key conflicts.
    #[test]
    fn summary_cache_key_is_unique() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let insert = "INSERT INTO tafsir_translations
            (pack_id, tafsir_id, mode, target_lang, pack_version, text, provider, model, created_at)
            VALUES ('ar.jalalayn', 1, 'summary', 'tr', '1.0.0', ?1, 'p', 'm', '2026-10-21')";
        conn.execute(insert, ["first"]).unwrap();
        assert!(conn.execute(insert, ["second"]).is_err());
    }
}
