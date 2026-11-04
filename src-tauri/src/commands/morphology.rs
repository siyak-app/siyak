//! Morphology commands (spec §5.3, §7.3, §7.5).

use tauri::State;

use crate::AppState;
use crate::error::AppResult;
use crate::morphology::{
    self, Concordance, ConcordanceTarget, MorphologyPack, RootSearch, VerseWord, WordDetail,
};

/// The installed morphology pack, if any.
#[tauri::command]
#[specta::specta]
pub async fn get_morphology_pack(state: State<'_, AppState>) -> AppResult<Option<MorphologyPack>> {
    morphology::installed_pack(&*state.content()?)
}

/// The words of a verse, in order; empty without a morphology pack.
#[tauri::command]
#[specta::specta]
pub async fn get_verse_words(
    state: State<'_, AppState>,
    verse_id: u32,
) -> AppResult<Vec<VerseWord>> {
    morphology::get_verse_words(&*state.content()?, verse_id)
}

/// A word with its full analysis.
#[tauri::command]
#[specta::specta]
pub async fn get_word(state: State<'_, AppState>, word_id: u32) -> AppResult<WordDetail> {
    morphology::get_word(&*state.content()?, word_id)
}

/// Every occurrence of a root or lemma: the summary and one page of verses,
/// each with the given translation if it has one.
#[tauri::command]
#[specta::specta]
pub async fn concordance(
    state: State<'_, AppState>,
    target: ConcordanceTarget,
    translation_id: Option<String>,
    limit: u32,
    offset: u32,
) -> AppResult<Concordance> {
    morphology::concordance(
        &*state.content()?,
        target,
        translation_id.as_deref(),
        limit,
        offset,
    )
}

/// Finds roots and lemmas by an Arabic query or a Latin-script root.
#[tauri::command]
#[specta::specta]
pub async fn search_roots(state: State<'_, AppState>, query: String) -> AppResult<RootSearch> {
    morphology::search_roots(&*state.content()?, &query)
}
