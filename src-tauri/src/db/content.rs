//! `content.db`: the Quran text and installed pack content.
//!
//! On first launch the bundled seed database is copied into place (spec §5);
//! afterwards only the app writes to this file.
//! TODO(M3): pack installation writes translation/tafsir/morphology rows here.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use specta::Type;

use super::{Migration, connect, migrate};
use crate::error::{AppError, AppErrorCode, AppResult};

/// File name of the content database inside the app data directory.
pub const FILE_NAME: &str = "content.db";

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

/// Lists all surahs in Mushaf order.
pub fn list_surahs(conn: &Connection) -> AppResult<Vec<Surah>> {
    let mut stmt = conn.prepare(
        "SELECT number, name_ar, name_tr, name_en, revelation_type, ayah_count
         FROM surahs ORDER BY number",
    )?;
    let rows = stmt.query_map([], |row| {
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
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
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
        let conn = open(&path, &seed).unwrap();
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
    fn missing_seed_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let err = open(&dir.path().join(FILE_NAME), &dir.path().join("nope.db")).unwrap_err();
        assert_eq!(err.code, AppErrorCode::SeedMissing);
    }
}
