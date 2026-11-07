//! Anthropic Messages API provider with streaming (spec §8).
//!
//! Requests carry `x-api-key` and `anthropic-version`. The stream sends
//! `message_start` (input tokens), `content_block_delta` with `text_delta`
//! pieces, `message_delta` (stop reason, cumulative output tokens) and
//! `message_stop`; an `error` event reports a failure mid-stream.

use reqwest::Url;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{
    Client, Completion, Finish, Flow, LlmFuture, LlmProvider, LlmRequest, ModelInfo, TextSink,
    Usage, count, endpoint, event_json, read_events, stream_error,
};
use crate::error::AppResult;
use crate::secrets::ProviderId;

/// The API version sent with every request.
const API_VERSION: &str = "2023-06-01";

/// Anthropic's API.
pub struct Anthropic {
    client: Client,
    base: Url,
}

impl Anthropic {
    pub(crate) fn new(client: Client, base: Url) -> Self {
        Self { client, base }
    }

    fn headers(&self) -> AppResult<Vec<(&'static str, String)>> {
        let mut headers = vec![("anthropic-version", API_VERSION.to_owned())];
        if let Some(key) = self.client.key()? {
            headers.push(("x-api-key", key));
        }
        Ok(headers)
    }

    async fn run(
        &self,
        request: &LlmRequest,
        on_text: TextSink<'_>,
        cancel: &CancellationToken,
    ) -> AppResult<Completion> {
        let headers = self.headers()?;
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let url = endpoint(&self.base, "messages")?;
        let Some(response) = self
            .client
            .post_stream(&url, &headers, &body(request), request.purpose, cancel)
            .await?
        else {
            return Ok(Completion {
                usage: None,
                finish: Finish::Cancelled,
            });
        };
        let host = response.host().to_owned();
        let mut usage = Usage::default();
        let mut seen_usage = false;
        let mut finish = Finish::Complete;
        let cancelled = read_events(response, cancel, |event| {
            let value = event_json(&host, &event)?;
            if let Some(err) = stream_error(&host, &value) {
                return Err(err);
            }
            match value["type"].as_str() {
                Some("message_start") => {
                    let u = &value["message"]["usage"];
                    usage.input_tokens = input_tokens(u);
                    usage.output_tokens = count(&u["output_tokens"]);
                    seen_usage = true;
                }
                Some("content_block_delta") => {
                    if value["delta"]["type"] == "text_delta"
                        && let Some(text) = value["delta"]["text"].as_str()
                    {
                        on_text(text);
                    }
                }
                Some("message_delta") => {
                    let u = &value["usage"];
                    usage.output_tokens = count(&u["output_tokens"]);
                    if u.get("input_tokens").is_some_and(|v| !v.is_null()) {
                        usage.input_tokens = input_tokens(u);
                    }
                    if value["delta"]["stop_reason"] == "max_tokens" {
                        finish = Finish::Truncated;
                    }
                }
                Some("message_stop") => return Ok(Flow::Stop),
                _ => {}
            }
            Ok(Flow::Continue)
        })
        .await?;
        Ok(Completion {
            usage: seen_usage.then_some(usage),
            finish: if cancelled { Finish::Cancelled } else { finish },
        })
    }

    async fn models(&self) -> AppResult<Vec<ModelInfo>> {
        let headers = self.headers()?;
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let url = endpoint(&self.base, "models?limit=1000")?;
        let value = self.client.get_json(&url, &headers).await?;
        let mut models: Vec<ModelInfo> = value["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| {
                Some(ModelInfo {
                    id: m["id"].as_str()?.to_owned(),
                    name: m["display_name"].as_str().map(str::to_owned),
                })
            })
            .collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }
}

impl LlmProvider for Anthropic {
    fn id(&self) -> ProviderId {
        ProviderId::Anthropic
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

/// The request body.
fn body(request: &LlmRequest) -> Value {
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|m| json!({ "role": m.role.as_str(), "content": m.content }))
        .collect();
    json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "system": request.system,
        "messages": messages,
        "stream": true,
    })
}

/// Input tokens, including any read from or written to the prompt cache.
fn input_tokens(usage: &Value) -> u32 {
    count(&usage["input_tokens"])
        + count(&usage["cache_creation_input_tokens"])
        + count(&usage["cache_read_input_tokens"])
}
