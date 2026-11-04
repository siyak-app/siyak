//! Citation text for verses, translations and tafsirs from pack `citation`
//! metadata and template files (spec §11.2).
//!
//! A style is one `citation/<id>.tmpl` file: lines of `key = template` with
//! the keys `name`, `quran`, `translation`, `tafsir` and `bibliography`. The
//! build script finds the files, so adding a style needs no code change.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::db::content::{self, VerseIndex, VerseRef};
use crate::error::{AppError, AppErrorCode, AppResult};

include!(concat!(env!("OUT_DIR"), "/citation_styles.rs"));

/// A citation style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct CitationStyle {
    /// Stable id, the template's file name.
    pub id: String,
    /// Name to show.
    pub name: String,
}

/// What to cite.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CitationRef {
    /// Verses, with a line for the Quran and one per translation.
    Verses {
        /// First verse, by global id.
        from: u32,
        /// Last verse, by global id.
        to: u32,
        /// The translations to cite.
        translation_ids: Vec<String>,
    },
    /// The commentary of a tafsir on a verse.
    Tafsir {
        /// Tafsir pack id.
        pack_id: String,
        /// Global id of the verse.
        verse: u32,
    },
}

/// A parsed style file.
struct Style {
    templates: BTreeMap<String, String>,
}

impl Style {
    fn parse(text: &str) -> Self {
        let templates = text
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .map(|(key, template)| (key.trim().to_owned(), template.trim().to_owned()))
            .collect();
        Self { templates }
    }

    fn find(id: &str) -> AppResult<Self> {
        STYLE_FILES
            .iter()
            .find(|(style, _)| *style == id)
            .map(|(_, text)| Self::parse(text))
            .ok_or_else(|| {
                AppError::new(AppErrorCode::InvalidInput, format!("citation style {id}"))
            })
    }

    fn render(&self, key: &str, fields: &BTreeMap<String, String>) -> String {
        self.templates
            .get(key)
            .map(|template| render(template, fields))
            .unwrap_or_default()
    }
}

/// The available styles, by id.
pub fn list_styles() -> Vec<CitationStyle> {
    STYLE_FILES
        .iter()
        .map(|(id, text)| CitationStyle {
            id: (*id).to_owned(),
            name: Style::parse(text)
                .templates
                .remove("name")
                .unwrap_or_else(|| (*id).to_owned()),
        })
        .collect()
}

/// Fills a template: `{name}` is a field and a `[ … ]` section is dropped when
/// a field directly inside it has no value. Field values are never parsed.
fn render(template: &str, fields: &BTreeMap<String, String>) -> String {
    let mut chars = template.chars();
    render_section(&mut chars, fields).0
}

/// Renders up to the closing `]` or the end. Returns the text and whether all
/// the section's fields had a value.
fn render_section(
    chars: &mut std::str::Chars<'_>,
    fields: &BTreeMap<String, String>,
) -> (String, bool) {
    let mut out = String::new();
    let mut complete = true;
    while let Some(c) = chars.next() {
        match c {
            '[' => {
                let (section, section_complete) = render_section(chars, fields);
                if section_complete {
                    out.push_str(&section);
                }
            }
            ']' => break,
            '{' => {
                let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                match fields.get(&name) {
                    Some(value) => out.push_str(value),
                    None => complete = false,
                }
            }
            c => out.push(c),
        }
    }
    (out, complete)
}

fn surah_fields(conn: &Connection, surah: u16) -> AppResult<BTreeMap<String, String>> {
    let (name_tr, name_en): (String, String) = conn
        .query_row(
            "SELECT name_tr, name_en FROM surahs WHERE number = ?1",
            [surah],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("surah {surah}")))?;
    Ok(BTreeMap::from([
        ("surah".to_owned(), surah.to_string()),
        ("surah_tr".to_owned(), name_tr),
        ("surah_en".to_owned(), name_en),
    ]))
}

/// `255`, or `153-157` for a range. A range across surahs names the last verse in full.
fn ayah_field(from: VerseRef, to: VerseRef) -> String {
    if from == to {
        from.ayah.to_string()
    } else if from.surah == to.surah {
        format!("{}-{}", from.ayah, to.ayah)
    } else {
        format!("{}-{}/{}", from.ayah, to.surah, to.ayah)
    }
}

/// Citation text for `reference` in style `style`, one source per line.
/// A pack that is not installed is left out.
pub fn format_citation(
    conn: &Connection,
    index: &VerseIndex,
    reference: &CitationRef,
    style: &str,
) -> AppResult<String> {
    let style = Style::find(style)?;
    match reference {
        CitationRef::Verses {
            from,
            to,
            translation_ids,
        } => {
            index.check_range(*from, *to)?;
            let (Some(first), Some(last)) = (index.verse(*from), index.verse(*to)) else {
                return Err(AppError::new(AppErrorCode::InvalidInput, "verses"));
            };
            let mut fields = surah_fields(conn, first.surah)?;
            fields.insert("ayah".to_owned(), ayah_field(first, last));
            let mut lines = vec![style.render("quran", &fields)];
            for pack_id in translation_ids {
                if let Some(citation) = content::pack_citation(conn, pack_id)? {
                    let mut fields = fields.clone();
                    fields.extend(citation.fields);
                    lines.push(style.render("translation", &fields));
                }
            }
            Ok(lines.join("\n"))
        }
        CitationRef::Tafsir { pack_id, verse } => {
            index.check_range(*verse, *verse)?;
            let citation = content::pack_citation(conn, pack_id)?
                .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("pack {pack_id}")))?;
            let mut fields = citation.fields;
            if let Some(passage) = content::get_tafsir(conn, *verse, pack_id)? {
                let page = passage.paragraphs.first();
                let locator = match (page.and_then(|p| p.part.clone()), page.and_then(|p| p.page)) {
                    (Some(part), Some(page)) => Some(format!("{part}/{page}")),
                    (Some(part), None) => Some(part),
                    (None, Some(page)) => Some(page.to_string()),
                    (None, None) => None,
                };
                if let Some(locator) = locator {
                    fields.insert("locator".to_owned(), locator);
                }
            }
            Ok(style.render("tafsir", &fields))
        }
    }
}

/// The bibliography entry of a pack in style `style`, or `None` if it is not installed.
pub fn bibliography_entry(
    conn: &Connection,
    pack_id: &str,
    style: &str,
) -> AppResult<Option<String>> {
    let style = Style::find(style)?;
    Ok(content::pack_citation(conn, pack_id)?
        .map(|citation| style.render("bibliography", &citation.fields)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::test_support::content;

    fn fields(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn templates_drop_sections_with_a_missing_field() {
        let template = "{a}[, {b}][ ({c}[, {d}])].";
        assert_eq!(render(template, &fields(&[("a", "A")])), "A.");
        assert_eq!(
            render(template, &fields(&[("a", "A"), ("b", "B")])),
            "A, B."
        );
        assert_eq!(render(template, &fields(&[("a", "A"), ("d", "D")])), "A.");
        assert_eq!(
            render(template, &fields(&[("a", "A"), ("c", "C"), ("d", "D")])),
            "A (C, D)."
        );
    }

    #[test]
    fn field_values_are_not_parsed() {
        assert_eq!(render("{a}", &fields(&[("a", "[{b}]")])), "[{b}]");
    }

    #[test]
    fn every_style_file_is_complete() {
        let styles = list_styles();
        assert!(styles.iter().any(|s| s.id == "isnad" && s.name == "İSNAD"));
        assert!(styles.iter().any(|s| s.id == "apa" && s.name == "APA"));
        for (id, text) in STYLE_FILES {
            let style = Style::parse(text);
            for key in ["name", "quran", "translation", "tafsir", "bibliography"] {
                assert!(style.templates.contains_key(key), "{id} lacks {key}");
            }
        }
    }

    fn citation(reference: &CitationRef, style: &str) -> AppResult<String> {
        let conn = content();
        let index = VerseIndex::load(&conn).unwrap();
        format_citation(&conn, &index, reference, style)
    }

    #[test]
    fn cites_verses_and_their_translations() {
        let reference = CitationRef::Verses {
            from: 1,
            to: 2,
            translation_ids: vec![
                "en.test".to_owned(),
                "tr.bare".to_owned(),
                "gone".to_owned(),
            ],
        };
        assert_eq!(
            citation(&reference, "isnad").unwrap(),
            "Kur'ân-ı Kerîm, Fâtiha 1/1-2.\n\
             A. Translator, The Meaning (Knopf, 1930), Fâtiha 1/1-2.\n\
             Bir Mütercim, Yalın Meal, Fâtiha 1/1-2."
        );
        assert_eq!(
            citation(&reference, "apa").unwrap(),
            "The Qur'an 1:1-2.\n\
             A. Translator. (1930). The Meaning. Knopf. Al-Fatihah 1:1-2.\n\
             Bir Mütercim. Yalın Meal. Al-Fatihah 1:1-2."
        );
    }

    #[test]
    fn cites_a_tafsir_with_its_page() {
        let reference = CitationRef::Tafsir {
            pack_id: "ar.tafsir".to_owned(),
            verse: 2,
        };
        assert_eq!(
            citation(&reference, "isnad").unwrap(),
            "al-Author, Great Tafsir (Dar, 1417), 1/113."
        );
    }

    #[test]
    fn rejects_an_unknown_style_and_a_bad_range() {
        let verse = CitationRef::Verses {
            from: 1,
            to: 1,
            translation_ids: Vec::new(),
        };
        assert_eq!(
            citation(&verse, "mla").unwrap_err().code,
            AppErrorCode::InvalidInput
        );
        let bad = CitationRef::Verses {
            from: 2,
            to: 1,
            translation_ids: Vec::new(),
        };
        assert_eq!(
            citation(&bad, "apa").unwrap_err().code,
            AppErrorCode::InvalidInput
        );
    }

    #[test]
    fn writes_a_bibliography_entry() {
        let conn = content();
        assert_eq!(
            bibliography_entry(&conn, "morphology.qac", "apa")
                .unwrap()
                .unwrap(),
            "Kais Dukes. Quranic Arabic Corpus. https://corpus.quran.com"
        );
        assert_eq!(bibliography_entry(&conn, "gone", "apa").unwrap(), None);
    }
}
