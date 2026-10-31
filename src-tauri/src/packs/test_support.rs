//! Builders for packs, archives and content databases used by the pack tests.
//! Packs are built in code (no fixtures), so malicious variants are easy to make.

use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use super::download::hex;
use super::manifest::tests::{entry, manifest_json};
use super::manifest::{PackEntry, parse};
use crate::db::content::MIGRATIONS;
use crate::db::migrate;

/// A migrated content database at `path` with surah 1 (verses 1–7), surah 2
/// (verses 8–20) and the bundled translation `en.core`.
pub(crate) fn content_db(path: &Path) -> Connection {
    let mut conn = crate::db::connect(path).unwrap();
    migrate(&mut conn, MIGRATIONS).unwrap();
    conn.execute_batch(
        "INSERT INTO surahs VALUES
           (1, 'الفاتحة', 'Fâtiha', 'Al-Faatiha', 'meccan', 7, NULL),
           (2, 'البقرة', 'Bakara', 'Al-Baqara', 'medinan', 13, 'بسم');
         INSERT INTO packs (id, type, lang, title, version, license, installed_at, bundled)
           VALUES ('en.core', 'translation', 'en', 'Core', '1.0.0', 'PD', 'build', 1);",
    )
    .unwrap();
    for id in 1..=20 {
        let (surah, ayah) = if id <= 7 { (1, id) } else { (2, id - 7) };
        conn.execute(
            "INSERT INTO verses VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            (id, surah, ayah, format!("verse {id}")),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO translations VALUES ('en.core', ?1, ?2)",
            (id, format!("core {id}")),
        )
        .unwrap();
    }
    conn
}

/// Writes `pack_info`.
pub(crate) fn pack_info(conn: &Connection, id: &str, kind: &str, version: &str, schema: i64) {
    conn.execute_batch(
        "CREATE TABLE pack_info (id TEXT NOT NULL, type TEXT NOT NULL, version TEXT NOT NULL,
           schema_version INTEGER NOT NULL, built_at TEXT NOT NULL, source_version TEXT NOT NULL)",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO pack_info VALUES (?1, ?2, ?3, ?4, '2026-10-21T00:00:00Z', 'src@1')",
        (id, kind, version, schema),
    )
    .unwrap();
}

/// A translation pack database with one row per `(verse_id, text)`.
pub(crate) fn translation_db(
    path: &Path,
    id: &str,
    version: &str,
    rows: &[(i64, &str)],
) -> Connection {
    let conn = Connection::open(path).unwrap();
    pack_info(&conn, id, "translation", version, 2);
    conn.execute_batch(
        "CREATE TABLE translations (pack_id TEXT NOT NULL, verse_id INTEGER NOT NULL,
           text TEXT NOT NULL, PRIMARY KEY (pack_id, verse_id)) WITHOUT ROWID",
    )
    .unwrap();
    for (verse, text) in rows {
        conn.execute(
            "INSERT INTO translations VALUES ('ignored', ?1, ?2)",
            (verse, text),
        )
        .unwrap();
    }
    conn
}

/// One tafsir passage: `(verse_from, verse_to, kind, paragraphs)`.
pub(crate) type Passage<'a> = (i64, i64, &'a str, &'a [&'a str]);

/// A tafsir pack database; passage ids are 1..N in the given order.
pub(crate) fn tafsir_db(
    path: &Path,
    id: &str,
    version: &str,
    passages: &[Passage<'_>],
) -> Connection {
    let conn = Connection::open(path).unwrap();
    pack_info(&conn, id, "tafsir", version, 2);
    conn.execute_batch(
        "CREATE TABLE tafsir (id INTEGER PRIMARY KEY, pack_id TEXT NOT NULL,
           verse_from INTEGER NOT NULL, verse_to INTEGER NOT NULL,
           kind TEXT NOT NULL DEFAULT 'commentary');
         CREATE TABLE tafsir_paragraphs (tafsir_id INTEGER NOT NULL, idx INTEGER NOT NULL,
           text TEXT NOT NULL, part TEXT, page INTEGER, UNIQUE (tafsir_id, idx));
         CREATE INDEX tafsir_lookup ON tafsir(pack_id, verse_from, verse_to);",
    )
    .unwrap();
    for (n, (from, to, kind, paragraphs)) in passages.iter().enumerate() {
        let tafsir_id = n as i64 + 1;
        conn.execute(
            "INSERT INTO tafsir VALUES (?1, 'x', ?2, ?3, ?4)",
            (tafsir_id, from, to, kind),
        )
        .unwrap();
        for (idx, text) in paragraphs.iter().enumerate() {
            conn.execute(
                "INSERT INTO tafsir_paragraphs VALUES (?1, ?2, ?3, '1', 10)",
                (tafsir_id, idx as i64, text),
            )
            .unwrap();
        }
    }
    conn
}

/// A morphology pack database: two words of verse 1 with one segment each.
pub(crate) fn morphology_db(path: &Path, id: &str, version: &str, root: &str) -> Connection {
    let conn = Connection::open(path).unwrap();
    pack_info(&conn, id, "morphology", version, 2);
    conn.execute_batch(&format!(
        "CREATE TABLE words (id INTEGER PRIMARY KEY, verse_id INTEGER NOT NULL,
           position INTEGER NOT NULL, text_ar TEXT NOT NULL, translit TEXT, gloss_en TEXT);
         CREATE TABLE roots (id INTEGER PRIMARY KEY, root TEXT NOT NULL);
         CREATE TABLE lemmas (id INTEGER PRIMARY KEY, lemma TEXT NOT NULL, root_id INTEGER);
         CREATE TABLE word_segments (word_id INTEGER NOT NULL, idx INTEGER NOT NULL,
           kind TEXT NOT NULL, text_ar TEXT NOT NULL, pos TEXT NOT NULL, lemma_id INTEGER,
           root_id INTEGER, features TEXT, PRIMARY KEY (word_id, idx)) WITHOUT ROWID;
         INSERT INTO roots VALUES (1, '{root}');
         INSERT INTO lemmas VALUES (1, 'lemma', 1);
         INSERT INTO words VALUES (1, 1, 1, 'بِسْمِ', NULL, NULL), (2, 1, 2, 'ٱللَّهِ', NULL, NULL);
         INSERT INTO word_segments VALUES (1, 1, 'stem', 'سْمِ', 'N', 1, 1, '{{\"case\": \"GEN\"}}'),
           (2, 1, 'stem', 'ٱللَّهِ', 'PN', NULL, NULL, NULL);"
    ))
    .unwrap();
    conn
}

/// Compresses `src` like `pipeline/compress.py`: one frame with content size
/// and checksum. Returns `<src>.zst`.
pub(crate) fn compress(src: &Path) -> PathBuf {
    let dest = PathBuf::from(format!("{}.zst", src.display()));
    let data = std::fs::read(src).unwrap();
    compress_bytes(&data, &dest);
    dest
}

pub(crate) fn compress_bytes(data: &[u8], dest: &Path) {
    let mut encoder = zstd::Encoder::new(std::fs::File::create(dest).unwrap(), 3).unwrap();
    encoder.include_checksum(true).unwrap();
    encoder.include_contentsize(true).unwrap();
    encoder
        .set_pledged_src_size(Some(data.len() as u64))
        .unwrap();
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap();
}

/// One tar member: name, content and type.
pub(crate) struct Member<'a> {
    pub name: &'a [u8],
    pub data: &'a [u8],
    pub kind: tar::EntryType,
}

impl<'a> Member<'a> {
    pub(crate) fn file(name: &'a str, data: &'a [u8]) -> Self {
        Self {
            name: name.as_bytes(),
            data,
            kind: tar::EntryType::Regular,
        }
    }
}

/// Writes a `.tar.zst` with the given members. Names are written raw into the
/// header, so archives with `..`, absolute paths or links can be made.
pub(crate) fn tar_zst(dest: &Path, members: &[Member<'_>]) {
    let mut builder = tar::Builder::new(Vec::new());
    for member in members {
        let mut header = tar::Header::new_ustar();
        header.as_old_mut().name[..member.name.len()].copy_from_slice(member.name);
        header.set_size(member.data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(member.kind);
        if member.kind == tar::EntryType::Symlink {
            header.set_link_name("/etc/passwd").unwrap();
        }
        header.set_cksum();
        builder.append(&header, member.data).unwrap();
    }
    let tar = builder.into_inner().unwrap();
    compress_bytes(&tar, dest);
}

/// The members of a valid morphology archive around the database at `db`.
pub(crate) fn morphology_archive(dest: &Path, db: &Path) {
    let db = std::fs::read(db).unwrap();
    tar_zst(
        dest,
        &[
            Member::file("morphology.sqlite", &db),
            Member::file("qac/COPYING", b"GPL"),
            Member::file("qac/NOTICE", b"QAC notice"),
            Member::file("qac/quranic-corpus-morphology-0.4.txt", b"LOCATION\tFORM"),
        ],
    );
}

/// A manifest entry describing `file`, as `siyak-data manifest` would.
pub(crate) fn entry_for(
    file: &Path,
    id: &str,
    kind: &str,
    version: &str,
    installed_size: u64,
) -> PackEntry {
    let data = std::fs::read(file).unwrap();
    let mut value = entry(id, kind, version);
    value["size"] = data.len().into();
    value["installed_size"] = installed_size.into();
    value["sha256"] = hex(&Sha256::digest(&data)).into();
    value["description"] = serde_json::json!({ "tr": "açıklama", "en": "description" });
    value["meta"] = serde_json::json!({ "qac_version": "0.4" });
    parse(&serde_json::to_vec(&manifest_json(vec![value])).unwrap())
        .unwrap()
        .packs
        .remove(0)
}

/// Every row of every table (virtual tables included), as text, for
/// before/after comparisons.
pub(crate) fn dump(conn: &Connection) -> Vec<String> {
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table'
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '%fts_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut rows = Vec::new();
    for table in tables {
        let mut stmt = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let n = stmt.column_count();
        let mut table_rows: Vec<String> = stmt
            .query_map([], |r| {
                let values: Vec<String> = (0..n)
                    .map(|i| format!("{:?}", r.get::<_, rusqlite::types::Value>(i).unwrap()))
                    .collect();
                Ok(format!("{table}: {}", values.join(" | ")))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        table_rows.sort();
        rows.extend(table_rows);
    }
    rows
}
