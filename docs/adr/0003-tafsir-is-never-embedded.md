# 0003. Look tafsir up by verse; never embed it

- Status: Accepted
- Date: 2026-10-21

## Context

Classical tafsirs are very large. Embedding them for semantic search would multiply pack sizes
and blur the line between a verse and commentary about it (spec §5.2, §7.4).

## Decision

Tafsir is stored as passages keyed by verse range and always reached through a verse. It has no
full-text index and no stored vectors. For AI chat, paragraphs of a long passage may be ranked
against the question on the fly, but nothing is persisted (spec §10).

## Consequences

- Much smaller tafsir packs; a tafsir is always shown in the context of its verses.
- Searching inside tafsir text is not possible in v1.
