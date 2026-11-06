//! Picks the active vector pack, explains why semantic features are not
//! available and loads the embedder for it lazily (spec §7.4).
//!
//! Semantic search, cross-references and chat all ask this service for one
//! embedder. It checks, in order: is a vector pack installed, is its metadata
//! valid, is its model pack installed and in the same space (local), is the
//! server address entered and the key stored (API), and can this build run the
//! backend. The first failing check is the [`SemanticStatus`] the interface
//! shows.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use serde::Serialize;
use specta::Type;

use super::index::VectorIndex;
use super::space::{Backend, ModelPackSpec, VectorPackSpec};
use super::{ApiProvider, Embedder};
use crate::db::user::Settings;
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::packs::manifest::InstalledPack;
pub use crate::secrets::{KeyStore, NoKeyStore};

/// Setting that names the active vector pack (JSON string). Without it, or if
/// it names a pack that is not installed, the first installed one is used.
pub const VECTOR_PACK_SETTING: &str = "semantic.vector_pack";

/// Setting with the address of an OpenAI-compatible server (JSON string),
/// e.g. `http://localhost:11434/v1`.
pub const BASE_URL_SETTING: &str = "semantic.base_url";

/// Why semantic features are, or are not, available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SemanticStatus {
    /// No vector pack is installed.
    NoVectorPack,
    /// The metadata of `pack` is not valid.
    Invalid {
        /// The vector or model pack with the bad metadata.
        pack: String,
    },
    /// The model pack the local vector pack needs is not installed.
    ModelPackMissing {
        /// The vector pack.
        pack: String,
        /// The model pack to install.
        model_pack: String,
    },
    /// The model pack embeds into a different space than the vectors.
    Mismatch {
        /// The vector pack.
        pack: String,
        /// The installed model pack.
        model_pack: String,
    },
    /// The provider's API key is not stored.
    KeyMissing {
        /// The vector pack.
        pack: String,
        /// The provider whose key is needed.
        provider: ApiProvider,
    },
    /// The address of the OpenAI-compatible server is not entered.
    BaseUrlMissing {
        /// The vector pack.
        pack: String,
    },
    /// The entered address is not usable: not HTTP(S), plain HTTP to another
    /// computer, or with credentials in it (spec §14.1).
    BaseUrlInvalid {
        /// The vector pack.
        pack: String,
    },
    /// This app version cannot run the pack's backend yet.
    NotBuilt {
        /// The vector pack.
        pack: String,
        /// `local`, `api` or `dev`.
        backend: String,
    },
    /// Semantic features work.
    Ready {
        /// The vector pack.
        pack: String,
        /// The provider queries are sent to; `None` if they stay on this computer.
        provider: Option<ApiProvider>,
    },
}

/// The active vector pack, with everything an embedder needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivePack {
    /// Id of the vector pack.
    pub id: String,
    /// Installed version of the vector pack.
    pub version: String,
    /// Its metadata.
    pub spec: VectorPackSpec,
    /// The model pack's id, version and metadata (local backend).
    pub model: Option<(String, String, ModelPackSpec)>,
    /// Address of the OpenAI-compatible server.
    pub base_url: Option<String>,
}

impl ActivePack {
    /// Identifies everything the embedder was built from: a new version of
    /// either pack or a new address needs a new embedder.
    fn cache_key(&self) -> String {
        let model = self
            .model
            .as_ref()
            .map(|(id, version, _)| format!("{id}@{version}"));
        format!(
            "{}@{}|{}|{}",
            self.id,
            self.version,
            model.unwrap_or_default(),
            self.base_url.as_deref().unwrap_or_default()
        )
    }
}

/// Builds embedders for a backend.
pub trait EmbedderFactory: Send + Sync {
    /// Whether this build can run `backend`.
    fn supports(&self, backend: &Backend) -> bool;

    /// Builds the embedder of `pack`; called once per pack version, possibly
    /// slow (a local model is loaded here).
    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>>;
}

/// The factory of this build: no backend is implemented yet.
pub struct UnbuiltFactory;

impl EmbedderFactory for UnbuiltFactory {
    fn supports(&self, _backend: &Backend) -> bool {
        false
    }

    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
        Err(unavailable(&pack.id, "no embedder in this build"))
    }
}

/// Several factories; the first that supports a backend builds its embedders.
pub struct Backends(pub Vec<Arc<dyn EmbedderFactory>>);

impl EmbedderFactory for Backends {
    fn supports(&self, backend: &Backend) -> bool {
        self.0.iter().any(|f| f.supports(backend))
    }

    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
        match self.0.iter().find(|f| f.supports(&pack.spec.backend)) {
            Some(factory) => factory.create(pack),
            None => UnbuiltFactory.create(pack),
        }
    }
}

/// The semantic service, shared by all commands.
pub struct Semantic {
    keys: Arc<dyn KeyStore>,
    factory: Arc<dyn EmbedderFactory>,
    /// The embedder of the last active pack, with its [`ActivePack::cache_key`].
    cache: Mutex<Option<(String, Arc<dyn Embedder>)>>,
    /// The vectors of the last active pack, keyed by `id@version`.
    index: Mutex<Option<(String, Arc<VectorIndex>)>>,
}

impl Semantic {
    /// Creates the service.
    pub fn new(keys: Arc<dyn KeyStore>, factory: Arc<dyn EmbedderFactory>) -> Self {
        Self {
            keys,
            factory,
            cache: Mutex::new(None),
            index: Mutex::new(None),
        }
    }

    /// The service of this build: no keys, no backends.
    pub fn unbuilt() -> Self {
        Self::new(Arc::new(NoKeyStore), Arc::new(UnbuiltFactory))
    }

    /// Where API keys are kept.
    pub fn keys(&self) -> &Arc<dyn KeyStore> {
        &self.keys
    }

    /// Whether semantic features are available, and if not, why.
    pub fn status(
        &self,
        packs: &[InstalledPack],
        settings: &Settings,
    ) -> AppResult<SemanticStatus> {
        Ok(match self.resolve(packs, settings)? {
            Ok(pack) => SemanticStatus::Ready {
                provider: match pack.spec.backend {
                    Backend::Local { .. } | Backend::Dev => None,
                    Backend::Api { provider, .. } => Some(provider),
                },
                pack: pack.id,
            },
            Err(status) => status,
        })
    }

    /// The vectors of the active vector pack, loaded from `content` on first
    /// use and kept until the pack's version changes. Needs no embedder:
    /// "verses like this one" compares verse vectors only (spec §7.6).
    pub fn index(
        &self,
        content: &Connection,
        packs: &[InstalledPack],
        settings: &Settings,
    ) -> AppResult<Arc<VectorIndex>> {
        let Some(pack) = active_vector_pack(packs, settings) else {
            return Err(unavailable("-", "no vector pack is installed"));
        };
        let spec = VectorPackSpec::from_meta(&meta(pack))
            .map_err(|message| unavailable(&pack.id, &message))?;
        let key = format!("{}@{}", pack.id, pack.version);
        let mut cache = self
            .index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((cached, index)) = cache.as_ref()
            && *cached == key
        {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(VectorIndex::load(content, &pack.id, &spec.space)?);
        *cache = Some((key, Arc::clone(&index)));
        Ok(index)
    }

    /// The embedder of the active vector pack, built on first use and kept
    /// until either pack or the server address changes.
    pub fn embedder(
        &self,
        packs: &[InstalledPack],
        settings: &Settings,
    ) -> AppResult<Arc<dyn Embedder>> {
        let pack = match self.resolve(packs, settings)? {
            Ok(pack) => pack,
            Err(status) => {
                return Err(AppError::new(
                    AppErrorCode::SemanticUnavailable,
                    format!("{status:?}"),
                ));
            }
        };
        let key = pack.cache_key();
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((cached, embedder)) = cache.as_ref()
            && *cached == key
        {
            return Ok(Arc::clone(embedder));
        }
        let embedder = self.factory.create(&pack)?;
        if !embedder.space().matches(&pack.spec.space) {
            return Err(unavailable(
                &pack.id,
                &format!(
                    "the embedder produces {} ({}), the pack needs {} ({})",
                    embedder.space().model,
                    embedder.space().dim,
                    pack.spec.space.model,
                    pack.spec.space.dim
                ),
            ));
        }
        *cache = Some((key, Arc::clone(&embedder)));
        Ok(embedder)
    }

    /// The active pack, or the status that explains why there is none.
    fn resolve(
        &self,
        packs: &[InstalledPack],
        settings: &Settings,
    ) -> AppResult<Result<ActivePack, SemanticStatus>> {
        let Some(vectors) = active_vector_pack(packs, settings) else {
            return Ok(Err(SemanticStatus::NoVectorPack));
        };
        let pack = vectors.id.clone();
        let Ok(spec) = VectorPackSpec::from_meta(&meta(vectors)) else {
            return Ok(Err(SemanticStatus::Invalid { pack }));
        };
        let mut model = None;
        let mut base_url = None;
        match &spec.backend {
            Backend::Local { model_pack } => {
                let Some(installed) = packs
                    .iter()
                    .find(|p| p.id == *model_pack && p.kind == "model")
                else {
                    return Ok(Err(SemanticStatus::ModelPackMissing {
                        pack,
                        model_pack: model_pack.clone(),
                    }));
                };
                let Ok(model_spec) = ModelPackSpec::from_meta(&meta(installed)) else {
                    return Ok(Err(SemanticStatus::Invalid {
                        pack: installed.id.clone(),
                    }));
                };
                if spec.check_model(&model_spec).is_err() {
                    return Ok(Err(SemanticStatus::Mismatch {
                        pack,
                        model_pack: model_pack.clone(),
                    }));
                }
                model = Some((installed.id.clone(), installed.version.clone(), model_spec));
            }
            Backend::Api { provider, .. } => {
                if provider.needs_base_url() {
                    base_url = string_setting(settings, BASE_URL_SETTING);
                    let Some(address) = &base_url else {
                        return Ok(Err(SemanticStatus::BaseUrlMissing { pack }));
                    };
                    if crate::net::user_server_url(address).is_err() {
                        return Ok(Err(SemanticStatus::BaseUrlInvalid { pack }));
                    }
                }
                if provider.needs_key() && !self.keys.has_key(*provider)? {
                    return Ok(Err(SemanticStatus::KeyMissing {
                        pack,
                        provider: *provider,
                    }));
                }
            }
            Backend::Dev => {}
        }
        if !self.factory.supports(&spec.backend) {
            return Ok(Err(SemanticStatus::NotBuilt {
                pack,
                backend: spec.backend.as_str().to_owned(),
            }));
        }
        Ok(Ok(ActivePack {
            id: pack,
            version: vectors.version.clone(),
            spec,
            model,
            base_url,
        }))
    }
}

/// The vector pack named by the setting if it is installed, else the first
/// installed one (`packs` is ordered by id).
fn active_vector_pack<'a>(
    packs: &'a [InstalledPack],
    settings: &Settings,
) -> Option<&'a InstalledPack> {
    let mut vectors = packs.iter().filter(|p| p.kind == "vectors");
    let chosen = string_setting(settings, VECTOR_PACK_SETTING);
    match chosen {
        Some(id) => vectors
            .clone()
            .find(|p| p.id == id)
            .or_else(|| vectors.next()),
        None => vectors.next(),
    }
}

fn meta(pack: &InstalledPack) -> serde_json::Map<String, serde_json::Value> {
    pack.meta_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

/// A setting holding a non-empty JSON string.
fn string_setting(settings: &Settings, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(|json| serde_json::from_str::<String>(json).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn unavailable(pack: &str, message: &str) -> AppError {
    AppError::new(
        AppErrorCode::SemanticUnavailable,
        format!("{pack}: {message}"),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::db::content::LocalizedTitle;
    use crate::embed::fake::FakeEmbedder;
    use crate::embed::{EmbeddingSpace, Locality, TextKind};

    fn installed(id: &str, kind: &str, version: &str, meta: serde_json::Value) -> InstalledPack {
        InstalledPack {
            id: id.into(),
            kind: kind.into(),
            version: version.into(),
            bundled: false,
            title: id.into(),
            titles: LocalizedTitle { tr: None, en: None },
            lang: None,
            author: None,
            license: "x".into(),
            attribution: None,
            citation_json: None,
            meta_json: Some(meta.to_string()),
        }
    }

    fn local_vectors(id: &str) -> InstalledPack {
        installed(
            id,
            "vectors",
            "1.0.0",
            serde_json::json!({
                "embedding_model": "local/e5-small",
                "dim": 8,
                "backend": "local",
                "model_pack": "model.e5-small"
            }),
        )
    }

    fn model_pack(dim: u32) -> InstalledPack {
        installed(
            "model.e5-small",
            "model",
            "1.0.0",
            serde_json::json!({ "embedding_model": "local/e5-small", "dim": dim }),
        )
    }

    fn api_vectors(id: &str, provider: ApiProvider) -> InstalledPack {
        installed(
            id,
            "vectors",
            "1.0.0",
            serde_json::json!({
                "embedding_model": format!("{}model", provider.model_prefix()),
                "dim": 8,
                "backend": "api",
                "provider": provider.as_str(),
                "api_model": "model"
            }),
        )
    }

    fn translation() -> InstalledPack {
        installed(
            "en.pickthall",
            "translation",
            "1.0.0",
            serde_json::json!({}),
        )
    }

    struct Keys(HashSet<ApiProvider>);

    impl KeyStore for Keys {
        fn key(&self, provider: ApiProvider) -> AppResult<Option<String>> {
            Ok(self.0.contains(&provider).then(|| "key".to_owned()))
        }

        fn set_key(&self, _provider: ApiProvider, _key: &str) -> AppResult<()> {
            unreachable!("status checks never save keys")
        }

        fn delete_key(&self, _provider: ApiProvider) -> AppResult<()> {
            unreachable!("status checks never delete keys")
        }
    }

    /// Builds fake embedders and counts how often.
    struct Factory {
        created: AtomicUsize,
        /// The dimension the built embedders produce.
        dim: u32,
    }

    impl EmbedderFactory for Factory {
        fn supports(&self, _backend: &Backend) -> bool {
            true
        }

        fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
            self.created.fetch_add(1, Ordering::SeqCst);
            let mut embedder = FakeEmbedder::new(EmbeddingSpace {
                dim: self.dim,
                ..pack.spec.space.clone()
            });
            if let Backend::Api { provider, .. } = pack.spec.backend {
                embedder.locality = Locality::Remote(provider);
            }
            Ok(Arc::new(embedder))
        }
    }

    fn service(keys: &[ApiProvider], dim: u32) -> (Semantic, Arc<Factory>) {
        let factory = Arc::new(Factory {
            created: AtomicUsize::new(0),
            dim,
        });
        let semantic = Semantic::new(
            Arc::new(Keys(keys.iter().copied().collect())),
            Arc::clone(&factory) as Arc<dyn EmbedderFactory>,
        );
        (semantic, factory)
    }

    fn settings(values: &[(&str, &str)]) -> Settings {
        values
            .iter()
            .map(|(k, v)| ((*k).to_owned(), serde_json::to_string(v).unwrap()))
            .collect()
    }

    #[test]
    fn without_a_vector_pack_nothing_is_available() {
        let (semantic, _) = service(&[], 8);
        let status = semantic.status(&[translation()], &Settings::new()).unwrap();
        assert_eq!(status, SemanticStatus::NoVectorPack);
        let err = semantic
            .embedder(&[translation()], &Settings::new())
            .err()
            .unwrap();
        assert_eq!(err.code, AppErrorCode::SemanticUnavailable);
    }

    #[test]
    fn a_local_pack_needs_its_model_pack_in_the_same_space() {
        let (semantic, _) = service(&[], 8);
        let none = Settings::new();
        assert_eq!(
            semantic
                .status(&[local_vectors("vectors.e5")], &none)
                .unwrap(),
            SemanticStatus::ModelPackMissing {
                pack: "vectors.e5".into(),
                model_pack: "model.e5-small".into(),
            }
        );
        assert_eq!(
            semantic
                .status(&[local_vectors("vectors.e5"), model_pack(16)], &none)
                .unwrap(),
            SemanticStatus::Mismatch {
                pack: "vectors.e5".into(),
                model_pack: "model.e5-small".into(),
            }
        );
        assert_eq!(
            semantic
                .status(&[local_vectors("vectors.e5"), model_pack(8)], &none)
                .unwrap(),
            SemanticStatus::Ready {
                pack: "vectors.e5".into(),
                provider: None,
            }
        );
    }

    #[test]
    fn a_model_pack_of_another_type_does_not_count() {
        let (semantic, _) = service(&[], 8);
        let mut wrong = model_pack(8);
        wrong.kind = "translation".into();
        assert!(matches!(
            semantic
                .status(&[local_vectors("vectors.e5"), wrong], &Settings::new())
                .unwrap(),
            SemanticStatus::ModelPackMissing { .. }
        ));
    }

    #[test]
    fn invalid_metadata_names_the_pack() {
        let (semantic, _) = service(&[], 8);
        let broken = installed(
            "vectors.x",
            "vectors",
            "1.0.0",
            serde_json::json!({ "dim": 8 }),
        );
        assert_eq!(
            semantic.status(&[broken], &Settings::new()).unwrap(),
            SemanticStatus::Invalid {
                pack: "vectors.x".into()
            }
        );
        let broken_model = installed(
            "model.e5-small",
            "model",
            "1.0.0",
            serde_json::json!({ "embedding_model": "local/e5-small" }),
        );
        assert_eq!(
            semantic
                .status(
                    &[local_vectors("vectors.e5"), broken_model],
                    &Settings::new()
                )
                .unwrap(),
            SemanticStatus::Invalid {
                pack: "model.e5-small".into()
            }
        );
    }

    #[test]
    fn an_api_pack_needs_the_key_of_its_provider() {
        let packs = [api_vectors("vectors.openai", ApiProvider::OpenAi)];
        let (semantic, _) = service(&[ApiProvider::Voyage], 8);
        assert_eq!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::KeyMissing {
                pack: "vectors.openai".into(),
                provider: ApiProvider::OpenAi,
            }
        );
        let (semantic, _) = service(&[ApiProvider::OpenAi], 8);
        assert_eq!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::Ready {
                pack: "vectors.openai".into(),
                provider: Some(ApiProvider::OpenAi),
            }
        );
    }

    #[test]
    fn an_openai_compatible_pack_needs_an_address_but_no_key() {
        let packs = [api_vectors("vectors.ollama", ApiProvider::OpenAiCompatible)];
        let (semantic, _) = service(&[], 8);
        assert_eq!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::BaseUrlMissing {
                pack: "vectors.ollama".into()
            }
        );
        let blank = settings(&[(BASE_URL_SETTING, "  ")]);
        assert!(matches!(
            semantic.status(&packs, &blank).unwrap(),
            SemanticStatus::BaseUrlMissing { .. }
        ));
        for bad in ["localhost:11434", "http://192.168.1.5:11434/v1"] {
            let address = settings(&[(BASE_URL_SETTING, bad)]);
            assert_eq!(
                semantic.status(&packs, &address).unwrap(),
                SemanticStatus::BaseUrlInvalid {
                    pack: "vectors.ollama".into()
                },
                "{bad}"
            );
        }
        let address = settings(&[(BASE_URL_SETTING, "http://localhost:11434/v1")]);
        assert!(matches!(
            semantic.status(&packs, &address).unwrap(),
            SemanticStatus::Ready { .. }
        ));
    }

    #[test]
    fn backends_pick_the_factory_that_supports_the_pack() {
        let packs = [api_vectors("vectors.openai", ApiProvider::OpenAi)];
        let factory = Factory {
            created: AtomicUsize::new(0),
            dim: 8,
        };
        let backends = Backends(vec![Arc::new(UnbuiltFactory), Arc::new(factory)]);
        let semantic = Semantic::new(
            Arc::new(Keys([ApiProvider::OpenAi].into())),
            Arc::new(backends),
        );
        assert!(matches!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::Ready { .. }
        ));
        assert!(semantic.embedder(&packs, &Settings::new()).is_ok());
        let none = Semantic::new(Arc::new(NoKeyStore), Arc::new(Backends(Vec::new())));
        assert!(matches!(
            none.status(&[local_vectors("vectors.local")], &Settings::new())
                .unwrap(),
            SemanticStatus::ModelPackMissing { .. }
        ));
    }

    #[test]
    fn the_setting_picks_the_active_pack() {
        let packs = [
            api_vectors("vectors.a", ApiProvider::OpenAi),
            api_vectors("vectors.b", ApiProvider::Voyage),
            translation(),
        ];
        let (semantic, _) = service(&[ApiProvider::OpenAi, ApiProvider::Voyage], 8);
        let ready = |pack: &str, provider| SemanticStatus::Ready {
            pack: pack.into(),
            provider: Some(provider),
        };
        assert_eq!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            ready("vectors.a", ApiProvider::OpenAi)
        );
        let chosen = settings(&[(VECTOR_PACK_SETTING, "vectors.b")]);
        assert_eq!(
            semantic.status(&packs, &chosen).unwrap(),
            ready("vectors.b", ApiProvider::Voyage)
        );
        // A removed or non-vector pack falls back to the first installed one.
        for stale in ["vectors.gone", "en.pickthall"] {
            let chosen = settings(&[(VECTOR_PACK_SETTING, stale)]);
            assert_eq!(
                semantic.status(&packs, &chosen).unwrap(),
                ready("vectors.a", ApiProvider::OpenAi)
            );
        }
    }

    #[test]
    fn this_build_has_no_backend_yet() {
        let semantic = Semantic::unbuilt();
        let packs = [local_vectors("vectors.e5"), model_pack(8)];
        assert_eq!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::NotBuilt {
                pack: "vectors.e5".into(),
                backend: "local".into(),
            }
        );
        // Without a stored key the key is reported first; it is fixable now.
        let packs = [api_vectors("vectors.voyage", ApiProvider::Voyage)];
        assert!(matches!(
            semantic.status(&packs, &Settings::new()).unwrap(),
            SemanticStatus::KeyMissing { .. }
        ));
    }

    #[tokio::test]
    async fn the_embedder_is_built_once_per_pack_version() {
        let (semantic, factory) = service(&[ApiProvider::OpenAi], 8);
        let mut packs = vec![api_vectors("vectors.openai", ApiProvider::OpenAi)];
        let none = Settings::new();

        let embedder = semantic.embedder(&packs, &none).unwrap();
        assert_eq!(embedder.locality(), Locality::Remote(ApiProvider::OpenAi));
        let vectors = embedder
            .embed(vec!["sabır".into()], TextKind::Query)
            .await
            .unwrap();
        assert_eq!(vectors[0].len(), 8);
        semantic.embedder(&packs, &none).unwrap();
        assert_eq!(factory.created.load(Ordering::SeqCst), 1);

        packs[0].version = "1.1.0".into();
        semantic.embedder(&packs, &none).unwrap();
        assert_eq!(factory.created.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_embedder_in_another_space_is_refused() {
        let (semantic, _) = service(&[], 16);
        let packs = [local_vectors("vectors.e5"), model_pack(8)];
        let err = semantic.embedder(&packs, &Settings::new()).err().unwrap();
        assert_eq!(err.code, AppErrorCode::SemanticUnavailable);
    }

    #[test]
    fn the_index_is_loaded_once_per_pack_version() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             CREATE TABLE packs (id TEXT PRIMARY KEY);
             CREATE TABLE verses (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        crate::db::content::ensure_vector_table(&conn).unwrap();
        let blob: Vec<u8> = [1.0_f32; 8].iter().flat_map(|x| x.to_le_bytes()).collect();
        conn.execute(
            "INSERT INTO verse_vectors VALUES ('vectors.e5', 1, ?1)",
            [&blob],
        )
        .unwrap();
        // No embedder is needed: this build has none.
        let semantic = Semantic::unbuilt();
        let mut packs = vec![local_vectors("vectors.e5")];
        let none = Settings::new();
        let index = semantic.index(&conn, &packs, &none).unwrap();
        assert_eq!((index.len(), index.dim()), (1, 8));

        // Cached: a second vector is not seen until the version changes.
        conn.execute(
            "INSERT INTO verse_vectors VALUES ('vectors.e5', 2, ?1)",
            [&blob],
        )
        .unwrap();
        assert_eq!(semantic.index(&conn, &packs, &none).unwrap().len(), 1);
        packs[0].version = "1.1.0".into();
        assert_eq!(semantic.index(&conn, &packs, &none).unwrap().len(), 2);

        let err = semantic.index(&conn, &[translation()], &none).unwrap_err();
        assert_eq!(err.code, AppErrorCode::SemanticUnavailable);
    }

    #[test]
    fn status_serializes_with_a_state_tag() {
        let status = SemanticStatus::KeyMissing {
            pack: "vectors.openai".into(),
            provider: ApiProvider::OpenAi,
        };
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "state": "key_missing",
                "pack": "vectors.openai",
                "provider": "openai"
            })
        );
    }
}
