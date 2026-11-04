//! JSON export/import of bookmarks, notes, collections and the tafsir
//! translation cache, keyed by `surah:ayah` for portability (spec §11.2).
//!
//! Each direction has two steps, one per database, so no command holds both
//! database locks at once: export reads `user.db` ([`collect`]) and then makes
//! the ids portable with `content.db` ([`portable`]); import does the reverse
//! ([`prepare`], [`apply`]).
//!
//! A collection item of kind `note` refers to its note by position in the
//! file's `notes` array (`{"note": 3}`), since note ids do not survive a move.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::collections::{ItemRef, StoredItem, row_id, stored_items};
use crate::db::content::{VerseIndex, VerseRef};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::notes::NOW;

/// The `format` field of a backup.
const FORMAT: &str = "siyak-userdata";

/// The newest `version` this app reads and the one it writes.
const VERSION: u32 = 1;

/// A backup file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserData {
    format: String,
    version: u32,
    exported_at: String,
    #[serde(default)]
    bookmarks: Vec<BookmarkEntry>,
    #[serde(default)]
    notes: Vec<NoteEntry>,
    #[serde(default)]
    collections: Vec<CollectionEntry>,
    #[serde(default)]
    tafsir_translations: Vec<TranslationEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BookmarkEntry {
    surah: u16,
    ayah: u16,
    created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NoteEntry {
    from: String,
    to: String,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CollectionEntry {
    title: String,
    #[serde(default)]
    description: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    items: Vec<ItemEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ItemEntry {
    kind: String,
    #[serde(rename = "ref")]
    reference: serde_json::Value,
    #[serde(default)]
    annotation: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TranslationEntry {
    pack_id: String,
    surah: u16,
    ayah: u16,
    /// `true` for the author's introduction, which is keyed to 1:1 but is not commentary on it.
    #[serde(default)]
    introduction: bool,
    paragraph_idx: i32,
    mode: String,
    target_lang: String,
    text: String,
    provider: String,
    model: String,
    pack_version: String,
    created_at: String,
}

/// How an import treats what is already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// Keep everything and add what is new. Importing the same file twice adds nothing.
    Merge,
    /// Delete bookmarks, notes, collections and the translation cache first.
    Replace,
}

/// What an import did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
pub struct ImportReport {
    /// Bookmarks added.
    pub bookmarks: u32,
    /// Notes added.
    pub notes: u32,
    /// Collections added.
    pub collections: u32,
    /// Collection items added, also to collections that already existed.
    pub items: u32,
    /// Tafsir translations added.
    pub tafsir_translations: u32,
    /// Entries left out: unknown verses, tafsirs that are not installed, unreadable items.
    pub skipped: u32,
}

/// What [`collect`] reads from `user.db`.
#[derive(Debug, Clone, Default)]
pub struct Collected {
    bookmarks: Vec<(u32, String)>,
    notes: Vec<CollectedNote>,
    collections: Vec<(CollectionEntry, Vec<StoredItem>)>,
    translations: Vec<CollectedTranslation>,
    exported_at: String,
}

#[derive(Debug, Clone)]
struct CollectedNote {
    id: u32,
    from: u32,
    to: u32,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct CollectedTranslation {
    pack_id: String,
    tafsir_id: u32,
    entry: TranslationEntry,
}

/// Reads everything a backup holds from `user.db`.
pub fn collect(conn: &Connection) -> AppResult<Collected> {
    let bookmarks = conn
        .prepare("SELECT verse_id, created_at FROM bookmarks ORDER BY id")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let notes = conn
        .prepare(
            "SELECT id, verse_from, verse_to, body, created_at, updated_at FROM notes ORDER BY id",
        )?
        .query_map([], |row| {
            Ok(CollectedNote {
                id: row.get(0)?,
                from: row.get(1)?,
                to: row.get(2)?,
                body: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let rows: Vec<(u32, CollectionEntry)> = conn
        .prepare(
            "SELECT id, title, description, created_at, updated_at FROM collections ORDER BY id",
        )?
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                CollectionEntry {
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    items: Vec::new(),
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    let collections = rows
        .into_iter()
        .map(|(id, entry)| Ok((entry, stored_items(conn, id)?)))
        .collect::<AppResult<_>>()?;
    let translations = conn
        .prepare(
            "SELECT pack_id, tafsir_id, paragraph_idx, mode, target_lang, pack_version, text,
                    provider, model, created_at
             FROM tafsir_translations",
        )?
        .query_map([], |row| {
            Ok(CollectedTranslation {
                pack_id: row.get(0)?,
                tafsir_id: row.get(1)?,
                entry: TranslationEntry {
                    pack_id: row.get(0)?,
                    surah: 0,
                    ayah: 0,
                    introduction: false,
                    paragraph_idx: row.get(2)?,
                    mode: row.get(3)?,
                    target_lang: row.get(4)?,
                    pack_version: row.get(5)?,
                    text: row.get(6)?,
                    provider: row.get(7)?,
                    model: row.get(8)?,
                    created_at: row.get(9)?,
                },
            })
        })?
        .collect::<Result<_, _>>()?;
    let exported_at = conn.query_row(&format!("SELECT {NOW}"), [], |row| row.get(0))?;
    Ok(Collected {
        bookmarks,
        notes,
        collections,
        translations,
        exported_at,
    })
}

fn verse_text(verse: VerseRef) -> String {
    format!("{}:{}", verse.surah, verse.ayah)
}

fn parse_verse(text: &str, index: &VerseIndex) -> Option<u32> {
    let (surah, ayah) = text.split_once(':')?;
    index.id(surah.parse().ok()?, ayah.parse().ok()?)
}

/// Turns ids into `surah:ayah`. Something the current text or packs cannot
/// place (a tafsir translation whose pack was removed) stays out of the file.
pub fn portable(
    collected: Collected,
    content: &Connection,
    index: &VerseIndex,
) -> AppResult<UserData> {
    let bookmarks = collected
        .bookmarks
        .into_iter()
        .filter_map(|(id, created_at)| {
            let verse = index.verse(id)?;
            Some(BookmarkEntry {
                surah: verse.surah,
                ayah: verse.ayah,
                created_at,
            })
        })
        .collect();

    let mut note_positions = HashMap::new();
    let mut notes = Vec::new();
    for note in collected.notes {
        let (Some(from), Some(to)) = (index.verse(note.from), index.verse(note.to)) else {
            continue;
        };
        note_positions.insert(note.id, notes.len());
        notes.push(NoteEntry {
            from: verse_text(from),
            to: verse_text(to),
            body: note.body,
            created_at: note.created_at,
            updated_at: note.updated_at,
        });
    }

    let collections = collected
        .collections
        .into_iter()
        .map(|(mut entry, items)| {
            entry.items = items
                .into_iter()
                .filter_map(|stored| {
                    let (kind, reference) = portable_ref(&stored.item, index, &note_positions)?;
                    Some(ItemEntry {
                        kind,
                        reference,
                        annotation: stored.annotation,
                        created_at: stored.created_at,
                    })
                })
                .collect();
            entry
        })
        .collect();

    let mut tafsir_translations = Vec::new();
    let mut place = content.prepare(
        "SELECT verse_from, kind = 'introduction' FROM tafsir WHERE id = ?1 AND pack_id = ?2",
    )?;
    for translation in collected.translations {
        let placed: Option<(u32, bool)> = place
            .query_row((translation.tafsir_id, &translation.pack_id), |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        let Some((verse, introduction)) = placed else {
            continue;
        };
        let Some(verse) = index.verse(verse) else {
            continue;
        };
        tafsir_translations.push(TranslationEntry {
            surah: verse.surah,
            ayah: verse.ayah,
            introduction,
            ..translation.entry
        });
    }

    Ok(UserData {
        format: FORMAT.to_owned(),
        version: VERSION,
        exported_at: collected.exported_at,
        bookmarks,
        notes,
        collections,
        tafsir_translations,
    })
}

fn portable_ref(
    item: &ItemRef,
    index: &VerseIndex,
    note_positions: &HashMap<u32, usize>,
) -> Option<(String, serde_json::Value)> {
    use serde_json::json;
    Some(match item {
        ItemRef::VerseRange { from, to } => (
            "verse_range".to_owned(),
            json!({
                "from": verse_text(index.verse(*from)?),
                "to": verse_text(index.verse(*to)?),
            }),
        ),
        ItemRef::Note { id } => (
            "note".to_owned(),
            json!({ "note": note_positions.get(id)? }),
        ),
        ItemRef::Root { root } => ("root".to_owned(), json!({ "root": root })),
        ItemRef::Lemma { lemma } => ("lemma".to_owned(), json!({ "lemma": lemma })),
        ItemRef::Tafsir { pack_id, verse } => (
            "tafsir".to_owned(),
            json!({ "pack_id": pack_id, "verse": verse_text(index.verse(*verse)?) }),
        ),
    })
}

/// Serializes a backup for the file.
pub fn to_json(data: &UserData) -> AppResult<String> {
    serde_json::to_string_pretty(data)
        .map_err(|err| AppError::new(AppErrorCode::QueryFailed, err.to_string()))
}

/// Reads a backup file's text. Fails with [`AppErrorCode::BackupUnsupported`]
/// for a newer version and [`AppErrorCode::BackupInvalid`] for anything else
/// that is not a backup.
pub fn parse(text: &str) -> AppResult<UserData> {
    let invalid =
        |err: serde_json::Error| AppError::new(AppErrorCode::BackupInvalid, err.to_string());
    #[derive(Deserialize)]
    struct Header {
        format: String,
        version: u32,
    }
    let header: Header = serde_json::from_str(text).map_err(invalid)?;
    if header.format != FORMAT {
        return Err(AppError::new(
            AppErrorCode::BackupInvalid,
            format!("format {:?}", header.format),
        ));
    }
    if header.version > VERSION {
        return Err(AppError::new(
            AppErrorCode::BackupUnsupported,
            format!("version {}", header.version),
        ));
    }
    serde_json::from_str(text).map_err(invalid)
}

/// A backup with its references turned into ids of this installation.
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    bookmarks: Vec<(u32, String)>,
    notes: Vec<Option<CollectedNote>>,
    collections: Vec<(CollectionEntry, Vec<PreparedItem>)>,
    translations: Vec<(u32, TranslationEntry)>,
    skipped: u32,
}

#[derive(Debug, Clone)]
struct PreparedItem {
    reference: PreparedRef,
    annotation: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone)]
enum PreparedRef {
    Item(ItemRef),
    /// Position in the file's `notes`.
    Note(usize),
}

/// Resolves a backup's verses and tafsirs against `content.db`, counting what cannot be placed.
pub fn prepare(data: UserData, content: &Connection, index: &VerseIndex) -> AppResult<Prepared> {
    let mut prepared = Prepared::default();
    for bookmark in data.bookmarks {
        if let Some(id) = index.id(bookmark.surah, bookmark.ayah) {
            prepared.bookmarks.push((id, bookmark.created_at));
        } else {
            prepared.skipped += 1;
        }
    }
    for note in data.notes {
        let range = parse_verse(&note.from, index).zip(parse_verse(&note.to, index));
        let note = range
            .filter(|(from, to)| from <= to && !note.body.trim().is_empty())
            .map(|(from, to)| CollectedNote {
                id: 0,
                from,
                to,
                body: note.body,
                created_at: note.created_at,
                updated_at: note.updated_at,
            });
        if note.is_none() {
            prepared.skipped += 1;
        }
        prepared.notes.push(note);
    }
    for mut collection in data.collections {
        if collection.title.trim().is_empty() {
            prepared.skipped += 1;
            continue;
        }
        let mut items = Vec::new();
        for item in std::mem::take(&mut collection.items) {
            if let Some(reference) = prepared_ref(&item, index) {
                items.push(PreparedItem {
                    reference,
                    annotation: item.annotation,
                    created_at: item.created_at,
                });
            } else {
                prepared.skipped += 1;
            }
        }
        prepared.collections.push((collection, items));
    }
    let mut find = content.prepare(
        "SELECT id FROM tafsir WHERE pack_id = ?1 AND verse_from = ?2 AND kind = ?3 LIMIT 1",
    )?;
    for translation in data.tafsir_translations {
        let kind = if translation.introduction {
            "introduction"
        } else {
            "commentary"
        };
        let id: Option<u32> = match index.id(translation.surah, translation.ayah) {
            Some(verse) => find
                .query_row((&translation.pack_id, verse, kind), |row| row.get(0))
                .optional()?,
            None => None,
        };
        if let Some(id) = id {
            prepared.translations.push((id, translation));
        } else {
            prepared.skipped += 1;
        }
    }
    Ok(prepared)
}

fn prepared_ref(item: &ItemEntry, index: &VerseIndex) -> Option<PreparedRef> {
    let text = |key: &str| item.reference.get(key)?.as_str();
    Some(match item.kind.as_str() {
        "verse_range" => {
            let from = parse_verse(text("from")?, index)?;
            let to = parse_verse(text("to")?, index)?;
            index.check_range(from, to).ok()?;
            PreparedRef::Item(ItemRef::VerseRange { from, to })
        }
        "note" => {
            let position = item.reference.get("note")?.as_u64()?;
            PreparedRef::Note(usize::try_from(position).ok()?)
        }
        "root" => PreparedRef::Item(ItemRef::Root {
            root: text("root")
                .filter(|root| !root.trim().is_empty())?
                .to_owned(),
        }),
        "lemma" => PreparedRef::Item(ItemRef::Lemma {
            lemma: text("lemma")
                .filter(|lemma| !lemma.trim().is_empty())?
                .to_owned(),
        }),
        "tafsir" => PreparedRef::Item(ItemRef::Tafsir {
            pack_id: text("pack_id").filter(|id| !id.is_empty())?.to_owned(),
            verse: parse_verse(text("verse")?, index)?,
        }),
        _ => return None,
    })
}

/// Writes a prepared backup to `user.db` in one transaction: on any failure
/// nothing changes.
pub fn apply(
    conn: &mut Connection,
    prepared: Prepared,
    mode: ImportMode,
) -> AppResult<ImportReport> {
    let mut report = ImportReport {
        skipped: prepared.skipped,
        ..ImportReport::default()
    };
    let tx = conn.transaction()?;
    if mode == ImportMode::Replace {
        tx.execute_batch(
            "DELETE FROM bookmarks; DELETE FROM notes; DELETE FROM collection_items;
             DELETE FROM collections; DELETE FROM tafsir_translations;",
        )?;
    }
    for (verse, created_at) in &prepared.bookmarks {
        report.bookmarks += changes(tx.execute(
            "INSERT OR IGNORE INTO bookmarks (verse_id, created_at) VALUES (?1, ?2)",
            (verse, created_at),
        )?);
    }
    let note_ids = import_notes(&tx, &prepared.notes, &mut report)?;
    for (entry, items) in &prepared.collections {
        import_collection(&tx, entry, items, &note_ids, &mut report)?;
    }
    for (tafsir_id, entry) in &prepared.translations {
        report.tafsir_translations += changes(tx.execute(
            "INSERT OR IGNORE INTO tafsir_translations
               (pack_id, tafsir_id, paragraph_idx, mode, target_lang, pack_version, text,
                provider, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                entry.pack_id,
                tafsir_id,
                entry.paragraph_idx,
                entry.mode,
                entry.target_lang,
                entry.pack_version,
                entry.text,
                entry.provider,
                entry.model,
                entry.created_at,
            ],
        )?);
    }
    tx.commit()?;
    Ok(report)
}

fn changes(rows: usize) -> u32 {
    u32::from(rows > 0)
}

/// Adds the notes that are not there yet (same verses and text). Returns the
/// id of each file position.
fn import_notes(
    tx: &Connection,
    notes: &[Option<CollectedNote>],
    report: &mut ImportReport,
) -> AppResult<Vec<Option<u32>>> {
    notes
        .iter()
        .map(|note| {
            let Some(note) = note else { return Ok(None) };
            let existing: Option<u32> = tx
                .query_row(
                    "SELECT id FROM notes WHERE verse_from = ?1 AND verse_to = ?2 AND body = ?3",
                    (note.from, note.to, &note.body),
                    |row| row.get(0),
                )
                .optional()?;
            if existing.is_some() {
                return Ok(existing);
            }
            tx.execute(
                "INSERT INTO notes (verse_from, verse_to, body, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    note.from,
                    note.to,
                    &note.body,
                    &note.created_at,
                    &note.updated_at,
                ),
            )?;
            report.notes += 1;
            Ok(Some(row_id(tx)?))
        })
        .collect()
}

/// Adds a collection, or its new items to one with the same title.
fn import_collection(
    tx: &Connection,
    entry: &CollectionEntry,
    items: &[PreparedItem],
    note_ids: &[Option<u32>],
    report: &mut ImportReport,
) -> AppResult<()> {
    let existing: Option<u32> = tx
        .query_row(
            "SELECT id FROM collections WHERE title = ?1 ORDER BY id LIMIT 1",
            [&entry.title],
            |row| row.get(0),
        )
        .optional()?;
    let id = if let Some(id) = existing {
        id
    } else {
        tx.execute(
            "INSERT INTO collections (title, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            (
                &entry.title,
                &entry.description,
                &entry.created_at,
                &entry.updated_at,
            ),
        )?;
        report.collections += 1;
        row_id(tx)?
    };
    let mut present: HashSet<ItemRef> = stored_items(tx, id)?
        .into_iter()
        .map(|stored| stored.item)
        .collect();
    let mut position: i64 = tx.query_row(
        "SELECT coalesce(max(position) + 1, 0) FROM collection_items WHERE collection_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    for item in items {
        let reference = match &item.reference {
            PreparedRef::Item(reference) => reference.clone(),
            PreparedRef::Note(file_position) => match note_ids.get(*file_position) {
                Some(Some(id)) => ItemRef::Note { id: *id },
                _ => {
                    report.skipped += 1;
                    continue;
                }
            },
        };
        if !present.insert(reference.clone()) {
            continue;
        }
        let (kind, json) = reference.to_columns()?;
        tx.execute(
            "INSERT INTO collection_items
               (collection_id, position, kind, ref_json, annotation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (id, position, kind, json, &item.annotation, &item.created_at),
        )?;
        position += 1;
        report.items += 1;
    }
    Ok(())
}

#[cfg(test)]
#[expect(clippy::indexing_slicing, reason = "a panic fails the test")]
mod tests {
    use super::*;
    use crate::collections::test_support::{content, user};
    use crate::collections::{CollectionInput, add_item, list_collections, upsert_collection};
    use crate::notes::{NoteInput, list_bookmarks, list_notes, toggle_bookmark, upsert_note};

    fn note_bodies(conn: &Connection, index: &VerseIndex) -> Vec<String> {
        list_notes(conn, index, None)
            .unwrap()
            .into_iter()
            .map(|note| note.body)
            .collect()
    }

    /// A user database with one of everything; the note is in the collection.
    fn filled(index: &VerseIndex) -> Connection {
        let mut conn = user();
        toggle_bookmark(&mut conn, index, 3).unwrap();
        let note = upsert_note(
            &conn,
            index,
            &NoteInput {
                id: None,
                verse_from: 1,
                verse_to: 2,
                body: "sabır".to_owned(),
            },
        )
        .unwrap();
        let id = upsert_collection(
            &conn,
            &CollectionInput {
                id: None,
                title: "Sabır".to_owned(),
                description: Some("açıklama".to_owned()),
            },
        )
        .unwrap()
        .id;
        for item in [
            ItemRef::VerseRange { from: 1, to: 2 },
            ItemRef::Note { id: note.id },
            ItemRef::Root {
                root: "ر ح م".to_owned(),
            },
            ItemRef::Tafsir {
                pack_id: "ar.tafsir".to_owned(),
                verse: 2,
            },
        ] {
            add_item(&conn, index, id, &item, Some("kısa")).unwrap();
        }
        conn.execute(
            "INSERT INTO tafsir_translations (pack_id, tafsir_id, mode, target_lang, pack_version,
                text, provider, model, created_at)
             VALUES ('ar.tafsir', 7, 'summary', 'tr', '1.0.0', 'özet', 'p', 'm', 'now')",
            [],
        )
        .unwrap();
        conn
    }

    fn export(conn: &Connection, content_db: &Connection, index: &VerseIndex) -> UserData {
        portable(collect(conn).unwrap(), content_db, index).unwrap()
    }

    fn import(
        conn: &mut Connection,
        data: UserData,
        content_db: &Connection,
        index: &VerseIndex,
        mode: ImportMode,
    ) -> ImportReport {
        apply(conn, prepare(data, content_db, index).unwrap(), mode).unwrap()
    }

    #[test]
    fn the_file_has_the_format_of_the_spec() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = filled(&index);
        let data = export(&conn, &content_db, &index);
        let json: serde_json::Value = serde_json::from_str(&to_json(&data).unwrap()).unwrap();
        assert_eq!(json["format"], "siyak-userdata");
        assert_eq!(json["version"], 1);
        assert_eq!(json["bookmarks"][0]["surah"], 2);
        assert_eq!(json["bookmarks"][0]["ayah"], 1);
        assert_eq!(json["notes"][0]["from"], "1:1");
        assert_eq!(json["notes"][0]["to"], "1:2");
        let items = &json["collections"][0]["items"];
        assert_eq!(items[0]["kind"], "verse_range");
        assert_eq!(items[0]["ref"]["from"], "1:1");
        assert_eq!(items[0]["annotation"], "kısa");
        assert_eq!(items[1]["ref"]["note"], 0);
        assert_eq!(items[3]["ref"]["verse"], "1:2");
        let translation = &json["tafsir_translations"][0];
        assert_eq!(translation["surah"], 1);
        assert_eq!(translation["ayah"], 1);
        assert_eq!(translation["paragraph_idx"], -1);
    }

    #[test]
    fn everything_survives_a_round_trip_to_an_empty_database() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let conn = filled(&index);
        let data = export(&conn, &content_db, &index);
        let text = to_json(&data).unwrap();

        let mut fresh = user();
        let report = import(
            &mut fresh,
            parse(&text).unwrap(),
            &content_db,
            &index,
            ImportMode::Merge,
        );
        assert_eq!(
            report,
            ImportReport {
                bookmarks: 1,
                notes: 1,
                collections: 1,
                items: 4,
                tafsir_translations: 1,
                skipped: 0,
            }
        );
        let mut again = export(&fresh, &content_db, &index);
        again.exported_at = data.exported_at.clone();
        // Timestamps of a new note and collection are kept, so the copy is identical.
        assert_eq!(again, data);
    }

    #[test]
    fn merging_the_same_file_twice_adds_nothing() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let mut conn = filled(&index);
        let data = export(&conn, &content_db, &index);
        let report = import(&mut conn, data, &content_db, &index, ImportMode::Merge);
        assert_eq!(report, ImportReport::default());
    }

    #[test]
    fn merging_adds_new_items_to_a_collection_with_the_same_title() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let mut conn = filled(&index);
        let mut data = export(&conn, &content_db, &index);
        data.collections[0].items.push(ItemEntry {
            kind: "lemma".to_owned(),
            reference: serde_json::json!({ "lemma": "رَحْمَة" }),
            annotation: None,
            created_at: "then".to_owned(),
        });
        data.notes[0].body = "başka".to_owned();
        let report = import(&mut conn, data, &content_db, &index, ImportMode::Merge);
        assert_eq!(report.collections, 0);
        assert_eq!(report.notes, 1);
        // The lemma, and the item of the note whose text is new.
        assert_eq!(report.items, 2);
        let collections = list_collections(&conn).unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].item_count, 6);
    }

    #[test]
    fn replacing_removes_what_was_there() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let mut conn = filled(&index);
        let data = UserData {
            format: FORMAT.to_owned(),
            version: VERSION,
            exported_at: "now".to_owned(),
            bookmarks: Vec::new(),
            notes: vec![NoteEntry {
                from: "2:1".to_owned(),
                to: "2:1".to_owned(),
                body: "yeni".to_owned(),
                created_at: "c".to_owned(),
                updated_at: "u".to_owned(),
            }],
            collections: Vec::new(),
            tafsir_translations: Vec::new(),
        };
        import(&mut conn, data, &content_db, &index, ImportMode::Replace);
        assert_eq!(note_bodies(&conn, &index), ["yeni"]);
        assert!(list_collections(&conn).unwrap().is_empty());
        assert!(list_bookmarks(&conn).unwrap().is_empty());
    }

    #[test]
    fn what_cannot_be_placed_is_skipped_and_counted() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let text = r#"{
          "format": "siyak-userdata", "version": 1, "exported_at": "now",
          "bookmarks": [{ "surah": 200, "ayah": 1, "created_at": "c" }],
          "notes": [
            { "from": "2:999", "to": "2:999", "body": "yok", "created_at": "c", "updated_at": "u" },
            { "from": "1:1", "to": "1:1", "body": "var", "created_at": "c", "updated_at": "u" }
          ],
          "collections": [{
            "title": "T", "created_at": "c", "updated_at": "u",
            "items": [
              { "kind": "note", "ref": { "note": 0 }, "created_at": "c" },
              { "kind": "note", "ref": { "note": 1 }, "created_at": "c" },
              { "kind": "note", "ref": { "note": 9 }, "created_at": "c" },
              { "kind": "video", "ref": {}, "created_at": "c" },
              { "kind": "verse_range", "ref": { "from": "1:2", "to": "1:1" }, "created_at": "c" }
            ]
          }],
          "tafsir_translations": [{
            "pack_id": "gone", "surah": 1, "ayah": 1, "paragraph_idx": 0, "mode": "summary",
            "target_lang": "tr", "text": "t", "provider": "p", "model": "m",
            "pack_version": "1", "created_at": "c"
          }]
        }"#;
        let mut conn = user();
        let report = import(
            &mut conn,
            parse(text).unwrap(),
            &content_db,
            &index,
            ImportMode::Merge,
        );
        assert_eq!(report.notes, 1);
        assert_eq!(report.items, 1);
        // Bookmark, note, video, reversed range, translation; then two items of absent notes.
        assert_eq!(report.skipped, 7);
    }

    #[test]
    fn files_that_are_not_backups_are_refused() {
        for text in ["", "[]", "{}", r#"{"format":"other","version":1}"#] {
            assert_eq!(
                parse(text).unwrap_err().code,
                AppErrorCode::BackupInvalid,
                "{text}"
            );
        }
        let newer = r#"{"format":"siyak-userdata","version":2}"#;
        assert_eq!(
            parse(newer).unwrap_err().code,
            AppErrorCode::BackupUnsupported
        );
        let truncated = r#"{"format":"siyak-userdata","version":1}"#;
        assert_eq!(
            parse(truncated).unwrap_err().code,
            AppErrorCode::BackupInvalid
        );
    }

    #[test]
    fn a_failed_import_changes_nothing() {
        let content_db = content();
        let index = VerseIndex::load(&content_db).unwrap();
        let mut conn = filled(&index);
        let data = export(&conn, &content_db, &index);
        let prepared = prepare(data, &content_db, &index).unwrap();
        // The last of the deletions fails, after the others have run.
        conn.execute_batch("ALTER TABLE tafsir_translations RENAME TO hidden;")
            .unwrap();
        assert!(apply(&mut conn, prepared, ImportMode::Replace).is_err());
        assert_eq!(note_bodies(&conn, &index), ["sabır"]);
        assert_eq!(list_collections(&conn).unwrap().len(), 1);
    }
}
