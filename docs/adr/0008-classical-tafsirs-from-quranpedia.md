# 0008. Take classical tafsirs from the Quranpedia dumps; no contemporary works in v1

- Status: Accepted
- Date: 2026-10-21

## Context

Tafsir texts must be redistributable. Quranpedia publishes per-verse dumps of many tafsirs and
treats classical works as shared heritage; contemporary works belong to their authors and
publishers (spec §6.6).

## Decision

v1 tafsir packs contain only pre-modern works (authors who died in the 19th century or earlier),
from the Quranpedia dumps with attribution and dump version, each mapping verified by hand.
al-Qurtubi comes from QUL and is published only after Tarteel confirms permission.

## Consequences

- Popular 20th-century tafsirs are not available in v1; they are evaluated work by work later.
- The data pipeline must track Quranpedia's changes API so packs do not go stale.
