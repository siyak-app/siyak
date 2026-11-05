# 0012. Embed queries locally by default, or with an embedding API

- Status: Accepted
- Date: 2026-11-05

## Context

Semantic search, cross-references and chat retrieval compare a query with precomputed verse
vectors (spec §7.4, §10). The query must be embedded by the same model that made the vectors.
The spec assumed one kind of embedder: a local ONNX model shipped as a model pack. Users with an
API key may prefer a hosted embedding model, which can be stronger for Turkish and English
queries. Anthropic does not offer an embedding model; it recommends Voyage AI. OpenAI and Google
Gemini have embedding APIs, and OpenAI-compatible servers such as Ollama serve open models on the
user's own computer.

## Decision

- A **vector pack defines its embedding space**: `embedding_model` (canonical, with a backend
  prefix: `local/…`, `openai/…`, `voyage/…`, `gemini/…`, `compat/…`) and `dim`. Every embedder is
  checked against the active pack's space; vectors of different spaces are never compared.
- The pack's metadata names the **backend**:
  - `local`: the pack names a `model_pack` (and requires it). The ONNX model runs on this computer.
  - `api`: the pack names a `provider` (`openai`, `voyage`, `gemini`, `openai_compatible`) and the
    provider's model id. There is no model pack. The user's key comes from the keychain, shared with
    the AI layer (spec §8, §14.2). An OpenAI-compatible server needs the address the user entered.
- One `Embedder` trait serves both (`src-tauri/src/embed/`). It embeds batches and distinguishes
  queries from passages, because retrieval models treat them differently (e5 prefixes, Voyage
  `input_type`, Gemini task types). It reports whether texts leave the computer.
- `embed::service::Semantic` picks the active vector pack (setting `semantic.vector_pack`, else the
  first installed one), returns a `SemanticStatus` that tells the interface what is missing
  (pack, model pack, key, address), and builds the embedder lazily, once per pack version.
- **Local is the default.** The recommended bundle contains a local pack. API packs are listed
  under "Advanced" and show, before download, that searches are sent to the provider.
- Vectors for API packs are built once in the `siyak-data` pipeline with the maintainers' key and
  published like any pack, so the user pays only for their own queries.
- **Nearest neighbours are found in Rust, in memory.** Verse vectors are a plain `verse_vectors`
  table in `content.db` (created by the first vector pack install); the active pack's vectors are
  loaded once and scanned per query. The spec first named `sqlite-vec`, but loading it from Rust
  needs `unsafe` code, which the workspace forbids, and its `vec0` tables would make `content.db`
  unreadable by tools without the extension (pipeline, xtask, backups). For 6,236 verses a full
  scan takes milliseconds.

## Alternatives considered

- **Local models only.** Simplest and fully private, but excludes stronger hosted models and
  OpenAI-compatible servers the user already runs.
- **Let users embed all verses with their own key.** No redistribution question for the vectors,
  but every user pays for and waits for 6,236 embeddings, and results differ between users.
- **`sqlite-vec`.** Indexed search inside SQLite, but see above; a full scan is fast enough for
  one Quran.
- **Combine several spaces at once (ensemble).** Better recall is possible, but it multiplies
  packs, cost and complexity. Not in v1.

## Consequences

- With an API pack, semantic search and cross-references need the network and a key; the reason is
  shown when either is missing. Each request goes through `net.rs` (allow-list and connection log).
- In chat (§10), tafsir paragraphs ranked for a question are sent to the embedding provider too.
- Each API pack's provider terms must allow publishing the vectors; this is checked per pack.
- A better model later is a new pack pair (or one API pack); the app needs no code change.
- The model choice and the evaluation set (spec §18) decide the default local pack; API packs
  are measured against the same set.
