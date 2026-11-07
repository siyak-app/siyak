//! The HTTP providers against a local server that streams in each API's
//! format.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::routing::{get, post};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::*;
use crate::net::test_support::{local_net, serve};
use crate::secrets::MemoryKeys;

/// What the server received: path with query, auth headers, body.
type Seen = Arc<Mutex<Vec<(String, String, Value)>>>;

fn sse(events: &[(&str, Value)]) -> String {
    events
        .iter()
        .map(|(name, data)| {
            if name.is_empty() {
                format!("data: {data}\n\n")
            } else {
                format!("event: {name}\ndata: {data}\n\n")
            }
        })
        .collect()
}

fn anthropic_stream() -> String {
    sse(&[
        (
            "message_start",
            json!({"type":"message_start","message":{"usage":{"input_tokens":12,"cache_read_input_tokens":3,"output_tokens":1}}}),
        ),
        ("ping", json!({"type":"ping"})),
        (
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Sabır "}}),
        ),
        (
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"[2:153]"}}),
        ),
        (
            "message_delta",
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":7}}),
        ),
        ("message_stop", json!({"type":"message_stop"})),
    ])
}

fn openai_stream() -> String {
    let mut body = sse(&[
        (
            "",
            json!({"choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}),
        ),
        (
            "",
            json!({"choices":[{"index":0,"delta":{"content":"Sabır "}}]}),
        ),
        (
            "",
            json!({"choices":[{"index":0,"delta":{"content":"[2:153]"},"finish_reason":"length"}]}),
        ),
        (
            "",
            json!({"choices":[],"usage":{"prompt_tokens":15,"completion_tokens":7}}),
        ),
    ]);
    body.push_str("data: [DONE]\n\n");
    body
}

fn gemini_stream() -> String {
    sse(&[
        (
            "",
            json!({"candidates":[{"content":{"role":"model","parts":[{"text":"thinking","thought":true},{"text":"Sabır "}]}}]}),
        ),
        (
            "",
            json!({"candidates":[{"content":{"role":"model","parts":[{"text":"[2:153]"}]},"finishReason":"STOP"}],
                   "usageMetadata":{"promptTokenCount":15,"candidatesTokenCount":5,"thoughtsTokenCount":2}}),
        ),
    ])
}

async fn server(seen: Seen) -> String {
    async fn stream(
        State(seen): State<Seen>,
        uri: Uri,
        headers: HeaderMap,
        body: String,
    ) -> (StatusCode, [(&'static str, &'static str); 1], String) {
        let auth = auth_of(&headers);
        let path = uri.path_and_query().unwrap().to_string();
        seen.lock().unwrap().push((
            path.clone(),
            auth.clone(),
            serde_json::from_str(&body).unwrap(),
        ));
        let event_stream = [("content-type", "text/event-stream")];
        if auth.contains("bad") {
            return (
                StatusCode::UNAUTHORIZED,
                event_stream,
                json!({"error":{"type":"authentication_error","message":"invalid key"}})
                    .to_string(),
            );
        }
        if auth.contains("busy") {
            return (StatusCode::TOO_MANY_REQUESTS, event_stream, String::new());
        }
        let body = if path.starts_with("/anthropic") {
            anthropic_stream()
        } else if path.starts_with("/gemini") {
            gemini_stream()
        } else {
            openai_stream()
        };
        (StatusCode::OK, event_stream, body)
    }
    async fn models(uri: Uri, headers: HeaderMap) -> String {
        assert!(!auth_of(&headers).is_empty() || uri.path().starts_with("/compat"));
        if uri.path().starts_with("/anthropic") {
            json!({"data":[{"id":"claude-b","display_name":"Claude B"},{"id":"claude-a","display_name":"Claude A"}]})
        } else if uri.path().starts_with("/gemini") {
            json!({"models":[
                {"name":"models/gemini-pro","displayName":"Gemini Pro","supportedGenerationMethods":["generateContent"]},
                {"name":"models/gemini-embedding","supportedGenerationMethods":["embedContent"]},
                {"name":"models/gemini-2.5-flash-native-audio","supportedGenerationMethods":["generateContent"]},
                {"name":"models/gemini-2.5-flash-image","supportedGenerationMethods":["generateContent"]},
                {"name":"models/veo-3","supportedGenerationMethods":["generateContent"]}
            ]})
        } else {
            json!({"data":[{"id":"gpt-x"},{"id":"text-embedding-3-small"},{"id":"gpt-4o-mini-tts"},{"id":"gpt-realtime"},{"id":"llama3"},{"id":"nomic-embed-text"}]})
        }
        .to_string()
    }
    let router = Router::new()
        .route("/anthropic/messages", post(stream))
        .route("/openai/chat/completions", post(stream))
        .route("/compat/v1/chat/completions", post(stream))
        .route("/gemini/{*rest}", post(stream))
        .route("/anthropic/models", get(models))
        .route("/openai/models", get(models))
        .route("/compat/v1/models", get(models))
        .route("/gemini/models", get(models))
        .with_state(seen);
    serve(router).await
}

fn auth_of(headers: &HeaderMap) -> String {
    [
        "authorization",
        "x-api-key",
        "x-goog-api-key",
        "anthropic-version",
    ]
    .iter()
    .filter_map(|h| Some(format!("{h}={}", headers.get(*h)?.to_str().ok()?)))
    .collect::<Vec<_>>()
    .join(",")
}

async fn setup(
    keys: MemoryKeys,
) -> (
    HttpProviders,
    Seen,
    String,
    Arc<crate::net::test_support::MemoryLog>,
) {
    let seen = Seen::default();
    let base = server(Arc::clone(&seen)).await;
    let (net, log) = local_net();
    let providers = HttpProviders::new(
        net,
        Arc::new(keys),
        Endpoints {
            anthropic: format!("{base}/anthropic"),
            openai: format!("{base}/openai"),
            gemini: format!("{base}/gemini"),
        },
    );
    (providers, seen, base, log)
}

fn request() -> LlmRequest {
    LlmRequest {
        model: "m-1".into(),
        system: "Cite verses.".into(),
        messages: vec![
            Message {
                role: Role::User,
                content: "What is patience?".into(),
            },
            Message {
                role: Role::Assistant,
                content: "Earlier answer.".into(),
            },
            Message {
                role: Role::User,
                content: "More?".into(),
            },
        ],
        max_tokens: 100,
        purpose: Purpose::Chat,
    }
}

async fn collect(provider: &dyn LlmProvider) -> AppResult<(String, Completion)> {
    let text = Mutex::new(String::new());
    let on_text = |piece: &str| text.lock().unwrap().push_str(piece);
    let request = request();
    let completion = provider
        .stream(&request, &on_text, &CancellationToken::new())
        .await?;
    Ok((text.into_inner().unwrap(), completion))
}

#[tokio::test]
async fn anthropic_streams_text_and_usage() {
    let (providers, seen, _, log) = setup(MemoryKeys::with(ProviderId::Anthropic, "sk-ant")).await;
    let provider = providers.create(ProviderId::Anthropic, None).unwrap();
    let (text, completion) = collect(&*provider).await.unwrap();
    assert_eq!(text, "Sabır [2:153]");
    assert_eq!(
        completion,
        Completion {
            usage: Some(Usage {
                input_tokens: 15,
                output_tokens: 7
            }),
            finish: Finish::Complete,
        }
    );
    let (path, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(path, "/anthropic/messages");
    assert!(auth.contains("x-api-key=sk-ant") && auth.contains("anthropic-version=2023-06-01"));
    assert_eq!(body["system"], "Cite verses.");
    assert_eq!(body["stream"], true);
    assert_eq!(body["max_tokens"], 100);
    assert_eq!(
        body["messages"][1],
        json!({"role":"assistant","content":"Earlier answer."})
    );
    assert_eq!(log.0.lock().unwrap()[0].1, "chat");
}

#[tokio::test]
async fn openai_streams_text_usage_and_truncation() {
    let (providers, seen, _, _) = setup(MemoryKeys::with(ProviderId::OpenAi, "sk-1")).await;
    let provider = providers.create(ProviderId::OpenAi, None).unwrap();
    let (text, completion) = collect(&*provider).await.unwrap();
    assert_eq!(text, "Sabır [2:153]");
    assert_eq!(completion.finish, Finish::Truncated);
    assert_eq!(
        completion.usage,
        Some(Usage {
            input_tokens: 15,
            output_tokens: 7
        })
    );
    let (_, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(auth, "authorization=Bearer sk-1");
    assert_eq!(
        body["messages"][0],
        json!({"role":"system","content":"Cite verses."})
    );
    assert_eq!(body["max_completion_tokens"], 100);
    assert_eq!(body["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn compatible_server_needs_no_key_and_gets_max_tokens() {
    let (providers, seen, base, log) = setup(MemoryKeys::default()).await;
    let server = Url::parse(&format!("{base}/compat/v1")).unwrap();
    let provider = providers
        .create(ProviderId::OpenAiCompatible, Some(server))
        .unwrap();
    let (text, _) = collect(&*provider).await.unwrap();
    assert_eq!(text, "Sabır [2:153]");
    let (_, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(auth, "");
    assert_eq!(body["max_tokens"], 100);
    assert!(body.get("max_completion_tokens").is_none());
    let models = provider.list_models().await.unwrap();
    // A compatible server's list loses only clear non-chat models.
    let ids: Vec<_> = models.into_iter().map(|m| m.id).collect();
    assert_eq!(ids, ["gpt-realtime", "gpt-x", "llama3"]);
    assert_eq!(log.0.lock().unwrap()[0].0, "127.0.0.1");
}

#[tokio::test]
async fn gemini_skips_thoughts_and_counts_them_as_output() {
    let (providers, seen, _, _) = setup(MemoryKeys::with(ProviderId::Gemini, "AIza-1")).await;
    let provider = providers.create(ProviderId::Gemini, None).unwrap();
    let (text, completion) = collect(&*provider).await.unwrap();
    assert_eq!(text, "Sabır [2:153]");
    assert_eq!(
        completion.usage,
        Some(Usage {
            input_tokens: 15,
            output_tokens: 7
        })
    );
    let (path, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(path, "/gemini/models/m-1:streamGenerateContent?alt=sse");
    assert_eq!(auth, "x-goog-api-key=AIza-1");
    assert_eq!(
        body["systemInstruction"]["parts"][0]["text"],
        "Cite verses."
    );
    assert_eq!(body["contents"][1]["role"], "model");
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 100);
    // `m-1` is not a Gemini 3 model: its default thinking is kept.
    assert!(body["generationConfig"].get("thinkingConfig").is_none());
}

#[tokio::test]
async fn model_lists_keep_chat_models_only() {
    let keys = MemoryKeys::with(ProviderId::Anthropic, "sk-ant");
    keys.set_key(ProviderId::OpenAi, "sk-1").unwrap();
    keys.set_key(ProviderId::Gemini, "AIza-1").unwrap();
    let (providers, _, _, _) = setup(keys).await;
    let ids = |models: Vec<ModelInfo>| models.into_iter().map(|m| m.id).collect::<Vec<_>>();
    let anthropic = providers.create(ProviderId::Anthropic, None).unwrap();
    assert_eq!(
        ids(anthropic.list_models().await.unwrap()),
        ["claude-a", "claude-b"]
    );
    let openai = providers.create(ProviderId::OpenAi, None).unwrap();
    assert_eq!(
        ids(openai.list_models().await.unwrap()),
        ["gpt-x", "llama3"]
    );
    let gemini = providers.create(ProviderId::Gemini, None).unwrap();
    let models = gemini.list_models().await.unwrap();
    assert_eq!(models[0].name.as_deref(), Some("Gemini Pro"));
    assert_eq!(ids(models), ["gemini-pro"]);
}

#[tokio::test]
async fn failures_map_to_provider_error_codes() {
    let (providers, _, _, _) = setup(MemoryKeys::with(ProviderId::Anthropic, "bad")).await;
    let provider = providers.create(ProviderId::Anthropic, None).unwrap();
    let err = collect(&*provider).await.unwrap_err();
    assert_eq!(err.code, AppErrorCode::ProviderKeyRejected);
    assert!(err.message.contains("invalid key"));

    let (providers, _, _, _) = setup(MemoryKeys::with(ProviderId::OpenAi, "busy")).await;
    let provider = providers.create(ProviderId::OpenAi, None).unwrap();
    assert_eq!(
        collect(&*provider).await.unwrap_err().code,
        AppErrorCode::ProviderRateLimited
    );

    let (providers, _, _, _) = setup(MemoryKeys::default()).await;
    let provider = providers.create(ProviderId::Gemini, None).unwrap();
    assert_eq!(
        collect(&*provider).await.unwrap_err().code,
        AppErrorCode::AiUnavailable
    );
}

#[tokio::test]
async fn an_error_event_mid_stream_fails_the_answer() {
    async fn broken() -> String {
        sse(&[
            (
                "content_block_delta",
                json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"Half"}}),
            ),
            (
                "error",
                json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}),
            ),
        ])
    }
    let base = serve(Router::new().route("/messages", post(broken))).await;
    let (net, _) = local_net();
    let providers = HttpProviders::new(
        net,
        Arc::new(MemoryKeys::with(ProviderId::Anthropic, "sk")),
        Endpoints {
            anthropic: base,
            ..Endpoints::default()
        },
    );
    let provider = providers.create(ProviderId::Anthropic, None).unwrap();
    let err = collect(&*provider).await.unwrap_err();
    assert_eq!(err.code, AppErrorCode::ProviderFailed);
    assert!(err.message.contains("Overloaded"));
}

#[tokio::test]
async fn cancelling_stops_reading() {
    let (providers, _, _, _) = setup(MemoryKeys::with(ProviderId::OpenAi, "sk-1")).await;
    let provider = providers.create(ProviderId::OpenAi, None).unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let request = request();
    let completion = provider.stream(&request, &|_| {}, &cancel).await.unwrap();
    assert_eq!(completion.finish, Finish::Cancelled);
    assert_eq!(completion.usage, None);
}

#[tokio::test]
async fn stopping_works_before_a_thinking_model_answers() {
    // A thinking model sends nothing, not even the headers, until it is done.
    async fn thinking() -> String {
        std::future::pending::<()>().await;
        String::new()
    }
    let base = serve(Router::new().route("/models/{model}", post(thinking))).await;
    let (net, _) = local_net();
    let providers = HttpProviders::new(
        net,
        Arc::new(MemoryKeys::with(ProviderId::Gemini, "key")),
        Endpoints {
            gemini: base,
            ..Endpoints::default()
        },
    );
    let provider = providers.create(ProviderId::Gemini, None).unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        stop.cancel();
    });
    let request = request();
    let completion = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        provider.stream(&request, &|_| {}, &cancel),
    )
    .await
    .expect("stopping does not wait for the model")
    .unwrap();
    assert_eq!(completion.finish, Finish::Cancelled);
    assert_eq!(completion.usage, None);
}

#[tokio::test]
async fn an_overloaded_model_is_asked_once_more() {
    let calls = Arc::new(Mutex::new(0_u32));
    let router = Router::new()
        .route(
            "/models/{model}",
            post(|State(calls): State<Arc<Mutex<u32>>>| async move {
                let mut calls = calls.lock().unwrap();
                *calls += 1;
                if *calls == 1 {
                    (StatusCode::SERVICE_UNAVAILABLE, String::new())
                } else {
                    (StatusCode::OK, gemini_stream())
                }
            }),
        )
        .with_state(Arc::clone(&calls));
    let base = serve(router).await;
    let (net, _) = local_net();
    let providers = HttpProviders::new(
        net,
        Arc::new(MemoryKeys::with(ProviderId::Gemini, "key")),
        Endpoints {
            gemini: base,
            ..Endpoints::default()
        },
    );
    let provider = providers.create(ProviderId::Gemini, None).unwrap();
    let (text, _) = collect(&*provider).await.unwrap();
    assert_eq!(text, "Sabır [2:153]");
    assert_eq!(*calls.lock().unwrap(), 2);
}
