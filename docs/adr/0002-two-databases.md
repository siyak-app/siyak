# 0002. Keep content and user data in two databases

- Status: Accepted
- Date: 2026-10-21

## Context

Quran text, translations, tafsir, morphology and vectors come from downloadable packs and can be
rebuilt at any time. Bookmarks, notes, collections and settings are the user's own work and must
never be at risk from a pack operation (spec §5).

## Decision

Use two SQLite files in the app data directory: `content.db` (seeded from the bundled `core.db`,
written only by pack installation) and `user.db` (user data). Both are versioned with
`PRAGMA user_version` and migrated on startup; `user.db` is backed up to `user.db.bak` before any
migration.

## Consequences

- `content.db` can be deleted and recreated without losing anything personal.
- Verse references in `user.db` cannot be foreign keys into `content.db`; integrity is enforced in
  the application layer, and exports use `surah:ayah` for portability.
- Queries that need both (e.g. notes shown next to verses) are combined in Rust, not in SQL.
