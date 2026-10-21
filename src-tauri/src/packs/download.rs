//! Resumable, cancellable pack downloads to `<app_data>/downloads/<id>.part`
//! with progress events and sha256 verification (spec §6.4 steps 4–5).
//!
//! TODO(M3): streaming download via [`crate::net`].
