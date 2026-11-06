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
- **Local is the default.** The recommended bundle contains the local pack. API packs
  (`vectors.gemini-embedding-2`, `vectors.voyage-4-large`) are listed with the other semantic
  search packs, never in a bundle, and are installed only after the user accepts terms that say
  searches are sent to the provider (`terms_ack`).
- Vectors for API packs are built once in the `siyak-data` pipeline with the maintainers' key and
  published like any pack, so the user pays only for their own queries. A release does not embed
  again: every published vector pack is built from verse vectors pinned once by a maintainer
  (`export-vectors`, the `vector-inputs` pre-release); a local pack's are checked against the
  model pack the release builds (cosine ≥ 0.999 on 32 verses).
- **Nearest neighbours are found in Rust, in memory.** Verse vectors are a plain `verse_vectors`
  table in `content.db` (created by the first vector pack install); the active pack's vectors are
  loaded once and scanned per query. The spec first named `sqlite-vec`, but loading it from Rust
  needs `unsafe` code, which the workspace forbids, and its `vec0` tables would make `content.db`
  unreadable by tools without the extension (pipeline, xtask, backups). For 6,236 verses a full
  scan takes milliseconds.
- **The default local model is BGE-M3** (`model.bge-m3` + `vectors.bge-m3`, decided 2026-11-06).
  Four multilingual models were measured on the evaluation set (`siyak-data`
  `data/eval/semantic.toml`, 144 items) with Arabic, Turkish and English verse text. Scores are
  hit@10 / recall@10 / MRR@10, measured on the built int8 packs as the app runs them:

  | Model                 | Download | Turkish            | English            | Arabic             | Cross-references   | All                |
  | --------------------- | -------- | ------------------ | ------------------ | ------------------ | ------------------ | ------------------ |
  | BGE-M3                | 418 MB   | 0.90 / 0.56 / 0.79 | 0.83 / 0.48 / 0.63 | 0.80 / 0.52 / 0.65 | 0.67 / 0.43 / 0.53 | 0.81 / 0.51 / 0.67 |
  | multilingual-e5-small | 87 MB    | 0.76 / 0.41 / 0.58 | 0.63 / 0.29 / 0.40 | 0.45 / 0.15 / 0.18 | 0.56 / 0.32 / 0.44 | 0.64 / 0.33 / 0.45 |

  With PyTorch, before export, multilingual-e5-base scored 0.71 / 0.39 / 0.55 overall and
  Qwen3-Embedding-0.6B 0.74 / 0.46 / 0.58 (strong in Arabic, weak in Turkish); neither was
  packaged. BGE-M3 loses nothing in int8 (float32: 0.81 / 0.51 / 0.69). The app's int8 output
  differs slightly from the pipeline's (cosine about 0.975, because ONNX Runtime picks different
  int8 kernels), but rankings on the evaluation set do not change; in float32 the two agree to
  more than 0.995.

## Alternatives considered

- **Local models only.** Simplest and fully private, but excludes stronger hosted models and
  OpenAI-compatible servers the user already runs.
- **Let users embed all verses with their own key.** No redistribution question for the vectors,
  but every user pays for and waits for 6,236 embeddings, and results differ between users.
- **`sqlite-vec`.** Indexed search inside SQLite, but see above; a full scan is fast enough for
  one Quran.
- **Combine several spaces at once (ensemble).** Better recall is possible, but it multiplies
  packs, cost and complexity. Not in v1.
- **A smaller default model (multilingual-e5-small).** A fifth of the download, but much weaker,
  above all for Arabic queries and the first result of Turkish ones. It stays in `siyak-data` as
  a small pack for development and tests.
- **Offer both, BGE-M3 recommended and e5-small as a light option.** Two model and vector pairs to
  build and maintain for each release; can be added later without code changes.

## Consequences

- With an API pack, semantic search and cross-references need the network and a key; the reason is
  shown when either is missing. Each request goes through `net.rs` (allow-list and connection log).
- In chat (§10), tafsir paragraphs ranked for a question are sent to the embedding provider too.
- Each API pack's provider terms must allow publishing the vectors; this is checked per pack.
  Read on 2026-11-06 (not legal advice): neither the Gemini API Additional Terms nor Voyage's
  terms forbid publishing embeddings. Google claims no ownership of generated content but forbids
  extracting or replicating the Services and, for API use, expects professional users of 18+;
  in the EEA, Switzerland and the UK only paid services may back apps offered to users. Voyage's
  terms say nothing about outputs; inputs on an account without a payment method may be used
  for training (the inputs are public Quran texts and translations).
- A better model later is a new pack pair (or one API pack); the app needs no code change.
- The default local packs are large: about 420 MB to download, about 610 MB installed, and the
  loaded model takes several hundred MB of memory. Building `vectors.bge-m3` takes about 40
  minutes on an Apple M-series CPU, which is why releases use pinned vectors.
- API packs are measured against the same evaluation set.
