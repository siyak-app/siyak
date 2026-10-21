//! Query embeddings for semantic search behind a model-agnostic `Embedder`
//! trait (spec §7.4). The model is loaded lazily on first use.
//! Tafsir is never embedded (ADR 0003).
//!
//! TODO(M7): `Embedder` trait and compatibility check between the active
//! vector pack and the loaded model.

pub mod onnx;
