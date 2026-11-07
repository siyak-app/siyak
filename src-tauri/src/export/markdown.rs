//! Markdown export of a collection with Arabic text, translations, notes and
//! a bibliography (spec §11.2).
//!
//! Rust produces no user-facing prose (spec §16.1), so the words of the file
//! come from the interface as [`MarkdownLabels`].

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rusqlite::Connection;
use serde::Deserialize;
use specta::Type;

use crate::collections::{Collection, CollectionItem, ItemContent, ItemRef};
use crate::db::content::{self, VerseRef, VerseWithTranslations};
use crate::error::AppResult;
use crate::export::citation;
use crate::morphology;
use crate::notes::Note;

/// The interface's words for the Markdown file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
pub struct MarkdownLabels {
    /// Interface language: picks the surah names and pack titles.
    pub language: String,
    /// Heading of the notes under a verse.
    pub notes: String,
    /// Heading of the bibliography.
    pub bibliography: String,
    /// Label of a root item.
    pub root: String,
    /// Label of a lemma item.
    pub lemma: String,
    /// Label of the occurrence count.
    pub occurrences: String,
    /// Label of the data source of a root or lemma.
    pub source: String,
    /// Shown for an item whose pack is not installed.
    pub unavailable: String,
    /// Label of the Quran text source in the bibliography.
    pub quran_text: String,
}

struct Writer<'a> {
    conn: &'a Connection,
    labels: &'a MarkdownLabels,
    surahs: BTreeMap<u16, String>,
    translations: BTreeMap<String, String>,
    /// The packs to list in the bibliography, in order of first use.
    used_packs: Vec<String>,
    out: String,
}

/// Renders `collection` as Markdown. `notes` are all the user's notes: those
/// touching a verse item appear under it, unless they are items themselves. Entries in the bibliography use
/// citation style `style`.
pub fn render(
    conn: &Connection,
    collection: &Collection,
    notes: &[Note],
    labels: &MarkdownLabels,
    style: &str,
) -> AppResult<String> {
    let surahs = content::list_surahs(conn)?
        .into_iter()
        .map(|surah| {
            let name = if labels.language == "tr" {
                surah.name_tr
            } else {
                surah.name_en
            };
            (surah.number, name)
        })
        .collect();
    let translations = content::list_translations(conn)?
        .into_iter()
        .map(|pack| {
            let localized = if labels.language == "tr" {
                pack.titles.tr
            } else {
                pack.titles.en
            };
            (pack.id, localized.unwrap_or(pack.title))
        })
        .collect();
    let mut writer = Writer {
        conn,
        labels,
        surahs,
        translations,
        used_packs: Vec::new(),
        out: String::new(),
    };
    writer.line(&format!("# {}", collection.summary.title));
    if let Some(description) = &collection.summary.description {
        writer.line(description);
    }
    // A note that is its own item appears once, in the user's chosen place.
    let notes: Vec<Note> = notes
        .iter()
        .filter(|note| {
            !collection
                .items
                .iter()
                .any(|item| matches!(item.item, ItemRef::Note { id } if id == note.id))
        })
        .cloned()
        .collect();
    for item in &collection.items {
        writer.item(item, &notes)?;
    }
    writer.bibliography(style)?;
    Ok(writer.out)
}

impl Writer<'_> {
    /// Appends a block followed by a blank line.
    fn line(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push_str("\n\n");
    }

    fn use_pack(&mut self, id: &str) {
        if !self.used_packs.iter().any(|used| used == id) {
            self.used_packs.push(id.to_owned());
        }
    }

    fn range(&self, from: VerseRef, to: VerseRef) -> String {
        let surah = self.surahs.get(&from.surah).map_or("", String::as_str);
        let start = format!("{surah} {}:{}", from.surah, from.ayah);
        if from == to {
            start
        } else if from.surah == to.surah {
            format!("{start}-{}", to.ayah)
        } else {
            format!("{start}-{}:{}", to.surah, to.ayah)
        }
    }

    fn annotation(&mut self, item: &CollectionItem) {
        if let Some(annotation) = &item.annotation {
            self.line(&quote(annotation));
        }
    }

    fn item(&mut self, item: &CollectionItem, notes: &[Note]) -> AppResult<()> {
        match &item.content {
            ItemContent::VerseRange { verses } => self.verses(item, verses, notes),
            ItemContent::Note { note } => {
                if let Some(note) = note {
                    self.line(&format!("## {}", self.range(note.from, note.to)));
                    self.annotation(item);
                    self.line(&note.body);
                }
            }
            ItemContent::Root { root } => {
                let text = match &item.item {
                    ItemRef::Root { root } => root.as_str(),
                    _ => "",
                };
                self.morphology(
                    item,
                    &self.labels.root.clone(),
                    text,
                    root.as_ref().map(|r| r.occurrences),
                )?;
            }
            ItemContent::Lemma { lemma } => {
                let text = match &item.item {
                    ItemRef::Lemma { lemma } => {
                        lemma.trim_end_matches(|c: char| c.is_ascii_digit())
                    }
                    _ => "",
                };
                self.morphology(
                    item,
                    &self.labels.lemma.clone(),
                    text,
                    lemma.as_ref().map(|l| l.occurrences),
                )?;
            }
            ItemContent::Tafsir {
                verse,
                tafsir,
                passage,
            } => {
                let title = tafsir.as_ref().map_or_else(
                    || match &item.item {
                        ItemRef::Tafsir { pack_id, .. } => pack_id.clone(),
                        _ => String::new(),
                    },
                    |tafsir| {
                        let localized = if self.labels.language == "tr" {
                            tafsir.titles.tr.clone()
                        } else {
                            tafsir.titles.en.clone()
                        };
                        localized.unwrap_or_else(|| tafsir.title.clone())
                    },
                );
                let place = match (passage, verse) {
                    (Some(passage), _) => self.range(passage.verse_from, passage.verse_to),
                    (None, Some(verse)) => self.range(*verse, *verse),
                    (None, None) => String::new(),
                };
                self.line(&format!("## {title}: {place}"));
                self.annotation(item);
                match (tafsir, passage) {
                    (Some(tafsir), Some(passage)) => {
                        for paragraph in &passage.paragraphs {
                            self.line(&html_to_markdown(&paragraph.text));
                        }
                        if let Some(attribution) = &tafsir.attribution {
                            self.line(&format!("*{attribution}*"));
                        }
                        self.use_pack(&tafsir.id);
                    }
                    _ => self.line(&format!("*{}*", self.labels.unavailable)),
                }
            }
        }
        Ok(())
    }

    fn verses(&mut self, item: &CollectionItem, verses: &[VerseWithTranslations], notes: &[Note]) {
        let (Some(first), Some(last)) = (verses.first(), verses.last()) else {
            return;
        };
        let from = VerseRef {
            surah: first.surah,
            ayah: first.ayah,
        };
        let to = VerseRef {
            surah: last.surah,
            ayah: last.ayah,
        };
        self.line(&format!("## {}", self.range(from, to)));
        self.annotation(item);
        for verse in verses {
            self.line(&format!(
                "{} ({}:{})",
                verse.text_uthmani, verse.surah, verse.ayah
            ));
            for translation in &verse.translations {
                let title = self
                    .translations
                    .get(&translation.pack_id)
                    .cloned()
                    .unwrap_or_else(|| translation.pack_id.clone());
                self.line(&format!("**{title}:** {}", translation.text));
                self.use_pack(&translation.pack_id);
            }
        }
        let touching: Vec<&Note> = notes
            .iter()
            .filter(|note| note.verse_from <= last.id && note.verse_to >= first.id)
            .collect();
        if !touching.is_empty() {
            self.line(&format!("### {}", self.labels.notes));
            for note in touching {
                self.line(&format!("*{}*", self.range(note.from, note.to)));
                self.line(&note.body);
            }
        }
    }

    /// A root or lemma, with its data source wherever it is used (ADR 0007).
    fn morphology(
        &mut self,
        item: &CollectionItem,
        label: &str,
        text: &str,
        occurrences: Option<u32>,
    ) -> AppResult<()> {
        self.line(&format!("## {label}: {text}"));
        self.annotation(item);
        let Some(occurrences) = occurrences else {
            self.line(&format!("*{}*", self.labels.unavailable));
            return Ok(());
        };
        self.line(&format!("{}: {occurrences}", self.labels.occurrences));
        if let Some(pack) = morphology::installed_pack(self.conn)? {
            self.line(&format!("{}: {}", self.labels.source, pack.source));
            self.use_pack(&pack.id);
        }
        Ok(())
    }

    fn bibliography(&mut self, style: &str) -> AppResult<()> {
        let mut entries = Vec::new();
        for pack in &self.used_packs {
            if let Some(entry) = citation::bibliography_entry(self.conn, pack, style)? {
                entries.push(entry);
            }
        }
        // Validates the style even for a collection without packs.
        citation::bibliography_entry(self.conn, "", style)?;
        let source = content::quran_text_source(self.conn)?;
        if entries.is_empty() && source.is_none() {
            return Ok(());
        }
        let mut list = String::new();
        if let Some((title, url)) = source {
            let _ = write!(list, "- {}: {title}", self.labels.quran_text);
            if let Some(url) = url {
                let _ = write!(list, " {url}");
            }
            list.push('\n');
        }
        for entry in entries {
            let _ = writeln!(list, "- {entry}");
        }
        self.line(&format!("## {}", self.labels.bibliography));
        self.out.push_str(&list);
        Ok(())
    }
}

/// A Markdown blockquote of `text`.
fn quote(text: &str) -> String {
    text.lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The small HTML subset of tafsir text (spec §5.2) as Markdown: bold, italic
/// and line breaks are kept, any other tag goes and its text stays.
pub(crate) fn html_to_markdown(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        let (text, tail) = rest.split_at(start);
        out.push_str(&decode_entities(text));
        let Some(end) = tail.find('>') else {
            // A lone `<` is text, not a tag.
            out.push_str(&decode_entities(tail));
            rest = "";
            break;
        };
        let (tag, after) = tail.split_at(end);
        let name = tag
            .trim_start_matches('<')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('/');
        match name {
            "b" | "/b" => out.push_str("**"),
            "i" | "/i" => out.push('*'),
            "br" => out.push_str("  \n"),
            _ => {}
        }
        rest = after.strip_prefix('>').unwrap_or(after);
    }
    out.push_str(&decode_entities(rest));
    out
}

fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::test_support::{content, user};
    use crate::collections::{
        CollectionInput, add_item, resolve, stored_collection, upsert_collection,
    };
    use crate::db::content::VerseIndex;
    use crate::notes::{NoteInput, list_notes, upsert_note};

    fn labels() -> MarkdownLabels {
        MarkdownLabels {
            language: "tr".to_owned(),
            notes: "Notlar".to_owned(),
            bibliography: "Kaynakça".to_owned(),
            root: "Kök".to_owned(),
            lemma: "Sözlük biçimi".to_owned(),
            occurrences: "Geçtiği yer".to_owned(),
            source: "Kaynak".to_owned(),
            unavailable: "Paket kurulu değil".to_owned(),
            quran_text: "Kur'an metni".to_owned(),
        }
    }

    fn export(items: &[ItemRef], note_body: Option<&str>) -> String {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = user();
        let id = upsert_collection(
            &conn,
            &CollectionInput {
                id: None,
                title: "Sabır".to_owned(),
                description: Some("Bir araştırma.".to_owned()),
            },
        )
        .unwrap()
        .id;
        if let Some(body) = note_body {
            upsert_note(
                &conn,
                &index,
                &NoteInput {
                    id: None,
                    verse_from: 2,
                    verse_to: 2,
                    body: body.to_owned(),
                },
            )
            .unwrap();
        }
        for item in items {
            add_item(&conn, &index, id, item, Some("Önemli")).unwrap();
        }
        let notes = list_notes(&conn, &index, None).unwrap();
        let stored = stored_collection(&conn, &index, id).unwrap();
        let collection = resolve(&content_db, &index, stored, &["en.test".to_owned()]).unwrap();
        render(&content_db, &collection, &notes, &labels(), "isnad").unwrap()
    }

    #[test]
    fn writes_verses_with_translations_notes_and_bibliography() {
        let out = export(
            &[ItemRef::VerseRange { from: 1, to: 2 }],
            Some("Benim notum"),
        );
        assert!(
            out.starts_with("# Sabır\n\nBir araştırma.\n\n## Fâtiha 1:1-2\n\n> Önemli\n\n"),
            "{out}"
        );
        assert!(out.contains("بسم (1:1)"), "{out}");
        assert!(out.contains("**Test Translation:** Praise"), "{out}");
        assert!(
            out.contains("### Notlar\n\n*Fâtiha 1:2*\n\nBenim notum"),
            "{out}"
        );
        assert!(
            out.contains(
                "## Kaynakça\n\n- Kur'an metni: Tanzil Quran Text (Uthmani) https://tanzil.net\n"
            ),
            "{out}"
        );
        assert!(
            out.contains("- A. Translator. The Meaning. Knopf. 1930."),
            "{out}"
        );
    }

    #[test]
    fn a_note_that_is_an_item_appears_once() {
        let out = export(
            &[
                ItemRef::VerseRange { from: 1, to: 2 },
                ItemRef::Note { id: 1 },
            ],
            Some("Benim notum"),
        );
        assert_eq!(out.matches("Benim notum").count(), 1, "{out}");
        assert!(!out.contains("### Notlar"), "{out}");
    }

    #[test]
    fn names_the_source_of_roots_and_lemmas() {
        let out = export(
            &[
                ItemRef::Root {
                    root: "ر ح م".to_owned(),
                },
                ItemRef::Lemma {
                    lemma: "رَحْمَة".to_owned(),
                },
            ],
            None,
        );
        assert!(out.contains("## Kök: ر ح م\n\n> Önemli\n\nGeçtiği yer: 1\n\nKaynak: Quranic Arabic Corpus (corpus.quran.com)"), "{out}");
        assert!(out.contains("## Sözlük biçimi: رَحْمَة"), "{out}");
        assert!(
            out.contains("- Kais Dukes. Quranic Arabic Corpus. https://corpus.quran.com"),
            "{out}"
        );
    }

    #[test]
    fn writes_a_tafsir_passage_with_its_attribution() {
        let out = export(
            &[ItemRef::Tafsir {
                pack_id: "ar.tafsir".to_owned(),
                verse: 2,
            }],
            None,
        );
        assert!(out.contains("## Tafsir Test: Fâtiha 1:1-2"), "{out}");
        assert!(
            out.contains("Bu **ayet** & devamı  \nikinci satır"),
            "{out}"
        );
        assert!(out.contains("*Source: Quranpedia.net*"), "{out}");
    }

    #[test]
    fn marks_what_is_not_installed() {
        let out = export(
            &[
                ItemRef::Root {
                    root: "yok".to_owned(),
                },
                ItemRef::Tafsir {
                    pack_id: "gone".to_owned(),
                    verse: 1,
                },
            ],
            None,
        );
        assert_eq!(out.matches("*Paket kurulu değil*").count(), 2, "{out}");
    }

    #[test]
    fn a_note_item_is_written_with_its_verses() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = user();
        let note = upsert_note(
            &conn,
            &index,
            &NoteInput {
                id: None,
                verse_from: 3,
                verse_to: 3,
                body: "Tek not".to_owned(),
            },
        )
        .unwrap();
        let out = export_with(&conn, &content_db, &index, &[ItemRef::Note { id: note.id }]);
        assert!(out.contains("## Bakara 2:1\n\nTek not"), "{out}");
    }

    fn export_with(
        conn: &Connection,
        content_db: &Connection,
        index: &VerseIndex,
        items: &[ItemRef],
    ) -> String {
        let id = upsert_collection(
            conn,
            &CollectionInput {
                id: None,
                title: "T".to_owned(),
                description: None,
            },
        )
        .unwrap()
        .id;
        for item in items {
            add_item(conn, index, id, item, None).unwrap();
        }
        let stored = stored_collection(conn, index, id).unwrap();
        let collection = resolve(content_db, index, stored, &[]).unwrap();
        render(content_db, &collection, &[], &labels(), "apa").unwrap()
    }

    #[test]
    fn converts_the_tafsir_html_subset() {
        assert_eq!(
            html_to_markdown("a <i>b</i> <span class=\"ayah\">c</span>"),
            "a *b* c"
        );
        assert_eq!(html_to_markdown("1 < 2 &amp; 3"), "1 < 2 & 3");
        assert_eq!(html_to_markdown("x<br/>y"), "x  \ny");
    }

    #[test]
    fn quotes_every_line() {
        assert_eq!(quote("a\nb"), "> a\n> b");
    }
}
