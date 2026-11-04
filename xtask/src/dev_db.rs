//! Builds `dev-data/dev-content.db`, the small seed database used by debug
//! builds so contributors never need the data pipeline (spec §16.3).
//!
//! Inputs are the committed files in `dev-data/fixtures/` (see
//! `dev-data/README.md` for sources and licenses). The output is deterministic:
//! running the task twice produces the same content.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use rusqlite::{Connection, Transaction, params};
use serde::Deserialize;
use siyak_lib::db::{content::MIGRATIONS, migrate};
use siyak_lib::search::normalize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// `(surah, ayah)`, the natural key of a verse in every Tanzil file.
type VerseKey = (u16, u16);

/// `content_meta.core_version` of the dev seed (spec §6.2). Raise it whenever
/// the generated rows change, so existing dev `content.db` files are refreshed.
const CORE_VERSION: u32 = 4;

/// Version of the Tanzil text in the fixtures, as in their copyright block.
const TANZIL_VERSION: &str = "1.1";

/// One Tanzil XML file: its copyright block and its text.
struct TanzilText {
    /// The XML comment before `<quran>`, which must ship with the text.
    notice: String,
    /// The basmala Tanzil keeps apart from ayah 1, per surah (absent for 1 and 9).
    bismillah: BTreeMap<u16, String>,
    verses: BTreeMap<VerseKey, String>,
}

struct Surah {
    number: u16,
    name_ar: String,
    name_tr: String,
    name_en: String,
    revelation_type: String,
    ayah_count: u16,
}

/// A row of `packs.json`: metadata for a translation bundled into the seed.
#[derive(Deserialize)]
struct PackFixture {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    lang: String,
    title: String,
    author: String,
    version: String,
    license: String,
    attribution: String,
    installed_at: String,
    bundled: bool,
    file: String,
}

/// Regenerates the development database from the fixtures under `root`.
pub fn build(root: &Path) -> Result<()> {
    let fixtures = root.join("dev-data/fixtures");
    let out = root.join("dev-data/dev-content.db");
    let tmp = out.with_extension("db.tmp");
    if tmp.exists() {
        fs::remove_file(&tmp)?;
    }

    let surahs = read_surahs(&fixtures.join("surahs.tsv"))?;
    let juz_starts = read_juz_starts(&fixtures.join("juz.tsv"))?;
    let uthmani = read_tanzil_xml(&fixtures.join("quran-uthmani.xml"))?;
    let clean = read_tanzil_xml(&fixtures.join("quran-simple-clean.xml"))?;
    if uthmani.verses.keys().ne(clean.verses.keys()) {
        return Err("Uthmani and Simple Clean fixtures cover different verses".into());
    }
    let packs: Vec<PackFixture> =
        serde_json::from_str(&fs::read_to_string(fixtures.join("packs.json"))?)?;

    // Journal mode stays at the default so the seed is one self-contained file.
    let mut conn = Connection::open(&tmp)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&mut conn, MIGRATIONS)?;

    let tx = conn.transaction()?;
    insert_surahs(&tx, &surahs, &uthmani.bismillah)?;
    let verse_ids = insert_verses(&tx, &surahs, &juz_starts, &uthmani.verses, &clean.verses)?;
    for pack in &packs {
        let texts = read_tanzil(&fixtures.join(&pack.file))?;
        insert_translation(&tx, pack, &texts, &verse_ids)?;
    }
    insert_notices(&tx, &uthmani, &clean)?;
    let words = insert_morphology(&tx, &fixtures, &verse_ids)?;
    tx.execute(
        "INSERT INTO content_meta (key, value) VALUES ('core_version', ?1), ('tanzil_version', ?2)",
        params![CORE_VERSION.to_string(), TANZIL_VERSION],
    )?;
    // Filled with the app's port of the pipeline's normalizer, which is tested
    // to reproduce core.db's index (siyak-data docs/normalization.md).
    let clean: Vec<(u32, String)> = tx
        .prepare("SELECT id, text_clean FROM verses ORDER BY id")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, text) in &clean {
        tx.execute(
            "INSERT INTO verses_fts (rowid, text_norm) VALUES (?1, ?2)",
            params![id, normalize(text)],
        )?;
    }
    tx.commit()?;
    conn.execute_batch("VACUUM;")?;
    drop(conn);

    fs::rename(&tmp, &out)?;
    println!(
        "wrote {} ({} surahs, {} verses, {} translation pack(s), {} words, {} bytes)",
        out.display(),
        surahs.len(),
        verse_ids.len(),
        packs.len(),
        words,
        fs::metadata(&out)?.len()
    );
    Ok(())
}

/// Reads tab-separated rows, skipping the header line.
fn tsv_rows(path: &Path) -> Result<Vec<Vec<String>>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect())
}

fn field<'a>(row: &'a [String], idx: usize, path: &Path) -> Result<&'a str> {
    row.get(idx)
        .map(String::as_str)
        .ok_or_else(|| format!("{}: missing column {idx} in {row:?}", path.display()).into())
}

fn read_surahs(path: &Path) -> Result<Vec<Surah>> {
    let surahs = tsv_rows(path)?
        .iter()
        .map(|row| {
            Ok(Surah {
                number: field(row, 0, path)?.parse()?,
                name_ar: field(row, 1, path)?.to_owned(),
                name_tr: field(row, 2, path)?.to_owned(),
                name_en: field(row, 3, path)?.to_owned(),
                revelation_type: field(row, 4, path)?.to_owned(),
                ayah_count: field(row, 5, path)?.parse()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if surahs.len() != 114 {
        return Err(format!("expected 114 surahs, found {}", surahs.len()).into());
    }
    Ok(surahs)
}

/// Returns `(first verse, juz number)` pairs in Mushaf order.
fn read_juz_starts(path: &Path) -> Result<Vec<(VerseKey, u8)>> {
    tsv_rows(path)?
        .iter()
        .map(|row| {
            let key = (field(row, 1, path)?.parse()?, field(row, 2, path)?.parse()?);
            Ok((key, field(row, 0, path)?.parse()?))
        })
        .collect()
}

/// Reads the Arabic text from Tanzil's XML format.
///
/// The XML keeps the basmala of each surah in a separate `bismillah` attribute
/// of its first `<aya>`, whereas Tanzil's plain-text format prepends it to the
/// first verse; using the XML keeps `verses.text_*` limited to the verse itself
/// and gives `surahs.bismillah` Tanzil's own basmala (spec §5.1).
fn read_tanzil_xml(path: &Path) -> Result<TanzilText> {
    let xml = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let mut text = TanzilText {
        notice: read_notice(&xml)
            .ok_or_else(|| format!("{}: no copyright block", path.display()))?,
        bismillah: BTreeMap::new(),
        verses: BTreeMap::new(),
    };
    for sura in doc.descendants().filter(|n| n.has_tag_name("sura")) {
        let surah: u16 = sura
            .attribute("index")
            .ok_or("sura without index")?
            .parse()?;
        for aya in sura.children().filter(|n| n.has_tag_name("aya")) {
            let ayah: u16 = aya.attribute("index").ok_or("aya without index")?.parse()?;
            let verse = aya.attribute("text").ok_or("aya without text")?;
            text.verses.insert((surah, ayah), verse.to_owned());
            if let Some(bismillah) = aya.attribute("bismillah") {
                text.bismillah.insert(surah, bismillah.to_owned());
            }
        }
    }
    Ok(text)
}

/// The copyright block: the first XML comment, trimmed exactly like the
/// pipeline does (`siyak-data/pipeline/tanzil.py`), so both store the same notice.
fn read_notice(xml: &str) -> Option<String> {
    let start = xml.find("<!--")? + "<!--".len();
    let end = start + xml[start..].find("-->")?;
    Some(
        xml[start..end]
            .trim_matches(['\r', '\n', '\t', ' '])
            .to_owned(),
    )
}

/// Parses Tanzil's `surah|ayah|text` translation format, ignoring the header
/// block (`#` lines) that must stay in the fixture files.
fn read_tanzil(path: &Path) -> Result<BTreeMap<VerseKey, String>> {
    let mut verses = BTreeMap::new();
    for line in fs::read_to_string(path)?.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, '|');
        let (Some(surah), Some(ayah), Some(text)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(format!("{}: malformed line {line:?}", path.display()).into());
        };
        verses.insert((surah.parse()?, ayah.parse()?), text.to_owned());
    }
    Ok(verses)
}

/// Inserts all surahs. Only surahs present in the fixtures get a basmala; the
/// others have no verses in the dev seed either.
fn insert_surahs(
    tx: &Transaction,
    surahs: &[Surah],
    bismillah: &BTreeMap<u16, String>,
) -> Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count,
                             bismillah)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for s in surahs {
        stmt.execute(params![
            s.number,
            s.name_ar,
            s.name_tr,
            s.name_en,
            s.revelation_type,
            s.ayah_count,
            bismillah.get(&s.number)
        ])?;
    }
    Ok(())
}

/// The Tanzil copyright blocks, which must ship with the text (spec §5.1).
/// Ids and titles match the rows `siyak-data` writes to `core.db`.
fn insert_notices(tx: &Transaction, uthmani: &TanzilText, clean: &TanzilText) -> Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO notices (id, source, title, body, url)
         VALUES (?1, 'tanzil', ?2, ?3, 'https://tanzil.net')",
    )?;
    stmt.execute(params![
        "tanzil.text.simple-clean",
        "Tanzil Quran Text (Simple Clean)",
        clean.notice
    ])?;
    stmt.execute(params![
        "tanzil.text.uthmani",
        "Tanzil Quran Text (Uthmani)",
        uthmani.notice
    ])?;
    Ok(())
}

/// Inserts the fixture verses with their global ids (1..6236, derived from
/// the surah lengths) and juz numbers. Returns the id of every inserted verse.
fn insert_verses(
    tx: &Transaction,
    surahs: &[Surah],
    juz_starts: &[(VerseKey, u8)],
    uthmani: &BTreeMap<VerseKey, String>,
    clean: &BTreeMap<VerseKey, String>,
) -> Result<BTreeMap<VerseKey, u32>> {
    let mut first_id = BTreeMap::new();
    let mut next = 1u32;
    for s in surahs {
        first_id.insert(s.number, next);
        next += u32::from(s.ayah_count);
    }

    let mut stmt = tx.prepare(
        "INSERT INTO verses (id, surah, ayah, juz, text_uthmani, text_clean)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut ids = BTreeMap::new();
    for (&key @ (surah, ayah), text_uthmani) in uthmani {
        let base = first_id.get(&surah).ok_or("verse in unknown surah")?;
        let id = base + u32::from(ayah) - 1;
        let juz = juz_starts
            .iter()
            .rev()
            .find(|(start, _)| *start <= key)
            .map(|&(_, juz)| juz)
            .ok_or("verse before the first juz")?;
        let text_clean = clean.get(&key).ok_or("verse missing from Simple Clean")?;
        stmt.execute(params![id, surah, ayah, juz, text_uthmani, text_clean])?;
        ids.insert(key, id);
    }
    Ok(ids)
}

fn insert_translation(
    tx: &Transaction,
    pack: &PackFixture,
    texts: &BTreeMap<VerseKey, String>,
    verse_ids: &BTreeMap<VerseKey, u32>,
) -> Result<()> {
    tx.execute(
        "INSERT INTO packs (id, type, lang, title, author, version, license, attribution,
                            installed_at, bundled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            pack.id,
            pack.kind,
            pack.lang,
            pack.title,
            pack.author,
            pack.version,
            pack.license,
            pack.attribution,
            pack.installed_at,
            pack.bundled
        ],
    )?;
    let mut row =
        tx.prepare("INSERT INTO translations (pack_id, verse_id, text) VALUES (?1, ?2, ?3)")?;
    let mut fts =
        tx.prepare("INSERT INTO translations_fts (text, pack_id, verse_id) VALUES (?1, ?2, ?3)")?;
    for (key, verse_id) in verse_ids {
        let text = texts
            .get(key)
            .ok_or_else(|| format!("{} has no text for {key:?}", pack.id))?;
        row.execute(params![pack.id, verse_id, text])?;
        fts.execute(params![text, pack.id, verse_id])?;
    }
    Ok(())
}

/// Fills the morphology tables and the pack's row from `morphology.tsv`, an
/// excerpt of the `morphology.qac` tables for the surahs in the seed. Ids follow
/// the first occurrence, as in the pack. Returns the number of words.
fn insert_morphology(
    tx: &Transaction,
    fixtures: &Path,
    verse_ids: &BTreeMap<VerseKey, u32>,
) -> Result<u32> {
    tx.execute(
        "INSERT INTO packs (id, type, lang, title, author, version, license, attribution,
                            citation_json, installed_at, bundled)
         VALUES ('morphology.qac', 'morphology', 'ar', ?1, 'Kais Dukes', '1.0.0',
                 'GNU General Public License (QAC terms of use)',
                 'Source: Quranic Arabic Corpus (corpus.quran.com)',
                 '{\"url\":\"https://corpus.quran.com\"}', '2026-10-21T00:00:00Z', 0)",
        ["Quranic Arabic Corpus: word morphology"],
    )?;
    tx.execute(
        "INSERT INTO notices (id, source, title, body, url)
         VALUES ('morphology.qac', 'morphology.qac', 'Quranic Arabic Corpus', ?1,
                 'https://corpus.quran.com')",
        [fs::read_to_string(fixtures.join("qac-NOTICE.txt"))?],
    )?;

    let path = fixtures.join("morphology.tsv");
    let mut roots: BTreeMap<String, u32> = BTreeMap::new();
    let mut lemmas: BTreeMap<String, u32> = BTreeMap::new();
    let mut word_id = 0;
    let mut current: Option<(u32, u16)> = None;
    for row in tsv_rows(&path)? {
        let column = |idx| field(&row, idx, &path);
        let key: VerseKey = (column(0)?.parse()?, column(1)?.parse()?);
        let verse_id = *verse_ids
            .get(&key)
            .ok_or_else(|| format!("morphology.tsv: no verse {key:?}"))?;
        let position: u16 = column(2)?.parse()?;
        if current != Some((verse_id, position)) {
            current = Some((verse_id, position));
            word_id += 1;
            tx.execute(
                "INSERT INTO words (id, verse_id, position, text_ar) VALUES (?1, ?2, ?3, ?4)",
                params![word_id, verse_id, position, column(3)?],
            )?;
        }
        let root_id = match column(9)? {
            "" => None,
            root => Some(intern(
                tx,
                &mut roots,
                root,
                "INSERT INTO roots (id, root) VALUES (?1, ?2)",
            )?),
        };
        let lemma_id = match column(8)? {
            "" => None,
            lemma => {
                let id = intern(
                    tx,
                    &mut lemmas,
                    lemma,
                    "INSERT INTO lemmas (id, lemma) VALUES (?1, ?2)",
                )?;
                tx.execute(
                    "UPDATE lemmas SET root_id = ?2 WHERE id = ?1",
                    params![id, root_id],
                )?;
                Some(id)
            }
        };
        let features = column(10)?;
        tx.execute(
            "INSERT INTO word_segments
               (word_id, idx, kind, text_ar, pos, lemma_id, root_id, features)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                word_id,
                column(4)?.parse::<u16>()?,
                column(5)?,
                column(6)?,
                column(7)?,
                lemma_id,
                root_id,
                (!features.is_empty()).then_some(features),
            ],
        )?;
    }
    Ok(word_id)
}

/// The id of `value`, inserting it with the next id on its first occurrence.
fn intern(
    tx: &Transaction,
    known: &mut BTreeMap<String, u32>,
    value: &str,
    insert: &str,
) -> Result<u32> {
    if let Some(&id) = known.get(value) {
        return Ok(id);
    }
    let id = u32::try_from(known.len())? + 1;
    tx.execute(insert, params![id, value])?;
    known.insert(value.to_owned(), id);
    Ok(id)
}
