//! The API backend: embeds texts with OpenAI, Voyage, Gemini or an
//! OpenAI-compatible server (spec §7.4, ADR 0012).
//!
//! Requests go through [`crate::net`], so every provider host is on the
//! allow-list and every request is in the connection log (purpose `embed`).
//! The key is read from the keychain ([`crate::secrets`]) for each request, so
//! a new key takes effect at once; it never reaches the webview.
//!
//! The vector pack's metadata says how texts are sent: the model, the output
//! dimension, the query and passage prefixes and (Gemini's older model) the
//! task type. Voyage always gets `input_type`. Vectors are checked against the
//! pack's dimension and normalized here when the space is normalized (Gemini
//! does not normalize shortened `gemini-embedding-001` vectors).

use std::sync::Arc;

use reqwest::{StatusCode, Url};
use serde_json::{Value, json};

use super::service::{ActivePack, EmbedderFactory};
use super::space::Backend;
use super::{ApiProvider, EmbedFuture, Embedder, EmbeddingSpace, Locality, TextKind};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, Purpose, user_server_url};
use crate::secrets::KeyStore;

/// Texts per request. Gemini's batch endpoint takes at most 100, Voyage
/// 1000, OpenAI 2048; one query is one text anyway.
const BATCH: usize = 96;

/// Largest response read: 96 vectors of 3072 floats as JSON text, with room.
const MAX_RESPONSE: usize = 32 * 1024 * 1024;

/// Longest provider error message kept in a diagnostic.
const MAX_ERROR_MESSAGE: usize = 300;

/// The API addresses of the hosted providers; tests point them at a local
/// server.
#[derive(Debug, Clone)]
pub struct Endpoints {
    /// OpenAI's base URL, without a trailing slash.
    pub openai: String,
    /// Voyage's base URL.
    pub voyage: String,
    /// Gemini's base URL.
    pub gemini: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            openai: "https://api.openai.com/v1".into(),
            voyage: "https://api.voyageai.com/v1".into(),
            gemini: "https://generativelanguage.googleapis.com/v1beta".into(),
        }
    }
}

/// Builds [`ApiEmbedder`]s for API vector packs.
pub struct ApiFactory {
    net: Net,
    keys: Arc<dyn KeyStore>,
    endpoints: Endpoints,
}

impl ApiFactory {
    /// A factory sending requests with `net` and keys from `keys`.
    pub fn new(net: Net, keys: Arc<dyn KeyStore>, endpoints: Endpoints) -> Self {
        Self {
            net,
            keys,
            endpoints,
        }
    }
}

impl EmbedderFactory for ApiFactory {
    fn supports(&self, backend: &Backend) -> bool {
        matches!(backend, Backend::Api { .. })
    }

    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
        let Backend::Api {
            provider,
            model,
            dimensions,
            query_prefix,
            passage_prefix,
            task_type,
        } = &pack.spec.backend
        else {
            return Err(unavailable(format!("{}: not an API pack", pack.id)));
        };
        let (url, user_server) = match provider {
            ApiProvider::OpenAi => (format!("{}/embeddings", self.endpoints.openai), None),
            ApiProvider::Voyage => (format!("{}/embeddings", self.endpoints.voyage), None),
            ApiProvider::Gemini => (
                format!(
                    "{}/models/{model}:batchEmbedContents",
                    self.endpoints.gemini
                ),
                None,
            ),
            ApiProvider::OpenAiCompatible => {
                let base = pack
                    .base_url
                    .as_deref()
                    .ok_or_else(|| unavailable(format!("{}: no server address", pack.id)))?;
                let server = user_server_url(base)?;
                (format!("{server}/embeddings"), Some(server))
            }
        };
        let url = Url::parse(&url).map_err(|err| unavailable(format!("{url}: {err}")))?;
        Ok(Arc::new(ApiEmbedder {
            net: self.net.clone(),
            keys: Arc::clone(&self.keys),
            provider: *provider,
            model: model.clone(),
            dimensions: *dimensions,
            query_prefix: query_prefix.clone(),
            passage_prefix: passage_prefix.clone(),
            task_type: *task_type,
            space: pack.spec.space.clone(),
            url,
            user_server,
        }))
    }
}

/// Embeds texts with one provider's API, in the space of one vector pack.
pub struct ApiEmbedder {
    net: Net,
    keys: Arc<dyn KeyStore>,
    provider: ApiProvider,
    model: String,
    dimensions: Option<u32>,
    query_prefix: String,
    passage_prefix: String,
    task_type: bool,
    space: EmbeddingSpace,
    url: Url,
    /// The user's OpenAI-compatible server, which is not on the allow-list.
    user_server: Option<Url>,
}

impl ApiEmbedder {
    /// The request body for `texts`, already prefixed.
    fn body(&self, texts: &[String], kind: TextKind) -> Value {
        match self.provider {
            ApiProvider::OpenAi | ApiProvider::OpenAiCompatible => {
                let mut body = json!({
                    "model": self.model,
                    "input": texts,
                    "encoding_format": "float",
                });
                if let Some(dimensions) = self.dimensions {
                    body["dimensions"] = dimensions.into();
                }
                body
            }
            ApiProvider::Voyage => {
                let mut body = json!({
                    "model": self.model,
                    "input": texts,
                    "input_type": match kind {
                        TextKind::Query => "query",
                        TextKind::Passage => "document",
                    },
                });
                if let Some(dimensions) = self.dimensions {
                    body["output_dimension"] = dimensions.into();
                }
                body
            }
            ApiProvider::Gemini => {
                let requests: Vec<Value> = texts
                    .iter()
                    .map(|text| {
                        let mut request = json!({
                            "model": format!("models/{}", self.model),
                            "content": { "parts": [{ "text": text }] },
                        });
                        if self.task_type {
                            request["taskType"] = match kind {
                                TextKind::Query => "RETRIEVAL_QUERY",
                                TextKind::Passage => "RETRIEVAL_DOCUMENT",
                            }
                            .into();
                        }
                        if let Some(dimensions) = self.dimensions {
                            request["outputDimensionality"] = dimensions.into();
                        }
                        request
                    })
                    .collect();
                json!({ "requests": requests })
            }
        }
    }

    /// The authentication header, if a key is needed or saved.
    fn auth(&self) -> AppResult<Option<(&'static str, String)>> {
        let key = self.keys.key(self.provider)?;
        match (key, self.provider) {
            (None, provider) if provider.needs_key() => Err(AppError::new(
                AppErrorCode::SemanticUnavailable,
                format!("no {} key is saved", provider.as_str()),
            )),
            (None, _) => Ok(None),
            (Some(key), ApiProvider::Gemini) => Ok(Some(("x-goog-api-key", key))),
            (Some(key), _) => Ok(Some(("authorization", format!("Bearer {key}")))),
        }
    }

    async fn embed_batch(&self, texts: &[String], kind: TextKind) -> AppResult<Vec<Vec<f32>>> {
        let auth = self.auth()?;
        let headers: Vec<(&str, &str)> = auth.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let (status, body) = self
            .net
            .post_json(
                &self.url,
                self.user_server.as_ref(),
                Purpose::Embedding,
                &headers,
                &self.body(texts, kind),
                MAX_RESPONSE,
            )
            .await?;
        let host = self.url.host_str().unwrap_or_default();
        if !status.is_success() {
            return Err(provider_error(host, status, &body));
        }
        let vectors = parse_vectors(self.provider, &body)
            .ok_or_else(|| failed(format!("{host}: unexpected response")))?;
        if vectors.len() != texts.len() {
            return Err(failed(format!(
                "{host}: {} vectors for {} texts",
                vectors.len(),
                texts.len()
            )));
        }
        vectors
            .into_iter()
            .map(|vector| self.check(host, vector))
            .collect()
    }

    /// Checks a vector's dimension and values, normalizing it if the space is.
    fn check(&self, host: &str, mut vector: Vec<f32>) -> AppResult<Vec<f32>> {
        let dim = self.space.dim as usize;
        if vector.len() != dim {
            return Err(failed(format!(
                "{host}: vectors have {} components, the pack needs {dim}",
                vector.len()
            )));
        }
        if !vector.iter().all(|x| x.is_finite()) {
            return Err(failed(format!("{host}: a vector is not finite")));
        }
        if self.space.normalized {
            let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm == 0.0 {
                return Err(failed(format!("{host}: a vector is zero")));
            }
            vector.iter_mut().for_each(|x| *x /= norm);
        }
        Ok(vector)
    }
}

impl Embedder for ApiEmbedder {
    fn space(&self) -> &EmbeddingSpace {
        &self.space
    }

    fn locality(&self) -> Locality {
        Locality::Remote(self.provider)
    }

    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_> {
        Box::pin(async move {
            let prefix = match kind {
                TextKind::Query => &self.query_prefix,
                TextKind::Passage => &self.passage_prefix,
            };
            let texts: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
            let mut vectors = Vec::with_capacity(texts.len());
            for batch in texts.chunks(BATCH) {
                vectors.extend(self.embed_batch(batch, kind).await?);
            }
            Ok(vectors)
        })
    }
}

/// The vectors of a successful response, in input order.
fn parse_vectors(provider: ApiProvider, body: &[u8]) -> Option<Vec<Vec<f32>>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let vector = |v: &Value| -> Option<Vec<f32>> {
        v.as_array()?
            .iter()
            .map(|x| x.as_f64().map(|x| x as f32))
            .collect()
    };
    match provider {
        ApiProvider::Gemini => value["embeddings"]
            .as_array()?
            .iter()
            .map(|e| vector(&e["values"]))
            .collect(),
        _ => {
            let mut data: Vec<(u64, Vec<f32>)> = value["data"]
                .as_array()?
                .iter()
                .map(|e| Some((e["index"].as_u64()?, vector(&e["embedding"])?)))
                .collect::<Option<_>>()?;
            data.sort_by_key(|(index, _)| *index);
            let in_order = data
                .iter()
                .enumerate()
                .all(|(i, (index, _))| *index == i as u64);
            in_order.then(|| data.into_iter().map(|(_, v)| v).collect())
        }
    }
}

/// The error for a failed request, with the provider's message if it gave one.
fn provider_error(host: &str, status: StatusCode, body: &[u8]) -> AppError {
    let code = match status.as_u16() {
        401 | 403 => AppErrorCode::ProviderKeyRejected,
        429 => AppErrorCode::ProviderRateLimited,
        _ => AppErrorCode::ProviderFailed,
    };
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            let message = &value["error"]["message"];
            let detail = &value["detail"];
            message
                .as_str()
                .or_else(|| value["error"].as_str())
                .or_else(|| detail.as_str())
                .map(str::to_owned)
        })
        .map(|m| m.chars().take(MAX_ERROR_MESSAGE).collect::<String>())
        .unwrap_or_default();
    AppError::new(
        code,
        format!("{host}: {status} {message}").trim_end().to_owned(),
    )
}

fn failed(message: String) -> AppError {
    AppError::new(AppErrorCode::ProviderFailed, message)
}

fn unavailable(message: String) -> AppError {
    AppError::new(AppErrorCode::SemanticUnavailable, message)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use axum::Router;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode as AxumStatus};
    use axum::routing::post;

    use super::*;
    use crate::embed::space::VectorPackSpec;
    use crate::net::test_support::{local_net, serve};
    use crate::secrets::MemoryKeys;

    /// What the fake provider received: path, headers it cares about, body.
    type Seen = Arc<Mutex<Vec<(String, String, Value)>>>;

    /// A provider that answers every request with vectors of `dim` components
    /// (`[i + 1, 0, ...]` scaled by 2, so they need normalizing), in the shape
    /// of its API.
    fn provider_router(dim: usize, seen: Seen) -> Router {
        async fn answer(
            State((dim, seen)): State<(usize, Seen)>,
            uri: axum::http::Uri,
            headers: HeaderMap,
            body: String,
        ) -> (AxumStatus, String) {
            let body: Value = serde_json::from_str(&body).unwrap();
            let auth = ["authorization", "x-goog-api-key"]
                .iter()
                .filter_map(|h| headers.get(*h))
                .map(|v| v.to_str().unwrap().to_owned())
                .collect::<Vec<_>>()
                .join(",");
            seen.lock()
                .unwrap()
                .push((uri.path().to_owned(), auth.clone(), body.clone()));
            if auth.contains("bad") {
                return (
                    AxumStatus::UNAUTHORIZED,
                    json!({ "error": { "message": "Incorrect API key" } }).to_string(),
                );
            }
            let vector = |i: usize| {
                let mut v = vec![0.0_f32; dim];
                v[i % dim] = 2.0;
                v
            };
            let answer = if uri.path().contains(":batchEmbedContents") {
                let n = body["requests"].as_array().unwrap().len();
                json!({ "embeddings": (0..n).map(|i| json!({ "values": vector(i) })).collect::<Vec<_>>() })
            } else {
                let n = body["input"].as_array().unwrap().len();
                // Reversed, as the index field allows.
                let data: Vec<Value> = (0..n)
                    .rev()
                    .map(|i| json!({ "index": i, "embedding": vector(i), "object": "embedding" }))
                    .collect();
                json!({ "object": "list", "data": data })
            };
            (AxumStatus::OK, answer.to_string())
        }
        Router::new()
            .route("/{*path}", post(answer))
            .with_state((dim, seen))
    }

    fn pack(meta: Value, base_url: Option<String>) -> ActivePack {
        ActivePack {
            id: "vectors.test".into(),
            version: "1.0.0".into(),
            spec: VectorPackSpec::from_meta(meta.as_object().unwrap()).unwrap(),
            model: None,
            base_url,
        }
    }

    fn meta(provider: &str, model: &str, extra: Value) -> Value {
        let mut meta = json!({
            "embedding_model": format!("{}{model}", ApiProvider::parse(provider).unwrap().model_prefix()),
            "dim": 4,
            "backend": "api",
            "provider": provider,
            "api_model": model,
        });
        for (k, v) in extra.as_object().unwrap() {
            meta[k] = v.clone();
        }
        meta
    }

    async fn setup(
        keys: MemoryKeys,
    ) -> (
        ApiFactory,
        Seen,
        String,
        Arc<crate::net::test_support::MemoryLog>,
    ) {
        let seen: Seen = Arc::default();
        let base = serve(provider_router(4, Arc::clone(&seen))).await;
        let (net, log) = local_net();
        let endpoints = Endpoints {
            openai: format!("{base}/openai/v1"),
            voyage: format!("{base}/voyage/v1"),
            gemini: format!("{base}/gemini/v1beta"),
        };
        (
            ApiFactory::new(net, Arc::new(keys), endpoints),
            seen,
            base,
            log,
        )
    }

    fn assert_unit(vectors: &[Vec<f32>], n: usize) {
        assert_eq!(vectors.len(), n);
        for (i, v) in vectors.iter().enumerate() {
            let mut expected = vec![0.0; 4];
            expected[i % 4] = 1.0;
            assert_eq!(*v, expected, "vector {i} in input order, unit length");
        }
    }

    #[tokio::test]
    async fn openai_gets_the_model_dimensions_and_bearer_key() {
        let (factory, seen, _, log) = setup(MemoryKeys::with(ApiProvider::OpenAi, "sk-1")).await;
        let embedder = factory
            .create(&pack(
                meta(
                    "openai",
                    "text-embedding-3-small",
                    json!({ "api_dimensions": 4 }),
                ),
                None,
            ))
            .unwrap();
        assert_eq!(embedder.locality(), Locality::Remote(ApiProvider::OpenAi));
        let vectors = embedder
            .embed(vec!["sabır".into(), "namaz".into()], TextKind::Query)
            .await
            .unwrap();
        assert_unit(&vectors, 2);
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0, "/openai/v1/embeddings");
        assert_eq!(seen[0].1, "Bearer sk-1");
        assert_eq!(
            seen[0].2,
            json!({
                "model": "text-embedding-3-small",
                "input": ["sabır", "namaz"],
                "encoding_format": "float",
                "dimensions": 4
            })
        );
        assert_eq!(log.0.lock().unwrap()[0].1, "embed");
    }

    #[tokio::test]
    async fn voyage_gets_the_input_type() {
        let (factory, seen, _, _) = setup(MemoryKeys::with(ApiProvider::Voyage, "pa-1")).await;
        let embedder = factory
            .create(&pack(
                meta("voyage", "voyage-4", json!({ "api_dimensions": 4 })),
                None,
            ))
            .unwrap();
        embedder
            .embed(vec!["q".into()], TextKind::Query)
            .await
            .unwrap();
        embedder
            .embed(vec!["p".into()], TextKind::Passage)
            .await
            .unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0, "/voyage/v1/embeddings");
        assert_eq!(seen[0].2["input_type"], "query");
        assert_eq!(seen[0].2["output_dimension"], 4);
        assert_eq!(seen[1].2["input_type"], "document");
    }

    #[tokio::test]
    async fn gemini_gets_prefixes_task_types_and_its_key_header() {
        let (factory, seen, _, _) = setup(MemoryKeys::with(ApiProvider::Gemini, "AIza-1")).await;
        let extra = json!({
            "api_dimensions": 4,
            "query_prefix": "task: search result | query: ",
            "passage_prefix": "title: none | text: ",
            "task_type": true
        });
        let embedder = factory
            .create(&pack(meta("gemini", "gemini-embedding-001", extra), None))
            .unwrap();
        let texts: Vec<String> = (0..100).map(|i| format!("t{i}")).collect();
        let vectors = embedder.embed(texts, TextKind::Query).await.unwrap();
        assert_unit(&vectors, 100);
        embedder
            .embed(vec!["p".into()], TextKind::Passage)
            .await
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(
            seen.len(),
            3,
            "100 texts in batches of 96, then one passage"
        );
        assert_eq!(
            seen[0].0,
            "/gemini/v1beta/models/gemini-embedding-001:batchEmbedContents"
        );
        assert_eq!(seen[0].1, "AIza-1");
        let first = &seen[0].2["requests"][0];
        assert_eq!(first["model"], "models/gemini-embedding-001");
        assert_eq!(
            first["content"]["parts"][0]["text"],
            "task: search result | query: t0"
        );
        assert_eq!(first["taskType"], "RETRIEVAL_QUERY");
        assert_eq!(first["outputDimensionality"], 4);
        assert_eq!(seen[1].2["requests"].as_array().unwrap().len(), 4);
        let passage = &seen[2].2["requests"][0];
        assert_eq!(
            passage["content"]["parts"][0]["text"],
            "title: none | text: p"
        );
        assert_eq!(passage["taskType"], "RETRIEVAL_DOCUMENT");
    }

    #[tokio::test]
    async fn an_openai_compatible_server_needs_no_key() {
        let (factory, seen, base, _) = setup(MemoryKeys::default()).await;
        let base_url = format!("{}/v1/", base.replace("127.0.0.1", "localhost"));
        let embedder = factory
            .create(&pack(
                meta("openai_compatible", "nomic-embed-text", json!({})),
                Some(base_url),
            ))
            .unwrap();
        embedder
            .embed(vec!["q".into()], TextKind::Query)
            .await
            .unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0, "/v1/embeddings");
        assert_eq!(seen[0].1, "", "no key was saved, none was sent");
        assert!(seen[0].2.get("dimensions").is_none());
    }

    #[tokio::test]
    async fn a_missing_or_rejected_key_is_reported() {
        let (factory, seen, _, _) = setup(MemoryKeys::default()).await;
        let embedder = factory
            .create(&pack(
                meta("openai", "text-embedding-3-small", json!({})),
                None,
            ))
            .unwrap();
        let err = embedder
            .embed(vec!["q".into()], TextKind::Query)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::SemanticUnavailable);
        assert!(
            seen.lock().unwrap().is_empty(),
            "nothing was sent without a key"
        );

        factory
            .keys
            .set_key(ApiProvider::OpenAi, "bad-key")
            .unwrap();
        let err = embedder
            .embed(vec!["q".into()], TextKind::Query)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::ProviderKeyRejected);
        assert!(err.message.contains("Incorrect API key"), "{}", err.message);
        assert!(!err.message.contains("bad-key"));
    }

    #[tokio::test]
    async fn vectors_of_the_wrong_dimension_are_refused() {
        let (factory, _, _, _) = setup(MemoryKeys::with(ApiProvider::OpenAi, "sk-1")).await;
        let mut wrong = meta("openai", "text-embedding-3-small", json!({}));
        wrong["dim"] = 8.into();
        let embedder = factory.create(&pack(wrong, None)).unwrap();
        let err = embedder
            .embed(vec!["q".into()], TextKind::Query)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::ProviderFailed);
    }

    #[test]
    fn provider_errors_map_to_codes() {
        let cases = [
            (401, AppErrorCode::ProviderKeyRejected),
            (403, AppErrorCode::ProviderKeyRejected),
            (429, AppErrorCode::ProviderRateLimited),
            (500, AppErrorCode::ProviderFailed),
            (400, AppErrorCode::ProviderFailed),
        ];
        for (status, code) in cases {
            let err = provider_error(
                "h",
                StatusCode::from_u16(status).unwrap(),
                b"{\"detail\":\"x\"}",
            );
            assert_eq!(err.code, code);
            assert!(err.message.ends_with(" x"), "{}", err.message);
        }
    }

    #[test]
    fn responses_must_list_every_vector_once() {
        let ok = br#"{"data":[{"index":1,"embedding":[2]},{"index":0,"embedding":[1]}]}"#;
        assert_eq!(
            parse_vectors(ApiProvider::OpenAi, ok),
            Some(vec![vec![1.0], vec![2.0]])
        );
        let gap = br#"{"data":[{"index":0,"embedding":[1]},{"index":2,"embedding":[2]}]}"#;
        assert_eq!(parse_vectors(ApiProvider::OpenAi, gap), None);
        let text = br#"{"data":[{"index":0,"embedding":"AAAA"}]}"#;
        assert_eq!(parse_vectors(ApiProvider::Voyage, text), None);
        let gemini = br#"{"embeddings":[{"values":[0.5,1]}]}"#;
        assert_eq!(
            parse_vectors(ApiProvider::Gemini, gemini),
            Some(vec![vec![0.5, 1.0]])
        );
    }
}
