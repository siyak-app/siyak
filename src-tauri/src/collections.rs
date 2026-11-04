//! Research collections (spec §11.1): an ordered list of verses, notes, roots,
//! lemmas and tafsir passages with a short annotation on each.
//!
//! Items refer to content by what stays true when packs change: roots and
//! lemmas by their text, a tafsir passage by pack and verse. An item whose
//! content is missing (the pack was removed) stays in the collection and shows
//! up again when the pack is back.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::db::content::{
    self, TafsirInfo, TafsirPassage, VerseIndex, VerseRef, VerseWithTranslations,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::morphology::{self, LemmaInfo, RootInfo};
use crate::notes::{self, NOW, Note};

/// What a collection item refers to. Stored in `collection_items` as `kind`
/// and `ref_json` (the other fields), e.g. `{"from":262,"to":263}` (spec §5.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemRef {
    /// A verse or a range of verses, by global id.
    VerseRange {
        /// First verse.
        from: u32,
        /// Last verse.
        to: u32,
    },
    /// One of the user's notes.
    Note {
        /// Note id.
        id: u32,
    },
    /// A root, e.g. `ر ح م`.
    Root {
        /// Root text.
        root: String,
    },
    /// A lemma, as in the morphology pack.
    Lemma {
        /// Lemma text.
        lemma: String,
    },
    /// The commentary of a tafsir on a verse.
    Tafsir {
        /// Tafsir pack id.
        pack_id: String,
        /// Global id of the verse.
        verse: u32,
    },
}

impl ItemRef {
    /// The `kind` and `ref_json` columns.
    pub(crate) fn to_columns(&self) -> AppResult<(String, String)> {
        let invalid =
            |err: serde_json::Error| AppError::new(AppErrorCode::QueryFailed, err.to_string());
        let mut value = serde_json::to_value(self).map_err(invalid)?;
        let kind = value
            .as_object_mut()
            .and_then(|fields| fields.remove("kind"))
            .and_then(|kind| kind.as_str().map(str::to_owned))
            .unwrap_or_default();
        Ok((kind, value.to_string()))
    }

    /// An item from its `kind` and `ref_json` columns.
    pub(crate) fn from_columns(kind: &str, json: &str) -> Result<Self, serde_json::Error> {
        let mut value: serde_json::Value = serde_json::from_str(json)?;
        if let Some(fields) = value.as_object_mut() {
            fields.insert("kind".to_owned(), kind.into());
        }
        serde_json::from_value(value)
    }

    fn check(&self, conn: &Connection, index: &VerseIndex) -> AppResult<()> {
        let invalid = |what: &str| Err(AppError::new(AppErrorCode::InvalidInput, what.to_owned()));
        match self {
            Self::VerseRange { from, to } => index.check_range(*from, *to),
            Self::Tafsir { pack_id, verse } => {
                index.check_range(*verse, *verse)?;
                if pack_id.is_empty() {
                    return invalid("empty tafsir pack id");
                }
                Ok(())
            }
            Self::Note { id } => {
                if notes::get_note(conn, index, *id)?.is_none() {
                    return invalid("no such note");
                }
                Ok(())
            }
            Self::Root { root: text } | Self::Lemma { lemma: text } => {
                if text.trim().is_empty() {
                    return invalid("empty root or lemma");
                }
                Ok(())
            }
        }
    }
}

/// A collection without its items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct CollectionSummary {
    /// Collection id.
    pub id: u32,
    /// Title.
    pub title: String,
    /// Markdown description.
    pub description: Option<String>,
    /// Number of items.
    pub item_count: u32,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub updated_at: String,
}

/// A collection to create (no `id`) or rename.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
pub struct CollectionInput {
    /// The collection to change, or `None` for a new one.
    pub id: Option<u32>,
    /// Title; must not be blank.
    pub title: String,
    /// Markdown description.
    pub description: Option<String>,
}

/// An item of a collection, without its content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct StoredItem {
    /// Item id.
    pub id: u32,
    /// What the item refers to.
    pub item: ItemRef,
    /// Short note on this item.
    pub annotation: Option<String>,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
}

/// The content an item refers to, as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemContent {
    /// The verses, with the requested translations.
    VerseRange {
        /// The verses in order.
        verses: Vec<VerseWithTranslations>,
    },
    /// The note, unless it has gone.
    Note {
        /// The note.
        note: Option<Note>,
    },
    /// The root, if the installed morphology pack has it.
    Root {
        /// The root.
        root: Option<RootInfo>,
    },
    /// The lemma, if the installed morphology pack has it.
    Lemma {
        /// The lemma.
        lemma: Option<LemmaInfo>,
    },
    /// The commentary, if the pack is installed and covers the verse.
    Tafsir {
        /// The verse the item was added on.
        verse: Option<VerseRef>,
        /// The tafsir pack.
        tafsir: Option<TafsirInfo>,
        /// The passage covering the verse.
        passage: Option<TafsirPassage>,
    },
}

/// An item with its content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct CollectionItem {
    /// Item id.
    pub id: u32,
    /// What the item refers to.
    pub item: ItemRef,
    /// Short note on this item.
    pub annotation: Option<String>,
    /// The content it refers to.
    pub content: ItemContent,
}

/// A collection with its items in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Collection {
    /// The collection.
    pub summary: CollectionSummary,
    /// The items in the user's order.
    pub items: Vec<CollectionItem>,
}

/// A collection as `user.db` knows it; [`resolve`] adds what `content.db` knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCollection {
    /// The collection.
    pub summary: CollectionSummary,
    /// The items in the user's order, each with its note if it is one.
    pub items: Vec<(StoredItem, Option<Note>)>,
}

const SUMMARY_SELECT: &str = "SELECT c.id, c.title, c.description, c.created_at, c.updated_at,
        (SELECT count(*) FROM collection_items i WHERE i.collection_id = c.id)
     FROM collections c";

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CollectionSummary> {
    Ok(CollectionSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        item_count: row.get(5)?,
    })
}

/// Every collection, the most recently changed first.
pub fn list_collections(conn: &Connection) -> AppResult<Vec<CollectionSummary>> {
    let mut stmt = conn.prepare(&format!(
        "{SUMMARY_SELECT} ORDER BY c.updated_at DESC, c.id DESC"
    ))?;
    let rows = stmt.query_map([], summary_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn summary(conn: &Connection, id: u32) -> AppResult<CollectionSummary> {
    conn.query_row(
        &format!("{SUMMARY_SELECT} WHERE c.id = ?1"),
        [id],
        summary_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("collection {id}")))
}

/// Creates or changes a collection and returns it.
pub fn upsert_collection(
    conn: &Connection,
    input: &CollectionInput,
) -> AppResult<CollectionSummary> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::new(AppErrorCode::InvalidInput, "empty title"));
    }
    let description = input
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty());
    let id = match input.id {
        Some(id) => {
            let changed = conn.execute(
                &format!(
                    "UPDATE collections SET title = ?2, description = ?3, updated_at = {NOW}
                     WHERE id = ?1"
                ),
                (id, title, description),
            )?;
            if changed == 0 {
                return Err(AppError::new(
                    AppErrorCode::NotFound,
                    format!("collection {id}"),
                ));
            }
            id
        }
        None => {
            conn.execute(
                &format!(
                    "INSERT INTO collections (title, description, created_at, updated_at)
                     VALUES (?1, ?2, {NOW}, {NOW})"
                ),
                (title, description),
            )?;
            row_id(conn)?
        }
    };
    summary(conn, id)
}

pub(crate) fn row_id(conn: &Connection) -> AppResult<u32> {
    u32::try_from(conn.last_insert_rowid())
        .map_err(|err| AppError::new(AppErrorCode::QueryFailed, err.to_string()))
}

/// Deletes a collection and its items; the notes themselves stay.
pub fn delete_collection(conn: &Connection, id: u32) -> AppResult<()> {
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM collections WHERE id = ?1", [id])?;
    Ok(())
}

fn touch(conn: &Connection, collection_id: u32) -> AppResult<()> {
    conn.execute(
        &format!("UPDATE collections SET updated_at = {NOW} WHERE id = ?1"),
        [collection_id],
    )?;
    Ok(())
}

fn clean_annotation(annotation: Option<&str>) -> Option<&str> {
    annotation.map(str::trim).filter(|a| !a.is_empty())
}

/// Adds an item at the end of a collection and returns its id.
pub fn add_item(
    conn: &Connection,
    index: &VerseIndex,
    collection_id: u32,
    item: &ItemRef,
    annotation: Option<&str>,
) -> AppResult<u32> {
    summary(conn, collection_id)?;
    item.check(conn, index)?;
    let (kind, json) = item.to_columns()?;
    conn.execute(
        &format!(
            "INSERT INTO collection_items
               (collection_id, position, kind, ref_json, annotation, created_at)
             VALUES (?1,
               (SELECT coalesce(max(position) + 1, 0) FROM collection_items WHERE collection_id = ?1),
               ?2, ?3, ?4, {NOW})"
        ),
        (collection_id, kind, json, clean_annotation(annotation)),
    )?;
    let id = row_id(conn)?;
    touch(conn, collection_id)?;
    Ok(id)
}

fn item_collection(conn: &Connection, item_id: u32) -> AppResult<u32> {
    conn.query_row(
        "SELECT collection_id FROM collection_items WHERE id = ?1",
        [item_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("collection item {item_id}")))
}

/// Changes an item's annotation.
pub fn update_item(conn: &Connection, item_id: u32, annotation: Option<&str>) -> AppResult<()> {
    let collection_id = item_collection(conn, item_id)?;
    conn.execute(
        "UPDATE collection_items SET annotation = ?2 WHERE id = ?1",
        (item_id, clean_annotation(annotation)),
    )?;
    touch(conn, collection_id)
}

/// Moves an item to `to_index` in its collection (0 is first; past the end is last).
pub fn move_item(conn: &mut Connection, item_id: u32, to_index: u32) -> AppResult<()> {
    let tx = conn.transaction()?;
    let collection_id = item_collection(&tx, item_id)?;
    let mut order: Vec<u32> = tx
        .prepare("SELECT id FROM collection_items WHERE collection_id = ?1 ORDER BY position, id")?
        .query_map([collection_id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    order.retain(|&id| id != item_id);
    let at = usize::try_from(to_index)
        .unwrap_or(usize::MAX)
        .min(order.len());
    order.insert(at, item_id);
    for (position, id) in order.into_iter().enumerate() {
        tx.execute(
            "UPDATE collection_items SET position = ?2 WHERE id = ?1",
            (id, i64::try_from(position).unwrap_or(i64::MAX)),
        )?;
    }
    touch(&tx, collection_id)?;
    tx.commit()?;
    Ok(())
}

/// Removes an item from its collection.
pub fn remove_item(conn: &Connection, item_id: u32) -> AppResult<()> {
    let collection_id = item_collection(conn, item_id)?;
    conn.execute("DELETE FROM collection_items WHERE id = ?1", [item_id])?;
    touch(conn, collection_id)
}

/// The items of a collection, in order. A row that cannot be read (a future
/// kind, a hand-edited database) is left out rather than failing the screen.
pub(crate) fn stored_items(conn: &Connection, collection_id: u32) -> AppResult<Vec<StoredItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, ref_json, annotation, created_at FROM collection_items
         WHERE collection_id = ?1 ORDER BY position, id",
    )?;
    let rows = stmt.query_map([collection_id], |row| {
        let kind: String = row.get(1)?;
        let json: String = row.get(2)?;
        Ok(ItemRef::from_columns(&kind, &json).ok().map(|item| {
            Ok::<_, rusqlite::Error>(StoredItem {
                id: row.get(0)?,
                item,
                annotation: row.get(3)?,
                created_at: row.get(4)?,
            })
        }))
    })?;
    let mut items = Vec::new();
    for row in rows {
        if let Some(item) = row? {
            items.push(item?);
        }
    }
    Ok(items)
}

/// Reads a collection from `user.db`.
pub fn stored_collection(
    conn: &Connection,
    index: &VerseIndex,
    id: u32,
) -> AppResult<StoredCollection> {
    let summary = summary(conn, id)?;
    let items = stored_items(conn, id)?
        .into_iter()
        .map(|item| {
            let note = match item.item {
                ItemRef::Note { id } => notes::get_note(conn, index, id)?,
                _ => None,
            };
            Ok((item, note))
        })
        .collect::<AppResult<_>>()?;
    Ok(StoredCollection { summary, items })
}

/// Adds the content of each item from `content.db`, verses with the given translations.
pub fn resolve(
    conn: &Connection,
    index: &VerseIndex,
    stored: StoredCollection,
    translation_ids: &[String],
) -> AppResult<Collection> {
    let items = stored
        .items
        .into_iter()
        .map(|(item, note)| {
            let content = item_content(conn, index, &item.item, note, translation_ids)?;
            Ok(CollectionItem {
                id: item.id,
                item: item.item,
                annotation: item.annotation,
                content,
            })
        })
        .collect::<AppResult<_>>()?;
    Ok(Collection {
        summary: stored.summary,
        items,
    })
}

fn item_content(
    conn: &Connection,
    index: &VerseIndex,
    item: &ItemRef,
    note: Option<Note>,
    translation_ids: &[String],
) -> AppResult<ItemContent> {
    Ok(match item {
        ItemRef::VerseRange { from, to } => ItemContent::VerseRange {
            verses: content::get_verse_range(conn, *from, *to, translation_ids)?,
        },
        ItemRef::Note { .. } => ItemContent::Note { note },
        ItemRef::Root { root } => ItemContent::Root {
            root: morphology::root_by_text(conn, root)?,
        },
        ItemRef::Lemma { lemma } => ItemContent::Lemma {
            lemma: morphology::lemma_by_text(conn, lemma)?,
        },
        ItemRef::Tafsir { pack_id, verse } => {
            let tafsir = content::list_tafsirs(conn)?
                .into_iter()
                .find(|tafsir| tafsir.id == *pack_id);
            let passage = match tafsir {
                Some(_) => content::get_tafsir(conn, *verse, pack_id)?,
                None => None,
            };
            ItemContent::Tafsir {
                verse: index.verse(*verse),
                tafsir,
                passage,
            }
        }
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Small databases for the tests of the user data modules.

    use rusqlite::Connection;

    use crate::db::{content, migrate, user};

    /// Three verses, a translation and a tafsir with citation metadata, and a root.
    pub(crate) fn content() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, content::MIGRATIONS).unwrap();
        conn.execute_batch(
            "INSERT INTO surahs (number, name_ar, name_tr, name_en, revelation_type, ayah_count)
               VALUES (1, 'ف', 'Fâtiha', 'Al-Fatihah', 'meccan', 2),
                      (2, 'ب', 'Bakara', 'Al-Baqarah', 'medinan', 1);
             INSERT INTO verses (id, surah, ayah, juz, text_uthmani, text_clean) VALUES
               (1, 1, 1, 1, 'بسم', 'x'), (2, 1, 2, 1, 'الحمد', 'y'), (3, 2, 1, 1, 'الم', 'z');
             INSERT INTO packs (id, type, lang, title, author, version, license, attribution,
                                citation_json, installed_at) VALUES
               ('en.test', 'translation', 'en', 'Test Translation', NULL, '1.0.0', 'PD', NULL,
                '{\"type\":\"translation\",\"author\":\"A. Translator\",\"title\":\"The Meaning\",\"publisher\":\"Knopf\",\"year\":\"1930\"}', 'now'),
               ('tr.bare', 'translation', 'tr', 'Yalın Meal', 'Bir Mütercim', '1.0.0', 'PD', NULL, NULL, 'now'),
               ('ar.tafsir', 'tafsir', 'ar', 'Tafsir Test', 'al-Author', '1.0.0', 'PD',
                'Source: Quranpedia.net',
                '{\"type\":\"tafsir\",\"author\":\"al-Author\",\"title\":\"Great Tafsir\",\"publisher\":\"Dar\",\"year\":\"1417\"}', 'now'),
               ('morphology.qac', 'morphology', NULL, 'QAC', 'K. Dukes', '1.0.0', 'GPL',
                'Source: Quranic Arabic Corpus (corpus.quran.com)',
                '{\"type\":\"dataset\",\"author\":\"Kais Dukes\",\"title\":\"Quranic Arabic Corpus\",\"url\":\"https://corpus.quran.com\"}', 'now');
             INSERT INTO translations (pack_id, verse_id, text) VALUES
               ('en.test', 1, 'In the name'), ('en.test', 2, 'Praise'), ('tr.bare', 1, 'Adıyla');
             INSERT INTO tafsir (id, pack_id, verse_from, verse_to) VALUES (7, 'ar.tafsir', 1, 2);
             INSERT INTO tafsir_paragraphs (tafsir_id, idx, text, part, page) VALUES
               (7, 0, 'Bu <b>ayet</b> &amp; devamı<br>ikinci satır', '1', 113), (7, 1, 'ikinci', '1', 114);
             INSERT INTO roots (id, root) VALUES (1, 'ر ح م');
             INSERT INTO lemmas (id, lemma, root_id) VALUES (1, 'رَحْمَة', 1);
             INSERT INTO words (id, verse_id, position, text_ar) VALUES (1, 1, 1, 'بسم');
             INSERT INTO word_segments (word_id, idx, kind, text_ar, pos, lemma_id, root_id)
               VALUES (1, 1, 'stem', 'بسم', 'N', 1, 1);
             INSERT INTO notices (id, source, title, body, url)
               VALUES ('tanzil.text.uthmani', 'tanzil', 'Tanzil Quran Text (Uthmani)', 'x', 'https://tanzil.net');",
        )
        .unwrap();
        conn
    }

    /// An empty `user.db`.
    pub(crate) fn user() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&mut conn, user::MIGRATIONS).unwrap();
        conn
    }
}

#[cfg(test)]
#[expect(clippy::indexing_slicing, reason = "a panic fails the test")]
mod tests {
    use super::test_support::{content, user};
    use super::*;
    use crate::notes::{NoteInput, upsert_note};

    fn new_collection(conn: &Connection, title: &str) -> u32 {
        upsert_collection(
            conn,
            &CollectionInput {
                id: None,
                title: title.to_owned(),
                description: None,
            },
        )
        .unwrap()
        .id
    }

    fn root(text: &str) -> ItemRef {
        ItemRef::Root {
            root: text.to_owned(),
        }
    }

    fn order(conn: &Connection, id: u32) -> Vec<ItemRef> {
        stored_items(conn, id)
            .unwrap()
            .into_iter()
            .map(|item| item.item)
            .collect()
    }

    #[test]
    fn references_are_stored_as_the_spec_says() {
        let (kind, json) = ItemRef::VerseRange { from: 262, to: 263 }
            .to_columns()
            .unwrap();
        assert_eq!(kind, "verse_range");
        assert_eq!(json, r#"{"from":262,"to":263}"#);
        let (kind, json) = root("ر ح م").to_columns().unwrap();
        assert_eq!(kind, "root");
        assert_eq!(json, r#"{"root":"ر ح م"}"#);
        assert_eq!(ItemRef::from_columns(&kind, &json).unwrap(), root("ر ح م"));
    }

    #[test]
    fn collections_are_created_renamed_and_listed() {
        let conn = user();
        let id = new_collection(&conn, "  Sabır ");
        let changed = upsert_collection(
            &conn,
            &CollectionInput {
                id: Some(id),
                title: "Sabır ve namaz".to_owned(),
                description: Some("  ".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(changed.title, "Sabır ve namaz");
        assert_eq!(changed.description, None);
        assert_eq!(list_collections(&conn).unwrap(), [changed]);
    }

    #[test]
    fn a_collection_needs_a_title() {
        let err = upsert_collection(
            &user(),
            &CollectionInput {
                id: None,
                title: " ".to_owned(),
                description: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, AppErrorCode::InvalidInput);
    }

    #[test]
    fn items_are_added_moved_and_removed() {
        let index = VerseIndex::load(&content()).unwrap();
        let mut conn = user();
        let id = new_collection(&conn, "t");
        let a = add_item(&conn, &index, id, &root("a"), Some(" note ")).unwrap();
        let b = add_item(&conn, &index, id, &root("b"), None).unwrap();
        let c = add_item(&conn, &index, id, &root("c"), None).unwrap();
        assert_eq!(
            stored_items(&conn, id).unwrap()[0].annotation.as_deref(),
            Some("note")
        );
        assert_eq!(list_collections(&conn).unwrap()[0].item_count, 3);

        move_item(&mut conn, c, 0).unwrap();
        assert_eq!(order(&conn, id), [root("c"), root("a"), root("b")]);
        move_item(&mut conn, c, 99).unwrap();
        assert_eq!(order(&conn, id), [root("a"), root("b"), root("c")]);
        move_item(&mut conn, a, 1).unwrap();
        assert_eq!(order(&conn, id), [root("b"), root("a"), root("c")]);

        remove_item(&conn, b).unwrap();
        update_item(&conn, a, None).unwrap();
        assert_eq!(order(&conn, id), [root("a"), root("c")]);
        assert_eq!(stored_items(&conn, id).unwrap()[0].annotation, None);
        // Positions left with gaps still add at the end.
        add_item(&conn, &index, id, &root("d"), None).unwrap();
        assert_eq!(order(&conn, id), [root("a"), root("c"), root("d")]);
    }

    #[test]
    fn invalid_items_are_rejected() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        let id = new_collection(&conn, "t");
        for bad in [
            ItemRef::VerseRange { from: 2, to: 1 },
            ItemRef::VerseRange { from: 1, to: 999 },
            ItemRef::Note { id: 42 },
            root(" "),
            ItemRef::Tafsir {
                pack_id: "ar.tafsir".to_owned(),
                verse: 999,
            },
        ] {
            let err = add_item(&conn, &index, id, &bad, None).unwrap_err();
            assert_eq!(err.code, AppErrorCode::InvalidInput, "{bad:?}");
        }
        let err = add_item(&conn, &index, 99, &root("a"), None).unwrap_err();
        assert_eq!(err.code, AppErrorCode::NotFound);
    }

    #[test]
    fn deleting_a_collection_keeps_the_notes() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        let id = new_collection(&conn, "t");
        let note = upsert_note(
            &conn,
            &index,
            &NoteInput {
                id: None,
                verse_from: 1,
                verse_to: 1,
                body: "kalır".to_owned(),
            },
        )
        .unwrap();
        add_item(&conn, &index, id, &ItemRef::Note { id: note.id }, None).unwrap();
        delete_collection(&conn, id).unwrap();
        assert!(list_collections(&conn).unwrap().is_empty());
        assert!(notes::get_note(&conn, &index, note.id).unwrap().is_some());
        let items: u32 = conn
            .query_row("SELECT count(*) FROM collection_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(items, 0);
    }

    #[test]
    fn items_resolve_to_their_content() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = user();
        let id = new_collection(&conn, "t");
        let note = upsert_note(
            &conn,
            &index,
            &NoteInput {
                id: None,
                verse_from: 2,
                verse_to: 2,
                body: "not".to_owned(),
            },
        )
        .unwrap();
        for item in [
            ItemRef::VerseRange { from: 1, to: 2 },
            ItemRef::Note { id: note.id },
            root("ر ح م"),
            ItemRef::Lemma {
                lemma: "رَحْمَة".to_owned(),
            },
            ItemRef::Tafsir {
                pack_id: "ar.tafsir".to_owned(),
                verse: 2,
            },
        ] {
            add_item(&conn, &index, id, &item, None).unwrap();
        }
        let stored = stored_collection(&conn, &index, id).unwrap();
        let found = resolve(&content_db, &index, stored, &["en.test".to_owned()]).unwrap();

        let ItemContent::VerseRange { verses } = &found.items[0].content else {
            panic!("{:?}", found.items[0].content);
        };
        assert_eq!(verses.len(), 2);
        assert_eq!(verses[1].translations[0].text, "Praise");
        assert!(
            matches!(&found.items[1].content, ItemContent::Note { note: Some(n) } if n.body == "not")
        );
        assert!(
            matches!(&found.items[2].content, ItemContent::Root { root: Some(r) } if r.occurrences == 1)
        );
        assert!(
            matches!(&found.items[3].content, ItemContent::Lemma { lemma: Some(l) } if l.occurrences == 1)
        );
        let ItemContent::Tafsir {
            verse,
            tafsir,
            passage,
        } = &found.items[4].content
        else {
            panic!("{:?}", found.items[4].content);
        };
        assert_eq!(*verse, Some(VerseRef { surah: 1, ayah: 2 }));
        assert_eq!(tafsir.as_ref().unwrap().id, "ar.tafsir");
        assert_eq!(
            passage.as_ref().unwrap().verse_from,
            VerseRef { surah: 1, ayah: 1 }
        );
    }

    #[test]
    fn items_survive_their_content_going_away() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = user();
        let id = new_collection(&conn, "t");
        add_item(&conn, &index, id, &root("ر ح م"), None).unwrap();
        add_item(
            &conn,
            &index,
            id,
            &ItemRef::Tafsir {
                pack_id: "ar.tafsir".to_owned(),
                verse: 1,
            },
            None,
        )
        .unwrap();
        content_db
            .execute_batch(
                "DELETE FROM word_segments; DELETE FROM lemmas; DELETE FROM roots;
                 DELETE FROM packs WHERE type IN ('tafsir', 'morphology');",
            )
            .unwrap();
        let stored = stored_collection(&conn, &index, id).unwrap();
        let found = resolve(&content_db, &index, stored, &[]).unwrap();
        assert!(matches!(
            found.items[0].content,
            ItemContent::Root { root: None }
        ));
        assert!(matches!(
            found.items[1].content,
            ItemContent::Tafsir {
                tafsir: None,
                passage: None,
                verse: Some(_)
            }
        ));
    }

    #[test]
    fn an_unreadable_item_does_not_break_the_collection() {
        let index = VerseIndex::load(&content()).unwrap();
        let conn = user();
        let id = new_collection(&conn, "t");
        add_item(&conn, &index, id, &root("a"), None).unwrap();
        conn.execute(
            "INSERT INTO collection_items (collection_id, position, kind, ref_json, created_at)
             VALUES (?1, 5, 'root', 'not json', 'now')",
            [id],
        )
        .unwrap();
        assert_eq!(order(&conn, id), [root("a")]);
    }
}
