//! Search: query classification and full-text, root/lemma and semantic search
//! behind the single search box (spec §7.1–§7.4).
//!
//! [`normalize`] is the Rust port of the pipeline's Arabic normalizer
//! (`siyak-data/docs/normalization.md`); both are tested against the shared
//! `normalize_vectors.json`. User input never reaches FTS5 as query syntax:
//! [`fts_query`] rebuilds it from quoted strings.
//!
//! Roots and lemmas are found by [`crate::morphology::search_roots`].
//!
//! TODO(M7): semantic search via [`crate::embed`].

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::db::content::{self, RevelationType, Surah};
use crate::error::AppResult;

/// Largest number of results returned at once.
const MAX_LIMIT: u32 = 2000;

/// Words of context around a match in a translation snippet.
const SNIPPET_TOKENS: u32 = 48;

/// Marks the start of a match in FTS5 output; cannot occur in normalized text.
const MATCH_START: char = '\u{1}';
/// Marks the end of a match in FTS5 output.
const MATCH_END: char = '\u{2}';

/// Normalizes Arabic text for search (rules N10, N11, N1–N9, N12 of
/// `siyak-data/docs/normalization.md`, in that order). Everything else, Latin
/// text included, passes through unchanged.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}') {
            // N10: only these ranges, one character at a time, never the whole string.
            for decomposed in std::iter::once(c).nfkc() {
                push_normalized(decomposed, &mut out);
            }
        } else {
            push_normalized(c, &mut out);
        }
    }
    // N12: `split_whitespace` splits on the Unicode `White_Space` property.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_normalized(c: char, out: &mut String) {
    match c {
        // N11 invisible format characters.
        '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2066}'..='\u{2069}'
        | '\u{061C}'
        | '\u{FEFF}'
        // N1 harakat and other combining marks, N2 dagger alef.
        | '\u{064B}'..='\u{065F}'
        | '\u{0670}'
        // N3 Quranic annotation marks, N4 tatweel.
        | '\u{0610}'..='\u{061A}'
        | '\u{06D6}'..='\u{06ED}'
        | '\u{08D3}'..='\u{08FF}'
        | '\u{0640}' => {}
        // N5 alef variants.
        'أ' | 'إ' | 'آ' | 'ٱ' => out.push('ا'),
        // N6 alef maqsura, N8 and N9 yeh.
        'ى' | 'ئ' | 'ی' | 'ے' => out.push('ي'),
        // N7 teh marbuta, N9 heh.
        'ة' | 'ہ' | 'ھ' | 'ۀ' => out.push('ه'),
        // N8 waw.
        'ؤ' => out.push('و'),
        // N9 kaf.
        'ک' => out.push('ك'),
        other => out.push(other),
    }
}

/// Why a query that looks like a verse reference leads nowhere. The values are
/// the `command_palette.errors` keys of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InvalidReference {
    /// There are 114 surahs.
    SurahOutOfRange,
    /// The surah has no such verse, or the range is reversed.
    AyahOutOfRange,
}

/// What a query is (spec §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum QueryClass {
    /// Nothing to search for.
    Empty,
    /// A verse or range to jump to: no search.
    Reference {
        /// Surah number.
        surah: u16,
        /// First verse; `None` for a whole surah.
        ayah: Option<u16>,
        /// Last verse of a range.
        ayah_to: Option<u16>,
    },
    /// A verse reference that does not exist.
    InvalidReference {
        /// Why.
        reason: InvalidReference,
    },
    /// Arabic letters: searched in the Arabic text.
    Arabic,
    /// A root in Latin script, e.g. `s-b-r`.
    LatinRoot,
    /// Any other text: searched in the translations.
    Text,
}

/// Classifies a query, in the order of spec §7.1.
pub fn classify_query(conn: &Connection, query: &str) -> AppResult<QueryClass> {
    let text = ascii_digits(query.trim());
    if text.is_empty() {
        return Ok(QueryClass::Empty);
    }
    let surahs = content::list_surahs(conn)?;
    if let Some(reference) = parse_reference(&text, &surahs) {
        return Ok(reference);
    }
    let mut letters = text.chars().filter(|c| c.is_alphabetic()).peekable();
    if letters.peek().is_some() && letters.all(is_arabic) {
        return Ok(QueryClass::Arabic);
    }
    if is_latin_root(&text) {
        return Ok(QueryClass::LatinRoot);
    }
    Ok(QueryClass::Text)
}

pub(crate) fn is_arabic(c: char) -> bool {
    matches!(
        c,
        '\u{0600}'..='\u{06FF}'
            | '\u{0750}'..='\u{077F}'
            | '\u{08A0}'..='\u{08FF}'
            | '\u{FB50}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFF}'
    )
}

/// Three or four dash-separated groups of one or two letters, or an apostrophe
/// for a hamza or ayn: `s-b-r`, `s-'-l`.
pub(crate) fn is_latin_root(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    (3..=4).contains(&groups.len())
        && groups.iter().all(|group| {
            (1..=2).contains(&group.chars().count())
                && group
                    .chars()
                    .all(|c| matches!(c, '\'' | '`') || (c.is_alphabetic() && !is_arabic(c)))
        })
}

/// Arabic-Indic and Extended Arabic-Indic digits as ASCII digits.
fn ascii_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '٠'..='٩' => char::from(b'0' + (u32::from(c) - 0x0660) as u8),
            '۰'..='۹' => char::from(b'0' + (u32::from(c) - 0x06F0) as u8),
            other => other,
        })
        .collect()
}

const fn is_reference_separator(c: char) -> bool {
    matches!(c, ':' | '.' | '/')
}

const fn is_range_separator(c: char) -> bool {
    matches!(c, '-' | '–' | '—')
}

/// Numbers parse saturated: `99999:1` is out of range, not text.
fn number(digits: &str) -> u32 {
    digits.parse().unwrap_or(u32::MAX)
}

/// `2`, `2:255`, `2 255`, `2:153-157`, `Bakara 255`, `Baqarah 153-157`. A name
/// needs a verse number: a bare `Yusuf` is a word to search for.
fn parse_reference(text: &str, surahs: &[Surah]) -> Option<QueryClass> {
    let (head, ayah, ayah_to) = split_verses(text);
    if head.is_empty() {
        return None;
    }
    if head.chars().all(|c| c.is_ascii_digit()) {
        return Some(reference(number(head), ayah, ayah_to, surahs));
    }
    let ayah = ayah?;
    let surah = find_surah(head, surahs)?;
    Some(reference(
        u32::from(surah.number),
        Some(ayah),
        ayah_to,
        surahs,
    ))
}

/// Splits `<head><sep><from>[-<to>]` from the end of `text`. Without a
/// separator and verse number, all of `text` is the head.
fn split_verses(text: &str) -> (&str, Option<u32>, Option<u32>) {
    let whole = (text, None, None);
    let Some((rest, last)) = take_number(text) else {
        return whole;
    };
    let (rest, first, to) = match rest.trim_end().strip_suffix(is_range_separator) {
        Some(before) => {
            let Some((rest, first)) = take_number(before.trim_end()) else {
                return whole;
            };
            (rest, first, Some(number(last)))
        }
        None => (rest, last, None),
    };
    let trimmed = rest.trim_end();
    // The verse number must be set apart from what it belongs to.
    if !trimmed.ends_with(is_reference_separator) && trimmed.len() == rest.len() {
        return whole;
    }
    let head = trimmed.trim_end_matches(is_reference_separator).trim_end();
    if head.is_empty() {
        return whole;
    }
    (head, Some(number(first)), to)
}

/// Splits the ASCII digits off the end of `text`: what precedes them, and them.
fn take_number(text: &str) -> Option<(&str, &str)> {
    let rest = text.trim_end_matches(|c: char| c.is_ascii_digit());
    (rest.len() < text.len()).then(|| text.split_at(rest.len()))
}

fn reference(surah: u32, ayah: Option<u32>, ayah_to: Option<u32>, surahs: &[Surah]) -> QueryClass {
    let invalid = |reason| QueryClass::InvalidReference { reason };
    let Some(found) = surahs.iter().find(|s| u32::from(s.number) == surah) else {
        return invalid(InvalidReference::SurahOutOfRange);
    };
    let count = u32::from(found.ayah_count);
    let in_surah = |ayah: u32| (1..=count).contains(&ayah);
    if ayah.is_some_and(|ayah| !in_surah(ayah))
        || ayah_to.is_some_and(|to| !in_surah(to) || ayah.is_some_and(|ayah| to < ayah))
    {
        return invalid(InvalidReference::AyahOutOfRange);
    }
    QueryClass::Reference {
        surah: found.number,
        ayah: ayah.map(|ayah| ayah as u16),
        ayah_to: ayah_to.map(|to| to as u16),
    }
}

/// The surah called `name` (any of its names), or the only one starting so.
/// The same rules as `findSurah` in `src/lib/goTo.ts`.
fn find_surah<'s>(name: &str, surahs: &'s [Surah]) -> Option<&'s Surah> {
    let folded = fold(name);
    if folded.is_empty() {
        return None;
    }
    let latin = fold_name(name);
    let keys = |surah: &Surah| {
        [
            fold(&surah.name_ar),
            fold(&surah.name_tr),
            fold_name(&surah.name_tr),
            fold_name(&surah.name_en),
        ]
    };
    let exact = surahs
        .iter()
        .find(|s| keys(s).iter().any(|key| *key == folded || *key == latin));
    if exact.is_some() {
        return exact;
    }
    let mut prefixed = surahs.iter().filter(|s| {
        keys(s)
            .iter()
            .any(|key| key.starts_with(&folded) || key.starts_with(&latin))
    });
    let first = prefixed.next()?;
    prefixed.next().is_none().then_some(first)
}

fn without_marks_lowercase(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Folds text to a matching key, like `fold` in `src/lib/fold.ts`.
fn fold(text: &str) -> String {
    without_marks_lowercase(text)
        .chars()
        .filter_map(|c| match c {
            'ı' => Some('i'),
            'أ' | 'إ' | 'آ' | 'ٱ' => Some('ا'),
            'ى' => Some('ي'),
            'ة' => Some('ه'),
            'ـ' => None,
            other => other.is_alphanumeric().then_some(other),
        })
        .collect()
}

/// Folds a Latin-script surah name further, like `foldName` in `src/lib/fold.ts`.
fn fold_name(name: &str) -> String {
    let lower = without_marks_lowercase(name);
    let stripped = ["adh", "ash", "al", "an", "ar", "as", "at", "ad", "az"]
        .iter()
        .find_map(|article| {
            let rest = lower.strip_prefix(article)?;
            let body = rest.trim_start_matches(|c: char| {
                c == '-' || c == '\'' || c == '’' || c.is_whitespace()
            });
            (body.len() < rest.len()).then_some(body)
        })
        .unwrap_or(&lower);
    let mut out = String::new();
    for c in fold(stripped).replace("ee", "i").chars() {
        if !(matches!(c, 'a' | 'i' | 'u') && out.ends_with(c)) {
            out.push(c);
        }
    }
    if out.ends_with('h') && out[..out.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']) {
        out.pop();
    }
    out
}

/// Builds an FTS5 query from user input, which is never used as query syntax.
/// Text in double quotes is an exact phrase; any other word also matches by
/// prefix, because Turkish and Arabic words take suffixes (`sabır` finds
/// `sabırlı`). All parts must match. `None` if nothing searchable is left.
pub fn fts_query(text: &str) -> Option<String> {
    let cleaned: String = text
        .chars()
        .filter(|c| *c != '\0')
        .map(|c| {
            if matches!(c, '“' | '”' | '„') {
                '"'
            } else {
                c
            }
        })
        .collect();
    let searchable = |s: &str| s.chars().any(char::is_alphanumeric);
    let mut parts = Vec::new();
    for (i, segment) in cleaned.split('"').enumerate() {
        if i % 2 == 1 {
            if searchable(segment) {
                parts.push(format!("\"{segment}\""));
            }
        } else {
            parts.extend(
                segment
                    .split_whitespace()
                    .filter(|word| searchable(word))
                    .map(|word| format!("\"{word}\"*")),
            );
        }
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// What to search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    /// The Arabic text (`verses_fts`).
    Arabic,
    /// The translations (`translations_fts`).
    Translations,
}

/// Limits on the verses searched (spec §7.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Type)]
pub struct SearchFilters {
    /// First surah.
    pub surah_from: Option<u16>,
    /// Last surah.
    pub surah_to: Option<u16>,
    /// Meccan or Medinan surahs only.
    pub revelation: Option<RevelationType>,
}

/// A piece of a result's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TextPart {
    /// The text.
    pub text: String,
    /// Whether it matched the query.
    pub matched: bool,
}

/// One result: a verse, or one translation of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SearchHit {
    /// Surah number.
    pub surah: u16,
    /// Verse number within the surah.
    pub ayah: u16,
    /// The translation pack; `None` for the Arabic text.
    pub pack_id: Option<String>,
    /// The text with its matches marked: never markup.
    pub parts: Vec<TextPart>,
}

/// The results of a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SearchResults {
    /// How many results there are in all.
    pub total: u32,
    /// The best results, up to the limit.
    pub hits: Vec<SearchHit>,
}

/// Searches the Arabic text or the translations, best matches first. An empty
/// `pack_ids` searches every installed translation; unknown ids find nothing.
pub fn search_text(
    conn: &Connection,
    query: &str,
    scope: SearchScope,
    pack_ids: &[String],
    filters: SearchFilters,
    limit: u32,
) -> AppResult<SearchResults> {
    match scope {
        SearchScope::Arabic => search_arabic(conn, query, filters, limit),
        SearchScope::Translations => search_translations(conn, query, pack_ids, filters, limit),
    }
}

/// Surah filter over `v` (verses) and `s` (surahs), with parameters ?2–?4.
const FILTER_SQL: &str = "(?2 IS NULL OR v.surah >= ?2)
       AND (?3 IS NULL OR v.surah <= ?3)
       AND (?4 IS NULL OR s.revelation_type = ?4)";

fn revelation_value(filters: SearchFilters) -> Option<&'static str> {
    filters.revelation.map(|revelation| match revelation {
        RevelationType::Meccan => "meccan",
        RevelationType::Medinan => "medinan",
    })
}

fn search_arabic(
    conn: &Connection,
    query: &str,
    filters: SearchFilters,
    limit: u32,
) -> AppResult<SearchResults> {
    let Some(fts) = fts_query(&normalize(query)) else {
        return Ok(SearchResults::empty());
    };
    let params = (
        fts.as_str(),
        filters.surah_from,
        filters.surah_to,
        revelation_value(filters),
    );
    // CROSS JOIN keeps the FTS table outermost, as `highlight()` needs.
    let from = format!(
        "FROM verses_fts CROSS JOIN verses v ON v.id = verses_fts.rowid
         JOIN surahs s ON s.number = v.surah
         WHERE verses_fts MATCH ?1 AND {FILTER_SQL}"
    );
    let total = conn.query_row(&format!("SELECT count(*) {from}"), params, |r| r.get(0))?;
    let mut stmt = conn.prepare(&format!(
        "SELECT v.surah, v.ayah, v.text_clean,
                highlight(verses_fts, 0, char(1), char(2))
         {from} ORDER BY rank, v.id LIMIT {}",
        limit.min(MAX_LIMIT)
    ))?;
    let hits = stmt
        .query_map(params, |row| {
            let clean: String = row.get(2)?;
            let highlighted: String = row.get(3)?;
            Ok(SearchHit {
                surah: row.get(0)?,
                ayah: row.get(1)?,
                pack_id: None,
                parts: mark_words(&clean, &highlighted),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(SearchResults { total, hits })
}

fn search_translations(
    conn: &Connection,
    query: &str,
    pack_ids: &[String],
    filters: SearchFilters,
    limit: u32,
) -> AppResult<SearchResults> {
    let Some(fts) = fts_query(query) else {
        return Ok(SearchResults::empty());
    };
    let packs = serde_json::to_string(pack_ids).unwrap_or_else(|_| "[]".to_owned());
    let params = (
        fts.as_str(),
        filters.surah_from,
        filters.surah_to,
        revelation_value(filters),
        packs.as_str(),
    );
    let from = format!(
        "FROM translations_fts f CROSS JOIN verses v ON v.id = f.verse_id
         JOIN surahs s ON s.number = v.surah
         WHERE translations_fts MATCH ?1 AND {FILTER_SQL}
           AND (?5 = '[]' OR f.pack_id IN (SELECT value FROM json_each(?5)))"
    );
    let total = conn.query_row(&format!("SELECT count(*) {from}"), params, |r| r.get(0))?;
    let mut stmt = conn.prepare(&format!(
        "SELECT v.surah, v.ayah, f.pack_id,
                snippet(translations_fts, 0, char(1), char(2), '…', {SNIPPET_TOKENS})
         {from} ORDER BY f.rank, v.id LIMIT {}",
        limit.min(MAX_LIMIT)
    ))?;
    let hits = stmt
        .query_map(params, |row| {
            let snippet: String = row.get(3)?;
            Ok(SearchHit {
                surah: row.get(0)?,
                ayah: row.get(1)?,
                pack_id: row.get(2)?,
                parts: split_marked(&snippet),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(SearchResults { total, hits })
}

impl SearchResults {
    fn empty() -> Self {
        Self {
            total: 0,
            hits: Vec::new(),
        }
    }
}

/// Splits FTS5 output with match markers into parts.
fn split_marked(marked: &str) -> Vec<TextPart> {
    let mut parts = Vec::new();
    let mut matched = false;
    for piece in marked.split([MATCH_START, MATCH_END]) {
        if !piece.is_empty() {
            parts.push(TextPart {
                text: piece.to_owned(),
                matched,
            });
        }
        matched = !matched;
    }
    parts
}

/// The index holds normalized text, which is never displayed (spec §5.4). Its
/// words correspond one to one to those of `text_clean`, so the matches found
/// in the index are marked on the text itself. If the words ever disagree the
/// text is shown without marks.
fn mark_words(clean: &str, highlighted: &str) -> Vec<TextPart> {
    let words: Vec<&str> = clean.split_whitespace().collect();
    let marked: Vec<&str> = highlighted.split_whitespace().collect();
    if words.len() != marked.len() {
        return vec![TextPart {
            text: clean.to_owned(),
            matched: false,
        }];
    }
    let mut parts: Vec<TextPart> = Vec::new();
    let mut inside = false;
    for (word, mark) in words.iter().zip(marked) {
        let matched = inside || mark.contains(MATCH_START);
        // A phrase's markers may span several words.
        if let Some(last) = mark.rfind([MATCH_START, MATCH_END]) {
            inside = mark[last..].starts_with(MATCH_START);
        }
        match parts.last_mut() {
            Some(part) if part.matched == matched => {
                part.text.push(' ');
                part.text.push_str(word);
            }
            Some(part) => {
                // The space between parts stays outside the match.
                if !matched {
                    parts.push(TextPart {
                        text: format!(" {word}"),
                        matched,
                    });
                } else {
                    part.text.push(' ');
                    parts.push(TextPart {
                        text: (*word).to_owned(),
                        matched,
                    });
                }
            }
            None => parts.push(TextPart {
                text: (*word).to_owned(),
                matched,
            }),
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rusqlite::OpenFlags;

    use super::*;

    fn repo_db(relative: &str) -> Connection {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }

    fn dev() -> Connection {
        repo_db("../dev-data/dev-content.db")
    }

    fn core() -> Connection {
        repo_db("resources/core.db")
    }

    const VECTORS: &str = include_str!("../../dev-data/fixtures/normalize_vectors.json");

    #[test]
    fn normalize_matches_the_shared_vectors_and_is_idempotent() {
        let doc: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        let vectors = doc["vectors"].as_array().unwrap();
        assert!(!vectors.is_empty());
        for vector in vectors {
            let input = vector["input"].as_str().unwrap();
            let expected = vector["expected"].as_str().unwrap();
            assert_eq!(normalize(input), expected, "{}", vector["id"]);
            assert_eq!(normalize(expected), expected, "{} twice", vector["id"]);
        }
    }

    /// The copy in this repository must not drift from the contract.
    #[test]
    fn vectors_are_the_pipelines_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../siyak-data/tests/normalize_vectors.json");
        if let Ok(original) = std::fs::read_to_string(path) {
            assert_eq!(original, VECTORS);
        }
    }

    /// The query meets the index in the same form: this normalizer gives what
    /// the pipeline stored for every verse of the bundled `core.db`.
    #[test]
    fn normalize_reproduces_the_core_index() {
        let conn = core();
        let mut stmt = conn
            .prepare(
                "SELECT v.text_clean, f.text_norm FROM verses v
                 JOIN verses_fts f ON f.rowid = v.id",
            )
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows.len(), 6236);
        for (clean, indexed) in rows {
            assert_eq!(normalize(&clean), indexed);
        }
    }

    fn classify(query: &str) -> QueryClass {
        classify_query(&dev(), query).unwrap()
    }

    fn verse(surah: u16, ayah: u16) -> QueryClass {
        QueryClass::Reference {
            surah,
            ayah: Some(ayah),
            ayah_to: None,
        }
    }

    #[test]
    fn classifies_verse_references() {
        for query in [
            "2:255",
            " 2 255 ",
            "2.255",
            "2/255",
            "٢:٢٥٥",
            "Bakara 255",
            "bakara:255",
            "Baqarah 255",
            "Al-Baqara 255",
            "البقرة ٢٥٥",
        ] {
            assert_eq!(classify(query), verse(2, 255), "{query}");
        }
        for query in ["2:153-157", "2:153–157", "Bakara 153-157", "2 153 - 157"] {
            assert_eq!(
                classify(query),
                QueryClass::Reference {
                    surah: 2,
                    ayah: Some(153),
                    ayah_to: Some(157)
                },
                "{query}"
            );
        }
        assert_eq!(
            classify("36"),
            QueryClass::Reference {
                surah: 36,
                ayah: None,
                ayah_to: None
            }
        );
    }

    #[test]
    fn reports_references_that_do_not_exist() {
        let surah = QueryClass::InvalidReference {
            reason: InvalidReference::SurahOutOfRange,
        };
        let ayah = QueryClass::InvalidReference {
            reason: InvalidReference::AyahOutOfRange,
        };
        assert_eq!(classify("115:1"), surah);
        assert_eq!(classify("0"), surah);
        assert_eq!(classify("99999999999999:1"), surah);
        assert_eq!(classify("2:999"), ayah);
        assert_eq!(classify("2:0"), ayah);
        assert_eq!(classify("2:157-153"), ayah);
        assert_eq!(classify("Bakara 999"), ayah);
    }

    #[test]
    fn classifies_arabic_roots_and_text() {
        for query in ["صبر", "الصَّبْرِ", "ص ب ر"] {
            assert_eq!(classify(query), QueryClass::Arabic, "{query}");
        }
        for query in ["s-b-r", "S-B-R", "k-t-b", "z-l-z-l", "s-'-l"] {
            assert_eq!(classify(query), QueryClass::LatinRoot, "{query}");
        }
        for query in [
            "sabır",
            "patience in hard times",
            "Yusuf",
            "Bakara",
            "no such surah 5",
            "sabır صبر",
            "well-known-fact",
        ] {
            assert_eq!(classify(query), QueryClass::Text, "{query}");
        }
        assert_eq!(classify("   "), QueryClass::Empty);
    }

    #[test]
    fn user_input_is_never_query_syntax() {
        assert_eq!(fts_query("sabır"), Some("\"sabır\"*".to_owned()));
        assert_eq!(
            fts_query("\"sabredenlerle beraberdir\" Allah"),
            Some("\"sabredenlerle beraberdir\" \"Allah\"*".to_owned())
        );
        assert_eq!(fts_query("“iki kelime”"), Some("\"iki kelime\"".to_owned()));
        assert_eq!(fts_query("   "), None);
        assert_eq!(fts_query("- * ( ) \" \""), None);
        // Every one of these would be an error or an operator as raw FTS5.
        let conn = dev();
        for hostile in [
            "NEAR(a b",
            "text:allah",
            "a OR b",
            "NOT a",
            "a AND",
            "\"unbalanced",
            "(((",
            "*",
            "^a",
            "a\0b",
            "pack_id:en.pickthall",
            "'; DROP TABLE verses; --",
        ] {
            for scope in [SearchScope::Translations, SearchScope::Arabic] {
                search_text(&conn, hostile, scope, &[], SearchFilters::default(), 5)
                    .unwrap_or_else(|err| panic!("{hostile:?}: {err}"));
            }
        }
    }

    fn translations(query: &str, filters: SearchFilters) -> SearchResults {
        search_text(&dev(), query, SearchScope::Translations, &[], filters, 50).unwrap()
    }

    fn matched(hit: &SearchHit) -> Vec<&str> {
        hit.parts
            .iter()
            .filter(|p| p.matched)
            .map(|p| p.text.as_str())
            .collect()
    }

    #[test]
    fn searches_translations_with_marks_and_prefixes() {
        let results = translations("merciful", SearchFilters::default());
        assert!(results.total > 0);
        let hit = &results.hits[0];
        assert_eq!(hit.pack_id.as_deref(), Some("en.pickthall"));
        assert_eq!(matched(hit)[0].to_lowercase(), "merciful");
        // A prefix finds the longer word; a phrase must match exactly.
        assert!(translations("mercif", SearchFilters::default()).total > 0);
        assert_eq!(
            translations("\"mercif\"", SearchFilters::default()).total,
            0
        );
        assert!(translations("\"the beneficent\"", SearchFilters::default()).total > 0);
    }

    #[test]
    fn filters_by_pack_surah_and_revelation() {
        let conn = dev();
        let all = translations("merciful", SearchFilters::default()).total;
        let search = |packs: &[String], filters| {
            search_text(
                &conn,
                "merciful",
                SearchScope::Translations,
                packs,
                filters,
                50,
            )
            .unwrap()
        };
        assert_eq!(
            search(&["en.pickthall".to_owned()], SearchFilters::default()).total,
            all
        );
        assert_eq!(
            search(&["tr.none".to_owned()], SearchFilters::default()).total,
            0
        );
        let first = search(
            &[],
            SearchFilters {
                surah_from: Some(1),
                surah_to: Some(1),
                revelation: None,
            },
        );
        assert!(first.total > 0 && first.total < all);
        assert!(first.hits.iter().all(|hit| hit.surah == 1));
        let medinan = search(
            &[],
            SearchFilters {
                revelation: Some(RevelationType::Medinan),
                ..SearchFilters::default()
            },
        );
        assert!(medinan.hits.iter().all(|hit| hit.surah != 1));
    }

    #[test]
    fn limits_results_but_counts_them_all() {
        let results = search_text(
            &dev(),
            "merciful",
            SearchScope::Translations,
            &[],
            SearchFilters::default(),
            1,
        )
        .unwrap();
        assert_eq!(results.hits.len(), 1);
        assert!(results.total > 1);
    }

    #[test]
    fn arabic_search_ignores_diacritics_and_shows_the_clean_text() {
        for conn in [dev(), core()] {
            for query in ["الصابرين", "الصَّابِرِينَ"] {
                let results = search_text(
                    &conn,
                    query,
                    SearchScope::Arabic,
                    &[],
                    SearchFilters::default(),
                    2000,
                )
                .unwrap();
                assert!(results.total > 0, "{query}");
                let hit = results
                    .hits
                    .iter()
                    .find(|h| (h.surah, h.ayah) == (2, 153))
                    .unwrap();
                assert_eq!(hit.pack_id, None);
                let text: String = hit.parts.iter().map(|p| p.text.as_str()).collect();
                let original: String = conn
                    .query_row(
                        "SELECT text_clean FROM verses WHERE surah = ?1 AND ayah = ?2",
                        (hit.surah, hit.ayah),
                        |r| r.get(0),
                    )
                    .unwrap();
                // The text is exactly the verse: normalized text is never shown.
                assert_eq!(text, original);
                assert_eq!(matched(hit), ["الصابرين"]);
            }
        }
    }

    #[test]
    fn marks_whole_words_and_phrases_on_the_clean_text() {
        let parts = mark_words("قال هدى للناس", "قال \u{1}هدي\u{2} للناس");
        assert_eq!(
            parts,
            [
                TextPart {
                    text: "قال ".to_owned(),
                    matched: false
                },
                TextPart {
                    text: "هدى".to_owned(),
                    matched: true
                },
                TextPart {
                    text: " للناس".to_owned(),
                    matched: false
                },
            ]
        );
        let phrase = mark_words("a b c d", "a \u{1}b c\u{2} d");
        let matched: Vec<_> = phrase.iter().filter(|p| p.matched).collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].text, "b c");
        // Words that disagree: the text, unmarked.
        assert_eq!(
            mark_words("a b", "\u{1}a\u{2}"),
            [TextPart {
                text: "a b".to_owned(),
                matched: false
            }]
        );
    }
}
