# 0004. Translate Arabic tafsir on demand with the user's own model

- Status: Accepted
- Date: 2026-10-21

## Context

Most classical tafsirs exist only in Arabic. Shipping machine-translated packs would distribute
unreviewed text as if it were a source; human translations are rarely licensed for
redistribution (spec §9).

## Decision

Packs contain the original Arabic. A summary or translation is produced only when the user asks,
with the user's own API key (BYOK) or a local model, after showing an estimated cost. Results are
cached in `user.db`, always displayed next to the original and labelled with provider, model and
date.

## Consequences

- No machine translation is ever presented as a source, and the project carries no API costs.
- Users without a key still get the full original text.
- The cache must be invalidated when a tafsir pack changes version.
