//! The local backend: ONNX Runtime (`ort`) + `tokenizers`, configured from the
//! model pack's metadata ([`ModelPackSpec`]: prefixes, pooling, token limit).
//!
//! A model pack installs `model/model.onnx`, `model/tokenizer.json` and the
//! model's `model/LICENSE` under
//! `<app_data>/packs/<id>/` (spec §6.1). The model is loaded when semantic
//! search is first used; inference runs on a blocking thread (spec §7.4).
//!
//! The model's inputs are fed by name: `input_ids` and `attention_mask`, and
//! `token_type_ids` (zeros) or `position_ids` when the model has them. Its
//! first output is either the token vectors (`[batch, tokens, dim]`, pooled
//! here) or already one vector per text (`[batch, dim]`). The pipeline that
//! builds the vector pack runs the same files, so queries and verses share
//! one space.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ort::environment::Environment;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use tokenizers::{PaddingStrategy, Tokenizer, TruncationParams};

use super::service::{ActivePack, EmbedderFactory};
use super::space::{Backend, ModelPackSpec, Pooling};
use super::{EmbedFuture, Embedder, EmbeddingSpace, Locality, TextKind};
use crate::error::{AppError, AppErrorCode, AppResult};

/// Texts run through the model at once; bounds memory for long batches.
const BATCH: usize = 16;

/// The directory a model pack's files are installed in.
pub fn model_dir(packs_dir: &Path, model_pack: &str) -> PathBuf {
    packs_dir.join(model_pack).join("model")
}

/// Builds [`OnnxEmbedder`]s for local vector packs.
pub struct OnnxFactory {
    packs_dir: PathBuf,
}

impl OnnxFactory {
    /// Factory reading model packs from `packs_dir` (`<app_data>/packs`).
    pub fn new(packs_dir: PathBuf) -> Self {
        Self { packs_dir }
    }
}

impl EmbedderFactory for OnnxFactory {
    fn supports(&self, backend: &Backend) -> bool {
        matches!(backend, Backend::Local { .. })
    }

    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
        let Some((model_pack, _, spec)) = &pack.model else {
            return Err(unavailable(format!("{}: no model pack", pack.id)));
        };
        let embedder = OnnxEmbedder::load(&model_dir(&self.packs_dir, model_pack), spec.clone())?;
        Ok(Arc::new(embedder))
    }
}

/// The process's ONNX Runtime environment, created on first use and shared
/// by every model.
fn environment() -> AppResult<Environment> {
    static ENV: Mutex<Option<Environment>> = Mutex::new(None);
    let mut env = ENV
        .lock()
        .map_err(|_| unavailable("ONNX Runtime failed earlier".into()))?;
    if let Some(env) = env.as_ref() {
        return Ok(env.clone());
    }
    let created = ort::init()
        .with_name("siyak")
        .build()
        .map_err(|e| unavailable(format!("ONNX Runtime: {e}")))?;
    *env = Some(created.clone());
    Ok(created)
}

/// A local ONNX embedding model.
pub struct OnnxEmbedder {
    space: EmbeddingSpace,
    inner: Arc<Inner>,
}

struct Inner {
    spec: ModelPackSpec,
    tokenizer: Tokenizer,
    /// `Session::run` needs `&mut`; queries are short, so one at a time is fine.
    session: Mutex<Session>,
    _env: Environment,
    token_type_ids: bool,
    position_ids: bool,
}

impl OnnxEmbedder {
    /// Loads `model.onnx` and `tokenizer.json` from `dir`.
    pub fn load(dir: &Path, spec: ModelPackSpec) -> AppResult<Self> {
        let mut tokenizer = Tokenizer::from_file(dir.join("tokenizer.json"))
            .map_err(|e| unavailable(format!("tokenizer.json: {e}")))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: spec.max_tokens as usize,
                ..TruncationParams::default()
            }))
            .map_err(|e| unavailable(format!("tokenizer.json: {e}")))?;
        // Keep the pad token the pipeline wrote; pad each batch to its longest text.
        let mut padding = tokenizer.get_padding().cloned().unwrap_or_default();
        padding.strategy = PaddingStrategy::BatchLongest;
        tokenizer.with_padding(Some(padding));

        let threads = std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).clamp(1, 4));
        let env = environment()?;
        let model_error = |e: &dyn std::fmt::Display| unavailable(format!("model.onnx: {e}"));
        let session = Session::builder(&env)
            .map_err(|e| model_error(&e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| model_error(&e))?
            .with_intra_threads(threads)
            .map_err(|e| model_error(&e))?
            .commit_from_file(dir.join("model.onnx"))
            .map_err(|e| model_error(&e))?;
        let has = |name: &str| session.inputs().iter().any(|i| i.name() == name);
        if !has("input_ids") || !has("attention_mask") {
            return Err(unavailable(
                "model.onnx needs input_ids and attention_mask inputs".into(),
            ));
        }
        let (token_type_ids, position_ids) = (has("token_type_ids"), has("position_ids"));
        Ok(Self {
            space: spec.space.clone(),
            inner: Arc::new(Inner {
                spec,
                tokenizer,
                session: Mutex::new(session),
                _env: env,
                token_type_ids,
                position_ids,
            }),
        })
    }
}

impl Embedder for OnnxEmbedder {
    fn space(&self) -> &EmbeddingSpace {
        &self.space
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            tokio::task::spawn_blocking(move || inner.embed(texts, kind))
                .await
                .map_err(|e| unavailable(format!("embedding failed: {e}")))?
        })
    }
}

impl Inner {
    fn embed(&self, texts: Vec<String>, kind: TextKind) -> AppResult<Vec<Vec<f32>>> {
        let prefix = match kind {
            TextKind::Query => &self.spec.query_prefix,
            TextKind::Passage => &self.spec.passage_prefix,
        };
        let texts: Vec<String> = texts.into_iter().map(|t| format!("{prefix}{t}")).collect();
        let mut out = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(BATCH) {
            out.extend(self.embed_batch(chunk.to_vec())?);
        }
        Ok(out)
    }

    fn embed_batch(&self, texts: Vec<String>) -> AppResult<Vec<Vec<f32>>> {
        let encodings = self
            .tokenizer
            .encode_batch(texts, true)
            .map_err(|e| unavailable(format!("tokenizing failed: {e}")))?;
        let batch = encodings.len();
        let tokens = encodings.first().map_or(0, |e| e.get_ids().len());
        if batch == 0 || tokens == 0 {
            return Ok(vec![Vec::new(); batch]);
        }
        let flat = |f: fn(&tokenizers::Encoding) -> &[u32]| -> Vec<i64> {
            encodings
                .iter()
                .flat_map(|e| f(e).iter().map(|&x| i64::from(x)))
                .collect()
        };
        let ids = flat(tokenizers::Encoding::get_ids);
        let mask = flat(tokenizers::Encoding::get_attention_mask);
        let shape = [batch, tokens];
        let tensor = |data: Vec<i64>| {
            Tensor::from_array((shape, data)).map_err(|e| unavailable(format!("{e}")))
        };
        let mut inputs = vec![
            ("input_ids", tensor(ids)?.into_dyn()),
            ("attention_mask", tensor(mask.clone())?.into_dyn()),
        ];
        if self.token_type_ids {
            inputs.push((
                "token_type_ids",
                tensor(vec![0; batch * tokens])?.into_dyn(),
            ));
        }
        if self.position_ids {
            inputs.push((
                "position_ids",
                tensor(position_ids(&mask, tokens))?.into_dyn(),
            ));
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| unavailable("the model failed earlier".into()))?;
        let outputs = session
            .run(inputs)
            .map_err(|e| unavailable(format!("running the model failed: {e}")))?;
        let (out_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| unavailable(format!("unexpected model output: {e}")))?;
        let dims: Vec<usize> = out_shape.iter().map(|&d| d.max(0) as usize).collect();
        let dim = self.spec.space.dim as usize;
        let mut vectors = match dims[..] {
            [b, t, d] if b == batch && t == tokens && d == dim => {
                pool(data, &mask, batch, tokens, dim, self.spec.pooling)
            }
            [b, d] if b == batch && d == dim => {
                data.chunks_exact(dim).map(<[f32]>::to_vec).collect()
            }
            _ => {
                return Err(unavailable(format!(
                    "the model's output is {dims:?}, expected [{batch}, {tokens}, {dim}] or [{batch}, {dim}]"
                )));
            }
        };
        if self.spec.space.normalized {
            vectors.iter_mut().for_each(|v| normalize(v));
        }
        Ok(vectors)
    }
}

/// Positions of the tokens that are not padding, counting from 0 (padding
/// gets 0); decoder models with a `position_ids` input need them.
fn position_ids(mask: &[i64], tokens: usize) -> Vec<i64> {
    mask.chunks_exact(tokens)
        .flat_map(|row| {
            let mut next = 0;
            row.iter().map(move |&m| {
                if m == 0 {
                    0
                } else {
                    next += 1;
                    next - 1
                }
            })
        })
        .collect()
}

/// Pools `[batch, tokens, dim]` token vectors into one vector per text.
fn pool(
    data: &[f32],
    mask: &[i64],
    batch: usize,
    tokens: usize,
    dim: usize,
    pooling: Pooling,
) -> Vec<Vec<f32>> {
    (0..batch)
        .map(|b| {
            let row = |t: usize| &data[(b * tokens + t) * dim..(b * tokens + t + 1) * dim];
            let used = |t: usize| mask[b * tokens + t] != 0;
            match pooling {
                Pooling::Cls => row(0).to_vec(),
                Pooling::Last => (0..tokens)
                    .rev()
                    .find(|&t| used(t))
                    .map_or_else(|| vec![0.0; dim], |t| row(t).to_vec()),
                Pooling::Mean => {
                    let mut sum = vec![0.0_f32; dim];
                    let mut count = 0.0_f32;
                    for t in (0..tokens).filter(|&t| used(t)) {
                        sum.iter_mut().zip(row(t)).for_each(|(s, x)| *s += x);
                        count += 1.0;
                    }
                    if count > 0.0 {
                        sum.iter_mut().for_each(|s| *s /= count);
                    }
                    sum
                }
            }
        })
        .collect()
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|x| *x /= norm);
    }
}

fn unavailable(message: String) -> AppError {
    AppError::new(AppErrorCode::SemanticUnavailable, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two texts, three tokens, two dimensions; the second text has one pad token.
    const DATA: [f32; 12] = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
    ];
    const MASK: [i64; 6] = [1, 1, 1, 1, 1, 0];

    #[test]
    fn pools_token_vectors() {
        let mean = pool(&DATA, &MASK, 2, 3, 2, Pooling::Mean);
        assert_eq!(mean, [vec![3.0, 4.0], vec![8.0, 9.0]]);
        let cls = pool(&DATA, &MASK, 2, 3, 2, Pooling::Cls);
        assert_eq!(cls, [vec![1.0, 2.0], vec![7.0, 8.0]]);
        let last = pool(&DATA, &MASK, 2, 3, 2, Pooling::Last);
        assert_eq!(last, [vec![5.0, 6.0], vec![9.0, 10.0]]);
        let none = pool(&DATA, &[0; 6], 2, 3, 2, Pooling::Last);
        assert_eq!(none, [vec![0.0, 0.0], vec![0.0, 0.0]]);
    }

    #[test]
    fn numbers_positions_without_padding() {
        assert_eq!(position_ids(&MASK, 3), [0, 1, 2, 0, 1, 0]);
        assert_eq!(position_ids(&[0, 1, 1], 3), [0, 0, 1]);
    }

    /// Runs a real model pack against vectors the pipeline wrote:
    /// `SIYAK_TEST_MODEL_DIR=<dir with model/ and reference.json>`, where
    /// `reference.json` holds `{"spec": {model pack meta}, "texts": [...],
    /// "kind": "query"|"passage", "vectors": [[...]]}`.
    #[tokio::test]
    #[ignore = "needs a model pack (SIYAK_TEST_MODEL_DIR)"]
    async fn matches_the_pipeline() {
        let dir = PathBuf::from(std::env::var("SIYAK_TEST_MODEL_DIR").unwrap());
        let reference: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("reference.json")).unwrap()).unwrap();
        let spec = ModelPackSpec::from_meta(reference["spec"].as_object().unwrap()).unwrap();
        let embedder = OnnxEmbedder::load(&dir.join("model"), spec).unwrap();
        let texts: Vec<String> = serde_json::from_value(reference["texts"].clone()).unwrap();
        let kind = if reference["kind"] == "query" {
            TextKind::Query
        } else {
            TextKind::Passage
        };
        let expected: Vec<Vec<f32>> = serde_json::from_value(reference["vectors"].clone()).unwrap();
        let actual = embedder.embed(texts, kind).await.unwrap();
        assert_eq!(actual.len(), expected.len());
        let cosines: Vec<f32> = actual
            .iter()
            .zip(&expected)
            .map(|(a, e)| a.iter().zip(e).map(|(x, y)| x * y).sum())
            .collect();
        // A float32 model agrees to > 0.9999. The int8 kernels of ONNX Runtime
        // builds (the Python wheel, this static build, other CPUs) differ: about
        // 0.998 for e5-small and 0.975 for bge-m3, as far as the float32 model is
        // from its own int8 version. Rankings on the evaluation set do not change.
        assert!(cosines.iter().all(|&c| c > 0.97), "cosines {cosines:?}");
    }

    /// The local backend end to end: the e5-small packs built in `siyak-data`
    /// (listed by `siyak-data manifest --unpublished`), installed into the real
    /// `core.db`, answer a Turkish query. Skipped when they are not built.
    #[tokio::test]
    async fn answers_a_query_with_real_packs_when_present() {
        use crate::embed::service::{NoKeyStore, Semantic, SemanticStatus};
        use crate::packs::install::{install_file, open_content, packs_dir};
        use crate::packs::manifest::{installed_packs, parse};

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist = root.join("../../siyak-data/dist");
        let Ok(bytes) = std::fs::read(dist.join("manifest.json")) else {
            return;
        };
        let manifest = parse(&bytes).unwrap();
        let (Some(model), Some(vectors)) = (
            manifest.pack("model.e5-small"),
            manifest.pack("vectors.e5-small"),
        ) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("content.db");
        std::fs::copy(root.join("resources/core.db"), &path).unwrap();
        let mut conn = open_content(&path).unwrap();
        for entry in [model, vectors] {
            install_file(
                &mut conn,
                &dist.join(entry.file_name()),
                entry,
                dir.path(),
                &tokio_util::sync::CancellationToken::new(),
                &|_, _, _| {},
            )
            .unwrap();
        }

        let packs = installed_packs(&conn).unwrap();
        let settings = crate::db::user::Settings::new();
        let semantic = Semantic::new(
            Arc::new(NoKeyStore),
            Arc::new(OnnxFactory::new(packs_dir(dir.path()))),
        );
        let status = semantic.status(&packs, &settings).unwrap();
        assert!(matches!(status, SemanticStatus::Ready { .. }), "{status:?}");
        let embedder = semantic.embedder(&packs, &settings).unwrap();
        let query = embedder
            .embed(
                vec!["Ramazan ayında oruç tutmanın farz kılınması".to_owned()],
                TextKind::Query,
            )
            .await
            .unwrap();
        let index = semantic.index(&conn, &packs, &settings).unwrap();
        let found: Vec<u32> = index
            .nearest(&query[0], 10, &[])
            .iter()
            .map(|n| n.verse_id)
            .collect();
        // 2:183-185 (verses 190-192) prescribe the fast of Ramadan.
        assert!(found.iter().any(|v| (190..=192).contains(v)), "{found:?}");
    }
}
