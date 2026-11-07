//! OpenAI provider, also used for OpenAI-compatible endpoints (Ollama,
//! LM Studio, OpenRouter) with a user-entered `base_url` (spec §8).
//!
//! Chat Completions with `stream: true`: each event carries
//! `choices[0].delta.content`; with `stream_options.include_usage` the last
//! event before `[DONE]` carries the usage. OpenAI's own models take
//! `max_completion_tokens`; compatible servers are sent the older
//! `max_tokens`, which they all understand. No `temperature` is sent:
//! reasoning models refuse it.

use reqwest::Url;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{
    Client, Completion, Finish, Flow, LlmFuture, LlmProvider, LlmRequest, ModelInfo, TextSink,
    Usage, count, endpoint, event_json, is_chat_model, read_events, stream_error,
};
use crate::error::AppResult;
use crate::secrets::ProviderId;

/// OpenAI's API, or a server that speaks it.
pub struct OpenAi {
    client: Client,
    base: Url,
}

impl OpenAi {
    pub(crate) fn new(client: Client, base: Url) -> Self {
        Self { client, base }
    }

    fn compatible(&self) -> bool {
        self.client.provider == ProviderId::OpenAiCompatible
    }

    fn headers(&self) -> AppResult<Vec<(&'static str, String)>> {
        Ok(self
            .client
            .key()?
            .map(|key| ("authorization", format!("Bearer {key}")))
            .into_iter()
            .collect())
    }

    fn body(&self, request: &LlmRequest) -> Value {
        let mut messages = vec![json!({ "role": "system", "content": request.system })];
        messages.extend(
            request
                .messages
                .iter()
                .map(|m| json!({ "role": m.role.as_str(), "content": m.content })),
        );
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true },
        });
        let limit = if self.compatible() {
            "max_tokens"
        } else {
            "max_completion_tokens"
        };
        body[limit] = request.max_tokens.into();
        body
    }

    async fn run(
        &self,
        request: &LlmRequest,
        on_text: TextSink<'_>,
        cancel: &CancellationToken,
    ) -> AppResult<Completion> {
        let headers = self.headers()?;
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let url = endpoint(&self.base, "chat/completions")?;
        let Some(response) = self
            .client
            .post_stream(&url, &headers, &self.body(request), request.purpose, cancel)
            .await?
        else {
            return Ok(Completion {
                usage: None,
                finish: Finish::Cancelled,
            });
        };
        let host = response.host().to_owned();
        let mut usage = None;
        let mut finish = Finish::Complete;
        let cancelled = read_events(response, cancel, |event| {
            if event.data.trim() == "[DONE]" {
                return Ok(Flow::Stop);
            }
            let value = event_json(&host, &event)?;
            if let Some(err) = stream_error(&host, &value) {
                return Err(err);
            }
            let choice = &value["choices"][0];
            if let Some(text) = choice["delta"]["content"].as_str()
                && !text.is_empty()
            {
                on_text(text);
            }
            if choice["finish_reason"] == "length" {
                finish = Finish::Truncated;
            }
            let u = &value["usage"];
            if u.is_object() {
                usage = Some(Usage {
                    input_tokens: count(&u["prompt_tokens"]),
                    output_tokens: count(&u["completion_tokens"]),
                });
            }
            Ok(Flow::Continue)
        })
        .await?;
        Ok(Completion {
            usage,
            finish: if cancelled { Finish::Cancelled } else { finish },
        })
    }

    async fn models(&self) -> AppResult<Vec<ModelInfo>> {
        let headers = self.headers()?;
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let url = endpoint(&self.base, "models")?;
        let value = self.client.get_json(&url, &headers).await?;
        let compatible = self.compatible();
        let mut models: Vec<ModelInfo> = value["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["id"].as_str())
            .filter(|id| is_chat_model(id, compatible))
            .map(|id| ModelInfo {
                id: id.to_owned(),
                name: None,
            })
            .collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }
}

impl LlmProvider for OpenAi {
    fn id(&self) -> ProviderId {
        self.client.provider
    }

    fn list_models(&self) -> LlmFuture<'_, Vec<ModelInfo>> {
        Box::pin(self.models())
    }

    fn stream<'a>(
        &'a self,
        request: &'a LlmRequest,
        on_text: TextSink<'a>,
        cancel: &'a CancellationToken,
    ) -> LlmFuture<'a, Completion> {
        Box::pin(self.run(request, on_text, cancel))
    }
}
