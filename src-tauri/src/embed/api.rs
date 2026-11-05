//! The API backend: embeds queries with OpenAI, Voyage, Gemini or an
//! OpenAI-compatible server (spec §7.4, ADR 0012).
//!
//! Requests go through [`crate::net`], so every provider host is on the
//! allow-list and every request is in the connection log. Keys come from the
//! keychain ([`crate::secrets`]) and never reach the webview.
//!
//! TODO(M7): one embedder per provider, keychain lookup, `Purpose::Embedding`.
