//! Collection commands (spec §11.1, §12).

use tauri::State;

use crate::AppState;
use crate::collections::{self, Collection, CollectionInput, CollectionSummary, ItemRef};
use crate::db::content::VerseIndex;
use crate::error::AppResult;

/// Every collection, the most recently changed first.
#[tauri::command]
#[specta::specta]
pub async fn list_collections(state: State<'_, AppState>) -> AppResult<Vec<CollectionSummary>> {
    collections::list_collections(&*state.user()?)
}

/// A collection with the content of its items, verses with the given translations.
#[tauri::command]
#[specta::specta]
pub async fn get_collection(
    state: State<'_, AppState>,
    id: u32,
    translation_ids: Vec<String>,
) -> AppResult<Collection> {
    read_collection(&state, id, &translation_ids)
}

/// Reads a collection from both databases, holding one lock at a time.
pub fn read_collection(
    state: &AppState,
    id: u32,
    translation_ids: &[String],
) -> AppResult<Collection> {
    let index = VerseIndex::load(&*state.content()?)?;
    let stored = collections::stored_collection(&*state.user()?, &index, id)?;
    collections::resolve(&*state.content()?, &index, stored, translation_ids)
}

/// Creates or changes a collection and returns it.
#[tauri::command]
#[specta::specta]
pub async fn upsert_collection(
    state: State<'_, AppState>,
    collection: CollectionInput,
) -> AppResult<CollectionSummary> {
    collections::upsert_collection(&*state.user()?, &collection)
}

/// Deletes a collection and its items; the notes themselves stay.
#[tauri::command]
#[specta::specta]
pub async fn delete_collection(state: State<'_, AppState>, id: u32) -> AppResult<()> {
    collections::delete_collection(&*state.user()?, id)
}

/// Adds an item at the end of a collection and returns its id.
#[tauri::command]
#[specta::specta]
pub async fn add_collection_item(
    state: State<'_, AppState>,
    collection_id: u32,
    item: ItemRef,
    annotation: Option<String>,
) -> AppResult<u32> {
    let index = VerseIndex::load(&*state.content()?)?;
    collections::add_item(
        &*state.user()?,
        &index,
        collection_id,
        &item,
        annotation.as_deref(),
    )
}

/// Changes an item's annotation.
#[tauri::command]
#[specta::specta]
pub async fn update_collection_item(
    state: State<'_, AppState>,
    id: u32,
    annotation: Option<String>,
) -> AppResult<()> {
    collections::update_item(&*state.user()?, id, annotation.as_deref())
}

/// Moves an item to `to_index` in its collection (0 is first).
#[tauri::command]
#[specta::specta]
pub async fn move_collection_item(
    state: State<'_, AppState>,
    id: u32,
    to_index: u32,
) -> AppResult<()> {
    collections::move_item(&mut *state.user()?, id, to_index)
}

/// Removes an item from its collection.
#[tauri::command]
#[specta::specta]
pub async fn remove_collection_item(state: State<'_, AppState>, id: u32) -> AppResult<()> {
    collections::remove_item(&*state.user()?, id)
}
