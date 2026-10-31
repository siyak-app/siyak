//! Pack system: manifest, download, verification and installation of
//! translation, tafsir, morphology, vector and model packs (spec §6).
//! Pack files are untrusted input (spec §14.3).
//!
//! TODO(M3): `fetch_manifest`, `install_pack`, `install_bundle`,
//! `cancel_install`, `uninstall_pack`.

pub mod download;
pub mod install;
pub mod manifest;

#[cfg(test)]
mod test_support;
