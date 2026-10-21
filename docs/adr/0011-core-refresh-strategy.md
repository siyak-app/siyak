# 0011. Refresh the core data in place, in one transaction

- Status: Accepted
- Date: 2026-10-21

## Context

On first launch the bundled `core.db` is copied to `content.db` (spec §5). Later app versions ship
a newer `core.db` when the Quran text, surah metadata, notices or the bundled translations change.
`siyak-data` then raises `content_meta.core_version` (siyak-data `docs/pack-format.md` §3). By then
`content.db` also holds packs the user installed (translations, tafsirs, morphology). These must
survive the update, and they reference verses by their global id.

## Decision

On startup, after migrations, the app compares the bundled `core_version` with the one in
`content.db`. A missing value counts as 0. If the bundled one is higher (spec §6.2):

1. `content.db` is backed up to `content.db.bak` with SQLite's backup API.
2. The bundled `core.db` is attached. In one transaction with foreign keys deferred to commit, the
   core tables are replaced from it: `surahs`, `verses`, `verses_fts`, `notices`, the bundled packs
   (`packs.bundled = 1`, plus any pack whose id the new core bundles) with their `translations` and
   `translations_fts` rows, and the `content_meta` keys the core carries.
3. Packs the user installed (`bundled = 0`) and their tables are not touched. Verses are deleted
   and inserted again with the same ids (1..6236 never change), so their rows stay valid. The
   deferred foreign key check at commit proves it.
4. On any error the transaction rolls back. The backup is kept, the app keeps running on the old
   core, and `get_core_status` reports `core_refresh_failed`, which the interface shows as a
   dismissible message.

The refresh is refused unless both databases have the same schema version. The bundled `core.db`
always matches the app's migrations: `cargo xtask sync-core` and a Rust test check this.

## Alternatives considered

- **Copy the new `core.db` over `content.db` and reinstall the user's packs.** Simple, but
  reinstalling needs the original pack files or a download, and the app must work offline.
- **Ship core changes as migrations.** Mixes data with schema, and the text updates would have to
  be expressed as SQL in the app repository instead of coming from the data pipeline.
- **Keep core data in a separate, read-only database attached at runtime.** Clean separation, but
  full-text search and joins would then span two files, and every query would need the attachment.
  Revisit if `content.db` grows a lot.

## Consequences

- A data-only update needs no migration; `siyak-data` raises `core_version` and the app picks it up.
- Verse ids are a hard invariant: renumbering verses would need a new strategy (and a new ADR).
- The refresh briefly needs disk space for the backup (about the size of `content.db`).
- Debug builds use the same mechanism with the development dataset (`CORE_VERSION` in
  `xtask/src/dev_db.rs`).
