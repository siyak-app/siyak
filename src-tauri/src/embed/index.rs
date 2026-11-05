//! Nearest-neighbour search over the verse vectors of one pack (spec §7.4).
//!
//! The vectors live in `content.db` (`verse_vectors`) as little-endian
//! float32 blobs and are scanned in memory: 6,236 verses of a few hundred to a
//! few thousand dimensions take milliseconds, need no SQLite extension (and so
//! no `unsafe`), and keep `content.db` readable by every tool (ADR 0012).

use rusqlite::Connection;

use super::EmbeddingSpace;
use crate::db::content::vector_rows;
use crate::error::{AppError, AppErrorCode, AppResult};

/// Decodes a little-endian float32 blob; a trailing partial value is ignored.
pub fn decode_vector(bytes: &[u8]) -> impl Iterator<Item = f32> + '_ {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
}

/// A verse and how similar it is to the query (cosine similarity, -1 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neighbor {
    /// The verse.
    pub verse_id: u32,
    /// Cosine similarity with the query.
    pub score: f32,
}

/// The unit-length vectors of one vector pack, kept in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorIndex {
    dim: usize,
    verse_ids: Vec<u32>,
    /// `verse_ids.len() × dim` values, row by row.
    data: Vec<f32>,
}

impl VectorIndex {
    /// Loads the vectors of `pack_id`. Each must have `space.dim` values;
    /// vectors are normalized here, so scores are cosine similarities even if
    /// the pack's are not.
    pub fn load(conn: &Connection, pack_id: &str, space: &EmbeddingSpace) -> AppResult<Self> {
        let dim = space.dim as usize;
        let rows = vector_rows(conn, pack_id)?;
        if rows.is_empty() {
            return Err(AppError::new(
                AppErrorCode::SemanticUnavailable,
                format!("{pack_id}: no vectors installed"),
            ));
        }
        let mut verse_ids = Vec::with_capacity(rows.len());
        let mut data = Vec::with_capacity(rows.len() * dim);
        for (verse_id, blob) in rows {
            if blob.len() != dim * 4 {
                return Err(AppError::new(
                    AppErrorCode::SemanticUnavailable,
                    format!("{pack_id}: the vector of verse {verse_id} has the wrong size"),
                ));
            }
            let start = data.len();
            data.extend(decode_vector(&blob));
            normalize(&mut data[start..]);
            verse_ids.push(verse_id);
        }
        Ok(Self {
            dim,
            verse_ids,
            data,
        })
    }

    /// Builds an index from `(verse, vector)` pairs (tests and tools).
    pub fn from_vectors(dim: usize, vectors: Vec<(u32, Vec<f32>)>) -> Self {
        let mut index = Self {
            dim,
            verse_ids: Vec::with_capacity(vectors.len()),
            data: Vec::with_capacity(vectors.len() * dim),
        };
        for (verse_id, vector) in vectors {
            let start = index.data.len();
            index.data.extend(vector.into_iter().take(dim));
            index.data.resize(start + dim, 0.0);
            normalize(&mut index.data[start..]);
            index.verse_ids.push(verse_id);
        }
        index
    }

    /// Number of components per vector.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Number of verses.
    pub fn len(&self) -> usize {
        self.verse_ids.len()
    }

    /// Whether the index has no verses.
    pub fn is_empty(&self) -> bool {
        self.verse_ids.is_empty()
    }

    /// The (unit-length) vector of a verse, for "verses like this one".
    pub fn vector(&self, verse_id: u32) -> Option<&[f32]> {
        let row = self.verse_ids.binary_search(&verse_id).ok()?;
        self.data.get(row * self.dim..(row + 1) * self.dim)
    }

    /// The `k` verses most similar to `query`, best first, leaving out
    /// `exclude` (e.g. the verse a cross-reference starts from).
    pub fn nearest(&self, query: &[f32], k: usize, exclude: &[u32]) -> Vec<Neighbor> {
        if query.len() != self.dim || k == 0 {
            return Vec::new();
        }
        let mut query = query.to_vec();
        normalize(&mut query);
        let mut scored: Vec<Neighbor> = self
            .verse_ids
            .iter()
            .zip(self.data.chunks_exact(self.dim))
            .filter(|(id, _)| !exclude.contains(id))
            .map(|(&verse_id, vector)| Neighbor {
                verse_id,
                score: vector.iter().zip(&query).map(|(a, b)| a * b).sum(),
            })
            .collect();
        let by_score = |a: &Neighbor, b: &Neighbor| {
            b.score
                .total_cmp(&a.score)
                .then(a.verse_id.cmp(&b.verse_id))
        };
        if scored.len() > k {
            scored.select_nth_unstable_by(k - 1, by_score);
            scored.truncate(k);
        }
        scored.sort_by(by_score);
        scored
    }
}

/// Scales `vector` to unit length; a zero vector stays zero.
fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|x| *x /= norm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(vector: &[f32]) -> Vec<u8> {
        vector.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn index() -> VectorIndex {
        VectorIndex::from_vectors(
            3,
            vec![
                (1, vec![1.0, 0.0, 0.0]),
                (2, vec![0.9, 0.1, 0.0]),
                (3, vec![0.0, 1.0, 0.0]),
                (4, vec![0.0, 0.0, 2.0]),
                (5, vec![-1.0, 0.0, 0.0]),
            ],
        )
    }

    #[test]
    fn decodes_little_endian_floats() {
        let values: Vec<f32> = decode_vector(&bytes(&[1.5, -2.0, 0.25])).collect();
        assert_eq!(values, [1.5, -2.0, 0.25]);
        assert_eq!(decode_vector(&[0, 0, 128]).count(), 0);
    }

    #[test]
    fn finds_the_nearest_verses_best_first() {
        let index = index();
        let found = index.nearest(&[2.0, 0.0, 0.0], 3, &[]);
        let ids: Vec<u32> = found.iter().map(|n| n.verse_id).collect();
        assert_eq!(ids[..2], [1, 2]);
        assert!((found[0].score - 1.0).abs() < 1e-6);
        assert!(found[0].score >= found[1].score && found[1].score >= found[2].score);
        assert_eq!(index.nearest(&[1.0, 0.0, 0.0], 10, &[]).len(), 5);
        assert_eq!(index.nearest(&[1.0, 0.0, 0.0], 0, &[]), Vec::new());
    }

    #[test]
    fn leaves_out_excluded_verses_and_wrong_dimensions() {
        let index = index();
        let query = index.vector(1).unwrap().to_vec();
        let found = index.nearest(&query, 1, &[1]);
        assert_eq!(found[0].verse_id, 2);
        assert!(index.nearest(&[1.0, 0.0], 3, &[]).is_empty());
    }

    #[test]
    fn stores_unit_vectors() {
        let index = index();
        let v = index.vector(4).unwrap();
        assert!((v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(index.vector(9), None);
        assert_eq!((index.len(), index.dim(), index.is_empty()), (5, 3, false));
    }

    #[test]
    fn loads_the_vectors_of_a_pack() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             CREATE TABLE packs (id TEXT PRIMARY KEY);
             CREATE TABLE verses (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        let space = EmbeddingSpace {
            model: "local/x".into(),
            dim: 2,
            normalized: false,
        };
        assert_eq!(
            VectorIndex::load(&conn, "vectors.x", &space)
                .unwrap_err()
                .code,
            AppErrorCode::SemanticUnavailable
        );
        crate::db::content::ensure_vector_table(&conn).unwrap();
        for (pack, verse, vector) in [
            ("vectors.x", 2, [0.0_f32, 3.0]),
            ("vectors.x", 1, [4.0, 0.0]),
            ("vectors.y", 1, [1.0, 1.0]),
        ] {
            conn.execute(
                "INSERT INTO verse_vectors VALUES (?1, ?2, ?3)",
                rusqlite::params![pack, verse, bytes(&vector)],
            )
            .unwrap();
        }
        let index = VectorIndex::load(&conn, "vectors.x", &space).unwrap();
        assert_eq!(index.len(), 2);
        assert_eq!(index.vector(1).unwrap(), [1.0, 0.0]);
        assert_eq!(index.vector(2).unwrap(), [0.0, 1.0]);

        conn.execute(
            "INSERT INTO verse_vectors VALUES ('vectors.x', 3, ?1)",
            [bytes(&[1.0])],
        )
        .unwrap();
        assert!(VectorIndex::load(&conn, "vectors.x", &space).is_err());
    }
}
