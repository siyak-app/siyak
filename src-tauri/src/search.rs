//! Search: query classification and full-text, root/lemma and semantic search
//! behind the single search box (spec §7.1–§7.4).
//!
//! TODO(M4): `classify_query` and FTS5 text search over `verses_fts` and
//! `translations_fts`, including the Rust port of the pipeline's diacritics
//! stripping (tested against the shared `normalize_vectors.json`).
//! TODO(M5): root and lemma search. TODO(M7): semantic search via [`crate::embed`].
