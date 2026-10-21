//! `content.db`: the Quran text and installed pack content.
//!
//! On first launch the bundled seed database is copied into place (spec §5);
//! afterwards only the app writes to this file. When the app ships a newer
//! `core.db`, the core tables are refreshed in place (spec §6.2, ADR 0011).
//! TODO(M3): pack installation writes translation/tafsir/morphology rows here.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use specta::Type;

use super::{Migration, backup, connect, migrate, schema_version};
use crate::error::{AppError, AppErrorCode, AppResult};

/// File name of the content database inside the app data directory.
pub const FILE_NAME: &str = "content.db";

/// The database `content.db` is created from, and how this build names it.
#[derive(Debug, Clone)]
pub struct Seed {
    /// Copied to the content database on first launch: the bundled `core.db`,
    /// or the development dataset in debug builds.
    pub path: PathBuf,
    /// File name of the content database inside the app data directory.
    /// Debug builds use one file per seed so switching seeds never needs a reset.
    pub file_name: &'static str,
    /// Stored as `content_meta.seed` (debug builds only), so the interface can
    /// show which data is loaded.
    pub marker: Option<&'static str>,
}

impl Seed {
    /// The seed of release builds: the bundled `core.db` at `path`.
    pub fn bundled(path: PathBuf) -> Self {
        Self {
            path,
            file_name: FILE_NAME,
            marker: None,
        }
    }
}

/// Content schema migrations, oldest first.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("../../migrations/content/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "schema_2",
        sql: include_str!("../../migrations/content/0002_schema_2.sql"),
    },
];

/// The opened content database.
#[derive(Debug)]
pub struct Opened {
    /// Connection to the content database.
    pub conn: Connection,
    /// Why the core refresh failed, if it did. The database is still usable:
    /// it keeps the previous core data (spec §6.2).
    pub core_refresh_error: Option<AppError>,
}

/// Opens the content database in `data_dir`, copying the seed there first if
/// the file does not exist yet, applies pending migrations and refreshes the
/// core data if the seed's is newer.
pub fn open(data_dir: &Path, seed: &Seed) -> AppResult<Opened> {
    let path = data_dir.join(seed.file_name);
    if !path.exists() {
        copy_seed(&seed.path, &path)?;
    }
    let mut conn = connect(&path)?;
    migrate(&mut conn, MIGRATIONS)?;
    if let Some(marker) = seed.marker {
        conn.execute(
            "INSERT INTO content_meta (key, value) VALUES ('seed', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [marker],
        )?;
    }
    // `content.db` → `content.db.bak`, `content.full.db` → `content.full.db.bak`.
    let backup_path = path.with_extension("db.bak");
    let core_refresh_error = refresh_core(&mut conn, &seed.path, &backup_path).err();
    if let Some(err) = &core_refresh_error {
        eprintln!("core refresh failed: {err}");
    }
    Ok(Opened {
        conn,
        core_refresh_error,
    })
}

/// Reads `content_meta.core_version`; a database without one (created before
/// schema 2) counts as version 0.
pub fn core_version(conn: &Connection, schema: &str) -> AppResult<u32> {
    let value: Option<String> = conn
        .query_row(
            &format!("SELECT value FROM {schema}.content_meta WHERE key = 'core_version'"),
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(value.and_then(|v| v.parse().ok()).unwrap_or(0))
}

/// Reads `content_meta.seed`: which dataset a debug build loaded.
pub fn seed_marker(conn: &Connection) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM content_meta WHERE key = 'seed'",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

/// Replaces the core data of `conn` with that of the bundled `core` database
/// if the bundled `core_version` is higher (spec §6.2, ADR 0011). Returns
/// whether a refresh happened.
///
/// The core data is `surahs`, `verses`, `verses_fts`, `notices` and the
/// bundled packs (`packs.bundled = 1` with their `translations` and
/// `translations_fts` rows). Packs the user installed are left untouched; this
/// is safe because verse ids (1..6236) never change. The database is backed up
/// to `backup_path` first and the replacement runs in one transaction, so a
/// failure leaves the previous data in place (and the backup on disk).
pub fn refresh_core(conn: &mut Connection, core: &Path, backup_path: &Path) -> AppResult<bool> {
    let failed = |err: AppError| {
        AppError::new(
            AppErrorCode::CoreRefreshFailed,
            format!("{}: {}", core.display(), err.message),
        )
    };
    let bundled = Connection::open_with_flags(core, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| failed(err.into()))?;
    let (new, current) = (
        core_version(&bundled, "main").map_err(failed)?,
        core_version(conn, "main").map_err(failed)?,
    );
    if new <= current {
        return Ok(false);
    }
    let (core_schema, content_schema) = (
        schema_version(&bundled).map_err(failed)?,
        schema_version(conn).map_err(failed)?,
    );
    if core_schema != content_schema {
        return Err(failed(AppError::new(
            AppErrorCode::DbVersionUnsupported,
            format!("core.db has schema {core_schema}, content.db has {content_schema}"),
        )));
    }
    drop(bundled);

    backup(conn, backup_path).map_err(failed)?;
    conn.execute(
        "ATTACH DATABASE ?1 AS core",
        [core.to_string_lossy().as_ref()],
    )
    .map_err(|err| failed(err.into()))?;
    let result = replace_core_tables(conn);
    // Detaching fails only while a statement is still running; there is none.
    let _ = conn.execute("DETACH DATABASE core", []);
    result.map_err(failed)?;
    Ok(true)
}

/// The replacement itself, in one transaction with `core` attached.
fn replace_core_tables(conn: &mut Connection) -> AppResult<()> {
    let tx = conn.transaction()?;
    // Verses are deleted and inserted again with the same ids; checking
    // foreign keys at commit lets rows of installed packs keep pointing at them.
    tx.execute_batch(
        "PRAGMA defer_foreign_keys = ON;

         DELETE FROM main.translations_fts WHERE pack_id IN
           (SELECT id FROM main.packs WHERE bundled = 1 OR id IN (SELECT id FROM core.packs));
         DELETE FROM main.packs WHERE bundled = 1 OR id IN (SELECT id FROM core.packs);
         DELETE FROM main.verses_fts;
         DELETE FROM main.verses;
         DELETE FROM main.surahs;
         DELETE FROM main.notices;

         INSERT INTO main.surahs
           (number, name_ar, name_tr, name_en, revelation_type, ayah_count, bismillah)
           SELECT number, name_ar, name_tr, name_en, revelation_type, ayah_count, bismillah
           FROM core.surahs;
         INSERT INTO main.verses (id, surah, ayah, juz, text_uthmani, text_clean)
           SELECT id, surah, ayah, juz, text_uthmani, text_clean FROM core.verses;
         INSERT INTO main.verses_fts (rowid, text_norm)
           SELECT rowid, text_norm FROM core.verses_fts;
         INSERT INTO main.notices (id, source, title, body, url)
           SELECT id, source, title, body, url FROM core.notices;
         INSERT INTO main.packs
           (id, type, lang, title, author, version, license, attribution, citation_json,
            meta_json, installed_at, bundled)
           SELECT id, type, lang, title, author, version, license, attribution, citation_json,
                  meta_json, installed_at, bundled
           FROM core.packs;
         INSERT INTO main.translations (pack_id, verse_id, text)
           SELECT pack_id, verse_id, text FROM core.translations;
         INSERT INTO main.translations_fts (text, pack_id, verse_id)
           SELECT text, pack_id, verse_id FROM core.translations_fts;
         -- Keys only this database has (the debug `seed` marker) are kept.
         INSERT INTO main.content_meta (key, value)
           SELECT key, value FROM core.content_meta WHERE true
           ON CONFLICT (key) DO UPDATE SET value = excluded.value;",
    )?;
    tx.commit()?;
    Ok(())
}

/// Where a surah was revealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RevelationType {
    /// Revealed in Mecca.
    Meccan,
    /// Revealed in Medina.
    Medinan,
}

/// A row of the `surahs` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Surah {
    /// Surah number, 1..=114.
    pub number: u16,
    /// Arabic name.
    pub name_ar: String,
    /// Turkish name.
    pub name_tr: String,
    /// English transliteration of the name.
    pub name_en: String,
    /// Where the surah was revealed.
    pub revelation_type: RevelationType,
    /// Number of verses in the surah.
    pub ayah_count: u16,
    /// Tanzil's basmala, shown as the surah header; `None` for surah 1 (where
    /// it is verse 1) and surah 9 (spec §5.1).
    pub bismillah: Option<String>,
}

/// Writing direction of a text block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

/// The title of a pack in the interface languages, where the pack provides one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
pub struct LocalizedTitle {
    /// Turkish title.
    pub tr: Option<String>,
    /// English title.
    pub en: Option<String>,
}

/// An installed translation pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TranslationInfo {
    /// Pack id, e.g. `en.pickthall`.
    pub id: String,
    /// Title as published (`packs.title`).
    pub title: String,
    /// Title in the interface languages, from the pack's metadata.
    pub titles: LocalizedTitle,
    /// BCP 47 language of the translation, used for the `lang` attribute.
    pub lang: Option<String>,
    /// Writing direction of the translation's language, used for the `dir` attribute.
    pub dir: TextDirection,
    /// Whether the pack ships with the app (`core.db`).
    pub bundled: bool,
}

/// One translation of a verse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct VerseTranslation {
    /// Translation pack id, e.g. `en.pickthall`.
    pub pack_id: String,
    /// BCP 47 language of the translation, used for the `lang` attribute.
    pub lang: Option<String>,
    /// Translated text.
    pub text: String,
}

/// A verse with its Arabic text and the requested translations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct VerseWithTranslations {
    /// Global verse id, 1..=6236.
    pub id: u32,
    /// Surah number.
    pub surah: u16,
    /// Verse number within the surah.
    pub ayah: u16,
    /// Juz (part) number, 1..=30.
    pub juz: u8,
    /// Uthmani script for display.
    pub text_uthmani: String,
    /// Translations in the order they were requested; packs without a row
    /// for this verse are omitted.
    pub translations: Vec<VerseTranslation>,
}

const SURAH_COLUMNS: &str =
    "number, name_ar, name_tr, name_en, revelation_type, ayah_count, bismillah";

fn surah_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Surah> {
    let revelation_type = match row.get_ref(4)?.as_str()? {
        "meccan" => RevelationType::Meccan,
        "medinan" => RevelationType::Medinan,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                format!("unknown revelation_type {other:?}").into(),
            ));
        }
    };
    Ok(Surah {
        number: row.get(0)?,
        name_ar: row.get(1)?,
        name_tr: row.get(2)?,
        name_en: row.get(3)?,
        revelation_type,
        ayah_count: row.get(5)?,
        bismillah: row.get(6)?,
    })
}

/// Lists all surahs in Mushaf order.
pub fn list_surahs(conn: &Connection) -> AppResult<Vec<Surah>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SURAH_COLUMNS} FROM surahs ORDER BY number"
    ))?;
    let rows = stmt.query_map([], surah_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Returns one surah. Fails with [`AppErrorCode::NotFound`] if it does not exist.
pub fn get_surah(conn: &Connection, number: u16) -> AppResult<Surah> {
    conn.query_row(
        &format!("SELECT {SURAH_COLUMNS} FROM surahs WHERE number = ?1"),
        [number],
        surah_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("surah {number}")))
}

/// Lists the installed translation packs: bundled first, then by id.
pub fn list_translations(conn: &Connection) -> AppResult<Vec<TranslationInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, meta_json, lang, bundled FROM packs
         WHERE type = 'translation' ORDER BY bundled DESC, id",
    )?;
    let rows = stmt.query_map([], |row| {
        let lang: Option<String> = row.get(3)?;
        Ok(TranslationInfo {
            id: row.get(0)?,
            title: row.get(1)?,
            titles: localized_title(row.get::<_, Option<String>>(2)?.as_deref()),
            dir: text_direction(lang.as_deref()),
            lang,
            bundled: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Reads `title.tr` / `title.en` from a pack's `meta_json`. Missing or
/// malformed metadata only loses the localized title, never the pack.
fn localized_title(meta_json: Option<&str>) -> LocalizedTitle {
    let meta: serde_json::Value = meta_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    let title = |lang: &str| meta["title"][lang].as_str().map(str::to_owned);
    LocalizedTitle {
        tr: title("tr"),
        en: title("en"),
    }
}

/// Languages written right to left, by primary subtag.
const RTL_LANGUAGES: &[&str] = &[
    "ar", "arc", "ckb", "dv", "fa", "he", "ks", "ps", "sd", "ug", "ur", "yi",
];

/// The writing direction of a BCP 47 language tag: right to left for the
/// languages above and for any tag with an Arabic or Hebrew script subtag
/// (e.g. `az-Arab`), otherwise left to right.
pub fn text_direction(lang: Option<&str>) -> TextDirection {
    let Some(lang) = lang else {
        return TextDirection::Ltr;
    };
    let mut subtags = lang.split(['-', '_']).map(str::to_ascii_lowercase);
    let primary = subtags.next().unwrap_or_default();
    if RTL_LANGUAGES.contains(&primary.as_str()) || subtags.any(|s| s == "arab" || s == "hebr") {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    }
}

/// Returns the verses of `surah` with the translations in `translation_ids`.
///
/// Unknown translation ids are ignored and duplicates count once, so a stale
/// id in the user's settings never breaks reading. Fails with
/// [`AppErrorCode::NotFound`] if the surah does not exist.
pub fn get_verses(
    conn: &Connection,
    surah: u16,
    translation_ids: &[String],
) -> AppResult<Vec<VerseWithTranslations>> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM surahs WHERE number = ?1",
            [surah],
            |_| Ok(()),
        )
        .optional()?;
    if exists.is_none() {
        return Err(AppError::new(
            AppErrorCode::NotFound,
            format!("surah {surah}"),
        ));
    }

    let mut verses = conn
        .prepare(
            "SELECT id, surah, ayah, juz, text_uthmani FROM verses
             WHERE surah = ?1 ORDER BY ayah",
        )?
        .query_map([surah], |row| {
            Ok(VerseWithTranslations {
                id: row.get(0)?,
                surah: row.get(1)?,
                ayah: row.get(2)?,
                juz: row.get(3)?,
                text_uthmani: row.get(4)?,
                translations: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let index: HashMap<u32, usize> = verses.iter().enumerate().map(|(i, v)| (v.id, i)).collect();
    let mut stmt = conn.prepare(
        "SELECT t.verse_id, p.lang, t.text
         FROM translations t
         JOIN packs p ON p.id = t.pack_id
         JOIN verses v ON v.id = t.verse_id
         WHERE t.pack_id = ?1 AND v.surah = ?2",
    )?;
    let mut seen = Vec::with_capacity(translation_ids.len());
    for pack_id in translation_ids {
        if seen.contains(&pack_id) {
            continue;
        }
        seen.push(pack_id);
        let rows = stmt.query_map((pack_id, surah), |row| {
            Ok((row.get::<_, u32>(0)?, row.get(1)?, row.get(2)?))
        })?;
        for row in rows {
            let (verse_id, lang, text) = row?;
            if let Some(verse) = index.get(&verse_id).and_then(|&i| verses.get_mut(i)) {
                verse.translations.push(VerseTranslation {
                    pack_id: pack_id.clone(),
                    lang,
                    text,
                });
            }
        }
    }
    Ok(verses)
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
    use crate::db::{schema_shape, schema_version, tests::assert_contiguous};

    #[test]
    fn migrations_are_contiguous() {
        assert_contiguous(MIGRATIONS);
    }

    #[test]
    fn migrations_apply_on_empty_db_and_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, MIGRATIONS).unwrap(), MIGRATIONS.len());
        assert_eq!(migrate(&mut conn, MIGRATIONS).unwrap(), 0);
        assert_eq!(schema_version(&conn).unwrap(), 2);
        for table in [
            "surahs",
            "verses",
            "packs",
            "translations",
            "notices",
            "content_meta",
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

    /// A content.db created by M0 (schema 1) keeps its rows when it is
    /// migrated, including tafsir paragraphs, whose table is rebuilt.
    #[test]
    fn schema_1_database_migrates_with_its_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &MIGRATIONS[..1]).unwrap();
        conn.execute_batch(
            "INSERT INTO surahs VALUES (1, 'الفاتحة', 'Fâtiha', 'Al-Faatiha', 'meccan', 7);
             INSERT INTO verses VALUES (1, 1, 1, 1, 'بِسْمِ', 'بسم');
             INSERT INTO packs (id, type, title, version, license, installed_at)
               VALUES ('ar.test', 'tafsir', 'Test', '1.0.0', 'PD', 'now');
             INSERT INTO tafsir (id, pack_id, verse_from, verse_to) VALUES (7, 'ar.test', 1, 1);
             INSERT INTO tafsir_paragraphs VALUES (7, 0, 'first'), (7, 1, 'second');",
        )
        .unwrap();

        assert_eq!(migrate(&mut conn, MIGRATIONS).unwrap(), 1);
        let paragraphs: Vec<(i64, String, Option<String>)> = conn
            .prepare(
                "SELECT idx, text, part FROM tafsir_paragraphs WHERE tafsir_id = 7 ORDER BY idx",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            paragraphs,
            [
                (0, "first".to_owned(), None),
                (1, "second".to_owned(), None)
            ]
        );
        let kind: String = conn
            .query_row("SELECT kind FROM tafsir WHERE id = 7", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kind, "commentary");
        let bismillah: Option<String> = conn
            .query_row("SELECT bismillah FROM surahs WHERE number = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(bismillah, None);
        // The rebuilt table keeps its uniqueness and its foreign key cascade.
        assert!(
            conn.execute(
                "INSERT INTO tafsir_paragraphs (tafsir_id, idx, text) VALUES (7, 0, 'dup')",
                []
            )
            .is_err()
        );
        conn.execute("DELETE FROM tafsir WHERE id = 7", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT count(*) FROM tafsir_paragraphs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn verses_fts_indexes_normalized_text() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let shape = schema_shape(&conn).unwrap();
        assert_eq!(shape["virtual verses_fts"], "USING fts5(text_norm)");
    }

    #[test]
    fn first_open_copies_seed() {
        let dir = tempfile::tempdir().unwrap();
        let seed = dir.path().join("seed.db");
        {
            let mut conn = Connection::open(&seed).unwrap();
            migrate(&mut conn, MIGRATIONS).unwrap();
            conn.execute(
                "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count)
                 VALUES (1, 'الفاتحة', 'Fâtiha', 'Al-Faatiha', 'meccan', 7)",
                [],
            )
            .unwrap();
        }
        let path = dir.path().join(FILE_NAME);
        let conn = open(dir.path(), &Seed::bundled(seed)).unwrap().conn;
        let count: i64 = conn
            .query_row("SELECT count(*) FROM surahs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert!(!path.with_extension("db.tmp").exists());
    }

    fn dev_db() -> Connection {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-data/dev-content.db");
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }

    /// The dev seed has the schema the migrations create. Its verses_fts is
    /// empty until search arrives (TODO(M4) in `xtask/src/dev_db.rs`).
    #[test]
    fn dev_db_matches_the_migrations() {
        let mut fresh = Connection::open_in_memory().unwrap();
        migrate(&mut fresh, MIGRATIONS).unwrap();
        let dev = dev_db();
        assert_eq!(schema_shape(&dev).unwrap(), schema_shape(&fresh).unwrap());
        assert_eq!(
            schema_version(&dev).unwrap(),
            schema_version(&fresh).unwrap()
        );
    }

    /// The bundled `core.db` (copied from siyak-data by `cargo xtask sync-core`)
    /// has exactly the schema of these migrations, so the app can open it.
    #[test]
    fn bundled_core_db_matches_the_migrations() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/core.db");
        let core =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut fresh = Connection::open_in_memory().unwrap();
        migrate(&mut fresh, MIGRATIONS).unwrap();
        assert_eq!(schema_shape(&core).unwrap(), schema_shape(&fresh).unwrap());
        assert_eq!(
            schema_version(&core).unwrap(),
            schema_version(&fresh).unwrap()
        );
        let count = |table: &str| -> i64 {
            core.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("surahs"), 114);
        assert_eq!(count("verses"), 6236);
    }

    /// The basmala is a surah header for every surah except 1 (where it is
    /// verse 1) and 9 (spec §5.1); the dev seed has it for its surahs.
    #[test]
    fn dev_db_has_basmala_rules_and_al_baqarah() {
        let dev = dev_db();
        let bismillah = |surah: u16| -> Option<String> {
            dev.query_row(
                "SELECT bismillah FROM surahs WHERE number = ?1",
                [surah],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(bismillah(1), None);
        assert_eq!(bismillah(9), None);
        assert!(bismillah(2).is_some_and(|b| b.starts_with("بِسْمِ")));
        assert_eq!(get_verses(&dev, 2, &[]).unwrap().len(), 286);
        assert_eq!(get_verses(&dev, 9, &[]).unwrap().len(), 129);
    }

    fn bundled_core() -> Connection {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/core.db");
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }

    #[test]
    fn get_surah_follows_the_basmala_rules() {
        let dev = dev_db();
        assert_eq!(get_surah(&dev, 1).unwrap().bismillah, None);
        assert_eq!(get_surah(&dev, 9).unwrap().bismillah, None);
        let baqarah = get_surah(&dev, 2).unwrap();
        assert_eq!(baqarah.ayah_count, 286);
        assert!(baqarah.bismillah.is_some());
        let err = get_surah(&dev, 115).unwrap_err();
        assert_eq!(err.code, AppErrorCode::NotFound);
    }

    #[test]
    fn list_translations_reads_metadata() {
        let translations = list_translations(&bundled_core()).unwrap();
        let pickthall = translations
            .iter()
            .find(|t| t.id == "en.pickthall")
            .unwrap();
        assert_eq!(pickthall.lang.as_deref(), Some("en"));
        assert_eq!(pickthall.dir, TextDirection::Ltr);
        assert!(pickthall.bundled);
        assert_eq!(
            pickthall.titles.en.as_deref(),
            Some("The Meaning of the Glorious Koran")
        );
        assert!(pickthall.titles.tr.is_some());
        // The dev seed has no metadata; the published title still works.
        let dev = list_translations(&dev_db()).unwrap();
        assert_eq!(dev.len(), 1);
        assert_eq!(dev[0].titles, LocalizedTitle::default());
    }

    #[test]
    fn text_direction_from_language_tags() {
        for lang in ["ar", "fa", "ur", "he", "ckb", "az-Arab", "ku_arab", "AR"] {
            assert_eq!(text_direction(Some(lang)), TextDirection::Rtl, "{lang}");
        }
        for lang in ["en", "tr", "az", "ms-Latn", "ku"] {
            assert_eq!(text_direction(Some(lang)), TextDirection::Ltr, "{lang}");
        }
        assert_eq!(text_direction(None), TextDirection::Ltr);
    }

    /// The reader loads a whole surah in one call. Run with
    /// `cargo test -- --ignored --nocapture al_baqarah_load_time` to see how
    /// long the longest surah takes with the full data.
    #[test]
    #[ignore = "timing report, not a check"]
    fn al_baqarah_load_time() {
        let core = bundled_core();
        let ids = ["en.pickthall".to_owned()];
        let start = std::time::Instant::now();
        let verses = get_verses(&core, 2, &ids).unwrap();
        let query = start.elapsed();
        let json = serde_json::to_string(&verses).unwrap();
        println!(
            "al-Baqarah: {} verses, query {query:?}, total with JSON {:?}, {} bytes of JSON",
            verses.len(),
            start.elapsed(),
            json.len()
        );
    }

    #[test]
    fn list_surahs_returns_all_114() {
        let surahs = list_surahs(&dev_db()).unwrap();
        assert_eq!(surahs.len(), 114);
        assert_eq!(surahs[0].number, 1);
        assert_eq!(surahs[0].ayah_count, 7);
        assert_eq!(surahs[1].revelation_type, RevelationType::Medinan);
    }

    #[test]
    fn get_verses_returns_al_fatiha_with_translation() {
        let verses = get_verses(&dev_db(), 1, &["en.pickthall".to_owned()]).unwrap();
        assert_eq!(verses.len(), 7);
        assert_eq!(verses[0].id, 1);
        assert_eq!(verses[6].ayah, 7);
        for verse in &verses {
            assert_eq!(verse.translations.len(), 1);
            assert_eq!(verse.translations[0].pack_id, "en.pickthall");
            assert_eq!(verse.translations[0].lang.as_deref(), Some("en"));
        }
    }

    #[test]
    fn get_verses_ignores_unknown_and_duplicate_translation_ids() {
        let ids = ["xx.unknown", "en.pickthall", "en.pickthall"].map(str::to_owned);
        let verses = get_verses(&dev_db(), 114, &ids).unwrap();
        assert_eq!(verses.len(), 6);
        assert!(verses.iter().all(|v| v.translations.len() == 1));
    }

    #[test]
    fn get_verses_without_translations() {
        let verses = get_verses(&dev_db(), 112, &[]).unwrap();
        assert_eq!(verses.len(), 4);
        assert!(verses.iter().all(|v| v.translations.is_empty()));
    }

    #[test]
    fn get_verses_of_unknown_surah_is_not_found() {
        let err = get_verses(&dev_db(), 115, &[]).unwrap_err();
        assert_eq!(err.code, AppErrorCode::NotFound);
    }

    #[test]
    fn seed_marker_is_written_to_its_own_file() {
        let dir = tempfile::tempdir().unwrap();
        let seed = Seed {
            path: Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-data/dev-content.db"),
            file_name: "content.full.db",
            marker: Some("full"),
        };
        let conn = open(dir.path(), &seed).unwrap().conn;
        let marker: String = conn
            .query_row(
                "SELECT value FROM content_meta WHERE key = 'seed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, "full");
        assert!(dir.path().join("content.full.db").is_file());
        assert!(!dir.path().join(FILE_NAME).exists());
    }

    /// Writes a small core database: two surahs, three verses, the bundled
    /// pack `en.core` and `core_version`. `text` marks this build's rows.
    fn write_core(path: &Path, version: u32, text: &str) {
        let mut conn = Connection::open(path).unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        conn.execute_batch(&format!(
            "INSERT INTO surahs VALUES
               (1, 'الفاتحة', 'Fâtiha', 'Al-Faatiha', 'meccan', 7, NULL),
               (2, 'البقرة', 'Bakara', 'Al-Baqara', 'medinan', 286, 'بسم');
             INSERT INTO verses VALUES (1, 1, 1, 1, '{text} 1', 'c1'), (8, 2, 1, 1, '{text} 8', 'c8'),
               (9, 2, 2, 1, '{text} 9', 'c9');
             INSERT INTO verses_fts (rowid, text_norm) VALUES (1, 'n1'), (8, 'n8'), (9, 'n9');
             INSERT INTO notices VALUES ('tanzil.text.uthmani', 'tanzil', 'Tanzil', '{text}', NULL);
             INSERT INTO packs (id, type, lang, title, version, license, installed_at, bundled)
               VALUES ('en.core', 'translation', 'en', 'Core', '1.0.0', 'PD', 'build', 1);
             INSERT INTO translations VALUES ('en.core', 1, '{text} en 1'), ('en.core', 8, '{text} en 8');
             INSERT INTO translations_fts (text, pack_id, verse_id)
               VALUES ('{text} en 1', 'en.core', 1), ('{text} en 8', 'en.core', 8);
             INSERT INTO content_meta VALUES ('core_version', '{version}'), ('tanzil_version', '1.1');"
        ))
        .unwrap();
    }

    /// A content database seeded from a version 1 core, with a pack the user
    /// installed (`bundled = 0`) and the debug seed marker.
    fn content_with_user_pack(dir: &Path) -> (Connection, std::path::PathBuf) {
        let core = dir.join("core-v1.db");
        write_core(&core, 1, "old");
        let seed = Seed {
            path: core,
            file_name: FILE_NAME,
            marker: Some("dev"),
        };
        let conn = open(dir, &seed).unwrap().conn;
        conn.execute_batch(
            "INSERT INTO packs (id, type, lang, title, version, license, installed_at, bundled)
               VALUES ('tr.user', 'translation', 'tr', 'User', '1.0.0', 'PD', 'later', 0);
             INSERT INTO translations VALUES ('tr.user', 1, 'kullanıcı 1'), ('tr.user', 9, 'kullanıcı 9');
             INSERT INTO translations_fts (text, pack_id, verse_id)
               VALUES ('kullanıcı 1', 'tr.user', 1), ('kullanıcı 9', 'tr.user', 9);",
        )
        .unwrap();
        (conn, dir.join("content.db.bak"))
    }

    fn query<T: rusqlite::types::FromSql>(conn: &Connection, sql: &str) -> Vec<T> {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn refresh_replaces_core_and_keeps_installed_packs() {
        let dir = tempfile::tempdir().unwrap();
        let (mut conn, bak) = content_with_user_pack(dir.path());
        let core = dir.path().join("core-v2.db");
        write_core(&core, 2, "new");

        assert!(refresh_core(&mut conn, &core, &bak).unwrap());

        let verses: Vec<String> = query(&conn, "SELECT text_uthmani FROM verses ORDER BY id");
        assert_eq!(verses, ["new 1", "new 8", "new 9"]);
        let bundled: Vec<String> = query(
            &conn,
            "SELECT text FROM translations WHERE pack_id = 'en.core' ORDER BY verse_id",
        );
        assert_eq!(bundled, ["new en 1", "new en 8"]);
        let fts: Vec<String> = query(&conn, "SELECT text FROM translations_fts ORDER BY text");
        assert_eq!(fts, ["kullanıcı 1", "kullanıcı 9", "new en 1", "new en 8"]);
        let user: Vec<String> = query(
            &conn,
            "SELECT text FROM translations WHERE pack_id = 'tr.user' ORDER BY verse_id",
        );
        assert_eq!(user, ["kullanıcı 1", "kullanıcı 9"]);
        let packs: Vec<String> = query(&conn, "SELECT id FROM packs ORDER BY id");
        assert_eq!(packs, ["en.core", "tr.user"]);
        let notices: Vec<String> = query(&conn, "SELECT body FROM notices");
        assert_eq!(notices, ["new"]);
        assert_eq!(core_version(&conn, "main").unwrap(), 2);
        assert_eq!(seed_marker(&conn).unwrap().as_deref(), Some("dev"));
        assert!(
            conn.query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .unwrap()
                .is_none()
        );
        let databases: Vec<String> = query(&conn, "SELECT name FROM pragma_database_list");
        assert_eq!(databases, ["main"], "core stays attached");

        // The backup holds the data from before the refresh.
        let backup = Connection::open(&bak).unwrap();
        assert_eq!(core_version(&backup, "main").unwrap(), 1);
    }

    #[test]
    fn failed_refresh_rolls_back_and_keeps_the_backup() {
        let dir = tempfile::tempdir().unwrap();
        let (mut conn, bak) = content_with_user_pack(dir.path());
        let core = dir.path().join("core-broken.db");
        write_core(&core, 2, "new");
        {
            // Inserted last, so the foreign key fails only when committing.
            let broken = Connection::open(&core).unwrap();
            broken
                .execute_batch(
                    "PRAGMA foreign_keys = OFF;
                     INSERT INTO translations VALUES ('en.core', 9999, 'orphan');",
                )
                .unwrap();
        }

        let err = refresh_core(&mut conn, &core, &bak).unwrap_err();
        assert_eq!(err.code, AppErrorCode::CoreRefreshFailed);

        let verses: Vec<String> = query(&conn, "SELECT text_uthmani FROM verses ORDER BY id");
        assert_eq!(verses, ["old 1", "old 8", "old 9"]);
        let count: Vec<i64> = query(&conn, "SELECT count(*) FROM translations");
        assert_eq!(count, [4]);
        assert_eq!(core_version(&conn, "main").unwrap(), 1);
        assert!(bak.is_file());
        // Nothing is left half-done: a later refresh still works.
        let fixed = dir.path().join("core-v2.db");
        write_core(&fixed, 2, "new");
        assert!(refresh_core(&mut conn, &fixed, &bak).unwrap());
    }

    #[test]
    fn equal_core_version_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let (mut conn, bak) = content_with_user_pack(dir.path());
        let core = dir.path().join("core-v1-again.db");
        write_core(&core, 1, "other");

        assert!(!refresh_core(&mut conn, &core, &bak).unwrap());
        assert!(!bak.exists());
        let verses: Vec<String> = query(&conn, "SELECT text_uthmani FROM verses ORDER BY id");
        assert_eq!(verses, ["old 1", "old 8", "old 9"]);
    }

    /// A content.db from M0 has no `content_meta` row, so any bundled core
    /// counts as newer and is applied when the database is opened.
    #[test]
    fn database_without_core_version_is_refreshed_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        {
            let mut conn = Connection::open(&path).unwrap();
            migrate(&mut conn, &MIGRATIONS[..1]).unwrap();
            conn.execute_batch(
                "INSERT INTO surahs VALUES (1, 'الفاتحة', 'Fâtiha', 'Al-Fatihah', 'meccan', 7);
                 INSERT INTO verses VALUES (1, 1, 1, 1, 'm0', 'm0');",
            )
            .unwrap();
        }
        let core = dir.path().join("core.db");
        write_core(&core, 1, "new");

        let opened = open(dir.path(), &Seed::bundled(core)).unwrap();
        assert!(opened.core_refresh_error.is_none());
        let verses: Vec<String> =
            query(&opened.conn, "SELECT text_uthmani FROM verses ORDER BY id");
        assert_eq!(verses, ["new 1", "new 8", "new 9"]);
        assert!(dir.path().join("content.db.bak").is_file());
    }

    #[test]
    fn missing_seed_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let err = open(dir.path(), &Seed::bundled(dir.path().join("nope.db"))).unwrap_err();
        assert_eq!(err.code, AppErrorCode::SeedMissing);
    }
}
