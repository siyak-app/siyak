//! A deterministic embedder for tests: the same text always gives the same
//! unit vector, different texts almost always different ones.

use super::{EmbedFuture, Embedder, EmbeddingSpace, Locality, TextKind};

/// Embeds texts by hashing them; needs no model or network.
pub(crate) struct FakeEmbedder {
    pub(crate) space: EmbeddingSpace,
    pub(crate) locality: Locality,
}

impl FakeEmbedder {
    /// A local fake embedder for `space`.
    pub(crate) fn new(space: EmbeddingSpace) -> Self {
        Self {
            space,
            locality: Locality::Local,
        }
    }

    fn vector(&self, text: &str, kind: TextKind) -> Vec<f32> {
        let seed = text
            .bytes()
            .chain([kind as u8])
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        let mut state = seed;
        let mut vector: Vec<f32> = (0..self.space.dim)
            .map(|_| {
                // xorshift64: cheap, deterministic, spread over the components.
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                f32::from(u16::try_from(state >> 48).unwrap_or(0)) / 32768.0 - 1.0
            })
            .collect();
        if self.space.normalized {
            let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vector.iter_mut().for_each(|x| *x /= norm);
            }
        }
        vector
    }
}

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
}
