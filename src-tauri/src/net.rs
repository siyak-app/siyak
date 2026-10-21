//! The only HTTP client in the app: a fixed host allow-list plus a connection
//! log without content (spec §14.1). The webview never talks to the network.
//!
//! TODO(M3): allow-listed client for manifest and pack downloads.
//! TODO(M8): provider hosts, user-entered OpenAI-compatible `base_url`, connection log.
