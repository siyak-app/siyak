//! Embeddings for semantic search, cross-references and chat retrieval behind
//! one model-agnostic [`Embedder`] trait (spec §7.4, ADR 0012).
//!
//! Two backends embed queries in the space of the active vector pack:
//! - **local**: an ONNX model from a model pack, on this computer
//!   ([`onnx`]); the default, works offline and without a key.
//! - **api**: an embedding API with the user's key ([`api`]): OpenAI, Voyage,
//!   Gemini or an OpenAI-compatible server.
//!
//! A third, `dev`, hashes texts into meaningless vectors ([`fake`]); only debug
//! builds on the development database run it.
//!
//! [`space`] reads pack metadata, [`index`] finds the nearest verse vectors,
//! [`service`] picks the active vector pack,
//! reports why semantic features are unavailable and loads the embedder lazily.
//! Tafsir is never embedded ahead of time (ADR 0003).

pub mod api;
pub mod fake;
pub mod index;
pub mod onnx;
pub mod service;
pub mod space;

use std::future::Future;
use std::pin::Pin;

pub use space::{ApiProvider, Backend, EmbeddingSpace};

use crate::error::AppResult;

/// What an embedded text is. Retrieval models embed questions and the texts
/// that answer them differently (e5 `query: `/`passage: ` prefixes, Voyage
/// `input_type`, Gemini task types); each backend maps this itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// A search query or chat message.
    Query,
    /// A text searched over: tafsir paragraphs ranked for a question (spec §10).
    Passage,
}

/// Where an embedder sends the texts it embeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    /// Nothing leaves this computer.
    Local,
    /// Texts are sent to this provider (spec §14.1: allow-list and log).
    Remote(ApiProvider),
}

/// The vectors of a batch of texts, in input order.
pub type EmbedFuture<'a> = Pin<Box<dyn Future<Output = AppResult<Vec<Vec<f32>>>> + Send + 'a>>;

/// Turns texts into vectors of one [`EmbeddingSpace`].
///
/// The method returns a boxed future because async trait methods cannot be
/// called through `dyn Embedder`; local embedders run their model on a
/// blocking thread inside it, remote ones await the request.
pub trait Embedder: Send + Sync {
    /// The space of the vectors this embedder produces.
    fn space(&self) -> &EmbeddingSpace;

    /// Whether texts leave this computer.
    fn locality(&self) -> Locality;

    /// Embeds `texts`, returning one vector of `space().dim` components per
    /// text, unit length if `space().normalized`.
    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_>;
}
