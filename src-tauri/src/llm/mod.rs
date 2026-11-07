//! AI provider layer: the [`LlmProvider`] trait, shared request/response types
//! and model roles (`chat_model`, `translation_model`) (spec §8).
//! AI is an optional layer; core features never require an API key.
//!
//! Providers speak their APIs directly over [`crate::net`] (ADR 0013): every
//! host is on the allow-list or is the user's own OpenAI-compatible server,
//! and every request is in the connection log. Answers are streamed as
//! server-sent events ([`sse`]) and passed on piece by piece.
//!
//! - [`anthropic`], [`openai`] (also OpenAI-compatible servers), [`gemini`]:
//!   the providers.
//! - [`cost`]: token estimates, prices and the spending log.
//! - [`translate`]: tafsir summaries and translations (spec §9).
//! - [`chat`]: the RAG chat (spec §10).

pub mod anthropic;
pub mod chat;
pub mod cost;
#[cfg(test)]
pub mod fake;
pub mod gemini;
#[cfg(test)]
mod http_tests;
pub mod openai;
pub mod sse;
pub mod translate;

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use tokio_util::sync::CancellationToken;

use crate::db::user::Settings;
use crate::embed::service::BASE_URL_SETTING;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, Purpose, Response, user_server_url};
use crate::secrets::{KeyStore, ProviderId};
use sse::{SseEvent, SseParser};

/// Setting: the provider of chat and translation (JSON string, a [`ProviderId`]).
pub const PROVIDER_SETTING: &str = "ai.provider";
/// Setting: the model answering chat messages (JSON string).
pub const CHAT_MODEL_SETTING: &str = "ai.chat_model";
/// Setting: the model summarizing and translating tafsir (JSON string);
/// unset means the chat model.
pub const TRANSLATION_MODEL_SETTING: &str = "ai.translation_model";
/// Setting: `arabic_sources` overrides by `provider/model` (JSON object of
/// booleans, spec §8).
pub const ARABIC_SOURCES_SETTING: &str = "ai.arabic_sources";

/// Largest error or model list body read from a provider.
const MAX_BODY: usize = 4 * 1024 * 1024;

/// Wait before asking an overloaded model (HTTP 503) once more.
const OVERLOADED_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// Longest provider error message kept in a diagnostic.
const MAX_ERROR_MESSAGE: usize = 300;

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The user.
    User,
    /// The model.
    Assistant,
}

impl Role {
    /// The value in `chat_messages.role` and in OpenAI/Anthropic requests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// One turn of a conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Who wrote it.
    pub role: Role,
    /// Its text.
    pub content: String,
}

/// A request to a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    /// The provider's model id.
    pub model: String,
    /// The system prompt.
    pub system: String,
    /// The conversation, oldest first; the last message is the user's.
    pub messages: Vec<Message>,
    /// Most tokens the answer may have.
    pub max_tokens: u32,
    /// Why it is sent, for the connection log: chat or translate.
    pub purpose: Purpose,
}

/// Tokens a request used, as the provider counted them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Type)]
pub struct Usage {
    /// Prompt tokens.
    pub input_tokens: u32,
    /// Answer tokens, including any the model spent thinking.
    pub output_tokens: u32,
}

/// Why an answer ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    /// The model finished.
    Complete,
    /// The answer reached `max_tokens`.
    Truncated,
    /// The user stopped it.
    Cancelled,
}

/// The end of a streamed answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completion {
    /// Tokens used, if the provider said (not when cancelled early).
    pub usage: Option<Usage>,
    /// Why the answer ended.
    pub finish: Finish,
}

/// A model a provider offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ModelInfo {
    /// The id sent in requests.
    pub id: String,
    /// A display name, if the provider gives one.
    pub name: Option<String>,
}

/// Parts of model ids that are not text-generation models: embeddings,
/// speech, images, video, music, live audio, moderation, and models served
/// only by other APIs (search, computer use, Codex).
const NOT_CHAT: &[&str] = &[
    "embed",
    "tts",
    "whisper",
    "transcribe",
    "dall-e",
    "image",
    "imagen",
    "audio",
    "realtime",
    "live",
    "moderation",
    "davinci",
    "babbage",
    "sora",
    "veo",
    "lyria",
    "search",
    "computer-use",
    "robotics",
    "aqa",
    "deep-research",
    "codex",
];

/// Parts of model ids that are surely not chat models on any server; the
/// only filter applied to an OpenAI-compatible server's list.
const NOT_CHAT_ANYWHERE: &[&str] = &["embed", "tts", "whisper"];

/// Whether a provider's model `id` writes text, so it belongs in the chat
/// and translation model lists. `compatible` servers name models freely and
/// are filtered only by the clearest markers.
pub(crate) fn is_chat_model(id: &str, compatible: bool) -> bool {
    let id = id.to_ascii_lowercase();
    let markers = if compatible {
        NOT_CHAT_ANYWHERE
    } else {
        NOT_CHAT
    };
    !markers.iter().any(|part| id.contains(part))
}

/// A boxed future: async trait methods cannot be called through `dyn`.
pub type LlmFuture<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

/// Receives the text of an answer as it arrives.
pub type TextSink<'a> = &'a (dyn Fn(&str) + Send + Sync);

/// A chat model provider (spec §8).
pub trait LlmProvider: Send + Sync {
    /// Which provider this is.
    fn id(&self) -> ProviderId;

    /// The models usable for chat, sorted by id.
    fn list_models(&self) -> LlmFuture<'_, Vec<ModelInfo>>;

    /// Streams the answer to `request` into `on_text` until it ends or
    /// `cancel` fires.
    fn stream<'a>(
        &'a self,
        request: &'a LlmRequest,
        on_text: TextSink<'a>,
        cancel: &'a CancellationToken,
    ) -> LlmFuture<'a, Completion>;
}

/// Builds providers.
pub trait ProviderFactory: Send + Sync {
    /// The provider `id`; `server` is the checked address of the
    /// OpenAI-compatible server, which only that provider needs.
    fn create(&self, id: ProviderId, server: Option<Url>) -> AppResult<Arc<dyn LlmProvider>>;
}

/// The API addresses of the hosted providers; tests point them at a local
/// server.
#[derive(Debug, Clone)]
pub struct Endpoints {
    /// Anthropic's base URL, without a trailing slash.
    pub anthropic: String,
    /// OpenAI's base URL.
    pub openai: String,
    /// Gemini's base URL.
    pub gemini: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            anthropic: "https://api.anthropic.com/v1".into(),
            openai: "https://api.openai.com/v1".into(),
            gemini: "https://generativelanguage.googleapis.com/v1beta".into(),
        }
    }
}

/// The real providers, over HTTP.
pub struct HttpProviders {
    net: Net,
    keys: Arc<dyn KeyStore>,
    endpoints: Endpoints,
}

impl HttpProviders {
    /// Providers sending requests with `net` and keys from `keys`.
    pub fn new(net: Net, keys: Arc<dyn KeyStore>, endpoints: Endpoints) -> Self {
        Self {
            net,
            keys,
            endpoints,
        }
    }
}

impl ProviderFactory for HttpProviders {
    fn create(&self, id: ProviderId, server: Option<Url>) -> AppResult<Arc<dyn LlmProvider>> {
        let client = Client {
            net: self.net.clone(),
            keys: Arc::clone(&self.keys),
            provider: id,
            user_server: None,
        };
        Ok(match id {
            ProviderId::Anthropic => Arc::new(anthropic::Anthropic::new(
                client,
                parse_url(&self.endpoints.anthropic)?,
            )),
            ProviderId::OpenAi => Arc::new(openai::OpenAi::new(
                client,
                parse_url(&self.endpoints.openai)?,
            )),
            ProviderId::Gemini => Arc::new(gemini::Gemini::new(
                client,
                parse_url(&self.endpoints.gemini)?,
            )),
            ProviderId::OpenAiCompatible => {
                let server = server.ok_or_else(|| unavailable("no server address"))?;
                Arc::new(openai::OpenAi::new(
                    Client {
                        user_server: Some(server.clone()),
                        ..client
                    },
                    server,
                ))
            }
            ProviderId::Voyage => return Err(unavailable("Voyage offers no chat models")),
        })
    }
}

/// What every HTTP provider shares: the client, the key and the user's
/// server, which is not on the allow-list.
pub(crate) struct Client {
    net: Net,
    keys: Arc<dyn KeyStore>,
    provider: ProviderId,
    user_server: Option<Url>,
}

impl Client {
    /// The saved key; an error if the provider needs one and none is saved.
    /// Read for every request, so a new key works at once.
    fn key(&self) -> AppResult<Option<String>> {
        let key = self.keys.key(self.provider)?;
        if key.is_none() && self.provider.needs_key() {
            return Err(unavailable(&format!(
                "no {} key is saved",
                self.provider.as_str()
            )));
        }
        Ok(key)
    }

    /// Sends a streamed request; a failure status becomes the provider error.
    /// `None` if `cancel` fires before the answer starts: a thinking model may
    /// send nothing for minutes.
    async fn post_stream(
        &self,
        url: &Url,
        headers: &[(&str, &str)],
        body: &Value,
        purpose: Purpose,
        cancel: &CancellationToken,
    ) -> AppResult<Option<Response>> {
        let mut retried = false;
        loop {
            let sent = self
                .net
                .post_stream(url, self.user_server.as_ref(), purpose, headers, body);
            let mut response = tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(None),
                response = sent => response?,
            };
            let status = response.status();
            if status.is_success() {
                return Ok(Some(response));
            }
            // A busy model often answers a moment later; nothing was shown yet.
            if status == StatusCode::SERVICE_UNAVAILABLE && !retried {
                retried = true;
                drop(response);
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => return Ok(None),
                    () = tokio::time::sleep(OVERLOADED_RETRY_DELAY) => continue,
                }
            }
            let body = response.bytes_limited(MAX_BODY).await.unwrap_or_default();
            return Err(provider_error(response.host(), status, &body));
        }
    }

    /// Sends a GET request and parses the JSON answer.
    async fn get_json(&self, url: &Url, headers: &[(&str, &str)]) -> AppResult<Value> {
        let (status, body) = self
            .net
            .get_api(
                url,
                self.user_server.as_ref(),
                Purpose::Chat,
                headers,
                MAX_BODY,
            )
            .await?;
        let host = url.host_str().unwrap_or_default();
        if !status.is_success() {
            return Err(provider_error(host, status, &body));
        }
        serde_json::from_slice(&body).map_err(|_| failed(format!("{host}: unexpected response")))
    }
}

/// What to do after a streamed event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Stop,
}

/// Reads server-sent events from `response` into `on_event` until it says
/// stop, the body ends or `cancel` fires. Returns whether it was cancelled.
async fn read_events(
    mut response: Response,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(SseEvent) -> AppResult<Flow> + Send,
) -> AppResult<bool> {
    let mut parser = SseParser::default();
    loop {
        let chunk = tokio::select! {
            biased;
            () = cancel.cancelled() => return Ok(true),
            chunk = response.chunk() => chunk?,
        };
        let Some(chunk) = chunk else { break };
        for event in parser.push(&chunk)? {
            if on_event(event)? == Flow::Stop {
                return Ok(false);
            }
        }
    }
    if let Some(event) = parser.finish() {
        on_event(event)?;
    }
    Ok(false)
}

/// Parses the JSON data of an event.
fn event_json(host: &str, event: &SseEvent) -> AppResult<Value> {
    serde_json::from_str(&event.data)
        .map_err(|_| failed(format!("{host}: a streamed event is not JSON")))
}

/// The error a provider reported inside a stream (`{"error": {...}}`).
fn stream_error(host: &str, value: &Value) -> Option<AppError> {
    let error = value.get("error")?;
    let kind = error["type"].as_str().or(error["status"].as_str());
    let code = match kind {
        Some("rate_limit_error" | "RESOURCE_EXHAUSTED") => AppErrorCode::ProviderRateLimited,
        Some("authentication_error" | "permission_error" | "PERMISSION_DENIED") => {
            AppErrorCode::ProviderKeyRejected
        }
        _ => AppErrorCode::ProviderFailed,
    };
    let message = error["message"]
        .as_str()
        .or(error.as_str())
        .unwrap_or_default();
    Some(AppError::new(
        code,
        format!("{host}: {}", truncate(message, MAX_ERROR_MESSAGE)),
    ))
}

/// The error for a failed request, with the provider's message if it gave one.
fn provider_error(host: &str, status: StatusCode, body: &[u8]) -> AppError {
    let code = match status.as_u16() {
        401 | 403 => AppErrorCode::ProviderKeyRejected,
        429 => AppErrorCode::ProviderRateLimited,
        503 => AppErrorCode::ProviderOverloaded,
        _ => AppErrorCode::ProviderFailed,
    };
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            let error = &value["error"];
            error["message"]
                .as_str()
                .or(error.as_str())
                .or(value["detail"].as_str())
                .map(str::to_owned)
        })
        .map(|m| truncate(&m, MAX_ERROR_MESSAGE))
        .unwrap_or_default();
    AppError::new(
        code,
        format!("{host}: {status} {message}").trim_end().to_owned(),
    )
}

/// A token count in a provider's JSON; 0 if missing.
fn count(value: &Value) -> u32 {
    value
        .as_u64()
        .map_or(0, |n| u32::try_from(n).unwrap_or(u32::MAX))
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn parse_url(url: &str) -> AppResult<Url> {
    Url::parse(url).map_err(|err| failed(format!("{url}: {err}")))
}

/// Joins `path` to a base URL without a trailing slash.
fn endpoint(base: &Url, path: &str) -> AppResult<Url> {
    parse_url(&format!("{}/{path}", base.as_str().trim_end_matches('/')))
}

fn failed(message: String) -> AppError {
    AppError::new(AppErrorCode::ProviderFailed, message)
}

/// The error when AI features are not set up.
pub fn unavailable(message: &str) -> AppError {
    AppError::new(AppErrorCode::AiUnavailable, message)
}

/// The AI settings (spec §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiConfig {
    /// The chosen provider, if any.
    pub provider: Option<ProviderId>,
    /// The chat model.
    pub chat_model: Option<String>,
    /// The tafsir model; falls back to the chat model.
    pub translation_model: Option<String>,
    /// The OpenAI-compatible server's address, shared with semantic search.
    pub base_url: Option<String>,
    /// `arabic_sources` overrides by `provider/model`.
    pub arabic_sources: BTreeMap<String, bool>,
}

impl AiConfig {
    /// Reads the AI settings; invalid values count as unset.
    pub fn from_settings(settings: &Settings) -> Self {
        let provider = string_setting(settings, PROVIDER_SETTING)
            .and_then(|p| ProviderId::parse(&p))
            .filter(|p| *p != ProviderId::Voyage);
        Self {
            provider,
            chat_model: string_setting(settings, CHAT_MODEL_SETTING),
            translation_model: string_setting(settings, TRANSLATION_MODEL_SETTING),
            base_url: string_setting(settings, BASE_URL_SETTING),
            arabic_sources: settings
                .get(ARABIC_SOURCES_SETTING)
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default(),
        }
    }

    /// The model of `role`, if one is chosen.
    pub fn model(&self, role: ModelRole) -> Option<&str> {
        match role {
            ModelRole::Chat => self.chat_model.as_deref(),
            ModelRole::Translation => self
                .translation_model
                .as_deref()
                .or(self.chat_model.as_deref()),
        }
    }

    /// Whether `model` of the chosen provider may be given Arabic sources:
    /// the user's override, else on for the hosted providers and off for an
    /// OpenAI-compatible server, which usually runs a small local model.
    pub fn arabic_sources(&self, model: &str) -> bool {
        let Some(provider) = self.provider else {
            return false;
        };
        self.arabic_sources
            .get(&model_key(provider, model))
            .copied()
            .unwrap_or(provider != ProviderId::OpenAiCompatible)
    }
}

/// The key of a model in `pricing.json` and in `ai.arabic_sources`:
/// `provider/model`.
pub fn model_key(provider: ProviderId, model: &str) -> String {
    format!("{}/{model}", provider.as_str())
}

/// What a model is used for (spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Chat answers.
    Chat,
    /// Tafsir summaries and translations.
    Translation,
}

/// Whether AI features can run, and if not, what the user can do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AiStatus {
    /// No provider is chosen.
    NoProvider,
    /// The provider's key is not saved.
    KeyMissing {
        /// The provider.
        provider: ProviderId,
    },
    /// The OpenAI-compatible server's address is not entered.
    BaseUrlMissing,
    /// The entered address is not usable (spec §14.1).
    BaseUrlInvalid,
    /// No chat model is chosen.
    NoModel {
        /// The provider.
        provider: ProviderId,
    },
    /// Ready.
    Ready {
        /// The provider.
        provider: ProviderId,
        /// The chat model.
        chat_model: String,
        /// The tafsir model.
        translation_model: String,
    },
}

/// A provider and model ready to answer.
pub struct Model {
    /// The provider.
    pub provider: Arc<dyn LlmProvider>,
    /// The model id.
    pub model: String,
}

/// The AI service in the app state: builds providers from the settings and
/// tracks running answers so they can be cancelled.
pub struct Llm {
    keys: Arc<dyn KeyStore>,
    factory: Option<Arc<dyn ProviderFactory>>,
    running: Mutex<HashMap<String, CancellationToken>>,
}

impl Llm {
    /// The service; without a factory (no network client) nothing runs.
    pub fn new(keys: Arc<dyn KeyStore>, factory: Option<Arc<dyn ProviderFactory>>) -> Self {
        Self {
            keys,
            factory,
            running: Mutex::default(),
        }
    }

    /// Whether AI features can run with these settings.
    pub fn status(&self, settings: &Settings) -> AppResult<AiStatus> {
        let config = AiConfig::from_settings(settings);
        let Some(provider) = config.provider else {
            return Ok(AiStatus::NoProvider);
        };
        if provider.needs_key() && !self.keys.has_key(provider)? {
            return Ok(AiStatus::KeyMissing { provider });
        }
        if provider == ProviderId::OpenAiCompatible {
            let Some(address) = &config.base_url else {
                return Ok(AiStatus::BaseUrlMissing);
            };
            if user_server_url(address).is_err() {
                return Ok(AiStatus::BaseUrlInvalid);
            }
        }
        let (Some(chat), Some(translation)) = (
            config.model(ModelRole::Chat),
            config.model(ModelRole::Translation),
        ) else {
            return Ok(AiStatus::NoModel { provider });
        };
        Ok(AiStatus::Ready {
            provider,
            chat_model: chat.to_owned(),
            translation_model: translation.to_owned(),
        })
    }

    /// The provider `id`, configured from the settings.
    pub fn provider(&self, id: ProviderId, settings: &Settings) -> AppResult<Arc<dyn LlmProvider>> {
        let factory = self
            .factory
            .as_ref()
            .ok_or_else(|| unavailable("the network client is off"))?;
        let server = if id == ProviderId::OpenAiCompatible {
            let config = AiConfig::from_settings(settings);
            let address = config
                .base_url
                .ok_or_else(|| unavailable("no server address"))?;
            Some(user_server_url(&address)?)
        } else {
            None
        };
        factory.create(id, server)
    }

    /// The chosen provider and the model of `role`; fails with
    /// `ai_unavailable` unless the status is ready.
    pub fn model(&self, settings: &Settings, role: ModelRole) -> AppResult<Model> {
        let AiStatus::Ready {
            provider,
            chat_model,
            translation_model,
        } = self.status(settings)?
        else {
            return Err(unavailable("AI is not set up"));
        };
        Ok(Model {
            provider: self.provider(provider, settings)?,
            model: match role {
                ModelRole::Chat => chat_model,
                ModelRole::Translation => translation_model,
            },
        })
    }

    /// Registers a running answer under `request_id`; it is cancelled with
    /// [`Llm::cancel`] and unregistered when the guard drops.
    pub fn start(&self, request_id: &str) -> Running<'_> {
        let token = CancellationToken::new();
        self.running_map()
            .insert(request_id.to_owned(), token.clone());
        Running {
            llm: self,
            id: request_id.to_owned(),
            token,
        }
    }

    /// Stops the answer `request_id`; nothing happens if it already ended.
    pub fn cancel(&self, request_id: &str) {
        if let Some(token) = self.running_map().get(request_id) {
            token.cancel();
        }
    }

    fn running_map(&self) -> std::sync::MutexGuard<'_, HashMap<String, CancellationToken>> {
        self.running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// A running answer; unregisters itself when dropped.
pub struct Running<'a> {
    llm: &'a Llm,
    id: String,
    token: CancellationToken,
}

impl Running<'_> {
    /// The token that fires when the user stops the answer.
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.llm.running_map().remove(&self.id);
    }
}

fn string_setting(settings: &Settings, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(|json| serde_json::from_str::<String>(json).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryKeys;

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn status_says_what_is_missing() {
        let llm = Llm::new(Arc::new(MemoryKeys::default()), None);
        let status = |pairs: &[(&str, &str)]| llm.status(&settings(pairs)).unwrap();
        assert_eq!(status(&[]), AiStatus::NoProvider);
        assert_eq!(
            status(&[(PROVIDER_SETTING, "\"voyage\"")]),
            AiStatus::NoProvider
        );
        assert_eq!(
            status(&[(PROVIDER_SETTING, "\"anthropic\"")]),
            AiStatus::KeyMissing {
                provider: ProviderId::Anthropic
            }
        );
        let compat = (PROVIDER_SETTING, "\"openai_compatible\"");
        assert_eq!(status(&[compat]), AiStatus::BaseUrlMissing);
        assert_eq!(
            status(&[compat, (BASE_URL_SETTING, "\"http://example.org/v1\"")]),
            AiStatus::BaseUrlInvalid
        );
        let server = (BASE_URL_SETTING, "\"http://localhost:11434/v1\"");
        assert_eq!(
            status(&[compat, server]),
            AiStatus::NoModel {
                provider: ProviderId::OpenAiCompatible
            }
        );
        assert_eq!(
            status(&[compat, server, (CHAT_MODEL_SETTING, "\"qwen3\"")]),
            AiStatus::Ready {
                provider: ProviderId::OpenAiCompatible,
                chat_model: "qwen3".into(),
                translation_model: "qwen3".into(),
            }
        );
    }

    #[test]
    fn arabic_sources_default_by_provider_and_follow_overrides() {
        let config = AiConfig::from_settings(&settings(&[
            (PROVIDER_SETTING, "\"openai_compatible\""),
            (ARABIC_SOURCES_SETTING, r#"{"openai_compatible/big":true}"#),
        ]));
        assert!(!config.arabic_sources("small"));
        assert!(config.arabic_sources("big"));
        let config = AiConfig::from_settings(&settings(&[
            (PROVIDER_SETTING, "\"gemini\""),
            (ARABIC_SOURCES_SETTING, r#"{"gemini/flash-lite":false}"#),
        ]));
        assert!(config.arabic_sources("pro"));
        assert!(!config.arabic_sources("flash-lite"));
    }

    #[test]
    fn cancelling_fires_the_running_token_only() {
        let llm = Llm::new(Arc::new(MemoryKeys::default()), None);
        let a = llm.start("a");
        let b = llm.start("b");
        llm.cancel("a");
        llm.cancel("unknown");
        assert!(a.token().is_cancelled());
        assert!(!b.token().is_cancelled());
        drop(a);
        assert_eq!(llm.running_map().len(), 1);
    }

    #[test]
    fn provider_errors_keep_the_status_and_message() {
        let body = br#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let err = provider_error("api.anthropic.com", StatusCode::UNAUTHORIZED, body);
        assert_eq!(err.code, AppErrorCode::ProviderKeyRejected);
        assert!(err.message.contains("invalid x-api-key"));
        let body = br#"{"error":{"code":503,"message":"The model is overloaded.","status":"UNAVAILABLE"}}"#;
        let err = provider_error(
            "generativelanguage.googleapis.com",
            StatusCode::SERVICE_UNAVAILABLE,
            body,
        );
        assert_eq!(err.code, AppErrorCode::ProviderOverloaded);
        assert!(err.message.contains("overloaded"));
        let err = stream_error(
            "api.anthropic.com",
            &serde_json::json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}),
        )
        .unwrap();
        assert_eq!(err.code, AppErrorCode::ProviderFailed);
    }
}
