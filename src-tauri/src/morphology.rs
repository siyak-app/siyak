//! Morphology: words, segments, roots, lemmas and the concordance (spec §5.3, §7.3, §7.5).
//!
//! Data comes from the Quranic Arabic Corpus pack and is presented unmodified
//! (ADR 0007). An occurrence is a word: a word with two segments of one root
//! counts once.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::db::content::VerseTranslation;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::search::{is_arabic, is_latin_root, normalize};

/// Largest number of verses returned at once.
const MAX_LIMIT: u32 = 2000;

/// The installed morphology pack (spec §5.3.1: its source is shown with the data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct MorphologyPack {
    /// Pack id, e.g. `morphology.qac`.
    pub id: String,
    /// Who to credit, e.g. `Quranic Arabic Corpus (corpus.quran.com)`.
    pub source: String,
}

/// One word of a verse. The words of a verse, joined by spaces, are its
/// `text_uthmani` (the pack's alignment gate checks this).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct VerseWord {
    /// Global word id.
    pub id: u32,
    /// Position in the verse, from 1.
    pub position: u16,
    /// Uthmani script; two tokens for the four words merged by the alignment.
    pub text_ar: String,
}

/// Which part of its word a segment is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// Before the stem.
    Prefix,
    /// The stem.
    Stem,
    /// After the stem.
    Suffix,
}

/// A root and how many words have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct RootInfo {
    /// Root id.
    pub id: u32,
    /// Spaced Arabic letters, e.g. `ر ح م`.
    pub root: String,
    /// Words with this root.
    pub occurrences: u32,
}

/// A lemma and how many words have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct LemmaInfo {
    /// Lemma id.
    pub id: u32,
    /// As in the source, including a trailing homograph digit (`عَاد2`).
    pub lemma: String,
    /// Words with this lemma.
    pub occurrences: u32,
}

/// One grammatical feature of a segment; both are the pack's stable codes
/// (`siyak-data/docs/pack-format.md` §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct WordFeature {
    /// Feature key, e.g. `case`.
    pub key: String,
    /// Feature value, e.g. `GEN`.
    pub value: String,
}

/// A prefix, stem or suffix of a word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct WordSegment {
    /// Prefix, stem or suffix.
    pub kind: SegmentKind,
    /// The segment in Arabic.
    pub text_ar: String,
    /// Part-of-speech tag, e.g. `N`.
    pub pos: String,
    /// Its root, if it has one.
    pub root: Option<RootInfo>,
    /// Its lemma, if it has one.
    pub lemma: Option<LemmaInfo>,
    /// Its features, sorted by key.
    pub features: Vec<WordFeature>,
}

/// A word with its full analysis (spec §7.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct WordDetail {
    /// Global word id.
    pub id: u32,
    /// Surah number.
    pub surah: u16,
    /// Verse number within the surah.
    pub ayah: u16,
    /// Position in the verse, from 1.
    pub position: u16,
    /// Uthmani script.
    pub text_ar: String,
    /// Transliteration, if the pack has one.
    pub translit: Option<String>,
    /// English meaning, if the pack has one.
    pub gloss_en: Option<String>,
    /// Its segments, in order.
    pub segments: Vec<WordSegment>,
}

/// What a concordance lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ConcordanceTarget {
    /// Every word with this root.
    Root {
        /// Root id.
        id: u32,
    },
    /// Every word with this lemma.
    Lemma {
        /// Lemma id.
        id: u32,
    },
}

/// Occurrences within one surah.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct SurahCount {
    /// Surah number.
    pub surah: u16,
    /// Words in this surah.
    pub occurrences: u32,
}

/// A word of a concordance verse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ConcordanceWord {
    /// Uthmani script.
    pub text_ar: String,
    /// Whether this word is what the concordance lists.
    pub matched: bool,
}

/// A verse of a concordance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ConcordanceVerse {
    /// Surah number.
    pub surah: u16,
    /// Verse number within the surah.
    pub ayah: u16,
    /// The words of the verse, in order.
    pub words: Vec<ConcordanceWord>,
    /// The requested translation, if the verse has one.
    pub translation: Option<VerseTranslation>,
}

/// Every occurrence of a root or lemma (spec §7.3, §13.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Concordance {
    /// The root, or the lemma's root.
    pub root: Option<RootInfo>,
    /// The lemma, when the concordance is narrowed to one.
    pub lemma: Option<LemmaInfo>,
    /// The lemmas of the root, most frequent first; for a lemma without a
    /// root, the lemma alone.
    pub lemmas: Vec<LemmaInfo>,
    /// Words listed.
    pub occurrences: u32,
    /// Verses containing them.
    pub verse_count: u32,
    /// Occurrences per surah, in Mushaf order.
    pub by_surah: Vec<SurahCount>,
    /// The requested page of verses, in Mushaf order.
    pub verses: Vec<ConcordanceVerse>,
}

/// Roots and lemmas matching a query.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
pub struct RootSearch {
    /// Matching roots, most frequent first.
    pub roots: Vec<RootInfo>,
    /// Matching lemmas, most frequent first.
    pub lemmas: Vec<LemmaInfo>,
}

/// The installed morphology pack, if any.
pub fn installed_pack(conn: &Connection) -> AppResult<Option<MorphologyPack>> {
    Ok(conn
        .query_row(
            "SELECT id, attribution, title FROM packs WHERE type = 'morphology'",
            [],
            |row| {
                let attribution: Option<String> = row.get(1)?;
                let source = match attribution {
                    Some(text) => text
                        .strip_prefix("Source: ")
                        .map_or_else(|| text.clone(), str::to_owned),
                    None => row.get(2)?,
                };
                Ok(MorphologyPack {
                    id: row.get(0)?,
                    source,
                })
            },
        )
        .optional()?)
}

/// The words of a verse, in order; empty without a morphology pack.
pub fn get_verse_words(conn: &Connection, verse_id: u32) -> AppResult<Vec<VerseWord>> {
    Ok(conn
        .prepare("SELECT id, position, text_ar FROM words WHERE verse_id = ?1 ORDER BY position")?
        .query_map([verse_id], |row| {
            Ok(VerseWord {
                id: row.get(0)?,
                position: row.get(1)?,
                text_ar: row.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

/// A word with its segments. Fails with [`AppErrorCode::NotFound`] for an
/// unknown id, e.g. one kept while the morphology pack was replaced.
pub fn get_word(conn: &Connection, word_id: u32) -> AppResult<WordDetail> {
    let mut word = conn
        .query_row(
            "SELECT w.id, v.surah, v.ayah, w.position, w.text_ar, w.translit, w.gloss_en
             FROM words w JOIN verses v ON v.id = w.verse_id
             WHERE w.id = ?1",
            [word_id],
            |row| {
                Ok(WordDetail {
                    id: row.get(0)?,
                    surah: row.get(1)?,
                    ayah: row.get(2)?,
                    position: row.get(3)?,
                    text_ar: row.get(4)?,
                    translit: row.get(5)?,
                    gloss_en: row.get(6)?,
                    segments: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("word {word_id}")))?;

    let rows = conn
        .prepare(
            "SELECT kind, text_ar, pos, root_id, lemma_id, features
             FROM word_segments WHERE word_id = ?1 ORDER BY idx",
        )?
        .query_map([word_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<u32>>(3)?,
                row.get::<_, Option<u32>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (kind, text_ar, pos, root_id, lemma_id, features) in rows {
        word.segments.push(WordSegment {
            kind: segment_kind(&kind),
            text_ar,
            pos,
            root: root_id.map(|id| root_info(conn, id)).transpose()?,
            lemma: lemma_id.map(|id| lemma_info(conn, id)).transpose()?,
            features: features.as_deref().map(parse_features).unwrap_or_default(),
        });
    }
    Ok(word)
}

/// The table's `CHECK` allows only these three.
fn segment_kind(kind: &str) -> SegmentKind {
    match kind {
        "prefix" => SegmentKind::Prefix,
        "suffix" => SegmentKind::Suffix,
        _ => SegmentKind::Stem,
    }
}

/// `{"case":"GEN"}` as a list, sorted by key.
fn parse_features(json: &str) -> Vec<WordFeature> {
    serde_json::from_str::<std::collections::BTreeMap<String, String>>(json)
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| WordFeature { key, value })
        .collect()
}

fn root_info(conn: &Connection, id: u32) -> AppResult<RootInfo> {
    let root = conn.query_row("SELECT root FROM roots WHERE id = ?1", [id], |row| {
        row.get(0)
    })?;
    Ok(RootInfo {
        id,
        root,
        occurrences: count_words(conn, "s.root_id", id)?,
    })
}

fn lemma_info(conn: &Connection, id: u32) -> AppResult<LemmaInfo> {
    let lemma = conn.query_row("SELECT lemma FROM lemmas WHERE id = ?1", [id], |row| {
        row.get(0)
    })?;
    Ok(LemmaInfo {
        id,
        lemma,
        occurrences: count_words(conn, "s.lemma_id", id)?,
    })
}

/// `column` is one of two constants, never user input.
fn count_words(conn: &Connection, column: &str, id: u32) -> AppResult<u32> {
    Ok(conn.query_row(
        &format!("SELECT count(DISTINCT s.word_id) FROM word_segments s WHERE {column} = ?1"),
        [id],
        |row| row.get(0),
    )?)
}

/// Every occurrence of a root or lemma: the summary and one page of verses.
/// Fails with [`AppErrorCode::NotFound`] for an unknown id.
pub fn concordance(
    conn: &Connection,
    target: ConcordanceTarget,
    translation_id: Option<&str>,
    limit: u32,
    offset: u32,
) -> AppResult<Concordance> {
    let (column, root, lemma) = match target {
        ConcordanceTarget::Root { id } => {
            let root = known(root_lookup(conn, id)?, "root", id)?;
            ("s.root_id", Some(root), None)
        }
        ConcordanceTarget::Lemma { id } => {
            let (lemma, root_id) = known(lemma_lookup(conn, id)?, "lemma", id)?;
            let root = root_id.map(|root| root_info(conn, root)).transpose()?;
            ("s.lemma_id", root, Some(lemma))
        }
    };
    let id = match target {
        ConcordanceTarget::Root { id } | ConcordanceTarget::Lemma { id } => id,
    };
    let lemmas = match (&root, &lemma) {
        (Some(root), _) => lemmas_of_root(conn, root.id)?,
        (None, Some(lemma)) => vec![lemma.clone()],
        (None, None) => Vec::new(),
    };
    let by_surah = conn
        .prepare(&format!(
            "SELECT v.surah, count(DISTINCT s.word_id)
             FROM word_segments s
             JOIN words w ON w.id = s.word_id
             JOIN verses v ON v.id = w.verse_id
             WHERE {column} = ?1
             GROUP BY v.surah ORDER BY v.surah"
        ))?
        .query_map([id], |row| {
            Ok(SurahCount {
                surah: row.get(0)?,
                occurrences: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let verse_count = conn.query_row(
        &format!(
            "SELECT count(DISTINCT w.verse_id)
             FROM word_segments s JOIN words w ON w.id = s.word_id
             WHERE {column} = ?1"
        ),
        [id],
        |row| row.get(0),
    )?;
    Ok(Concordance {
        occurrences: count_words(conn, column, id)?,
        verse_count,
        by_surah,
        verses: concordance_verses(conn, column, id, translation_id, limit, offset)?,
        root,
        lemma,
        lemmas,
    })
}

fn known<T>(found: Option<T>, what: &str, id: u32) -> AppResult<T> {
    found.ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("{what} {id}")))
}

fn root_lookup(conn: &Connection, id: u32) -> AppResult<Option<RootInfo>> {
    let exists = conn
        .query_row("SELECT 1 FROM roots WHERE id = ?1", [id], |_| Ok(()))
        .optional()?;
    exists.map(|()| root_info(conn, id)).transpose()
}

/// The lemma and the id of its root.
fn lemma_lookup(conn: &Connection, id: u32) -> AppResult<Option<(LemmaInfo, Option<u32>)>> {
    let root_id: Option<Option<u32>> = conn
        .query_row("SELECT root_id FROM lemmas WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?;
    root_id
        .map(|root_id| Ok((lemma_info(conn, id)?, root_id)))
        .transpose()
}

fn lemmas_of_root(conn: &Connection, root_id: u32) -> AppResult<Vec<LemmaInfo>> {
    Ok(conn
        .prepare(
            "SELECT l.id, l.lemma, count(DISTINCT s.word_id) AS occurrences
             FROM lemmas l JOIN word_segments s ON s.lemma_id = l.id
             WHERE l.root_id = ?1
             GROUP BY l.id ORDER BY occurrences DESC, l.id",
        )?
        .query_map([root_id], |row| {
            Ok(LemmaInfo {
                id: row.get(0)?,
                lemma: row.get(1)?,
                occurrences: row.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

fn concordance_verses(
    conn: &Connection,
    column: &str,
    id: u32,
    translation_id: Option<&str>,
    limit: u32,
    offset: u32,
) -> AppResult<Vec<ConcordanceVerse>> {
    let ids = conn
        .prepare(&format!(
            "SELECT DISTINCT w.verse_id
             FROM word_segments s JOIN words w ON w.id = s.word_id
             WHERE {column} = ?1
             ORDER BY w.verse_id LIMIT ?2 OFFSET ?3"
        ))?
        .query_map((id, limit.min(MAX_LIMIT), offset), |row| {
            row.get::<_, u32>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut position = conn.prepare("SELECT surah, ayah FROM verses WHERE id = ?1")?;
    let mut words = conn.prepare(&format!(
        "SELECT w.text_ar,
                EXISTS (SELECT 1 FROM word_segments s WHERE s.word_id = w.id AND {column} = ?2)
         FROM words w WHERE w.verse_id = ?1 ORDER BY w.position"
    ))?;
    let mut translation = conn.prepare(
        "SELECT p.lang, t.text FROM translations t JOIN packs p ON p.id = t.pack_id
         WHERE t.pack_id = ?1 AND t.verse_id = ?2",
    )?;
    let mut verses = Vec::with_capacity(ids.len());
    for verse_id in ids {
        let (surah, ayah) = position.query_row([verse_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        verses.push(ConcordanceVerse {
            surah,
            ayah,
            words: words
                .query_map((verse_id, id), |row| {
                    Ok(ConcordanceWord {
                        text_ar: row.get(0)?,
                        matched: row.get(1)?,
                    })
                })?
                .collect::<Result<_, _>>()?,
            translation: translation_id
                .map(|pack_id| {
                    translation
                        .query_row((pack_id, verse_id), |row| {
                            Ok(VerseTranslation {
                                pack_id: pack_id.to_owned(),
                                lang: row.get(0)?,
                                text: row.get(1)?,
                            })
                        })
                        .optional()
                })
                .transpose()?
                .flatten(),
        });
    }
    Ok(verses)
}

/// Finds roots and lemmas by an Arabic query (`ص ب ر`, `صبر`, `الصبر`) or a
/// Latin-script root (`s-b-r`). The tables hold a few thousand short rows, so
/// comparing them all is fast enough and needs no index.
pub fn search_roots(conn: &Connection, query: &str) -> AppResult<RootSearch> {
    let query = query.trim();
    let mut found = if is_latin_root(query) {
        search_latin_root(conn, query)?
    } else if query.chars().any(is_arabic) {
        search_arabic(conn, query)?
    } else {
        RootSearch::default()
    };
    found
        .roots
        .sort_by(|a, b| b.occurrences.cmp(&a.occurrences).then(a.id.cmp(&b.id)));
    found
        .lemmas
        .sort_by(|a, b| b.occurrences.cmp(&a.occurrences).then(a.id.cmp(&b.id)));
    Ok(found)
}

fn search_arabic(conn: &Connection, query: &str) -> AppResult<RootSearch> {
    let key = lookup_key(query);
    let exact = matches_key(conn, &key)?;
    if !exact.roots.is_empty() || !exact.lemmas.is_empty() {
        return Ok(exact);
    }
    // `الصبر` is the word; the lemma is `صبر`.
    match key.strip_prefix("ال") {
        Some(rest) if rest.chars().count() >= 2 => matches_key(conn, rest),
        _ => Ok(exact),
    }
}

fn matches_key(conn: &Connection, key: &str) -> AppResult<RootSearch> {
    let root_ids = ids_where(conn, "SELECT id, root FROM roots", |text| {
        lookup_key(text) == key
    })?;
    let lemma_ids = ids_where(conn, "SELECT id, lemma FROM lemmas", |text| {
        lookup_key(text) == key
    })?;
    Ok(RootSearch {
        roots: root_ids
            .into_iter()
            .map(|id| root_info(conn, id))
            .collect::<AppResult<_>>()?,
        lemmas: lemma_ids
            .into_iter()
            .map(|id| lemma_info(conn, id))
            .collect::<AppResult<_>>()?,
    })
}

fn search_latin_root(conn: &Connection, query: &str) -> AppResult<RootSearch> {
    let lowered = query.to_lowercase();
    let letters: Option<Vec<&[char]>> = lowered.split('-').map(latin_letter).collect();
    let Some(letters) = letters else {
        return Ok(RootSearch::default());
    };
    let root_ids = ids_where(conn, "SELECT id, root FROM roots", |text| {
        let key = lookup_key(text);
        key.chars().count() == letters.len()
            && key
                .chars()
                .zip(&letters)
                .all(|(letter, options)| options.contains(&letter))
    })?;
    Ok(RootSearch {
        roots: root_ids
            .into_iter()
            .map(|id| root_info(conn, id))
            .collect::<AppResult<_>>()?,
        lemmas: Vec::new(),
    })
}

/// Ids of the rows of `sql` (id, text) whose text passes `keep`.
fn ids_where(conn: &Connection, sql: &str, keep: impl Fn(&str) -> bool) -> AppResult<Vec<u32>> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        let text: String = row.get(1)?;
        if keep(&text) {
            ids.push(row.get(0)?);
        }
    }
    Ok(ids)
}

/// What root and lemma lookups compare: the search normalization, with every
/// hamza spelling as alef because QAC writes the root of `ءامن` as `ا م ن`, and
/// without spaces or a lemma's homograph digit.
fn lookup_key(text: &str) -> String {
    let hamza_as_alef: String = text
        .chars()
        .map(|c| {
            if matches!(c, 'ء' | 'ئ' | 'ؤ') {
                'ا'
            } else {
                c
            }
        })
        .collect();
    normalize(&hamza_as_alef)
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_digit())
        .collect()
}

/// The Arabic letters a group of a Latin-script root may stand for. Latin
/// spelling merges some letters, so `s-b-r` finds both `س ب ر` and `ص ب ر`.
/// The letters are those of [`lookup_key`]: hamza is alef.
fn latin_letter(group: &str) -> Option<&'static [char]> {
    Some(match group {
        "a" | "e" | "'" | "`" | "ʿ" | "ʾ" => &['ا', 'ع'],
        "b" => &['ب'],
        "t" => &['ت', 'ط'],
        "ṭ" => &['ط'],
        "th" => &['ث'],
        "j" | "c" => &['ج'],
        "h" => &['ه', 'ح'],
        "ḥ" => &['ح'],
        "kh" | "x" => &['خ'],
        "d" => &['د', 'ض'],
        "ḍ" => &['ض'],
        "dh" => &['ذ'],
        "r" => &['ر'],
        "z" => &['ز', 'ظ'],
        "ẓ" => &['ظ'],
        "s" => &['س', 'ص'],
        "ṣ" => &['ص'],
        "sh" | "ş" => &['ش'],
        "g" | "gh" | "ğ" => &['غ'],
        "f" => &['ف'],
        "q" => &['ق'],
        "k" => &['ك', 'ق'],
        "l" => &['ل'],
        "m" => &['م'],
        "n" => &['ن'],
        "w" | "v" => &['و'],
        "y" => &['ي'],
        _ => return None,
    })
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "a failing unwrap or index is a test failure"
)]
mod tests {
    use super::*;
    use crate::db::content::MIGRATIONS;
    use crate::db::migrate;

    /// Two verses of 1 and a verse of 2, with rows shaped like QAC's:
    /// - 1:1 بِسْمِ (prefix, stem) ٱلرَّحْمَـٰنِ (prefix, stem)
    /// - 1:2 رَحِمَ رَبِّ
    /// - 2:1 رَحْمَة (two stems of one root) سَأَلَ
    fn database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        conn.execute_batch(
            "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count)
               VALUES (1, 'a', 'a', 'a', 'meccan', 2), (2, 'b', 'b', 'b', 'medinan', 1);
             INSERT INTO verses (id, surah, ayah, juz, text_uthmani, text_clean) VALUES
               (1, 1, 1, 1, 'x', 'x'), (2, 1, 2, 1, 'y', 'y'), (3, 2, 1, 1, 'z', 'z');
             INSERT INTO packs (id, type, title, version, license, attribution, installed_at)
               VALUES ('morphology.qac', 'morphology', 'QAC', '1.0.0', 'GPL',
                       'Source: Quranic Arabic Corpus (corpus.quran.com)', 'now'),
                      ('en.test', 'translation', 'Test', '1.0.0', 'PD', NULL, 'now');
             UPDATE packs SET lang = 'en' WHERE id = 'en.test';
             INSERT INTO translations (pack_id, verse_id, text) VALUES ('en.test', 2, 'He had mercy');
             INSERT INTO roots (id, root) VALUES
               (1, 'س م و'), (2, 'ر ح م'), (3, 'ر ب ب'), (4, 'س ا ل');
             INSERT INTO lemmas (id, lemma, root_id) VALUES
               (1, 'ٱسْم', 1), (2, 'رَحْمَٰن', 2), (3, 'رَحِمَ', 2), (4, 'رَبّ', 3),
               (5, 'رَحْمَة', 2), (6, 'سَأَلَ', 4), (7, 'عَاد2', NULL);
             INSERT INTO words (id, verse_id, position, text_ar) VALUES
               (1, 1, 1, 'بِسْمِ'), (2, 1, 2, 'ٱلرَّحْمَـٰنِ'),
               (3, 2, 1, 'رَحِمَ'), (4, 2, 2, 'رَبِّ'),
               (5, 3, 1, 'رَحْمَة'), (6, 3, 2, 'سَأَلَ');
             INSERT INTO word_segments
               (word_id, idx, kind, text_ar, pos, lemma_id, root_id, features) VALUES
               (1, 1, 'prefix', 'بِ', 'P', NULL, NULL, '{\"prefix\":\"bi+\"}'),
               (1, 2, 'stem', 'سْمِ', 'N', 1, 1, '{\"gender\":\"M\",\"case\":\"GEN\"}'),
               (2, 1, 'prefix', 'ٱل', 'DET', NULL, NULL, NULL),
               (2, 2, 'stem', 'رَّحْمَٰنِ', 'ADJ', 2, 2, NULL),
               (3, 1, 'stem', 'رَحِمَ', 'V', 3, 2, NULL),
               (4, 1, 'stem', 'رَبِّ', 'N', 4, 3, NULL),
               (5, 1, 'stem', 'رَحْ', 'N', 5, 2, NULL),
               (5, 2, 'stem', 'مَة', 'N', 5, 2, NULL),
               (6, 1, 'stem', 'سَأَلَ', 'V', 6, 4, NULL);",
        )
        .unwrap();
        conn
    }

    fn root_names(found: &RootSearch) -> Vec<&str> {
        found.roots.iter().map(|r| r.root.as_str()).collect()
    }

    fn lemma_names(found: &RootSearch) -> Vec<&str> {
        found.lemmas.iter().map(|l| l.lemma.as_str()).collect()
    }

    #[test]
    fn reports_the_installed_pack_and_its_source() {
        let conn = database();
        let pack = installed_pack(&conn).unwrap().unwrap();
        assert_eq!(pack.id, "morphology.qac");
        assert_eq!(pack.source, "Quranic Arabic Corpus (corpus.quran.com)");
        conn.execute("DELETE FROM packs WHERE type = 'morphology'", [])
            .unwrap();
        assert_eq!(installed_pack(&conn).unwrap(), None);
    }

    #[test]
    fn lists_the_words_of_a_verse_in_order() {
        let words = get_verse_words(&database(), 1).unwrap();
        assert_eq!(words.iter().map(|w| w.position).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(words[0].id, 1);
        assert_eq!(words[0].text_ar, "بِسْمِ");
    }

    #[test]
    fn returns_a_word_with_its_segments_and_counts() {
        let word = get_word(&database(), 1).unwrap();
        assert_eq!((word.surah, word.ayah, word.position), (1, 1, 1));
        assert_eq!(word.segments.len(), 2);
        assert_eq!(word.segments[0].kind, SegmentKind::Prefix);
        assert_eq!(word.segments[0].root, None);
        let stem = &word.segments[1];
        assert_eq!(stem.kind, SegmentKind::Stem);
        assert_eq!(stem.root.as_ref().unwrap().root, "س م و");
        assert_eq!(stem.lemma.as_ref().unwrap().lemma, "ٱسْم");
        assert_eq!(
            stem.features,
            [
                WordFeature {
                    key: "case".into(),
                    value: "GEN".into()
                },
                WordFeature {
                    key: "gender".into(),
                    value: "M".into()
                }
            ]
        );
    }

    #[test]
    fn counts_a_word_with_two_segments_of_a_root_once() {
        let word = get_word(&database(), 5).unwrap();
        assert_eq!(word.segments[0].root.as_ref().unwrap().occurrences, 3);
        assert_eq!(word.segments[0].lemma.as_ref().unwrap().occurrences, 1);
    }

    #[test]
    fn an_unknown_word_or_target_is_not_found() {
        let conn = database();
        assert_eq!(
            get_word(&conn, 99).unwrap_err().code,
            AppErrorCode::NotFound
        );
        for target in [
            ConcordanceTarget::Root { id: 99 },
            ConcordanceTarget::Lemma { id: 99 },
        ] {
            assert_eq!(
                concordance(&conn, target, None, 10, 0).unwrap_err().code,
                AppErrorCode::NotFound
            );
        }
    }

    #[test]
    fn a_root_concordance_summarizes_and_marks_the_words() {
        let conn = database();
        let found = concordance(
            &conn,
            ConcordanceTarget::Root { id: 2 },
            Some("en.test"),
            10,
            0,
        )
        .unwrap();
        assert_eq!(found.root.as_ref().unwrap().root, "ر ح م");
        assert_eq!(found.lemma, None);
        assert_eq!(found.occurrences, 3);
        assert_eq!(found.verse_count, 3);
        assert_eq!(found.lemmas.len(), 3);
        assert_eq!(
            found.by_surah,
            [
                SurahCount {
                    surah: 1,
                    occurrences: 2
                },
                SurahCount {
                    surah: 2,
                    occurrences: 1
                }
            ]
        );
        let verse = &found.verses[1];
        assert_eq!((verse.surah, verse.ayah), (1, 2));
        assert_eq!(
            verse.words.iter().map(|w| w.matched).collect::<Vec<_>>(),
            [true, false]
        );
        assert_eq!(verse.translation.as_ref().unwrap().text, "He had mercy");
        assert_eq!(found.verses[0].translation, None);
    }

    #[test]
    fn a_lemma_concordance_is_narrowed_and_knows_its_root() {
        let found =
            concordance(&database(), ConcordanceTarget::Lemma { id: 3 }, None, 10, 0).unwrap();
        assert_eq!(found.lemma.as_ref().unwrap().lemma, "رَحِمَ");
        assert_eq!(found.root.as_ref().unwrap().root, "ر ح م");
        assert_eq!(found.occurrences, 1);
        assert_eq!(found.lemmas.len(), 3);
        assert_eq!(found.verses.len(), 1);
    }

    #[test]
    fn a_lemma_without_a_root_lists_itself() {
        let found =
            concordance(&database(), ConcordanceTarget::Lemma { id: 7 }, None, 10, 0).unwrap();
        assert_eq!(found.root, None);
        assert_eq!(found.lemmas.len(), 1);
        assert_eq!(found.occurrences, 0);
    }

    #[test]
    fn pages_through_the_verses() {
        let conn = database();
        let target = ConcordanceTarget::Root { id: 2 };
        let page = concordance(&conn, target, None, 2, 1).unwrap();
        assert_eq!(page.occurrences, 3);
        assert_eq!(
            page.verses
                .iter()
                .map(|v| (v.surah, v.ayah))
                .collect::<Vec<_>>(),
            [(1, 2), (2, 1)]
        );
    }

    #[test]
    fn finds_a_root_in_any_arabic_spelling() {
        let conn = database();
        for query in ["ر ح م", "رحم", " رَحْم ", "ر ح م "] {
            assert_eq!(
                root_names(&search_roots(&conn, query).unwrap()),
                ["ر ح م"],
                "{query}"
            );
        }
    }

    #[test]
    fn hamza_spellings_find_the_root_written_with_alef() {
        let conn = database();
        for query in ["سأل", "س ء ل", "سئل", "سؤل", "سال"] {
            assert_eq!(
                root_names(&search_roots(&conn, query).unwrap()),
                ["س ا ل"],
                "{query}"
            );
        }
    }

    #[test]
    fn finds_a_lemma_with_or_without_the_article() {
        let conn = database();
        assert_eq!(lemma_names(&search_roots(&conn, "رحمن").unwrap()), ["رَحْمَٰن"]);
        assert_eq!(
            lemma_names(&search_roots(&conn, "الرحمن").unwrap()),
            ["رَحْمَٰن"]
        );
        // The homograph digit is kept in the data but never typed.
        assert_eq!(lemma_names(&search_roots(&conn, "عاد").unwrap()), ["عَاد2"]);
    }

    #[test]
    fn a_root_and_its_homonymous_lemma_are_both_found() {
        let found = search_roots(&database(), "رحم").unwrap();
        assert_eq!(root_names(&found), ["ر ح م"]);
        assert_eq!(lemma_names(&found), ["رَحِمَ"]);
    }

    #[test]
    fn a_latin_root_finds_every_letter_it_may_stand_for() {
        let conn = database();
        assert_eq!(
            root_names(&search_roots(&conn, "r-h-m").unwrap()),
            ["ر ح م"]
        );
        assert_eq!(
            root_names(&search_roots(&conn, "R-H-M").unwrap()),
            ["ر ح م"]
        );
        assert_eq!(
            root_names(&search_roots(&conn, "s-m-w").unwrap()),
            ["س م و"]
        );
        assert_eq!(
            root_names(&search_roots(&conn, "s-'-l").unwrap()),
            ["س ا ل"]
        );
        conn.execute("INSERT INTO roots (id, root) VALUES (9, 'ص م و')", [])
            .unwrap();
        assert_eq!(search_roots(&conn, "s-m-w").unwrap().roots.len(), 2);
    }

    #[test]
    fn most_frequent_matches_come_first() {
        let conn = database();
        // `h` stands for both ح and ه; this root has no words at all.
        conn.execute("INSERT INTO roots (id, root) VALUES (10, 'ر ه م')", [])
            .unwrap();
        let found = search_roots(&conn, "r-h-m").unwrap();
        assert_eq!(root_names(&found), ["ر ح م", "ر ه م"]);
    }

    #[test]
    fn other_text_finds_nothing() {
        let conn = database();
        for query in ["", "sabır", "x-y-z-q", "1-2-3", "كتب"] {
            assert_eq!(
                search_roots(&conn, query).unwrap(),
                RootSearch::default(),
                "{query}"
            );
        }
    }
}
