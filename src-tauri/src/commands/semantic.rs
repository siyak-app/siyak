//! Semantic layer commands (spec §7.4, §7.6).

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::AppState;
use crate::commands::chat::write_search_query;
use crate::db::user;
use crate::embed::TextKind;
use crate::embed::service::SemanticStatus;
use crate::error::AppResult;
use crate::llm::ModelRole;
use crate::llm::chat::check_message;
use crate::packs::manifest::installed_packs;
use crate::related::{MAX_RELATED, RelatedVerse, verse_cards};

/// Whether semantic search and cross-references are available, and if not,
/// what the user can do about it (install a pack, store a key).
#[tauri::command]
#[specta::specta]
pub async fn get_semantic_status(state: State<'_, AppState>) -> AppResult<SemanticStatus> {
    let packs = installed_packs(&*state.content()?)?;
    let settings = user::get_settings(&*state.user()?)?;
    state.semantic().status(&packs, &settings)
}

/// The verses closest in meaning to a query, best first, each with the given
/// translation. Fails with `semantic_unavailable` unless the status is ready.
#[tauri::command]
#[specta::specta]
pub async fn search_meaning(
    state: State<'_, AppState>,
    query: String,
    translation_id: Option<String>,
    limit: u32,
) -> AppResult<Vec<RelatedVerse>> {
    nearest_verses(&state, query.trim(), translation_id.as_deref(), limit).await
}

/// The verses closest in meaning to `query`, best first; nothing for an empty query.
async fn nearest_verses(
    state: &AppState,
    query: &str,
    translation_id: Option<&str>,
    limit: u32,
) -> AppResult<Vec<RelatedVerse>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let packs = installed_packs(&*state.content()?)?;
    let settings = user::get_settings(&*state.user()?)?;
    let index = state
        .semantic()
        .index(&*state.content()?, &packs, &settings)?;
    // The first query loads the model, which takes a few seconds.
    let embedder = tokio::task::block_in_place(|| state.semantic().embedder(&packs, &settings))?;
    let vectors = embedder
        .embed(vec![query.to_owned()], TextKind::Query)
        .await?;
    let Some(vector) = vectors.first() else {
        return Ok(Vec::new());
    };
    let found = index.nearest(vector, limit.min(MAX_RELATED) as usize, &[]);
    verse_cards(
        &*state.content()?,
        found
            .into_iter()
            .map(|n| (n.verse_id, Vec::new()))
            .collect(),
        translation_id,
    )
}

/// What the Find screen found for a question.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct FoundVerses {
    /// The search sentence the chat model wrote from the question, if it did.
    pub search_query: Option<String>,
    /// The verses closest in meaning, best first.
    pub verses: Vec<RelatedVerse>,
    /// Whether the rewrite was stopped; nothing was searched then.
    pub cancelled: bool,
}

/// The verses closest in meaning to a question, with no answer written (the
/// Find screen). With `rewrite`, the chat model first turns the question into
/// a search sentence as in chat (spec §10.1), which needs the AI setup and is
/// counted as chat use; `request_id` names that request for `cancel_generation`.
#[tauri::command]
#[specta::specta]
pub async fn find_verses(
    state: State<'_, AppState>,
    request_id: String,
    query: String,
    rewrite: bool,
    translation_id: Option<String>,
    limit: u32,
) -> AppResult<FoundVerses> {
    let query = check_message(&query)?;
    let search_query = if rewrite {
        let settings = user::get_settings(&*state.user()?)?;
        let model = state.llm().model(&settings, ModelRole::Chat)?;
        let running = state.llm().start(&request_id);
        let Some(written) = write_search_query(&state, &model, &[], query, running.token()).await?
        else {
            return Ok(FoundVerses {
                search_query: None,
                verses: Vec::new(),
                cancelled: true,
            });
        };
        written
    } else {
        None
    };
    let verses = nearest_verses(
        &state,
        search_query.as_deref().unwrap_or(query),
        translation_id.as_deref(),
        limit,
    )
    .await?;
    Ok(FoundVerses {
        search_query,
        verses,
        cancelled: false,
    })
}

/// The verses closest in meaning to a verse, best first; the verse itself is
/// left out. Needs only the vector pack: verse vectors are compared, nothing
/// is embedded.
#[tauri::command]
#[specta::specta]
pub async fn related_by_meaning(
    state: State<'_, AppState>,
    verse_id: u32,
    translation_id: Option<String>,
    limit: u32,
) -> AppResult<Vec<RelatedVerse>> {
    let packs = installed_packs(&*state.content()?)?;
    let settings = user::get_settings(&*state.user()?)?;
    let content = state.content()?;
    let index = state.semantic().index(&content, &packs, &settings)?;
    let Some(vector) = index.vector(verse_id) else {
        return Ok(Vec::new());
    };
    let found = index.nearest(vector, limit.min(MAX_RELATED) as usize, &[verse_id]);
    verse_cards(
        &content,
        found
            .into_iter()
            .map(|n| (n.verse_id, Vec::new()))
            .collect(),
        translation_id.as_deref(),
    )
}

/// The verses that share the most rare roots and proper nouns with a verse,
/// best first, each with the shared terms; empty without a morphology pack.
#[tauri::command]
#[specta::specta]
pub async fn related_by_roots(
    state: State<'_, AppState>,
    verse_id: u32,
    translation_id: Option<String>,
    limit: u32,
) -> AppResult<Vec<RelatedVerse>> {
    let content = state.content()?;
    let Some(index) = state.related().index(&content)? else {
        return Ok(Vec::new());
    };
    let found = index.related(verse_id, limit.min(MAX_RELATED) as usize);
    verse_cards(
        &content,
        found.into_iter().map(|m| (m.verse_id, m.shared)).collect(),
        translation_id.as_deref(),
    )
}
