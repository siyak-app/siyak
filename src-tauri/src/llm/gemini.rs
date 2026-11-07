//! Google Gemini provider with streaming (spec §8).
//!
//! `models/<model>:streamGenerateContent?alt=sse` with the key in
//! `x-goog-api-key`. Each event is a `GenerateContentResponse`: text in
//! `candidates[0].content.parts` (thought summaries are skipped), the finish
//! reason, and cumulative `usageMetadata`; thinking tokens are billed as
//! output, so they count as output here.
//!
//! Gemini 3 and later think at `medium` or `high` unless told otherwise, and
//! send nothing until they are done; an answer from given sources, a search
//! sentence or a translation needs little reasoning, so they are asked to
//! think `low` ([`thinking_level`]).

use reqwest::Url;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{
    Client, Completion, Finish, Flow, LlmFuture, LlmProvider, LlmRequest, ModelInfo, Role,
    TextSink, Usage, count, endpoint, event_json, read_events, stream_error,
};
use crate::error::AppResult;
use crate::secrets::ProviderId;

/// Gemini's API.
pub struct Gemini {
    client: Client,
    base: Url,
}

impl Gemini {
    pub(crate) fn new(client: Client, base: Url) -> Self {
        Self { client, base }
    }

    fn headers(&self) -> AppResult<Vec<(&'static str, String)>> {
        Ok(self
            .client
            .key()?
            .map(|key| ("x-goog-api-key", key))
            .into_iter()
            .collect())
    }

    async fn run(
        &self,
        request: &LlmRequest,
        on_text: TextSink<'_>,
        cancel: &CancellationToken,
    ) -> AppResult<Completion> {
        let headers = self.headers()?;
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let model = request.model.trim_start_matches("models/");
        let url = endpoint(
            &self.base,
            &format!("models/{model}:streamGenerateContent?alt=sse"),
        )?;
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
        let mut usage = None;
        let mut finish = Finish::Complete;
        let cancelled = read_events(response, cancel, |event| {
            let value = event_json(&host, &event)?;
            if let Some(err) = stream_error(&host, &value) {
                return Err(err);
            }
            let candidate = &value["candidates"][0];
            for part in candidate["content"]["parts"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if part["thought"].as_bool() == Some(true) {
                    continue;
                }
                if let Some(text) = part["text"].as_str() {
                    on_text(text);
                }
            }
            if candidate["finishReason"] == "MAX_TOKENS" {
                finish = Finish::Truncated;
            }
            let u = &value["usageMetadata"];
            if u.is_object() {
                usage = Some(Usage {
                    input_tokens: count(&u["promptTokenCount"]),
                    output_tokens: count(&u["candidatesTokenCount"])
                        + count(&u["thoughtsTokenCount"]),
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
        let url = endpoint(&self.base, "models?pageSize=1000")?;
        let value = self.client.get_json(&url, &headers).await?;
        let mut models: Vec<ModelInfo> = value["models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| {
                m["supportedGenerationMethods"]
                    .as_array()
                    .is_some_and(|methods| methods.iter().any(|x| x == "generateContent"))
            })
            .filter_map(|m| {
                let id = m["name"].as_str()?.trim_start_matches("models/");
                (!id.contains("embedding") && !id.contains("tts") && !id.contains("image")).then(
                    || ModelInfo {
                        id: id.to_owned(),
                        name: m["displayName"].as_str().map(str::to_owned),
                    },
                )
            })
            .collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }
}

impl LlmProvider for Gemini {
    fn id(&self) -> ProviderId {
        ProviderId::Gemini
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
    let contents: Vec<Value> = request
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "model",
            };
            json!({ "role": role, "parts": [{ "text": m.content }] })
        })
        .collect();
    let mut config = json!({ "maxOutputTokens": request.max_tokens });
    if let Some(level) = thinking_level(&request.model) {
        config["thinkingConfig"] = json!({ "thinkingLevel": level });
    }
    json!({
        "systemInstruction": { "parts": [{ "text": request.system }] },
        "contents": contents,
        "generationConfig": config,
    })
}

/// The thinking level asked of `model`: `low` for Gemini 3 and later. Earlier
/// models take `thinkingBudget` instead and image models reject `low`, so
/// they keep their default.
fn thinking_level(model: &str) -> Option<&'static str> {
    let model = model.trim_start_matches("models/");
    let version = model.strip_prefix("gemini-")?;
    let major: u32 = version
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    (major >= 3 && !model.contains("image")).then_some("low")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_3_and_later_think_low() {
        for model in [
            "gemini-3.8-flash",
            "models/gemini-3-flash-preview",
            "gemini-3.5-flash-lite",
            "gemini-4-argon",
        ] {
            assert_eq!(thinking_level(model), Some("low"), "{model}");
        }
        for model in [
            "gemini-2.5-pro",
            "gemini-3.1-flash-lite-image",
            "gemma-3",
            "m-1",
        ] {
            assert_eq!(thinking_level(model), None, "{model}");
        }
    }
}
