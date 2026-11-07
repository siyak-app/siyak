//! Pack system: manifest, download, verification and installation of
//! translation, tafsir, morphology, vector, model and audio packs (spec §6).
//! Pack files are untrusted input (spec §14.3). [`service`] runs the queue;
//! the commands are in `commands/packs.rs`.

pub mod download;
pub mod install;
pub mod manifest;
pub mod service;

#[cfg(test)]
mod test_support;
