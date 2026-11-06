//! A deterministic embedder for tests and the development database: the same
//! text always gives the same unit vector, different texts almost always
//! different ones. The vectors carry no meaning.
//!
//! Debug builds started with `SIYAK_DEV_DATA=dev` run [`DevEmbedder`] for the
//! `dev` backend ([`DevFactory`]), so the small development database has a
//! vector pack that needs no download (spec §16.3); `cargo xtask dev-db` fills
//! that pack with [`words_vector`]. It hashes each word to one component, so
//! a query is closest to the verses that share its words: enough to try semantic
//! search, related verses and chat, not to judge their quality.

use std::sync::Arc;

use super::service::{ActivePack, EmbedderFactory};
use super::{Backend, EmbedFuture, Embedder, EmbeddingSpace, Locality, TextKind};
use crate::error::AppResult;

/// Embeds texts by hashing them; needs no model or network.
#[cfg(test)]
pub(crate) struct FakeEmbedder {
    pub(crate) space: EmbeddingSpace,
    pub(crate) locality: Locality,
}

#[cfg(test)]
impl FakeEmbedder {
    /// A local fake embedder for `space`.
    pub(crate) fn new(space: EmbeddingSpace) -> Self {
        Self {
            space,
            locality: Locality::Local,
        }
    }

    fn vector(&self, text: &str, kind: TextKind) -> Vec<f32> {
        hash_vector(&self.space, text, kind)
    }
}

/// The vector of `text` in `space`: a hash of the text and its kind spread over
/// `space.dim` components, unit length if the space is normalized.
pub fn hash_vector(space: &EmbeddingSpace, text: &str, kind: TextKind) -> Vec<f32> {
    let seed = text
        .bytes()
        .chain([kind as u8])
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let mut state = seed;
    let mut vector: Vec<f32> = (0..space.dim)
        .map(|_| {
            // xorshift64: cheap, deterministic, spread over the components.
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            f32::from(u16::try_from(state >> 48).unwrap_or(0)) / 32768.0 - 1.0
        })
        .collect();
    if space.normalized {
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vector.iter_mut().for_each(|x| *x /= norm);
        }
    }
    vector
}

/// The vector of `text` in `space` for the `dev` backend: each lowercased
/// word adds ±1 to one component chosen by its hash (feature hashing), so
/// texts without a shared word are orthogonal unless two words collide. Unit
/// length if the space is normalized; queries and passages are embedded alike.
/// Zero for a text without words.
pub fn words_vector(space: &EmbeddingSpace, text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; space.dim as usize];
    if vector.is_empty() {
        return vector;
    }
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let hash = word
            .to_lowercase()
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        let slot = usize::try_from(hash % u64::from(space.dim)).unwrap_or(0);
        vector[slot] += if hash >> 63 == 0 { 1.0 } else { -1.0 };
    }
    if space.normalized {
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vector.iter_mut().for_each(|x| *x /= norm);
        }
    }
    vector
}

/// The embedder of the `dev` backend: [`words_vector`], on this computer.
pub struct DevEmbedder {
    space: EmbeddingSpace,
}

impl Embedder for DevEmbedder {
    fn space(&self) -> &EmbeddingSpace {
        &self.space
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn embed(&self, texts: Vec<String>, _kind: TextKind) -> EmbedFuture<'_> {
        let vectors = texts.iter().map(|t| words_vector(&self.space, t)).collect();
        Box::pin(async move { Ok(vectors) })
    }
}

/// Builds [`DevEmbedder`]s for vector packs of the `dev` backend.
pub struct DevFactory;

impl EmbedderFactory for DevFactory {
    fn supports(&self, backend: &Backend) -> bool {
        matches!(backend, Backend::Dev)
    }

    fn create(&self, pack: &ActivePack) -> AppResult<Arc<dyn Embedder>> {
        Ok(Arc::new(DevEmbedder {
            space: pack.spec.space.clone(),
        }))
    }
}

#[cfg(test)]
impl Embedder for FakeEmbedder {
    fn space(&self) -> &EmbeddingSpace {
        &self.space
    }

    fn locality(&self) -> Locality {
        self.locality
    }

    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_> {
        let vectors = texts.iter().map(|t| self.vector(t, kind)).collect();
        Box::pin(async move { Ok(vectors) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embeds_deterministically_into_unit_vectors() {
        let embedder = FakeEmbedder::new(EmbeddingSpace {
            model: "local/fake".into(),
            dim: 16,
            normalized: true,
        });
        let texts = vec!["sabır".to_owned(), "rahmet".to_owned(), "sabır".to_owned()];
        let vectors = embedder.embed(texts, TextKind::Query).await.unwrap();
        assert_eq!(vectors.len(), 3);
        assert!(vectors.iter().all(|v| v.len() == 16));
        assert_eq!(vectors[0], vectors[2]);
        assert_ne!(vectors[0], vectors[1]);
        let norm: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);

        let passage = embedder
            .embed(vec!["sabır".to_owned()], TextKind::Passage)
            .await
            .unwrap();
        assert_ne!(passage[0], vectors[0]);
    }

    /// The development database's `vectors.dev` is ready with [`DevFactory`]
    /// alone, and a query finds the verses that share its words.
    #[tokio::test]
    async fn dev_database_vectors_answer_queries() {
        use crate::embed::index::VectorIndex;
        use crate::embed::service::{NoKeyStore, Semantic, SemanticStatus};
        use crate::packs::manifest::installed_packs;

        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-data/dev-content.db");
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let packs = installed_packs(&conn).unwrap();
        let settings = crate::db::user::Settings::new();
        let semantic = Semantic::new(Arc::new(NoKeyStore), Arc::new(DevFactory));
        assert_eq!(
            semantic.status(&packs, &settings).unwrap(),
            SemanticStatus::Ready {
                pack: "vectors.dev".into(),
                provider: None,
            }
        );
        let index: Arc<VectorIndex> = semantic.index(&conn, &packs, &settings).unwrap();
        assert_eq!(index.len(), 437);

        let embedder = semantic.embedder(&packs, &settings).unwrap();
        let query = embedder
            .embed(vec!["patience".into()], TextKind::Query)
            .await
            .unwrap();
        let best = index.nearest(&query[0], 1, &[])[0].verse_id;
        let text: String = conn
            .query_row(
                "SELECT text FROM translations WHERE verse_id = ?1",
                [best],
                |r| r.get(0),
            )
            .unwrap();
        assert!(text.to_lowercase().contains("patience"), "{text}");
    }

    /// Without [`DevFactory`] (release builds) the dev pack cannot run.
    #[test]
    fn dev_backend_is_not_built_elsewhere() {
        use crate::embed::service::{NoKeyStore, Semantic, SemanticStatus, UnbuiltFactory};
        use crate::packs::manifest::installed_packs;

        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-data/dev-content.db");
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let packs = installed_packs(&conn).unwrap();
        let semantic = Semantic::new(Arc::new(NoKeyStore), Arc::new(UnbuiltFactory));
        assert_eq!(
            semantic
                .status(&packs, &crate::db::user::Settings::new())
                .unwrap(),
            SemanticStatus::NotBuilt {
                pack: "vectors.dev".into(),
                backend: "dev".into(),
            }
        );
    }

    #[test]
    fn word_vectors_are_closest_for_shared_words() {
        let space = EmbeddingSpace {
            model: "dev/words".into(),
            dim: 64,
            normalized: true,
        };
        let cosine = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        let query = words_vector(&space, "Sabır");
        let patience = words_vector(&space, "sabır ve namaz ile yardım isteyin");
        let other = words_vector(&space, "göklerin ve yerin yaratılışında");
        assert!(cosine(&query, &patience) > cosine(&query, &other));
        let norm: f32 = patience.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
        assert!(words_vector(&space, " , ").iter().all(|x| *x == 0.0));
    }
}
