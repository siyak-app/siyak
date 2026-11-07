//! Chat commands (spec §10): send a message and stream the answer, estimate
//! its cost, and manage sessions.

use std::sync::Mutex;

use serde::Serialize;
use specta::Type;
use tauri::State;
use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::db::user;
use crate::embed::TextKind;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::llm::chat::{
    self, ANSWER_ESTIMATE_TOKENS, ANSWER_MAX_TOKENS, CONTEXT_ESTIMATE_TOKENS, ChatMessage,
    ChatSession, ChatSessionSummary, Context, QUERY_ESTIMATE_TOKENS, TOP_K,
};
use crate::llm::cost::{CostEstimate, Feature, estimate_prompt, estimate_tokens, record_usage};
use crate::llm::{AiConfig, Finish, LlmRequest, Message, Model, ModelRole, Role, Usage};
use crate::net::Purpose;
use crate::packs::manifest::installed_packs;

/// The end of a chat answer.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ChatReply {
    /// The session the messages were saved in; a new one if none was given.
    /// `None` only if the answer was stopped before any text arrived.
    pub session_id: Option<u32>,
    /// The saved question.
    pub question: Option<ChatMessage>,
    /// The saved answer, with its citations and sources.
    pub answer: Option<ChatMessage>,
    /// Why the answer ended.
    pub finish: Finish,
}

/// The language an answer is written in, from the interface language.
fn answer_language(language: &str) -> &'static str {
    if language == "tr" {
        "Turkish"
    } else {
        "English"
    }
}

/// Has the chat model rewrite `message`, with the end of `history`, into one
/// descriptive search query (spec §10.1) and logs its cost as chat use.
/// `Ok(None)` if the request was cancelled; `Ok(Some(None))` if the reply
/// held no usable query, in which case the message itself is searched.
pub(crate) async fn write_search_query(
    state: &AppState,
    model: &Model,
    history: &[Message],
    message: &str,
    token: &CancellationToken,
) -> AppResult<Option<Option<String>>> {
    let request = chat::search_query_request(&model.model, history, message);
    let reply = Mutex::new(String::new());
    let collect = |piece: &str| {
        reply
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_str(piece);
    };
    let completion = model.provider.stream(&request, &collect, token).await?;
    let reply = reply
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if completion.finish == Finish::Cancelled {
        return Ok(None);
    }
    let usage = completion.usage.unwrap_or_else(|| Usage {
        input_tokens: estimate_prompt(&request.system, &[&request.messages[0].content]),
        output_tokens: estimate_tokens(&reply),
    });
    let provider = model.provider.id();
    let cost = state.pricing().await.cost(provider, &model.model, usage);
    record_usage(
        &*state.user()?,
        Feature::Chat,
        provider,
        &model.model,
        usage,
        cost,
    )?;
    Ok(Some(chat::parse_search_query(&reply)))
}

/// Answers `message` from the retrieved verses and tafsir, streaming the text
/// into `on_text`, and saves both in `session_id` (a new session if `None`).
/// Needs a ready semantic pack and AI setup; `request_id` names the answer for
/// `cancel_generation`.
#[tauri::command]
#[specta::specta]
pub async fn chat_send(
    state: State<'_, AppState>,
    request_id: String,
    session_id: Option<u32>,
    message: String,
    language: String,
    on_text: Channel<String>,
) -> AppResult<ChatReply> {
    let message = chat::check_message(&message)?.to_owned();
    let settings = user::get_settings(&*state.user()?)?;
    let model = state.llm().model(&settings, ModelRole::Chat)?;
    let arabic_sources = AiConfig::from_settings(&settings).arabic_sources(&model.model);
    let mut messages = match session_id {
        Some(id) => {
            let user = state.user()?;
            chat::require_session(&user, id)?;
            chat::history(&user, id)?
        }
        None => Vec::new(),
    };

    // Retrieval (spec §10.1-3).
    let packs = installed_packs(&*state.content()?)?;
    let index = state
        .semantic()
        .index(&*state.content()?, &packs, &settings)?;
    let embedder = tokio::task::block_in_place(|| state.semantic().embedder(&packs, &settings))?;
    let running = state.llm().start(&request_id);
    let provider = model.provider.id();
    let Some(search_query) =
        write_search_query(&state, &model, &messages, &message, running.token()).await?
    else {
        return Ok(ChatReply {
            session_id,
            question: None,
            answer: None,
            finish: Finish::Cancelled,
        });
    };
    let question = embedder
        .embed(
            vec![search_query.clone().unwrap_or_else(|| message.clone())],
            TextKind::Query,
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::new(AppErrorCode::SemanticUnavailable, "no query vector"))?;
    let neighbors = index.nearest(&question, TOP_K, &[]);
    let (verses, candidates, notices) = {
        let content = state.content()?;
        let verses =
            chat::context_verses(&content, &neighbors, &chat::reader_translations(&settings))?;
        let (candidates, notices) = chat::tafsir_candidates(
            &content,
            &verses,
            &chat::chat_tafsirs(&settings),
            arabic_sources,
        )?;
        (verses, candidates, notices)
    };
    let tafsirs = chat::fit_tafsirs(candidates);
    let context = Context {
        verses,
        tafsirs,
        notices,
        search_query,
    };

    messages.push(Message {
        role: Role::User,
        content: message.clone(),
    });
    let request = LlmRequest {
        model: model.model.clone(),
        system: context.system_prompt(answer_language(&language)),
        messages,
        max_tokens: ANSWER_MAX_TOKENS,
        purpose: Purpose::Chat,
    };
    let text = Mutex::new(String::new());
    let collect = |piece: &str| {
        text.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_str(piece);
        // A closed window is not an error of the answer.
        let _ = on_text.send(piece.to_owned());
    };
    let completion = model
        .provider
        .stream(&request, &collect, running.token())
        .await?;
    drop(running);
    let text = text
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .trim()
        .to_owned();

    let usage = completion.usage.unwrap_or_else(|| Usage {
        input_tokens: estimate_prompt(
            &request.system,
            &request
                .messages
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>(),
        ),
        output_tokens: estimate_tokens(&text),
    });
    let pricing = state.pricing().await;
    let cost = pricing.cost(provider, &model.model, usage);
    let user = state.user()?;
    record_usage(&user, Feature::Chat, provider, &model.model, usage, cost)?;

    if text.is_empty() {
        if completion.finish == Finish::Cancelled {
            return Ok(ChatReply {
                session_id,
                question: None,
                answer: None,
                finish: completion.finish,
            });
        }
        return Err(AppError::new(
            AppErrorCode::ProviderFailed,
            "the model answered with no text",
        ));
    }
    let session_id = match session_id {
        Some(id) => id,
        None => chat::create_session(&user, &message, provider.as_str(), &model.model)?,
    };
    let cited = chat::citations(&*state.content()?, &text)?;
    let question = chat::add_message(&user, session_id, Role::User, &message, &[], None)?;
    let answer = chat::add_message(
        &user,
        session_id,
        Role::Assistant,
        &text,
        &cited,
        Some(&context.sources()),
    )?;
    Ok(ChatReply {
        session_id: Some(session_id),
        question: Some(question),
        answer: Some(answer),
        finish: completion.finish,
    })
}

/// What sending `message` in `session_id` is expected to cost: the history,
/// the message and a typical retrieved context (spec §10.3).
#[tauri::command]
#[specta::specta]
pub async fn estimate_chat(
    state: State<'_, AppState>,
    session_id: Option<u32>,
    message: String,
) -> AppResult<CostEstimate> {
    let settings = user::get_settings(&*state.user()?)?;
    let model = state.llm().model(&settings, ModelRole::Chat)?;
    let history = match session_id {
        Some(id) => chat::history(&*state.user()?, id)?,
        None => Vec::new(),
    };
    let mut texts: Vec<&str> = history.iter().map(|m| m.content.as_str()).collect();
    texts.push(&message);
    // The answer, and the search query written from the message first.
    let (query_input, query_output) = QUERY_ESTIMATE_TOKENS;
    let usage = Usage {
        input_tokens: estimate_prompt("", &texts)
            .saturating_add(CONTEXT_ESTIMATE_TOKENS)
            .saturating_add(estimate_tokens(&message))
            .saturating_add(query_input),
        output_tokens: ANSWER_ESTIMATE_TOKENS.saturating_add(query_output),
    };
    let pricing = state.pricing().await;
    CostEstimate::new(
        &pricing,
        model.provider.id(),
        &model.model,
        usage,
        &*state.user()?,
        &settings,
    )
}

/// Every chat session, the most recently used first.
#[tauri::command]
#[specta::specta]
pub async fn list_chat_sessions(state: State<'_, AppState>) -> AppResult<Vec<ChatSessionSummary>> {
    chat::list_sessions(&*state.user()?)
}

/// A chat session with its messages.
#[tauri::command]
#[specta::specta]
pub async fn get_chat_session(state: State<'_, AppState>, id: u32) -> AppResult<ChatSession> {
    chat::get_session(&*state.user()?, &*state.content()?, id)
}

/// Deletes a chat session and its messages.
#[tauri::command]
#[specta::specta]
pub async fn delete_chat_session(state: State<'_, AppState>, id: u32) -> AppResult<()> {
    chat::delete_session(&*state.user()?, id)
}

/// Renames a chat session.
#[tauri::command]
#[specta::specta]
pub async fn rename_chat_session(
    state: State<'_, AppState>,
    id: u32,
    title: String,
) -> AppResult<()> {
    chat::rename_session(&*state.user()?, id, &title)
}
