//! Installs a verified pack: SQLite packs via `ATTACH` and `INSERT … SELECT`
//! of expected columns only, inside one transaction; archive packs also move
//! their extra files to `<app_data>/packs/<id>/` (spec §6.4 step 6). No SQL
//! from a pack is ever executed.
//!
//! This is a port of `siyak-data/pipeline/install.py`, the reference
//! implementation (`docs/pack-format.md` §4). Differences, all stricter or
//! about reinstalling: the pack is opened read-only and `quick_check`ed, its
//! `pack_info.version` must match the manifest, and a reinstalled tafsir keeps
//! the ids of passages that start at the same verse with the same kind, so
//! cached translations in `user.db` keep pointing at the same passage.
//!
//! Archive files are staged next to their final directory before the database
//! transaction starts and swapped in while it is still open: if the swap
//! fails the transaction rolls back, and if the commit fails the swap is
//! undone. A crash between the two is repaired on the next start by
//! [`cleanup`], which compares each directory's `pack.json` with `packs.version`.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use tokio_util::sync::CancellationToken;

use super::download::Progress;
use super::manifest::{PackEntry, PackKind, Phase};
use crate::db::content::MIGRATIONS as CONTENT_MIGRATIONS;
use crate::error::{AppError, AppErrorCode, AppResult};

/// Tables (and their columns) a pack of each type may contain besides `pack_info`.
pub fn pack_tables(kind: PackKind) -> &'static [(&'static str, &'static [&'static str])] {
    match kind {
        PackKind::Translation => &[("translations", &["pack_id", "verse_id", "text"])],
        PackKind::Tafsir => &[
            (
                "tafsir",
                &["id", "pack_id", "verse_from", "verse_to", "kind"],
            ),
            (
                "tafsir_paragraphs",
                &["tafsir_id", "idx", "text", "part", "page"],
            ),
        ],
        PackKind::Morphology => &[
            (
                "words",
                &[
                    "id", "verse_id", "position", "text_ar", "translit", "gloss_en",
                ],
            ),
            ("roots", &["id", "root"]),
            ("lemmas", &["id", "lemma", "root_id"]),
            (
                "word_segments",
                &[
                    "word_id", "idx", "kind", "text_ar", "pos", "lemma_id", "root_id", "features",
                ],
            ),
        ],
    }
}

/// Columns of `pack_info`.
pub const PACK_INFO_COLUMNS: &[&str] = &[
    "id",
    "type",
    "version",
    "schema_version",
    "built_at",
    "source_version",
];

/// Archive packs: the database member and every member the archive must hold.
pub fn archive_layout(kind: PackKind) -> Option<(&'static str, &'static [&'static str])> {
    match kind {
        PackKind::Morphology => Some((
            "morphology.sqlite",
            &[
                "morphology.sqlite",
                "qac/COPYING",
                "qac/NOTICE",
                "qac/quranic-corpus-morphology-0.4.txt",
            ],
        )),
        PackKind::Translation | PackKind::Tafsir => None,
    }
}

/// The member of an archive pack shown in full on About → Sources.
pub fn notice_member(kind: PackKind) -> Option<&'static str> {
    match kind {
        PackKind::Morphology => Some("qac/NOTICE"),
        PackKind::Translation | PackKind::Tafsir => None,
    }
}

/// Insert order that satisfies the foreign keys (roots ← lemmas, words ← segments).
const MORPHOLOGY_ORDER: &[&str] = &["roots", "lemmas", "words", "word_segments"];

/// The content schema version packs must be built for (pack-format §2).
pub fn schema_version() -> u32 {
    CONTENT_MIGRATIONS.len() as u32
}

/// Marker file in an archive pack's directory: which version the files belong to.
const MARKER: &str = "pack.json";

/// Directory of the archive files of installed packs.
pub fn packs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("packs")
}

/// Directory for temporary files while installing.
pub fn tmp_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tmp")
}

/// Directory of downloads.
pub fn downloads_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("downloads")
}

/// What an installation changed besides the pack's own rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Installed {
    /// The morphology pack this one replaced.
    pub replaced: Option<String>,
}

/// Opens a connection to `content.db` for pack operations: the app's usual
/// settings, plus `trusted_schema = OFF` so nothing in an attached pack's
/// schema can call functions, and a busy timeout because the app's reader
/// connection is open at the same time.
pub fn open_content(path: &Path) -> AppResult<Connection> {
    let conn = crate::db::connect(path)?;
    conn.execute_batch("PRAGMA trusted_schema = OFF;")?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    Ok(conn)
}

/// Installs the verified pack file `file` described by `entry` into `conn`.
///
/// `data_dir` holds `tmp/` and `packs/`. Any error leaves `content.db` and
/// the pack directories as they were. This is the seam for a future
/// "install from file": the queue is only one caller.
pub fn install_file(
    conn: &mut Connection,
    file: &Path,
    entry: &PackEntry,
    data_dir: &Path,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<Installed> {
    let kind = entry.pack_kind().ok_or_else(|| {
        AppError::new(
            AppErrorCode::PackUnsupported,
            format!("{}: type {} cannot be installed", entry.id, entry.kind),
        )
    })?;
    // A bundled pack's rows come from core.db; the upsert would also clear its
    // `bundled` flag and make it removable.
    let bundled: Option<bool> = conn
        .query_row(
            "SELECT bundled FROM packs WHERE id = ?1",
            [&entry.id],
            |r| r.get(0),
        )
        .optional()?;
    if bundled == Some(true) {
        return Err(AppError::new(
            AppErrorCode::PackBundled,
            format!("{} ships with the app", entry.id),
        ));
    }
    let work = tmp_dir(data_dir).join(format!("{}-{}", entry.id, entry.version));
    remove_dir_if_exists(&work)?;
    fs::create_dir_all(&work).map_err(io_failed)?;
    let result = install_in(conn, file, entry, kind, data_dir, &work, cancel, progress);
    let _ = fs::remove_dir_all(&work);
    result
}

#[expect(clippy::too_many_arguments, reason = "internal step of install_file")]
fn install_in(
    conn: &mut Connection,
    file: &Path,
    entry: &PackEntry,
    kind: PackKind,
    data_dir: &Path,
    work: &Path,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<Installed> {
    let on_bytes = |done: u64| progress(Phase::Install, done, entry.installed_size);
    let (db, extra) = match archive_layout(kind) {
        None => {
            let db = work.join("pack.sqlite");
            decompress(file, &db, entry.installed_size, cancel, &on_bytes)?;
            (db, Vec::new())
        }
        Some((db_member, members)) => {
            let files = work.join("files");
            extract_archive(
                file,
                &files,
                members,
                entry.installed_size,
                cancel,
                &on_bytes,
            )?;
            let extra: Vec<&str> = members
                .iter()
                .copied()
                .filter(|m| *m != db_member)
                .collect();
            (files.join(db_member), extra)
        }
    };
    check_cancel(cancel)?;

    let staged = if extra.is_empty() {
        None
    } else {
        Some(stage_files(data_dir, &work.join("files"), &extra, entry)?)
    };

    conn.execute("ATTACH DATABASE ?1 AS pack", [sqlite_uri(&db)])
        .map_err(|err| invalid(entry, format!("cannot open the pack: {err}")))?;
    let result = check_and_copy(conn, entry, kind, data_dir, staged.as_deref(), cancel);
    // Detaching fails only while a statement is still running; there is none.
    let _ = conn.execute("DETACH DATABASE pack", []);
    if let Some(staged) = &staged {
        let _ = fs::remove_dir_all(staged);
    }
    let installed = result?;
    progress(Phase::Install, entry.installed_size, entry.installed_size);
    Ok(installed)
}

fn check_and_copy(
    conn: &mut Connection,
    entry: &PackEntry,
    kind: PackKind,
    data_dir: &Path,
    staged: Option<&Path>,
    cancel: &CancellationToken,
) -> AppResult<Installed> {
    let check: String = conn
        .query_row("PRAGMA pack.quick_check", [], |r| r.get(0))
        .map_err(|err| invalid(entry, format!("quick_check: {err}")))?;
    if check != "ok" {
        return Err(invalid(entry, format!("quick_check: {check}")));
    }
    check_pack_structure(conn, kind).map_err(|msg| invalid(entry, msg))?;
    let source_version = check_pack_info(conn, entry, kind)?;

    let tx = conn.transaction()?;
    upsert_packs_row(&tx, entry, &source_version)?;
    let replaced = copy_rows(&tx, &entry.id, kind)?;
    check_cancel(cancel)?;

    let Some(staged) = staged else {
        tx.commit()?;
        return Ok(Installed { replaced });
    };
    // Swap the staged files in while the transaction is still open.
    let packs = packs_dir(data_dir);
    let target = packs.join(&entry.id);
    let old = packs.join(format!(".old-{}", entry.id));
    remove_dir_if_exists(&old)?;
    let had_old = target.exists();
    if had_old {
        fs::rename(&target, &old).map_err(io_failed)?;
    }
    if let Err(err) = fs::rename(staged, &target) {
        if had_old {
            let _ = fs::rename(&old, &target);
        }
        return Err(io_failed(err));
    }
    if let Err(err) = tx.commit() {
        let _ = fs::remove_dir_all(&target);
        if had_old {
            let _ = fs::rename(&old, &target);
        }
        return Err(err.into());
    }
    let _ = fs::remove_dir_all(&old);
    if let Some(previous) = &replaced {
        let _ = fs::remove_dir_all(packs.join(previous));
    }
    Ok(Installed { replaced })
}

/// Fails unless the attached pack holds only allowed tables/columns and
/// plain indexes (`check_pack_structure` in `install.py`).
fn check_pack_structure(conn: &Connection, kind: PackKind) -> Result<(), String> {
    let mut expected: Vec<(&str, &[&str])> = vec![("pack_info", PACK_INFO_COLUMNS)];
    expected.extend_from_slice(pack_tables(kind));
    let mut stmt = conn
        .prepare("SELECT type, name, tbl_name, sql FROM pack.sqlite_master")
        .map_err(|e| e.to_string())?;
    let objects: Vec<(String, String, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .and_then(|rows| rows.collect())
        .map_err(|e| e.to_string())?;
    for (kind, name, table, sql) in &objects {
        if kind != "table" && kind != "index" {
            return Err(format!(
                "pack contains a {kind} '{name}'; only tables and indexes allowed"
            ));
        }
        if kind == "table"
            && sql
                .as_deref()
                .unwrap_or_default()
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("CREATE VIRTUAL")
        {
            return Err(format!("pack contains a virtual table '{name}'"));
        }
        if !expected.iter().any(|(t, _)| t == table) {
            return Err(format!("pack contains unexpected table '{table}'"));
        }
    }
    let present: Vec<&str> = objects
        .iter()
        .filter(|(kind, ..)| kind == "table")
        .map(|(_, name, ..)| name.as_str())
        .collect();
    let missing: Vec<&str> = expected
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| !present.contains(t))
        .collect();
    if !missing.is_empty() {
        return Err(format!("pack is missing table(s): {}", missing.join(", ")));
    }
    for (table, columns) in expected {
        let mut stmt = conn
            .prepare(&format!("PRAGMA pack.table_info({table})"))
            .map_err(|e| e.to_string())?;
        let actual: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .and_then(|rows| rows.collect())
            .map_err(|e| e.to_string())?;
        if actual != columns {
            return Err(format!(
                "pack table '{table}' has columns {actual:?}, expected {columns:?}"
            ));
        }
    }
    Ok(())
}

/// Checks `pack_info` against the manifest entry; returns its `source_version`.
fn check_pack_info(conn: &Connection, entry: &PackEntry, kind: PackKind) -> AppResult<String> {
    let mut stmt = conn.prepare(
        "SELECT id, type, version, schema_version, built_at, source_version FROM pack.pack_info",
    )?;
    let rows: Vec<Vec<Value>> = stmt
        .query_map([], |r| (0..6).map(|i| r.get::<_, Value>(i)).collect())?
        .collect::<Result<_, _>>()?;
    let [row] = rows.as_slice() else {
        return Err(invalid(
            entry,
            format!("pack_info must have exactly one row, found {}", rows.len()),
        ));
    };
    let text = |i: usize| match &row[i] {
        Value::Text(s) => Ok(s.clone()),
        other => Err(invalid(
            entry,
            format!("pack_info.{} is {other:?}", PACK_INFO_COLUMNS[i]),
        )),
    };
    let (id, pack_type, version, source_version) = (text(0)?, text(1)?, text(2)?, text(5)?);
    if (id.as_str(), pack_type.as_str()) != (entry.id.as_str(), kind.as_str()) {
        return Err(invalid(
            entry,
            format!(
                "pack_info says {id}/{pack_type}, expected {}/{}",
                entry.id,
                kind.as_str()
            ),
        ));
    }
    if version != entry.version {
        return Err(invalid(
            entry,
            format!("pack_info version {version}, manifest {}", entry.version),
        ));
    }
    let Value::Integer(schema) = row[3] else {
        return Err(invalid(entry, "pack_info.schema_version is not an integer"));
    };
    if schema != i64::from(schema_version()) {
        return Err(AppError::new(
            AppErrorCode::PackSchemaUnsupported,
            format!(
                "{}: schema_version {schema}, expected {}",
                entry.id,
                schema_version()
            ),
        ));
    }
    Ok(source_version)
}

/// Writes the `packs` row first, with `ON CONFLICT DO UPDATE` (never
/// `INSERT OR REPLACE`: REPLACE deletes the old row and, with foreign keys
/// on, cascades to the rows just copied; pack-format §4.2).
fn upsert_packs_row(
    tx: &Transaction<'_>,
    entry: &PackEntry,
    source_version: &str,
) -> AppResult<()> {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "title".into(),
        serde_json::json!({ "tr": entry.title.tr, "en": entry.title.en }),
    );
    meta.insert(
        "license_terms".into(),
        serde_json::to_value(&entry.license_terms).unwrap_or_default(),
    );
    meta.insert("source_version".into(), source_version.into());
    for (key, value) in entry.meta.iter().flatten() {
        meta.insert(key.clone(), value.clone());
    }
    if let Some(description) = &entry.description {
        meta.insert(
            "description".into(),
            serde_json::json!({ "tr": description.tr, "en": description.en }),
        );
    }
    tx.execute(
        "INSERT INTO packs (id, type, lang, title, author, version, license, attribution,
                            citation_json, meta_json, installed_at, bundled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 0)
         ON CONFLICT (id) DO UPDATE SET
           type = excluded.type, lang = excluded.lang, title = excluded.title,
           author = excluded.author, version = excluded.version, license = excluded.license,
           attribution = excluded.attribution, citation_json = excluded.citation_json,
           meta_json = excluded.meta_json, installed_at = excluded.installed_at,
           bundled = excluded.bundled",
        params![
            entry.id,
            entry.kind,
            entry.lang,
            entry.title.en,
            entry.author,
            entry.version,
            entry.license,
            entry.attribution,
            entry.citation.as_ref().map(python_json),
            python_json(&serde_json::Value::Object(meta)),
        ],
    )?;
    Ok(())
}

/// Copies the pack's rows; returns the morphology pack it replaced, if any.
fn copy_rows(tx: &Transaction<'_>, pack_id: &str, kind: PackKind) -> AppResult<Option<String>> {
    match kind {
        PackKind::Translation => {
            // pack_id is taken from the manifest, never from the pack's rows.
            tx.execute("DELETE FROM translations_fts WHERE pack_id = ?1", [pack_id])?;
            tx.execute("DELETE FROM translations WHERE pack_id = ?1", [pack_id])?;
            tx.execute(
                "INSERT INTO translations (pack_id, verse_id, text)
                 SELECT ?1, verse_id, text FROM pack.translations",
                [pack_id],
            )?;
            tx.execute(
                "INSERT INTO translations_fts (text, pack_id, verse_id)
                 SELECT text, pack_id, verse_id FROM translations WHERE pack_id = ?1",
                [pack_id],
            )?;
            Ok(None)
        }
        PackKind::Tafsir => {
            copy_tafsir(tx, pack_id)?;
            Ok(None)
        }
        PackKind::Morphology => {
            // v1 keeps one morphology pack at a time: the morphology tables have
            // no pack_id, so installing one replaces all morphology data (§4.4).
            let replaced: Option<String> = tx
                .query_row(
                    "SELECT id FROM packs WHERE type = 'morphology' AND id != ?1",
                    [pack_id],
                    |r| r.get(0),
                )
                .optional()?;
            tx.execute(
                "DELETE FROM packs WHERE type = 'morphology' AND id != ?1",
                [pack_id],
            )?;
            for table in MORPHOLOGY_ORDER.iter().rev() {
                tx.execute(&format!("DELETE FROM main.{table}"), [])?;
            }
            for table in MORPHOLOGY_ORDER {
                let names = column_list(PackKind::Morphology, table);
                tx.execute(
                    &format!("INSERT INTO main.{table} ({names}) SELECT {names} FROM pack.{table}"),
                    [],
                )?;
            }
            Ok(replaced)
        }
    }
}

fn column_list(kind: PackKind, table: &str) -> String {
    pack_tables(kind)
        .iter()
        .find(|(t, _)| *t == table)
        .map(|(_, columns)| columns.join(", "))
        .unwrap_or_default()
}

/// Copies a tafsir pack, giving its passages ids in content.db
/// (`_copy_tafsir` in `install.py`). `tafsir.id` is global, so the pack's ids
/// cannot be kept; `(verse_from, kind)` identifies a passage within a pack.
/// On a reinstall, passages with the same `(verse_from, kind)` keep their
/// previous id; the others get new ids in pack order, as on a first install.
fn copy_tafsir(tx: &Transaction<'_>, pack_id: &str) -> AppResult<()> {
    let duplicates: i64 = tx.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM pack.tafsir GROUP BY verse_from, kind HAVING count(*) > 1)",
        [],
        |r| r.get(0),
    )?;
    if duplicates > 0 {
        return Err(AppError::new(
            AppErrorCode::PackInvalid,
            format!("tafsir pack '{pack_id}' has two passages with the same start and kind"),
        ));
    }
    tx.execute_batch(
        "DROP TABLE IF EXISTS temp.tafsir_keep;
         CREATE TEMP TABLE tafsir_keep (verse_from INTEGER, kind TEXT, id INTEGER);",
    )?;
    tx.execute(
        "INSERT INTO temp.tafsir_keep SELECT verse_from, kind, id FROM main.tafsir WHERE pack_id = ?1",
        [pack_id],
    )?;
    tx.execute(
        "DELETE FROM tafsir_paragraphs WHERE tafsir_id IN (SELECT id FROM tafsir WHERE pack_id = ?1)",
        [pack_id],
    )?;
    tx.execute("DELETE FROM tafsir WHERE pack_id = ?1", [pack_id])?;
    tx.execute(
        "INSERT INTO tafsir (id, pack_id, verse_from, verse_to, kind)
         SELECT k.id, ?1, p.verse_from, p.verse_to, p.kind FROM pack.tafsir p
         JOIN temp.tafsir_keep k ON k.verse_from = p.verse_from AND k.kind = p.kind
         ORDER BY p.id",
        [pack_id],
    )?;
    tx.execute(
        "INSERT INTO tafsir (pack_id, verse_from, verse_to, kind)
         SELECT ?1, verse_from, verse_to, kind FROM pack.tafsir p
         WHERE NOT EXISTS (SELECT 1 FROM temp.tafsir_keep k
                           WHERE k.verse_from = p.verse_from AND k.kind = p.kind)
         ORDER BY id",
        [pack_id],
    )?;
    tx.execute(
        "INSERT INTO tafsir_paragraphs (tafsir_id, idx, text, part, page)
         SELECT t.id, p.idx, p.text, p.part, p.page FROM pack.tafsir_paragraphs p
         JOIN pack.tafsir pt ON pt.id = p.tafsir_id
         JOIN main.tafsir t ON t.pack_id = ?1 AND t.verse_from = pt.verse_from AND t.kind = pt.kind",
        [pack_id],
    )?;
    tx.execute_batch("DROP TABLE temp.tafsir_keep;")?;
    Ok(())
}

/// Removes an installed pack (`bundled = 0` only) and its files. Returns
/// whether a `VACUUM` is worth suggesting (it always is after a removal).
pub fn uninstall(conn: &mut Connection, data_dir: &Path, pack_id: &str) -> AppResult<()> {
    let (kind, bundled): (String, bool) = conn.query_row(
        "SELECT type, bundled FROM packs WHERE id = ?1",
        [pack_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if bundled {
        return Err(AppError::new(
            AppErrorCode::PackBundled,
            format!("{pack_id} ships with the app"),
        ));
    }
    let tx = conn.transaction()?;
    match PackKind::parse(&kind) {
        Some(PackKind::Translation) => {
            tx.execute("DELETE FROM translations_fts WHERE pack_id = ?1", [pack_id])?;
        }
        Some(PackKind::Morphology) => {
            for table in MORPHOLOGY_ORDER.iter().rev() {
                tx.execute(&format!("DELETE FROM main.{table}"), [])?;
            }
        }
        // Tafsir rows and paragraphs go with the packs row (ON DELETE CASCADE).
        Some(PackKind::Tafsir) | None => {}
    }
    tx.execute("DELETE FROM packs WHERE id = ?1", [pack_id])?;
    tx.commit()?;
    remove_dir_if_exists(&packs_dir(data_dir).join(pack_id))?;
    Ok(())
}

/// Sizes of `content.db` before and after [`vacuum`], in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VacuumResult {
    /// Size of the database file and its WAL before.
    pub before: u64,
    /// Size after.
    pub after: u64,
}

/// Compacts `content.db` after packs were removed and truncates its WAL so
/// the file really shrinks. Slow for large databases; run it off the main thread.
pub fn vacuum(content_path: &Path) -> AppResult<VacuumResult> {
    let size = || {
        let wal = content_path.with_extension("db-wal");
        file_len(content_path) + file_len(&wal)
    };
    let before = size();
    let conn = open_content(content_path)?;
    conn.execute_batch("VACUUM;")?;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    drop(conn);
    Ok(VacuumResult {
        before,
        after: size(),
    })
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Startup cleanup: deletes leftover downloads and temporary files, and
/// repairs `packs/` after an interrupted archive install. Directories whose
/// `pack.json` version matches `packs.version` are kept; after a crash
/// between the directory swap and the commit, the previous directory is put
/// back; directories of packs that are not installed are removed.
pub fn cleanup(conn: &Connection, data_dir: &Path) -> AppResult<()> {
    for dir in [tmp_dir(data_dir), downloads_dir(data_dir)] {
        remove_dir_if_exists(&dir)?;
    }
    let packs = packs_dir(data_dir);
    let Ok(entries) = fs::read_dir(&packs) else {
        return Ok(());
    };
    let installed: HashMap<String, String> = conn
        .prepare("SELECT id, version FROM packs WHERE bundled = 0")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    // `.old-` entries sort first, so they are restored before orphans are checked.
    names.sort();
    for name in names {
        let path = packs.join(&name);
        if name.starts_with(".staging-") {
            remove_dir_if_exists(&path)?;
        } else if let Some(id) = name.strip_prefix(".old-") {
            let target = packs.join(id);
            let wanted = installed.get(id);
            let current_ok = wanted.is_some_and(|v| marker_version(&target).as_ref() == Some(v));
            let old_ok = wanted.is_some_and(|v| marker_version(&path).as_ref() == Some(v));
            if !current_ok && old_ok {
                remove_dir_if_exists(&target)?;
                fs::rename(&path, &target).map_err(io_failed)?;
            } else {
                remove_dir_if_exists(&path)?;
            }
        } else if !name.starts_with('.') && !installed.contains_key(&name) {
            remove_dir_if_exists(&path)?;
        }
    }
    Ok(())
}

fn marker_version(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join(MARKER)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value["version"].as_str().map(str::to_owned)
}

/// Moves the extracted extra files to `packs/.staging-<id>/` (same volume as
/// their final place) with a marker.
fn stage_files(
    data_dir: &Path,
    files: &Path,
    extra: &[&str],
    entry: &PackEntry,
) -> AppResult<PathBuf> {
    let staged = packs_dir(data_dir).join(format!(".staging-{}", entry.id));
    remove_dir_if_exists(&staged)?;
    fs::create_dir_all(&staged).map_err(io_failed)?;
    for name in extra {
        let to = staged.join(name);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(io_failed)?;
        }
        move_file(&files.join(name), &to)?;
    }
    let marker = serde_json::json!({ "id": entry.id, "version": entry.version });
    fs::write(staged.join(MARKER), marker.to_string()).map_err(io_failed)?;
    Ok(staged)
}

/// Renames, or copies when `tmp/` and `packs/` are on different volumes.
fn move_file(from: &Path, to: &Path) -> AppResult<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(io_failed)?;
    let _ = fs::remove_file(from);
    Ok(())
}

/// Reads the decompressed size a zstd frame declares, if any.
fn frame_content_size(path: &Path) -> AppResult<Option<u64>> {
    let mut header = [0u8; 18];
    let mut file = File::open(path).map_err(io_failed)?;
    let n = read_up_to(&mut file, &mut header).map_err(io_failed)?;
    zstd::zstd_safe::get_frame_content_size(&header[..n]).map_err(|_| {
        AppError::new(
            AppErrorCode::PackInvalid,
            format!("{}: not a valid zstd file", path.display()),
        )
    })
}

fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match reader.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}

/// A reader that fails once more than `limit` bytes have been read, and
/// reports progress.
struct Capped<'a, R> {
    inner: R,
    read: u64,
    limit: u64,
    exceeded: Arc<AtomicBool>,
    cancel: &'a CancellationToken,
    on_bytes: &'a dyn Fn(u64),
    reported: u64,
}

impl<R: Read> Read for Capped<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel.is_cancelled() {
            // Not `Interrupted`: `io::copy` retries that kind forever.
            return Err(io::Error::other("cancelled"));
        }
        let n = self.inner.read(buf)?;
        self.read += n as u64;
        if self.read > self.limit {
            self.exceeded.store(true, Ordering::Relaxed);
            return Err(io::Error::other("decompressed size exceeds the limit"));
        }
        if self.read - self.reported >= 1 << 20 || n == 0 {
            self.reported = self.read;
            (self.on_bytes)(self.read);
        }
        Ok(n)
    }
}

/// Stream-decompresses a `.zst` file to `dest`, refusing more than `limit`
/// bytes whether the frame declares them or the stream produces them (§4.3).
fn decompress(
    src: &Path,
    dest: &Path,
    limit: u64,
    cancel: &CancellationToken,
    on_bytes: &dyn Fn(u64),
) -> AppResult<()> {
    let mut reader = capped_decoder(src, limit, cancel, on_bytes)?;
    let mut out = File::create(dest).map_err(io_failed)?;
    let result = io::copy(&mut reader, &mut out).and_then(|_| out.flush());
    result.map_err(|err| reader_error(src, &reader, err, cancel))
}

fn capped_decoder<'a>(
    src: &Path,
    limit: u64,
    cancel: &'a CancellationToken,
    on_bytes: &'a dyn Fn(u64),
) -> AppResult<Capped<'a, zstd::Decoder<'static, io::BufReader<File>>>> {
    if let Some(declared) = frame_content_size(src)?
        && declared > limit
    {
        return Err(AppError::new(
            AppErrorCode::PackTooLarge,
            format!(
                "{}: frame declares {declared} bytes, limit is {limit}",
                src.display()
            ),
        ));
    }
    let decoder = zstd::Decoder::new(File::open(src).map_err(io_failed)?)
        .map_err(io_failed)?
        .single_frame();
    Ok(Capped {
        inner: decoder,
        read: 0,
        limit,
        exceeded: Arc::new(AtomicBool::new(false)),
        cancel,
        on_bytes,
        reported: 0,
    })
}

fn reader_error<R>(
    src: &Path,
    reader: &Capped<'_, R>,
    err: io::Error,
    cancel: &CancellationToken,
) -> AppError {
    if cancel.is_cancelled() {
        AppError::new(AppErrorCode::InstallCancelled, "cancelled")
    } else if reader.exceeded.load(Ordering::Relaxed) {
        AppError::new(
            AppErrorCode::PackTooLarge,
            format!(
                "{}: decompressed size exceeds {} bytes",
                src.display(),
                reader.limit
            ),
        )
    } else {
        AppError::new(
            AppErrorCode::PackInvalid,
            format!("{}: {err}", src.display()),
        )
    }
}

/// Decompresses and unpacks an archive pack into `dest`, accepting only
/// regular files with exactly the `expected` names, each once (§4.4).
/// Output paths are built from the expected names, never from the archive.
fn extract_archive(
    src: &Path,
    dest: &Path,
    expected: &[&str],
    limit: u64,
    cancel: &CancellationToken,
    on_bytes: &dyn Fn(u64),
) -> AppResult<()> {
    let reader = capped_decoder(src, limit, cancel, on_bytes)?;
    let exceeded = Arc::clone(&reader.exceeded);
    let bad = |message: String| {
        AppError::new(
            AppErrorCode::PackInvalid,
            format!("{}: {message}", src.display()),
        )
    };
    let wrap = |err: io::Error| {
        if cancel.is_cancelled() {
            AppError::new(AppErrorCode::InstallCancelled, "cancelled")
        } else if exceeded.load(Ordering::Relaxed) {
            AppError::new(
                AppErrorCode::PackTooLarge,
                format!("{}: decompressed size exceeds {limit} bytes", src.display()),
            )
        } else {
            bad(format!("not a valid tar archive: {err}"))
        }
    };
    let mut archive = tar::Archive::new(reader);
    let mut seen: Vec<&str> = Vec::new();
    for member in archive.entries().map_err(wrap)? {
        let mut member = member.map_err(wrap)?;
        let raw = member.path_bytes().into_owned();
        let name = std::str::from_utf8(&raw)
            .ok()
            .and_then(|n| expected.iter().copied().find(|e| *e == n))
            .ok_or_else(|| {
                bad(format!(
                    "unexpected member {:?}",
                    String::from_utf8_lossy(&raw)
                ))
            })?;
        if !member.header().entry_type().is_file() {
            return Err(bad(format!("member {name:?} is not a regular file")));
        }
        if seen.contains(&name) {
            return Err(bad(format!("member {name:?} appears twice")));
        }
        seen.push(name);
        let to = dest.join(name);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(io_failed)?;
        }
        let mut out = File::create(&to).map_err(io_failed)?;
        io::copy(&mut member, &mut out).map_err(wrap)?;
    }
    if seen.len() != expected.len() {
        let missing: Vec<&str> = expected
            .iter()
            .copied()
            .filter(|e| !seen.contains(e))
            .collect();
        return Err(bad(format!("missing member(s) {missing:?}")));
    }
    Ok(())
}

/// A read-only SQLite URI for `path` (`file:…?mode=ro`), percent-encoded.
fn sqlite_uri(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from("file:");
    if !raw.starts_with('/') {
        uri.push('/');
    }
    for b in raw.bytes() {
        if b.is_ascii_alphanumeric() || b"/._-~:".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri.push_str("?mode=ro");
    uri
}

/// JSON text written like Python's `json.dumps(value, ensure_ascii=False,
/// sort_keys=True)`, so `citation_json` and `meta_json` are byte-identical to
/// the reference installer and to the bundled packs in `core.db`.
pub fn python_json(value: &serde_json::Value) -> String {
    let mut out = String::new();
    write_python_json(value, &mut out);
    out
}

fn write_python_json(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_python_json(item, out);
            }
            out.push(']');
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&serde_json::Value::String(key.clone()).to_string());
                out.push_str(": ");
                write_python_json(&map[key], out);
            }
            out.push('}');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

fn remove_dir_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_dir_all(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(io_failed(err)),
        _ => Ok(()),
    }
}

fn check_cancel(cancel: &CancellationToken) -> AppResult<()> {
    if cancel.is_cancelled() {
        return Err(AppError::new(AppErrorCode::InstallCancelled, "cancelled"));
    }
    Ok(())
}

fn invalid(entry: &PackEntry, message: impl std::fmt::Display) -> AppError {
    AppError::new(
        AppErrorCode::PackInvalid,
        format!("{}: {message}", entry.id),
    )
}

fn io_failed(err: io::Error) -> AppError {
    AppError::new(AppErrorCode::IoFailed, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::test_support::*;

    struct Env {
        dir: tempfile::TempDir,
        conn: Connection,
    }

    impl Env {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("content.db");
            drop(content_db(&path));
            let conn = open_content(&path).unwrap();
            Self { dir, conn }
        }

        fn path(&self, name: &str) -> PathBuf {
            self.dir.path().join(name)
        }

        fn install(&mut self, file: &Path, entry: &PackEntry) -> AppResult<Installed> {
            install_file(
                &mut self.conn,
                file,
                entry,
                self.dir.path(),
                &CancellationToken::new(),
                &|_, _, _| {},
            )
        }

        fn count(&self, sql: &str) -> i64 {
            self.conn.query_row(sql, [], |r| r.get(0)).unwrap()
        }

        /// Builds `<id>.sqlite(.zst)` from a translation database and its entry.
        fn translation(
            &self,
            id: &str,
            version: &str,
            rows: &[(i64, &str)],
        ) -> (PathBuf, PackEntry) {
            let db = self.path(&format!("{id}-{version}.sqlite"));
            let _ = fs::remove_file(&db);
            drop(translation_db(&db, id, version, rows));
            self.finish(&db, id, "translation", version)
        }

        fn tafsir(
            &self,
            id: &str,
            version: &str,
            passages: &[Passage<'_>],
        ) -> (PathBuf, PackEntry) {
            let db = self.path(&format!("{id}-{version}.sqlite"));
            let _ = fs::remove_file(&db);
            drop(tafsir_db(&db, id, version, passages));
            self.finish(&db, id, "tafsir", version)
        }

        fn morphology(&self, id: &str, version: &str, root: &str) -> (PathBuf, PackEntry) {
            let db = self.path(&format!("{id}-{version}.morph.sqlite"));
            let _ = fs::remove_file(&db);
            drop(morphology_db(&db, id, version, root));
            let archive = self.path(&format!("{id}-{version}.tar.zst"));
            morphology_archive(&archive, &db);
            let entry = entry_for(&archive, id, "morphology", version, 10 << 20);
            (archive, entry)
        }

        fn finish(&self, db: &Path, id: &str, kind: &str, version: &str) -> (PathBuf, PackEntry) {
            let zst = compress(db);
            let size = fs::metadata(db).unwrap().len();
            let entry = entry_for(&zst, id, kind, version, size);
            (zst, entry)
        }
    }

    /// The published pack format end to end: a real tafsir from `siyak-data`,
    /// installed into the real `core.db` and read back as the panel reads it.
    #[test]
    fn installs_and_reads_a_real_tafsir_when_present() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = root.join("../../siyak-data/dist");
        let Ok(bytes) = fs::read(dist.join("manifest.json")) else {
            return;
        };
        let manifest = crate::packs::manifest::parse(&bytes).unwrap();
        let entry = manifest.pack("ar.jalalayn").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.db");
        fs::copy(root.join("resources/core.db"), &path).unwrap();
        let mut conn = open_content(&path).unwrap();
        install_file(
            &mut conn,
            &dist.join(entry.file_name()),
            entry,
            dir.path(),
            &CancellationToken::new(),
            &|_, _, _| {},
        )
        .unwrap();

        let info = crate::db::content::list_tafsirs(&conn).unwrap();
        assert_eq!(info[0].id, "ar.jalalayn");
        // 2:2 is verse 9; al-Jalalayn has no commentary on 1:1.
        let passage = crate::db::content::get_tafsir(&conn, 9, "ar.jalalayn")
            .unwrap()
            .unwrap();
        assert!(passage.verse_from.surah == 2 && !passage.paragraphs.is_empty());
        assert_eq!(
            crate::db::content::get_tafsir(&conn, 1, "ar.jalalayn").unwrap(),
            None
        );
        // The compare screen's query over the same pack: 2:1–5.
        let compare = crate::db::content::get_compare(
            &conn,
            8,
            12,
            &["en.pickthall".to_owned()],
            &["ar.jalalayn".to_owned()],
        )
        .unwrap();
        assert_eq!(compare.verses.len(), 5);
        assert!(compare.verses.iter().all(|v| v.translations.len() == 1));
        assert!(!compare.tafsirs[0].passages.is_empty());
    }

    #[test]
    fn refuses_to_overwrite_a_bundled_pack() {
        let mut env = Env::new();
        let before = dump(&env.conn);
        let (file, entry) = env.translation("en.core", "2.0.0", &[(1, "replaced")]);
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackBundled
        );
        assert_eq!(dump(&env.conn), before);
    }

    #[test]
    fn installs_a_translation_with_fts_and_packs_row() {
        let mut env = Env::new();
        let (file, entry) = env.translation("tr.test", "1.0.0", &[(1, "bir"), (8, "sekiz")]);
        env.install(&file, &entry).unwrap();
        assert_eq!(
            env.count("SELECT count(*) FROM translations WHERE pack_id = 'tr.test'"),
            2
        );
        assert_eq!(
            env.count("SELECT count(*) FROM translations_fts WHERE translations_fts MATCH 'sekiz' AND pack_id = 'tr.test'"),
            1
        );
        let (bundled, meta, citation): (bool, String, String) = env
            .conn
            .query_row(
                "SELECT bundled, meta_json, citation_json FROM packs WHERE id = 'tr.test'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(!bundled);
        assert_eq!(
            meta,
            r#"{"description": {"en": "description", "tr": "açıklama"}, "license_terms": {"en": ["term"], "tr": ["koşul"]}, "qac_version": "0.4", "source_version": "src@1", "title": {"en": "tr.test en", "tr": "tr.test tr"}}"#
        );
        assert_eq!(
            citation,
            r#"{"type": "tafsir", "url": "https://example.org", "year": 1}"#
        );
        // Temporary files are gone.
        assert_eq!(fs::read_dir(tmp_dir(env.dir.path())).unwrap().count(), 0);
    }

    #[test]
    fn reinstalling_replaces_rows_instead_of_duplicating() {
        let mut env = Env::new();
        let (file, entry) = env.translation("tr.test", "1.0.0", &[(1, "bir"), (8, "sekiz")]);
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.translation("tr.test", "1.1.0", &[(1, "yeni")]);
        env.install(&file, &entry).unwrap();
        assert_eq!(
            env.count("SELECT count(*) FROM translations WHERE pack_id = 'tr.test'"),
            1
        );
        assert_eq!(
            env.count("SELECT count(*) FROM translations_fts WHERE pack_id = 'tr.test'"),
            1
        );
        let version: String = env
            .conn
            .query_row("SELECT version FROM packs WHERE id = 'tr.test'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, "1.1.0");
        // The bundled pack was not touched.
        assert_eq!(
            env.count("SELECT count(*) FROM translations WHERE pack_id = 'en.core'"),
            20
        );
    }

    const INTRO: &[&str] = &["intro"];
    const P1: &[&str] = &["one a", "one b"];
    const P2: &[&str] = &["two"];
    const P3: &[&str] = &["three"];

    fn passages(env: &Env, pack_id: &str) -> Vec<(i64, i64, String, Vec<String>)> {
        let mut stmt = env
            .conn
            .prepare("SELECT id, verse_from, kind FROM tafsir WHERE pack_id = ?1 ORDER BY id")
            .unwrap();
        let rows: Vec<(i64, i64, String)> = stmt
            .query_map([pack_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        rows.into_iter()
            .map(|(id, from, kind)| {
                let texts: Vec<String> = env
                    .conn
                    .prepare("SELECT text FROM tafsir_paragraphs WHERE tafsir_id = ?1 ORDER BY idx")
                    .unwrap()
                    .query_map([id], |r| r.get(0))
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap();
                (id, from, kind, texts)
            })
            .collect()
    }

    #[test]
    fn renumbers_tafsir_ids_and_keeps_paragraph_links() {
        let mut env = Env::new();
        let (file, entry) = env.tafsir("ar.one", "1.0.0", &[(1, 1, "commentary", P2)]);
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.tafsir(
            "ar.two",
            "1.0.0",
            &[
                (1, 1, "introduction", INTRO),
                (1, 2, "commentary", P1),
                (8, 10, "commentary", P3),
            ],
        );
        env.install(&file, &entry).unwrap();
        let got = passages(&env, "ar.two");
        // Global ids continue after ar.one's passage, in pack order.
        assert_eq!(
            got.iter()
                .map(|p| (p.0, p.1, p.2.as_str()))
                .collect::<Vec<_>>(),
            [
                (2, 1, "introduction"),
                (3, 1, "commentary"),
                (4, 8, "commentary")
            ]
        );
        assert_eq!(got[0].3, ["intro"]);
        assert_eq!(got[1].3, ["one a", "one b"]);
        assert_eq!(got[2].3, ["three"]);
        assert_eq!(
            env.count("SELECT count(*) FROM pragma_foreign_key_check"),
            0
        );
    }

    #[test]
    fn reinstalling_a_tafsir_keeps_the_ids_of_unchanged_passages() {
        let mut env = Env::new();
        let (file, entry) = env.tafsir(
            "ar.two",
            "1.0.0",
            &[(1, 2, "commentary", P1), (8, 10, "commentary", P3)],
        );
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.tafsir("ar.other", "1.0.0", &[(1, 1, "commentary", P2)]);
        env.install(&file, &entry).unwrap();
        // The update splits a passage and adds an introduction.
        let (file, entry) = env.tafsir(
            "ar.two",
            "1.1.0",
            &[
                (1, 1, "introduction", INTRO),
                (1, 1, "commentary", P1),
                (2, 2, "commentary", P2),
                (8, 10, "commentary", P3),
            ],
        );
        env.install(&file, &entry).unwrap();
        let got = passages(&env, "ar.two");
        assert_eq!(
            got.iter()
                .map(|p| (p.0, p.1, p.2.as_str()))
                .collect::<Vec<_>>(),
            [
                (1, 1, "commentary"),
                (2, 8, "commentary"),
                (4, 1, "introduction"),
                (5, 2, "commentary")
            ]
        );
        assert_eq!(got[0].3, ["one a", "one b"]);
        assert_eq!(got[1].3, ["three"]);
        assert_eq!(got[2].3, ["intro"]);
        assert_eq!(got[3].3, ["two"]);
        assert_eq!(
            env.count("SELECT count(*) FROM pragma_foreign_key_check"),
            0
        );
    }

    #[test]
    fn refuses_tafsir_with_duplicate_passage_starts() {
        let mut env = Env::new();
        let (file, entry) = env.tafsir(
            "ar.dup",
            "1.0.0",
            &[(1, 1, "commentary", P1), (1, 2, "commentary", P2)],
        );
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackInvalid
        );
    }

    #[test]
    fn installs_a_morphology_archive_and_its_files() {
        let mut env = Env::new();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        let installed = env.install(&file, &entry).unwrap();
        assert_eq!(installed.replaced, None);
        assert_eq!(env.count("SELECT count(*) FROM words"), 2);
        assert_eq!(env.count("SELECT count(*) FROM word_segments"), 2);
        let dir = packs_dir(env.dir.path()).join("morphology.qac");
        assert_eq!(fs::read(dir.join("qac/NOTICE")).unwrap(), b"QAC notice");
        assert!(dir.join("qac/quranic-corpus-morphology-0.4.txt").is_file());
        assert!(!dir.join("morphology.sqlite").exists());
        assert_eq!(marker_version(&dir).as_deref(), Some("1.0.0"));
        assert_eq!(fs::read_dir(packs_dir(env.dir.path())).unwrap().count(), 1);
    }

    #[test]
    fn a_second_morphology_pack_replaces_the_first() {
        let mut env = Env::new();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.morphology("morphology.masaq", "1.0.0", "ك ت ب");
        let installed = env.install(&file, &entry).unwrap();
        assert_eq!(installed.replaced.as_deref(), Some("morphology.qac"));
        assert_eq!(
            env.count("SELECT count(*) FROM packs WHERE type = 'morphology'"),
            1
        );
        assert_eq!(
            env.count("SELECT count(*) FROM roots WHERE root = 'ك ت ب'"),
            1
        );
        assert_eq!(env.count("SELECT count(*) FROM roots"), 1);
        let packs = packs_dir(env.dir.path());
        assert!(!packs.join("morphology.qac").exists());
        assert!(packs.join("morphology.masaq/qac/NOTICE").is_file());
    }

    #[test]
    fn updating_an_archive_pack_swaps_its_directory() {
        let mut env = Env::new();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.morphology("morphology.qac", "1.0.1", "ق و ل");
        env.install(&file, &entry).unwrap();
        let packs = packs_dir(env.dir.path());
        assert_eq!(
            marker_version(&packs.join("morphology.qac")).as_deref(),
            Some("1.0.1")
        );
        assert_eq!(
            fs::read_dir(&packs).unwrap().count(),
            1,
            "no .old or .staging left"
        );
    }

    #[test]
    fn a_failed_directory_swap_rolls_the_database_back() {
        let mut env = Env::new();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        env.install(&file, &entry).unwrap();
        let before = dump(&env.conn);
        // A file where the swap wants to put the old directory makes it fail.
        let packs = packs_dir(env.dir.path());
        fs::write(packs.join(".old-morphology.qac"), b"in the way").unwrap();
        let (file, entry) = env.morphology("morphology.qac", "1.0.1", "ك ت ب");
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::IoFailed
        );
        assert_eq!(dump(&env.conn), before);
        assert_eq!(
            marker_version(&packs.join("morphology.qac")).as_deref(),
            Some("1.0.0")
        );
        assert!(!packs.join(".staging-morphology.qac").exists());
    }

    #[test]
    fn a_failed_copy_leaves_content_db_unchanged() {
        let mut env = Env::new();
        let (file, entry) = env.translation("tr.test", "1.0.0", &[(1, "bir")]);
        env.install(&file, &entry).unwrap();
        let before = dump(&env.conn);
        // Verse 999 does not exist: the foreign key fails half-way through.
        let (file, entry) = env.translation("tr.test", "1.1.0", &[(2, "iki"), (999, "yok")]);
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::QueryFailed
        );
        assert_eq!(dump(&env.conn), before);
        assert_eq!(
            env.count("SELECT count(*) FROM pragma_database_list WHERE name = 'pack'"),
            0
        );
    }

    fn malicious_translation(env: &Env, extra_sql: &str) -> (PathBuf, PackEntry) {
        let db = env.path("bad.sqlite");
        let _ = fs::remove_file(&db);
        let conn = translation_db(&db, "tr.bad", "1.0.0", &[(1, "bir")]);
        conn.execute_batch(extra_sql).unwrap();
        drop(conn);
        env.finish(&db, "tr.bad", "translation", "1.0.0")
    }

    #[test]
    fn refuses_malicious_or_mismatched_packs() {
        let cases = [
            "CREATE TRIGGER t AFTER INSERT ON translations BEGIN DELETE FROM translations; END;",
            "CREATE VIEW v AS SELECT * FROM translations;",
            "CREATE VIRTUAL TABLE translations_fts USING fts5(text);",
            "CREATE TABLE extra (x);",
            // `ANALYZE` creates sqlite_stat1, a table outside the allow-list.
            "CREATE INDEX i ON translations(text); ANALYZE;",
            "ALTER TABLE translations ADD COLUMN evil TEXT;",
            "DROP TABLE pack_info; CREATE TABLE pack_info (id TEXT);",
            "UPDATE pack_info SET id = 'tr.other';",
            "UPDATE pack_info SET type = 'tafsir';",
            "UPDATE pack_info SET version = '9.9.9';",
            "INSERT INTO pack_info SELECT * FROM pack_info;",
            "UPDATE pack_info SET schema_version = 'two';",
        ];
        for sql in cases {
            let mut env = Env::new();
            let before = dump(&env.conn);
            let (file, entry) = malicious_translation(&env, sql);
            let err = env.install(&file, &entry).unwrap_err();
            assert_eq!(err.code, AppErrorCode::PackInvalid, "{sql}: {err:?}");
            assert_eq!(dump(&env.conn), before, "{sql}");
        }
        let mut env = Env::new();
        let (file, entry) = malicious_translation(&env, "UPDATE pack_info SET schema_version = 1;");
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackSchemaUnsupported
        );
        // A tafsir pack is not a translation pack.
        let (file, mut entry) = env.tafsir("tr.bad", "1.0.0", &[(1, 1, "commentary", P1)]);
        entry.kind = "translation".into();
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackInvalid
        );
    }

    #[test]
    fn refuses_files_that_are_not_packs() {
        let mut env = Env::new();
        let garbage = env.path("garbage.sqlite");
        fs::write(&garbage, vec![7u8; 8192]).unwrap();
        let (file, entry) = env.finish(&garbage, "tr.bad", "translation", "1.0.0");
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackInvalid
        );
        let not_zstd = env.path("plain.sqlite.zst");
        fs::write(&not_zstd, b"definitely not zstd").unwrap();
        assert_eq!(
            env.install(&not_zstd, &entry).unwrap_err().code,
            AppErrorCode::PackInvalid
        );
    }

    #[test]
    fn refuses_packs_larger_than_installed_size() {
        let mut env = Env::new();
        let (file, mut entry) = env.translation("tr.test", "1.0.0", &[(1, "bir")]);
        entry.installed_size -= 1;
        assert_eq!(
            env.install(&file, &entry).unwrap_err().code,
            AppErrorCode::PackTooLarge
        );
    }

    fn archive_with(env: &Env, members: &[Member<'_>]) -> (PathBuf, PackEntry) {
        let archive = env.path("bad.tar.zst");
        tar_zst(&archive, members);
        let entry = entry_for(&archive, "morphology.qac", "morphology", "1.0.0", 10 << 20);
        (archive, entry)
    }

    #[test]
    fn refuses_unsafe_archives() {
        let env = Env::new();
        let db_path = env.path("m.sqlite");
        drop(morphology_db(&db_path, "morphology.qac", "1.0.0", "ق و ل"));
        let db = fs::read(&db_path).unwrap();
        let valid = |name: &'static str| Member::file(name, b"x");
        let cases: Vec<Vec<Member<'_>>> = vec![
            // `..` and absolute paths.
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                valid("qac/NOTICE"),
                valid("qac/quranic-corpus-morphology-0.4.txt"),
                Member::file("../evil", b"x"),
            ],
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                valid("/qac/NOTICE"),
                valid("qac/quranic-corpus-morphology-0.4.txt"),
            ],
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                valid("qac/../qac/NOTICE"),
                valid("qac/quranic-corpus-morphology-0.4.txt"),
            ],
            // A link with an expected name.
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                Member {
                    name: b"qac/NOTICE",
                    data: b"",
                    kind: tar::EntryType::Symlink,
                },
                valid("qac/quranic-corpus-morphology-0.4.txt"),
            ],
            // Duplicate and missing members.
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                valid("qac/NOTICE"),
                valid("qac/NOTICE"),
                valid("qac/quranic-corpus-morphology-0.4.txt"),
            ],
            vec![
                Member::file("morphology.sqlite", &db),
                valid("qac/COPYING"),
                valid("qac/NOTICE"),
            ],
        ];
        for (i, members) in cases.iter().enumerate() {
            let mut env = Env::new();
            let before = dump(&env.conn);
            let (file, entry) = archive_with(&env, members);
            let err = env.install(&file, &entry).unwrap_err();
            assert_eq!(err.code, AppErrorCode::PackInvalid, "case {i}: {err:?}");
            assert_eq!(dump(&env.conn), before);
            assert!(!env.dir.path().join("evil").exists());
            assert!(fs::read_dir(packs_dir(env.dir.path())).map_or(true, |d| d.count() == 0));
        }
    }

    #[test]
    fn uninstalls_packs_and_their_files() {
        let mut env = Env::new();
        let (file, entry) = env.translation("tr.test", "1.0.0", &[(1, "bir")]);
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.tafsir("ar.two", "1.0.0", &[(1, 2, "commentary", P1)]);
        env.install(&file, &entry).unwrap();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        env.install(&file, &entry).unwrap();

        let data_dir = env.dir.path().to_owned();
        for id in ["tr.test", "ar.two", "morphology.qac"] {
            uninstall(&mut env.conn, &data_dir, id).unwrap();
        }
        assert_eq!(env.count("SELECT count(*) FROM packs"), 1);
        for table in [
            "tafsir",
            "tafsir_paragraphs",
            "words",
            "roots",
            "lemmas",
            "word_segments",
        ] {
            assert_eq!(
                env.count(&format!("SELECT count(*) FROM {table}")),
                0,
                "{table}"
            );
        }
        assert_eq!(
            env.count("SELECT count(*) FROM translations_fts WHERE pack_id = 'tr.test'"),
            0
        );
        assert!(!packs_dir(&data_dir).join("morphology.qac").exists());

        assert_eq!(
            uninstall(&mut env.conn, &data_dir, "en.core")
                .unwrap_err()
                .code,
            AppErrorCode::PackBundled
        );
        assert_eq!(
            uninstall(&mut env.conn, &data_dir, "tr.none")
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
    }

    #[test]
    fn vacuum_shrinks_the_file_after_a_removal() {
        let mut env = Env::new();
        let big = "x".repeat(4000);
        let rows: Vec<(i64, &str)> = (1..=20).map(|v| (v, big.as_str())).collect();
        let (file, entry) = env.translation("tr.big", "1.0.0", &rows);
        env.install(&file, &entry).unwrap();
        let data_dir = env.dir.path().to_owned();
        uninstall(&mut env.conn, &data_dir, "tr.big").unwrap();
        let result = vacuum(&env.path("content.db")).unwrap();
        assert!(result.after < result.before, "{result:?}");
    }

    #[test]
    fn cancelling_before_commit_changes_nothing() {
        let mut env = Env::new();
        let before = dump(&env.conn);
        let (file, entry) = env.translation("tr.test", "1.0.0", &[(1, "bir")]);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let data_dir = env.dir.path().to_owned();
        let err = install_file(
            &mut env.conn,
            &file,
            &entry,
            &data_dir,
            &cancel,
            &|_, _, _| {},
        )
        .unwrap_err();
        assert_eq!(err.code, AppErrorCode::InstallCancelled);
        assert_eq!(dump(&env.conn), before);
    }

    #[test]
    fn cleanup_repairs_pack_directories() {
        let mut env = Env::new();
        let (file, entry) = env.morphology("morphology.qac", "1.0.0", "ق و ل");
        env.install(&file, &entry).unwrap();
        let data = env.dir.path().to_owned();
        let packs = packs_dir(&data);
        // A crash after the swap but before the commit: the new files are in
        // place, the database still says 1.0.0, and the old files are in `.old-`.
        let current = packs.join("morphology.qac");
        let old = packs.join(".old-morphology.qac");
        fs::rename(&current, &old).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join(MARKER), r#"{"version": "2.0.0"}"#).unwrap();
        // Leftovers of other kinds.
        fs::create_dir_all(packs.join(".staging-ar.x")).unwrap();
        fs::create_dir_all(packs.join("orphan.pack")).unwrap();
        fs::create_dir_all(downloads_dir(&data)).unwrap();
        fs::write(downloads_dir(&data).join("x-1.0.0.part"), b"part").unwrap();
        fs::create_dir_all(tmp_dir(&data).join("x")).unwrap();

        cleanup(&env.conn, &data).unwrap();
        assert_eq!(marker_version(&current).as_deref(), Some("1.0.0"));
        assert!(current.join("qac/NOTICE").is_file());
        let left: Vec<String> = fs::read_dir(&packs)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(left, ["morphology.qac"]);
        assert!(!downloads_dir(&data).exists() && !tmp_dir(&data).exists());
    }

    #[test]
    fn python_json_matches_the_reference_format() {
        let value = serde_json::json!({
            "title": { "tr": "Pickthall İngilizce Meali", "en": "The \"Meaning\"" },
            "license_terms": { "en": ["a\nb"], "tr": [] },
            "n": 1, "x": null, "e": {}
        });
        assert_eq!(
            python_json(&value),
            r#"{"e": {}, "license_terms": {"en": ["a\nb"], "tr": []}, "n": 1, "title": {"en": "The \"Meaning\"", "tr": "Pickthall İngilizce Meali"}, "x": null}"#
        );
    }

    #[test]
    fn sqlite_uris_are_escaped() {
        assert_eq!(
            sqlite_uri(Path::new("/Users/a b/Application Support/x?#%.sqlite")),
            "file:/Users/a%20b/Application%20Support/x%3F%23%25.sqlite?mode=ro"
        );
    }
}
