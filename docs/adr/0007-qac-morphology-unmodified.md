# 0007. Use the Quranic Arabic Corpus v0.4 morphology, unmodified

- Status: Accepted
- Date: 2026-10-21

## Context

Word analysis needs root, lemma, part of speech and inflectional features for every word. Of the
evaluated sources, the Quranic Arabic Corpus (QAC) v0.4 is the most complete and reviewed. Its
terms allow verbatim redistribution only (spec §5.3.1).

## Decision

The morphology pack ships the original QAC file and notice unchanged; SQLite tables are derived
from it by changing format only. Reported tag errors are collected in an errata file and
forwarded to QAC, never applied to the data. The source and a link to corpus.quran.com are shown
wherever the data appears.

## Consequences

- Known errors stay visible until QAC fixes them; the app may show a note from the errata file.
- Human-readable labels for tags live in the app's i18n files, not in the data.
- MASAQ will be evaluated later as a permissively licensed alternative; the schema is source-agnostic.
