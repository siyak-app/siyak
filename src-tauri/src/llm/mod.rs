//! AI provider layer: the `LlmProvider` trait, shared request/response types
//! and model roles (`chat_model`, `translation_model`) (spec §8).
//! AI is an optional layer; core features never require an API key.
//!
//! TODO(M8): provider trait and registry.

pub mod anthropic;
pub mod chat;
pub mod cost;
pub mod gemini;
pub mod openai;
pub mod translate;
