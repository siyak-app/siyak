//! SQLite access: the two databases and their schema migrations.
//!
//! `content.db` holds the Quran text and installed packs and can be rebuilt at
//! any time; `user.db` holds the user's own data and is never touched by pack
//! operations (spec §5, ADR 0002). Both are versioned with `PRAGMA user_version`
//! and migrated on startup by [`migrate`] (spec §5.6).

pub mod content;
pub mod user;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::types::Value;
use rusqlite::{Connection, MAIN_DB};

use crate::error::{AppError, AppErrorCode, AppResult};

/// Open connections to both databases, shared by all commands.
pub struct Databases {
    /// `content.db`: Quran text and pack content.
    pub content: Mutex<Connection>,
    /// `user.db`: the user's own data.
    pub user: Mutex<Connection>,
    /// Why refreshing the core data failed at startup, if it did (spec §6.2).
    pub core_refresh_error: Option<AppError>,
}

impl Databases {
    /// Opens both databases inside `data_dir`, seeding the content database
    /// from `seed` on first launch and applying pending migrations.
    pub fn open(data_dir: &Path, seed: &content::Seed) -> AppResult<Self> {
        let content = content::open(data_dir, seed)?;
        let user = user::open(&data_dir.join(user::FILE_NAME))?;
        Ok(Self {
            content: Mutex::new(content.conn),
            user: Mutex::new(user),
            core_refresh_error: content.core_refresh_error,
        })
    }
}

/// One ordered schema migration, embedded into the binary at compile time.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Target `user_version` after this migration; versions start at 1 and are contiguous.
    pub version: u32,
    /// Short name, matching the SQL file name (`NNNN_<name>.sql`).
    pub name: &'static str,
    /// SQL executed as a batch.
    pub sql: &'static str,
    /// Run with foreign keys off, for a migration that rebuilds a table other tables
    /// reference (dropping it would cascade). The foreign keys are checked before commit.
    pub foreign_keys_off: bool,
}

/// Opens (creating if needed) a database file with the connection settings
/// every Siyak database uses.
pub fn connect(path: &Path) -> AppResult<Connection> {
    let open_failed = |err: rusqlite::Error| {
        AppError::new(
            AppErrorCode::DbOpenFailed,
            format!("{}: {err}", path.display()),
        )
    };
    let conn = Connection::open(path).map_err(open_failed)?;
    // Foreign keys are off by default in SQLite and must be enabled per connection.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(open_failed)?;
    Ok(conn)
}

/// Reads the schema version stored in the database header.
pub fn schema_version(conn: &Connection) -> AppResult<u32> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Returns the migrations that have not been applied to `conn` yet.
///
/// Fails with [`AppErrorCode::DbVersionUnsupported`] if the database is newer
/// than the newest migration this build knows, so an older app never writes to
/// a schema it does not understand.
pub fn pending<'m>(conn: &Connection, migrations: &'m [Migration]) -> AppResult<&'m [Migration]> {
    let current = schema_version(conn)?;
    let latest = migrations.last().map_or(0, |m| m.version);
    if current > latest {
        return Err(AppError::new(
            AppErrorCode::DbVersionUnsupported,
            format!("schema version {current} is newer than supported version {latest}"),
        ));
    }
    let start = migrations.partition_point(|m| m.version <= current);
    Ok(&migrations[start..])
}

/// Applies all pending migrations in a single transaction and returns how many
/// were applied. Either every pending migration succeeds or the database is
/// left exactly as it was.
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> AppResult<usize> {
    let todo = pending(conn, migrations)?;
    let Some(last) = todo.last() else {
        return Ok(0);
    };
    let failed = |m: &Migration, err: rusqlite::Error| {
        AppError::new(
            AppErrorCode::MigrationFailed,
            format!("{:04}_{}: {err}", m.version, m.name),
        )
    };
    // `PRAGMA foreign_keys` is a no-op inside a transaction, so it is switched around it.
    let keys_off = todo.iter().any(|m| m.foreign_keys_off);
    let keys_were_on: bool = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|err| failed(last, err))?;
    if keys_off && keys_were_on {
        conn.pragma_update(None, "foreign_keys", false)
            .map_err(|err| failed(last, err))?;
    }
    let result = apply(conn, todo, keys_off);
    if keys_off && keys_were_on {
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|err| failed(last, err))?;
    }
    result.map_err(|(m, err)| failed(m, err))?;
    Ok(todo.len())
}

/// Runs `todo` and sets `user_version` in one transaction; with `check_keys`, fails
/// before committing if any foreign key is broken.
fn apply<'m>(
    conn: &mut Connection,
    todo: &'m [Migration],
    check_keys: bool,
) -> Result<(), (&'m Migration, rusqlite::Error)> {
    let Some(last) = todo.last() else {
        return Ok(());
    };
    let tx = conn.transaction().map_err(|err| (last, err))?;
    for m in todo {
        tx.execute_batch(m.sql).map_err(|err| (m, err))?;
    }
    if check_keys {
        let broken: i64 = tx
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .map_err(|err| (last, err))?;
        if broken > 0 {
            let err = rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY),
                Some(format!("{broken} broken foreign key(s) after migrating")),
            );
            return Err((last, err));
        }
    }
    // `user_version` is part of the database header and is covered by the transaction.
    tx.pragma_update(None, "user_version", last.version)
        .map_err(|err| (last, err))?;
    tx.commit().map_err(|err| (last, err))
}

/// Structural description of a database schema, independent of DDL formatting:
/// object name → description.
///
/// Tables are described by their columns (name, type, not null, default,
/// primary key), foreign keys and `WITHOUT ROWID`; indexes by their table and
/// columns; virtual tables by their `USING …` clause with whitespace collapsed.
/// Comments and formatting do not matter. This is a port of `schema_shape` in
/// `siyak-data/pipeline/schema.py`, so both repositories compare schemas the
/// same way (e.g. `core.db` against these migrations).
pub fn schema_shape(conn: &Connection) -> AppResult<BTreeMap<String, String>> {
    let objects = conn
        .prepare(
            "SELECT type, name, tbl_name, coalesce(sql, '') FROM sqlite_master
             WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut shape = BTreeMap::new();
    for (kind, name, table, sql) in objects {
        let upper = sql.to_uppercase();
        if kind == "table" && upper.starts_with("CREATE VIRTUAL") {
            let body = upper.find("USING").map_or("", |at| &sql[at..]);
            let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
            shape.insert(format!("virtual {name}"), body);
        } else if kind == "table" {
            let columns = rows_of(
                conn,
                "SELECT name, type, \"notnull\", dflt_value, pk FROM pragma_table_info(?1)",
                &name,
            )?;
            let foreign_keys = rows_of(
                conn,
                "SELECT \"table\", \"from\", \"to\", on_update, on_delete, \"match\"
                 FROM pragma_foreign_key_list(?1) ORDER BY id, seq",
                &name,
            )?;
            let without_rowid = upper.contains("WITHOUT ROWID");
            shape.insert(
                format!("table {name}"),
                format!("columns {columns:?}; foreign keys {foreign_keys:?}; without rowid {without_rowid}"),
            );
        } else if kind == "index" {
            let columns = rows_of(
                conn,
                "SELECT name FROM pragma_index_info(?1) ORDER BY seqno",
                &name,
            )?;
            shape.insert(format!("index {name}"), format!("on {table} {columns:?}"));
        }
    }
    Ok(shape)
}

/// All rows of a single-parameter query, each as a list of values.
fn rows_of(conn: &Connection, sql: &str, param: &str) -> AppResult<Vec<Vec<Value>>> {
    let mut stmt = conn.prepare(sql)?;
    let width = stmt.column_count();
    let rows = stmt.query_map([param], |row| {
        (0..width).map(|i| row.get::<_, Value>(i)).collect()
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Copies the database behind `conn` to `dest` with SQLite's online backup
/// API rather than a file copy, so the backup also contains changes still
/// sitting in the write-ahead log.
pub fn backup(conn: &Connection, dest: &Path) -> AppResult<()> {
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

    const GOOD: &[Migration] = &[
        Migration {
            version: 1,
            name: "one",
            sql: "CREATE TABLE a (x INTEGER);",
            foreign_keys_off: false,
        },
        Migration {
            version: 2,
            name: "two",
            sql: "CREATE TABLE b (x INTEGER);",
            foreign_keys_off: false,
        },
    ];

    pub(crate) fn assert_contiguous(migrations: &[Migration]) {
        for (i, m) in migrations.iter().enumerate() {
            assert_eq!(
                m.version as usize,
                i + 1,
                "migration {} out of order",
                m.name
            );
        }
    }

    #[test]
    fn applies_in_order_and_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, GOOD).unwrap(), 2);
        assert_eq!(schema_version(&conn).unwrap(), 2);
        assert_eq!(migrate(&mut conn, GOOD).unwrap(), 0);
        assert_eq!(schema_version(&conn).unwrap(), 2);
    }

    #[test]
    fn failed_migration_rolls_back_everything() {
        let broken = &[
            GOOD[0],
            Migration {
                version: 2,
                name: "broken",
                sql: "CREATE TABLE oops (;",
                foreign_keys_off: false,
            },
        ];
        let mut conn = Connection::open_in_memory().unwrap();
        let err = migrate(&mut conn, broken).unwrap_err();
        assert_eq!(err.code, AppErrorCode::MigrationFailed);
        assert_eq!(schema_version(&conn).unwrap(), 0);
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }

    /// A table rebuild with foreign keys off keeps the rows that reference it, and
    /// the connection's foreign keys are on again afterwards.
    #[test]
    fn rebuild_with_foreign_keys_off_keeps_child_rows() {
        let parent = Migration {
            version: 1,
            name: "parent",
            sql: "CREATE TABLE p (id INTEGER PRIMARY KEY);
                  CREATE TABLE c (p INTEGER REFERENCES p(id) ON DELETE CASCADE);
                  INSERT INTO p VALUES (1); INSERT INTO c VALUES (1);",
            foreign_keys_off: false,
        };
        let rebuild = Migration {
            version: 2,
            name: "rebuild",
            sql: "CREATE TABLE p2 (id INTEGER PRIMARY KEY, x TEXT);
                  INSERT INTO p2 (id) SELECT id FROM p;
                  DROP TABLE p; ALTER TABLE p2 RENAME TO p;",
            foreign_keys_off: true,
        };
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, &[parent]).unwrap();
        migrate(&mut conn, &[parent, rebuild]).unwrap();
        let children: i64 = conn
            .query_row("SELECT count(*) FROM c", [], |r| r.get(0))
            .unwrap();
        assert_eq!(children, 1);
        let keys: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert!(keys);
    }

    /// A rebuild that leaves a reference dangling is rolled back.
    #[test]
    fn rebuild_that_breaks_a_foreign_key_rolls_back() {
        let parent = Migration {
            version: 1,
            name: "parent",
            sql: "CREATE TABLE p (id INTEGER PRIMARY KEY);
                  CREATE TABLE c (p INTEGER REFERENCES p(id));
                  INSERT INTO p VALUES (1); INSERT INTO c VALUES (1);",
            foreign_keys_off: false,
        };
        let lossy = Migration {
            version: 2,
            name: "lossy",
            sql: "CREATE TABLE p2 (id INTEGER PRIMARY KEY);
                  DROP TABLE p; ALTER TABLE p2 RENAME TO p;",
            foreign_keys_off: true,
        };
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, &[parent]).unwrap();
        let err = migrate(&mut conn, &[parent, lossy]).unwrap_err();
        assert_eq!(err.code, AppErrorCode::MigrationFailed);
        assert_eq!(schema_version(&conn).unwrap(), 1);
        let parents: i64 = conn
            .query_row("SELECT count(*) FROM p", [], |r| r.get(0))
            .unwrap();
        assert_eq!(parents, 1);
    }

    #[test]
    fn rejects_database_newer_than_app() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        let err = migrate(&mut conn, GOOD).unwrap_err();
        assert_eq!(err.code, AppErrorCode::DbVersionUnsupported);
    }
}
