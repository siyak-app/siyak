//! Installs a verified pack: SQLite packs via `ATTACH` and `INSERT … SELECT`
//! of expected columns only, inside one transaction; archive packs via an
//! atomic rename (spec §6.4 step 6). No SQL from a pack is ever executed.
//!
//! TODO(M3): install and uninstall transactions.
