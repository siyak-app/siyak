//! `user.db`: bookmarks, notes, collections, settings and other user data.
//!
//! This file is the only place user data lives, so it is backed up to
//! `user.db.bak` before any migration touches it (spec §5.6).

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::Connection;

use super::{Migration, backup, connect, migrate, pending};
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

/// All settings as `key → JSON text`.
///
/// Values cross the IPC boundary as JSON text rather than as a JSON value type
/// because the bindings generator refuses types that may contain 64-bit
/// integers; the frontend parses them with its own typed defaults.
pub type Settings = BTreeMap<String, String>;

/// Longest accepted setting key.
const MAX_KEY_LEN: usize = 128;

/// Returns every stored setting.
pub fn get_settings(conn: &Connection) -> AppResult<Settings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Stores `value` (JSON text) under `key`, replacing any previous value.
///
/// Keys are dotted lowercase paths such as `layout.left_pane` (spec §13.2);
/// values must be valid JSON because `settings.value` is a JSON column (spec §5.5).
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    if !is_valid_key(key) {
        return Err(AppError::new(
            AppErrorCode::InvalidSettingKey,
            format!("invalid setting key {key:?}"),
        ));
    }
    if let Err(err) = serde_json::from_str::<serde::de::IgnoredAny>(value) {
        return Err(AppError::new(
            AppErrorCode::InvalidSettingValue,
            format!("setting {key}: {err}"),
        ));
    }
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}

/// `^[a-z0-9_]+(\.[a-z0-9_]+)*$`, at most [`MAX_KEY_LEN`] bytes.
fn is_valid_key(key: &str) -> bool {
    key.len() <= MAX_KEY_LEN
        && key.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
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

    fn migrated() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        conn
    }

    #[test]
    fn settings_round_trip_and_overwrite() {
        let conn = migrated();
        assert!(get_settings(&conn).unwrap().is_empty());
        set_setting(&conn, "ui.language", r#""tr""#).unwrap();
        set_setting(
            &conn,
            "layout.left_pane",
            r#"{"size":20,"collapsed":false}"#,
        )
        .unwrap();
        set_setting(&conn, "ui.language", r#""en""#).unwrap();
        let settings = get_settings(&conn).unwrap();
        assert_eq!(settings.len(), 2);
        assert_eq!(settings["ui.language"], r#""en""#);
    }

    #[test]
    fn invalid_setting_keys_are_rejected() {
        let conn = migrated();
        for key in [
            "",
            "UI.language",
            "ui..language",
            "ui.language.",
            "ui language",
            "ü",
        ] {
            let err = set_setting(&conn, key, "1").unwrap_err();
            assert_eq!(err.code, AppErrorCode::InvalidSettingKey, "key {key:?}");
        }
        let long = "a".repeat(MAX_KEY_LEN + 1);
        assert!(set_setting(&conn, &long, "1").is_err());
    }

    #[test]
    fn invalid_setting_values_are_rejected() {
        let conn = migrated();
        let err = set_setting(&conn, "ui.theme", "dark").unwrap_err();
        assert_eq!(err.code, AppErrorCode::InvalidSettingValue);
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
