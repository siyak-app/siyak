# 0009. Write code in English; keep documentation bilingual

- Status: Accepted
- Date: 2026-10-21

## Context

The first audience is Turkish-speaking, but the project should welcome contributors from
anywhere, and consistent terminology matters for Islamic terms (spec §16.1).

## Decision

Identifiers, comments, commits, PRs, tests, `ARCHITECTURE.md` and ADRs are English only. The
README, CONTRIBUTING, website and the app interface are Turkish and English; English is the source
of truth and Turkish files carry a "last synced" line. Islamic terms in code follow
`docs/glossary.md` (`surah`, `verse`, `translation`, `tafsir`; never `sure`, `ayet`, `meal`).
User-visible strings exist only as i18n keys, enforced by ESLint.

## Consequences

- Every user-facing change touches both `src/i18n/tr.json` and `src/i18n/en.json`; a test checks
  that their keys match.
- Turkish documents can lag behind; the sync line makes that visible.
