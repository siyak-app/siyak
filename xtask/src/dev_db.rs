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

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// `(surah, ayah)`, the natural key of a verse in every Tanzil file.
type VerseKey = (u16, u16);

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
    if uthmani.keys().ne(clean.keys()) {
        return Err("Uthmani and Simple Clean fixtures cover different verses".into());
    }
    let packs: Vec<PackFixture> =
        serde_json::from_str(&fs::read_to_string(fixtures.join("packs.json"))?)?;

    // Journal mode stays at the default so the seed is one self-contained file.
    let mut conn = Connection::open(&tmp)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&mut conn, MIGRATIONS)?;

    let tx = conn.transaction()?;
    insert_surahs(&tx, &surahs)?;
    let verse_ids = insert_verses(&tx, &surahs, &juz_starts, &uthmani, &clean)?;
    for pack in &packs {
        let texts = read_tanzil(&fixtures.join(&pack.file))?;
        insert_translation(&tx, pack, &texts, &verse_ids)?;
    }
    // verses_fts is an external-content index over `verses` (spec §5.4).
    tx.execute("INSERT INTO verses_fts(verses_fts) VALUES ('rebuild')", [])?;
    tx.commit()?;
    conn.execute_batch("VACUUM;")?;
    drop(conn);

    fs::rename(&tmp, &out)?;
    println!(
        "wrote {} ({} surahs, {} verses, {} translation pack(s), {} bytes)",
        out.display(),
        surahs.len(),
        verse_ids.len(),
        packs.len(),
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
/// The XML keeps the basmala of each surah in a separate `bismillah` attribute,
/// whereas Tanzil's plain-text format prepends it to the first verse; using the
/// XML keeps `verses.text_*` limited to the verse itself.
fn read_tanzil_xml(path: &Path) -> Result<BTreeMap<VerseKey, String>> {
    let xml = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let mut verses = BTreeMap::new();
    for sura in doc.descendants().filter(|n| n.has_tag_name("sura")) {
        let surah: u16 = sura
            .attribute("index")
            .ok_or("sura without index")?
            .parse()?;
        for aya in sura.children().filter(|n| n.has_tag_name("aya")) {
            let ayah: u16 = aya.attribute("index").ok_or("aya without index")?.parse()?;
            let text = aya.attribute("text").ok_or("aya without text")?;
            verses.insert((surah, ayah), text.to_owned());
        }
    }
    Ok(verses)
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

fn insert_surahs(tx: &Transaction, surahs: &[Surah]) -> Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for s in surahs {
        stmt.execute(params![
            s.number,
            s.name_ar,
            s.name_tr,
            s.name_en,
            s.revelation_type,
            s.ayah_count
        ])?;
    }
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
