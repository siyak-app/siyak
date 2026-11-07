//! AI commands (spec §8, §9): status, model lists, spending, tafsir summaries
//! and translations, and stopping a streamed answer.

use serde::Serialize;
use specta::Type;
use tauri::State;
use tauri::ipc::Channel;

use crate::AppState;
use crate::db::user;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::llm::cost::{
    CostEstimate, Feature, UsagePeriod, UsageSummary, estimate_tokens, record_usage, usage_summary,
};
use crate::llm::translate::{self, Job, TafsirTranslation, TranslationMode};
use crate::llm::{AiStatus, Finish, ModelInfo, ModelRole, Usage};
use crate::secrets::ProviderId;

/// Whether AI features can run, and if not, what the user can do about it.
#[tauri::command]
#[specta::specta]
pub async fn get_ai_status(state: State<'_, AppState>) -> AppResult<AiStatus> {
    let settings = user::get_settings(&*state.user()?)?;
    state.llm().status(&settings)
}

/// The chat models a provider offers, asked from the provider with its saved
/// key (or the entered server address).
#[tauri::command]
#[specta::specta]
pub async fn list_models(
    state: State<'_, AppState>,
    provider: ProviderId,
) -> AppResult<Vec<ModelInfo>> {
    let settings = user::get_settings(&*state.user()?)?;
    let provider = state.llm().provider(provider, &settings)?;
    provider.list_models().await
}

/// Stops a streamed answer (chat, tafsir summary or translation); nothing
/// happens if it already ended.
#[tauri::command]
#[specta::specta]
pub async fn cancel_generation(state: State<'_, AppState>, request_id: String) -> AppResult<()> {
    state.llm().cancel(&request_id);
    Ok(())
}

/// Spending in a period, per feature and model (Settings → AI → Spending,
/// spec §9.4).
#[tauri::command]
#[specta::specta]
pub async fn get_usage_summary(
    state: State<'_, AppState>,
    period: UsagePeriod,
) -> AppResult<UsageSummary> {
    let conn = state.user()?;
    let settings = user::get_settings(&conn)?;
    usage_summary(&conn, period, &settings)
}

/// The saved summaries and translations of a tafsir passage in a language;
/// those made from an older pack version are left out.
#[tauri::command]
#[specta::specta]
pub async fn list_tafsir_translations(
    state: State<'_, AppState>,
    pack_id: String,
    tafsir_id: u32,
    target_lang: String,
) -> AppResult<Vec<TafsirTranslation>> {
    let version = translate::pack_version(&*state.content()?, &pack_id)?;
    translate::cached(&*state.user()?, &pack_id, tafsir_id, &target_lang, &version)
}

/// What a summary (`paragraph_idx` none) or a paragraph translation is
/// expected to cost with the tafsir model (spec §9.2).
#[tauri::command]
#[specta::specta]
pub async fn estimate_tafsir_translation(
    state: State<'_, AppState>,
    pack_id: String,
    tafsir_id: u32,
    paragraph_idx: Option<u32>,
    mode: TranslationMode,
    target_lang: String,
) -> AppResult<CostEstimate> {
    let job = Job::prepare(
        &*state.content()?,
        &pack_id,
        tafsir_id,
        paragraph_idx,
        mode,
        &target_lang,
    )?;
    let settings = user::get_settings(&*state.user()?)?;
    let model = state.llm().model(&settings, ModelRole::Translation)?;
    let pricing = state.pricing().await;
    CostEstimate::new(
        &pricing,
        model.provider.id(),
        &model.model,
        job.estimate(),
        &*state.user()?,
        &settings,
    )
}

/// The end of a tafsir summary or translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TranslateResult {
    /// The saved text; `None` if the user stopped it.
    pub translation: Option<TafsirTranslation>,
    /// Why it ended: a truncated text is saved as it is.
    pub finish: Finish,
}

/// Summarizes a passage or translates a paragraph with the tafsir model,
/// streaming the text into `on_text`, then saves it and logs its cost.
/// `request_id` names it for [`cancel_generation`].
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::too_many_arguments,
    reason = "one IPC call carries the whole request"
)]
pub async fn translate_tafsir(
    state: State<'_, AppState>,
    request_id: String,
    pack_id: String,
    tafsir_id: u32,
    paragraph_idx: Option<u32>,
    mode: TranslationMode,
    target_lang: String,
    on_text: Channel<String>,
) -> AppResult<TranslateResult> {
    let job = Job::prepare(
        &*state.content()?,
        &pack_id,
        tafsir_id,
        paragraph_idx,
        mode,
        &target_lang,
    )?;
    let settings = user::get_settings(&*state.user()?)?;
    let model = state.llm().model(&settings, ModelRole::Translation)?;
    let provider = model.provider.id();
    let running = state.llm().start(&request_id);
    let send = |piece: &str| {
        // A closed window is not an error of the answer.
        let _ = on_text.send(piece.to_owned());
    };
    let (text, completion) = job
        .run(&*model.provider, &model.model, &send, running.token())
        .await?;
    drop(running);
    let usage = completion.usage.unwrap_or_else(|| Usage {
        input_tokens: job.estimate().input_tokens,
        output_tokens: estimate_tokens(&text),
    });
    let pricing = state.pricing().await;
    let cost = pricing.cost(provider, &model.model, usage);
    let user = state.user()?;
    record_usage(
        &user,
        Feature::Translate,
        provider,
        &model.model,
        usage,
        cost,
    )?;
    let translation = match completion.finish {
        Finish::Cancelled => None,
        _ if text.is_empty() => {
            return Err(AppError::new(
                AppErrorCode::ProviderFailed,
                "the model answered with no text",
            ));
        }
        _ => Some(job.save(&user, &text, provider, &model.model)?),
    };
    Ok(TranslateResult {
        translation,
        finish: completion.finish,
    })
}
