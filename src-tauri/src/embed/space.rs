//! Embedding spaces and the metadata of vector and model packs (spec §6.1, §7.4).
//!
//! A vector pack defines the space its embeddings live in: the model that made
//! them and their dimension. A query can only be compared with those vectors if
//! it is embedded in the same space, so every embedder, local or remote, is
//! checked against the active vector pack's space.
//!
//! The backend says where queries are embedded: on this computer with the ONNX
//! model of a model pack, or by an embedding API with the user's key. The pack
//! metadata names the backend; the app has no other configuration for it.

use serde::Serialize;
use serde_json::{Map, Value};
use specta::Type;

/// Largest embedding dimension a pack may declare.
const MAX_DIM: u64 = 8192;

/// Largest `max_tokens` a model pack may declare.
const MAX_TOKENS: u64 = 8192;

/// Longest model name, prefix or provider model id accepted from metadata.
const MAX_TEXT: usize = 200;

/// The vector space of a pack or an embedder. Two vectors are comparable only
/// if both come from the same space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingSpace {
    /// Canonical model name with a backend prefix, e.g.
    /// `local/multilingual-e5-small` or `openai/text-embedding-3-small`.
    pub model: String,
    /// Number of components in a vector.
    pub dim: u32,
    /// Whether vectors have unit length (dot product equals cosine similarity).
    pub normalized: bool,
}

impl EmbeddingSpace {
    /// Whether vectors of `self` and `other` can be compared.
    pub fn matches(&self, other: &Self) -> bool {
        self.model == other.model && self.dim == other.dim
    }
}

/// Embedding APIs a vector pack can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Type)]
pub enum ApiProvider {
    /// OpenAI embeddings.
    #[serde(rename = "openai")]
    OpenAi,
    /// Voyage AI embeddings, the provider Anthropic recommends.
    #[serde(rename = "voyage")]
    Voyage,
    /// Google Gemini embeddings.
    #[serde(rename = "gemini")]
    Gemini,
    /// An OpenAI-compatible server at a user-entered address (Ollama, LM
    /// Studio); may run on this computer.
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

impl ApiProvider {
    /// Every provider, in display order.
    pub const ALL: [Self; 4] = [
        Self::OpenAi,
        Self::Voyage,
        Self::Gemini,
        Self::OpenAiCompatible,
    ];

    /// The metadata value, also the keychain account of the provider's key,
    /// shared with the AI layer (spec §8, §14.2).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Voyage => "voyage",
            Self::Gemini => "gemini",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }

    /// The provider for a metadata value.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == value)
    }

    /// The prefix of the canonical model names of this provider's spaces.
    pub fn model_prefix(self) -> &'static str {
        match self {
            Self::OpenAi => "openai/",
            Self::Voyage => "voyage/",
            Self::Gemini => "gemini/",
            Self::OpenAiCompatible => "compat/",
        }
    }

    /// Whether requests need an API key. OpenAI-compatible servers often run
    /// locally without one, so a key is optional there.
    pub fn needs_key(self) -> bool {
        !matches!(self, Self::OpenAiCompatible)
    }

    /// Whether the user must enter the server address (`semantic.base_url`).
    pub fn needs_base_url(self) -> bool {
        matches!(self, Self::OpenAiCompatible)
    }
}

/// Prefix of the canonical model names of local (ONNX) spaces.
pub const LOCAL_PREFIX: &str = "local/";

/// Where queries are embedded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// On this computer, with the ONNX model of a model pack.
    Local {
        /// Id of the model pack.
        model_pack: String,
    },
    /// By an embedding API.
    Api {
        /// The provider.
        provider: ApiProvider,
        /// The provider's model id, e.g. `text-embedding-3-small`.
        model: String,
        /// Output dimension to request from models that can shorten their
        /// vectors; equals the space's dimension when present.
        dimensions: Option<u32>,
    },
}

impl Backend {
    /// `local` or `api`, as in the metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            Self::Api { .. } => "api",
        }
    }
}

/// The metadata of a `vectors` pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorPackSpec {
    /// The space of the pack's vectors.
    pub space: EmbeddingSpace,
    /// What text was embedded per verse (spec §18), for display.
    pub source_text: Option<String>,
    /// Where queries are embedded.
    pub backend: Backend,
}

impl VectorPackSpec {
    /// Reads and checks the metadata of a vector pack.
    pub fn from_meta(meta: &Map<String, Value>) -> Result<Self, String> {
        let space = space(meta)?;
        let source_text = optional_text(meta, "source_text")?;
        let backend = match text(meta, "backend")?.as_str() {
            "local" => {
                if !space.model.starts_with(LOCAL_PREFIX) {
                    return Err(format!(
                        "a local embedding_model must start with {LOCAL_PREFIX}"
                    ));
                }
                Backend::Local {
                    model_pack: text(meta, "model_pack")?,
                }
            }
            "api" => {
                let name = text(meta, "provider")?;
                let provider = ApiProvider::parse(&name)
                    .ok_or_else(|| format!("unknown provider {name:?}"))?;
                if !space.model.starts_with(provider.model_prefix()) {
                    return Err(format!(
                        "embedding_model of a {name} pack must start with {}",
                        provider.model_prefix()
                    ));
                }
                let dimensions = optional_count(meta, "api_dimensions", MAX_DIM)?;
                if dimensions.is_some_and(|d| d != space.dim) {
                    return Err("api_dimensions differs from dim".into());
                }
                Backend::Api {
                    provider,
                    model: text(meta, "api_model")?,
                    dimensions,
                }
            }
            other => return Err(format!("unknown backend {other:?}")),
        };
        Ok(Self {
            space,
            source_text,
            backend,
        })
    }

    /// Checks the manifest's `requires` against the backend: a local pack must
    /// require its model pack, so both are downloaded together.
    pub fn check_requires(&self, requires: &[String]) -> Result<(), String> {
        match &self.backend {
            Backend::Local { model_pack } if !requires.contains(model_pack) => {
                Err(format!("requires must contain the model pack {model_pack}"))
            }
            _ => Ok(()),
        }
    }

    /// Checks that `model` embeds queries in this pack's space.
    pub fn check_model(&self, model: &ModelPackSpec) -> Result<(), String> {
        if self.space.matches(&model.space) {
            Ok(())
        } else {
            Err(format!(
                "the vectors are {} ({}), the model is {} ({})",
                self.space.model, self.space.dim, model.space.model, model.space.dim
            ))
        }
    }
}

/// How token vectors are pooled into one vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pooling {
    /// The mean of the token vectors, weighted by the attention mask.
    Mean,
    /// The vector of the first (`[CLS]`) token.
    Cls,
    /// The vector of the last token that is not padding (decoder models
    /// such as Qwen3-Embedding).
    Last,
}

/// The metadata of a `model` pack, which only local vector packs use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPackSpec {
    /// The space the model embeds into.
    pub space: EmbeddingSpace,
    /// Text put before a query, e.g. `query: ` for e5 models.
    pub query_prefix: String,
    /// Text put before a passage (tafsir paragraphs, spec §10).
    pub passage_prefix: String,
    /// How token vectors are pooled.
    pub pooling: Pooling,
    /// Longest input in tokens; longer inputs are truncated.
    pub max_tokens: u32,
}

impl ModelPackSpec {
    /// Reads and checks the metadata of a model pack.
    pub fn from_meta(meta: &Map<String, Value>) -> Result<Self, String> {
        let space = space(meta)?;
        if !space.model.starts_with(LOCAL_PREFIX) {
            return Err(format!(
                "a model pack's embedding_model must start with {LOCAL_PREFIX}"
            ));
        }
        let pooling = match optional_text(meta, "pooling")?.as_deref() {
            None | Some("mean") => Pooling::Mean,
            Some("cls") => Pooling::Cls,
            Some("last") => Pooling::Last,
            Some(other) => return Err(format!("unknown pooling {other:?}")),
        };
        Ok(Self {
            space,
            query_prefix: optional_text(meta, "query_prefix")?.unwrap_or_default(),
            passage_prefix: optional_text(meta, "passage_prefix")?.unwrap_or_default(),
            pooling,
            max_tokens: optional_count(meta, "max_tokens", MAX_TOKENS)?.unwrap_or(512),
        })
    }
}

fn space(meta: &Map<String, Value>) -> Result<EmbeddingSpace, String> {
    let model = text(meta, "embedding_model")?;
    let valid_name = model.split_once('/').is_some_and(|(prefix, name)| {
        !prefix.is_empty() && !name.is_empty() && !name.chars().any(char::is_whitespace)
    });
    if !valid_name {
        return Err(format!("embedding_model {model:?} is not <backend>/<name>"));
    }
    let dim = optional_count(meta, "dim", MAX_DIM)?.ok_or("dim is missing")?;
    let normalized = match meta.get("normalized") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("normalized must be a boolean".into()),
    };
    Ok(EmbeddingSpace {
        model,
        dim,
        normalized,
    })
}

fn text(meta: &Map<String, Value>, key: &str) -> Result<String, String> {
    optional_text(meta, key)?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} is missing"))
}

fn optional_text(meta: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match meta.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.len() <= MAX_TEXT => Ok(Some(s.clone())),
        Some(_) => Err(format!(
            "{key} must be a string of at most {MAX_TEXT} bytes"
        )),
    }
}

fn optional_count(meta: &Map<String, Value>, key: &str, max: u64) -> Result<Option<u32>, String> {
    match meta.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .filter(|n| (1..=max).contains(n))
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| format!("{key} must be an integer from 1 to {max}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn local() -> Value {
        serde_json::json!({
            "embedding_model": "local/multilingual-e5-small",
            "dim": 384,
            "normalized": true,
            "source_text": "ar+tr",
            "backend": "local",
            "model_pack": "model.e5-small"
        })
    }

    fn api() -> Value {
        serde_json::json!({
            "embedding_model": "openai/text-embedding-3-small",
            "dim": 512,
            "backend": "api",
            "provider": "openai",
            "api_model": "text-embedding-3-small",
            "api_dimensions": 512
        })
    }

    fn model() -> Value {
        serde_json::json!({
            "embedding_model": "local/multilingual-e5-small",
            "dim": 384,
            "query_prefix": "query: ",
            "passage_prefix": "passage: ",
            "pooling": "mean",
            "max_tokens": 512
        })
    }

    #[test]
    fn reads_a_local_vector_pack() {
        let spec = VectorPackSpec::from_meta(&meta(local())).unwrap();
        assert_eq!(
            spec.space,
            EmbeddingSpace {
                model: "local/multilingual-e5-small".into(),
                dim: 384,
                normalized: true,
            }
        );
        assert_eq!(spec.source_text.as_deref(), Some("ar+tr"));
        assert_eq!(
            spec.backend,
            Backend::Local {
                model_pack: "model.e5-small".into()
            }
        );
    }

    #[test]
    fn reads_an_api_vector_pack() {
        let spec = VectorPackSpec::from_meta(&meta(api())).unwrap();
        assert_eq!(spec.space.dim, 512);
        assert!(spec.space.normalized, "normalized defaults to true");
        assert_eq!(
            spec.backend,
            Backend::Api {
                provider: ApiProvider::OpenAi,
                model: "text-embedding-3-small".into(),
                dimensions: Some(512),
            }
        );
    }

    #[test]
    fn reads_every_provider() {
        for provider in ApiProvider::ALL {
            let mut value = api();
            value["provider"] = provider.as_str().into();
            value["embedding_model"] = format!("{}some-model", provider.model_prefix()).into();
            let spec = VectorPackSpec::from_meta(&meta(value)).unwrap();
            assert!(matches!(spec.backend, Backend::Api { provider: p, .. } if p == provider));
        }
    }

    #[test]
    fn rejects_invalid_vector_metadata() {
        let cases: Vec<(&str, Value)> = vec![
            ("embedding_model", Value::Null),
            ("embedding_model", "e5-small".into()),
            ("embedding_model", "local/ e5".into()),
            ("dim", Value::Null),
            ("dim", 0.into()),
            ("dim", 9000.into()),
            ("dim", "384".into()),
            ("normalized", "yes".into()),
            ("backend", "remote".into()),
            ("model_pack", Value::Null),
            ("source_text", 3.into()),
        ];
        for (key, value) in cases {
            let mut m = meta(local());
            m.insert(key.into(), value.clone());
            assert!(
                VectorPackSpec::from_meta(&m).is_err(),
                "{key} = {value} should fail"
            );
        }
    }

    #[test]
    fn rejects_invalid_api_metadata() {
        let cases: Vec<(&str, Value)> = vec![
            ("provider", "anthropic".into()),
            ("provider", Value::Null),
            ("api_model", "".into()),
            ("api_dimensions", 256.into()),
            // The space must belong to the provider that embeds the queries.
            ("embedding_model", "voyage/voyage-4".into()),
            ("embedding_model", "local/multilingual-e5-small".into()),
        ];
        for (key, value) in cases {
            let mut m = meta(api());
            m.insert(key.into(), value.clone());
            assert!(
                VectorPackSpec::from_meta(&m).is_err(),
                "{key} = {value} should fail"
            );
        }
        let mut m = meta(local());
        m.insert("embedding_model".into(), "openai/x".into());
        assert!(VectorPackSpec::from_meta(&m).is_err());
    }

    #[test]
    fn a_local_pack_must_require_its_model() {
        let spec = VectorPackSpec::from_meta(&meta(local())).unwrap();
        assert!(spec.check_requires(&["model.e5-small".into()]).is_ok());
        assert!(spec.check_requires(&[]).is_err());
        assert!(spec.check_requires(&["model.other".into()]).is_err());
        let spec = VectorPackSpec::from_meta(&meta(api())).unwrap();
        assert!(spec.check_requires(&[]).is_ok());
    }

    #[test]
    fn reads_a_model_pack() {
        let spec = ModelPackSpec::from_meta(&meta(model())).unwrap();
        assert_eq!(spec.query_prefix, "query: ");
        assert_eq!(spec.passage_prefix, "passage: ");
        assert_eq!(spec.pooling, Pooling::Mean);
        assert_eq!(spec.max_tokens, 512);

        let mut m = meta(model());
        for key in ["query_prefix", "passage_prefix", "pooling", "max_tokens"] {
            m.remove(key);
        }
        let spec = ModelPackSpec::from_meta(&m).unwrap();
        assert_eq!(spec.query_prefix, "");
        assert_eq!(spec.pooling, Pooling::Mean);
        assert_eq!(spec.max_tokens, 512);

        for (name, pooling) in [("cls", Pooling::Cls), ("last", Pooling::Last)] {
            m.insert("pooling".into(), name.into());
            assert_eq!(ModelPackSpec::from_meta(&m).unwrap().pooling, pooling);
        }
    }

    #[test]
    fn rejects_invalid_model_metadata() {
        let cases: Vec<(&str, Value)> = vec![
            ("embedding_model", "openai/text-embedding-3-small".into()),
            ("pooling", "max".into()),
            ("max_tokens", 0.into()),
            ("query_prefix", 1.into()),
        ];
        for (key, value) in cases {
            let mut m = meta(model());
            m.insert(key.into(), value.clone());
            assert!(
                ModelPackSpec::from_meta(&m).is_err(),
                "{key} = {value} should fail"
            );
        }
    }

    #[test]
    fn checks_that_the_model_matches_the_vectors() {
        let vectors = VectorPackSpec::from_meta(&meta(local())).unwrap();
        let model_spec = ModelPackSpec::from_meta(&meta(model())).unwrap();
        assert!(vectors.check_model(&model_spec).is_ok());

        let mut other = model();
        other["dim"] = 768.into();
        let other = ModelPackSpec::from_meta(&meta(other)).unwrap();
        assert!(vectors.check_model(&other).is_err());

        let mut other = model();
        other["embedding_model"] = "local/multilingual-e5-base".into();
        let other = ModelPackSpec::from_meta(&meta(other)).unwrap();
        assert!(vectors.check_model(&other).is_err());
    }

    #[test]
    fn provider_names_round_trip() {
        for provider in ApiProvider::ALL {
            assert_eq!(ApiProvider::parse(provider.as_str()), Some(provider));
            assert_eq!(
                serde_json::to_value(provider).unwrap(),
                Value::from(provider.as_str())
            );
        }
        assert_eq!(ApiProvider::parse("anthropic"), None);
    }
}
