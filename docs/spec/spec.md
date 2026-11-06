# Siyak — Quran Research Tool · Technical Specification (v1)

> Status: Draft · Last updated: 2026-10-21
> This document defines the core architecture of the application. Items marked as decisions are locked; open questions are listed in §18.
> This English version is the source of truth. A Turkish translation is kept alongside it as `spec.tr.md` (§16.1).

---

## 1. Vision and scope

A **free, open-source, all-in-one research tool** for anyone interested in the Quran: theology students, researchers and curious readers alike. Reading, comparison, word and root analysis, tafsir, search, note-taking and AI-assisted inquiry live in a single application.

The application has no backend; everything runs on the user's computer. All data (verses, translations, tafsirs, morphology, vectors, user notes) lives in local SQLite databases. Internet access is needed only to:
- download data packs,
- receive application updates,
- chat or translate with a cloud AI provider (not needed at all when a local model is used).

**Core principles**
1. **Faithfulness to sources:** The original of every text is always one click away. Machine output (translations, AI answers) is clearly labelled and shown next to its source.
2. **Full value without an API key:** Reading, comparison, root analysis, search and notes work without any API key. AI features are an additional layer.
3. **Verifiability:** The code, the data pipeline and the source licenses are open; it can be proven that the binary was built from the code.
4. **Licensing comes first:** Content whose redistribution rights have not been confirmed is not distributed.
5. **Simple for everyone, deep on demand:** The same interface serves both readers and researchers. Screens open simple; depth appears as the user asks for it (progressive disclosure, §13.2). There are no separate "basic/expert" modes.
6. **Open to contribution:** The code is readable, decisions are recorded, documentation is bilingual, and there are ways to contribute without writing code (§16).

### v1 features

| Area | Feature |
|---|---|
| Reading | Reading by surah/verse: Arabic text + selectable translation(s) |
| Comparison | Several translations and tafsirs for a verse or range, side by side in parallel columns |
| Word/root analysis | Click a word to see its morphology (root, lemma, part of speech, inflection); all verses where the root/lemma occurs (concordance) |
| Tafsir | Verse-linked tafsir panel; optional Turkish summary/translation of Arabic tafsirs (BYOK) |
| Cross-references | Verses related to a verse: semantic similarity + shared roots/lemmas |
| Search | Text search (FTS5), root/lemma search, semantic search (local embeddings) |
| Collections and notes | Bookmarks, verse-linked notes, collections that group verses by research topic |
| Export | Collection/notes → Markdown; academic citation text for verses and translations/tafsirs; JSON backup |
| AI chat (BYOK) | RAG chat: Anthropic, OpenAI, Google Gemini, OpenAI-compatible endpoints (Ollama, LM Studio, OpenRouter, etc.) |
| Pack manager | Download, update and remove translation/tafsir/morphology/vector/model packs |
| UI languages | Turkish + English |

### Out of scope for v1
Audio/recitation, hadith, cross-device sync, mobile version, community translation contributions (§9.5), pre-translated tafsir packs.

### Target platforms
Windows, macOS, Linux.

---

## 2. Decision summary

| Topic | Decision |
|---|---|
| License (code) | `MIT OR Apache-2.0` (dual license, Rust ecosystem convention). Data packs carry their own licenses |
| App framework | Tauri v2 (Rust core + system webview) |
| Frontend | React + TypeScript + Vite |
| UI kit | Tailwind CSS + shadcn/ui |
| Database | SQLite (`rusqlite`, bundled) |
| Vector search | Brute-force KNN in Rust over vectors kept in memory (ADR 0012) |
| Embedding runtime | ONNX Runtime (`ort`) + `tokenizers` on the Rust side |
| Embedding model | Local default: BGE-M3, int8 (ADR 0012). The system stays model-agnostic (§7.4) |
| Tafsir access | Keyed lookup by verse reference; tafsir is never embedded |
| Tafsir sources | v1: only pre-modern classical tafsirs, from the Quranpedia data dumps; al-Qurtubi from QUL (§6.6). Contemporary works are not included in v1 |
| Morphology source | Quranic Arabic Corpus (QAC) v0.4, following its terms of use to the letter; tags are never modified (§5.3.1). MASAQ will later be evaluated as a permissively licensed alternative |
| Turkish for Arabic tafsirs | The original is packaged; translation/summary is on demand, with the user's own model, cached and labelled (§9) |
| Network calls | Only from Rust (`reqwest`), through a fixed allow-list (§14) |
| API keys | OS keychain (`keyring` crate); never written to a database, never returned to the webview |
| Distribution | GitHub Releases (app and data in separate repositories) |
| Updates | `tauri-plugin-updater` + `latest.json` (§15.1) |
| Notes backup | JSON export/import |
| Telemetry | None |
| UI layout | A single interface with progressive disclosure; the experience profile chosen on first launch only sets defaults |
| Language policy | Code, identifiers, comments, commits, PRs: **English only**. Documentation and website: **Turkish + English** (§16.1) |
| Website | Landing page + documentation in one site: Astro + Starlight in `website/` of the app repository, GitHub Pages, `quran.kemalozyon.com` |
| Name | **Siyak** (from *siyāq-sibāq* in tafsir methodology: reading a verse in its context). Landing page tagline: "Ayeti bağlamıyla okuyun" / "Read every verse in context" |
| Identifiers | GitHub organization `siyak-app`; repositories `siyak`, `siyak-data`, `.github`; Tauri identifier and keychain service `com.kemalozyon.siyak` |
| GitHub | A dedicated organization, not a personal account |
| Embedded URLs | Primary address for the manifest and the updater is `quran.kemalozyon.com`, with GitHub Releases as fallback (§6.3, §15.1) |

---

## 3. Architecture overview

```
┌──────────────────────── Webview (React) ────────────────────────┐
│ Reader · Compare · Word panel · Tafsir · Search ·                │
│ Collections · Chat · Packs · Settings                            │
│              TanStack Query ── invoke() / events / Channel       │
└───────────────────────────────┬──────────────────────────────────┘
                                │ Tauri IPC
┌───────────────────────────────┴────────── Rust core ────────────┐
│  db        content.db (content) + user.db (user data)           │
│  search    FTS5 · root/lemma · in-memory KNN                     │
│  related   cross-references (vectors + shared roots)            │
│  embed     Embedder trait → local ONNX or embedding API (lazy)   │
│  packs     manifest · download · sha256 · install                │
│  llm       Provider trait → Anthropic/OpenAI/Gemini/OAI-compat  │
│            ├─ chat (RAG)                                         │
│            └─ translate (tafsir summary/translation, cost)      │
│  export    Markdown · citation · JSON                            │
│  net       allow-list + connection log                           │
│  secrets   keyring                                               │
└───────────────┬──────────────────────────────┬──────────────────┘
                │ HTTPS                         │ HTTPS
     GitHub Releases (siyak-data)        AI provider APIs
     manifest.json + packs               (or a local endpoint)
```

**Principle:** The webview is only the user interface. Database, model, file system and network access live entirely in Rust.

---

## 4. Repository structure

All repositories live under a dedicated GitHub organization: `github.com/siyak-app` (`github.com/siyak` belongs to an unrelated account). There are two main repositories and one organization repository:

1. **App repository** (`siyak`): the Tauri application, plus the landing page and documentation site (`website/`). License: `MIT OR Apache-2.0`. Its releases contain the installers and `latest.json`.
2. **`siyak-data`**: the pipeline that builds data packs, and the pack releases. Pipeline code is `MIT OR Apache-2.0`; every data source keeps its own license under `sources/<source>/LICENSE`. It is a separate repository so that `releases/latest` links never mix app and data versions.
3. **`.github`**: organization-wide community files. The `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`, shared issue templates and `profile/README.md` (organization profile page, bilingual) placed here automatically apply to every repository that has no copy of its own, so the same files are not maintained twice.

### 4.1 App repository

```
/
├─ src/                          # React frontend
│  ├─ app/                       # router, providers, layout
│  ├─ features/
│  │  ├─ reader/                 # reading, virtualized verse list
│  │  ├─ compare/                # parallel-column comparison
│  │  ├─ word/                   # word/morphology panel, concordance
│  │  ├─ tafsir/                 # tafsir panel, summary/translation
│  │  ├─ related/                # cross-reference panel
│  │  ├─ search/
│  │  ├─ collections/            # collections, notes, bookmarks
│  │  ├─ chat/
│  │  ├─ packs/
│  │  └─ settings/               # AI providers, spending, network log
│  ├─ components/ui/             # shadcn components
│  ├─ lib/
│  │  ├─ bindings.ts             # generated by tauri-specta
│  │  └─ queries.ts              # TanStack Query hooks
│  ├─ stores/                    # Zustand
│  ├─ i18n/{tr,en}.json
│  └─ assets/fonts/              # bundled fonts
├─ src-tauri/
│  ├─ src/
│  │  ├─ lib.rs                  # app setup, command registration
│  │  ├─ error.rs
│  │  ├─ db/{mod,content,user}.rs
│  │  ├─ search.rs
│  │  ├─ morphology.rs
│  │  ├─ related.rs
│  │  ├─ embed/{mod,space,service,onnx,api}.rs
│  │  ├─ packs/{manifest,download,install}.rs
│  │  ├─ llm/
│  │  │  ├─ mod.rs               # Provider trait, shared types
│  │  │  ├─ anthropic.rs
│  │  │  ├─ openai.rs            # OpenAI + OpenAI-compatible
│  │  │  ├─ gemini.rs
│  │  │  ├─ chat.rs              # RAG flow
│  │  │  ├─ translate.rs         # tafsir summary/translation
│  │  │  └─ cost.rs              # token estimates, pricing, usage log
│  │  ├─ export/{markdown,citation,backup}.rs
│  │  ├─ net.rs                  # allow-listed HTTP client + log
│  │  ├─ secrets.rs
│  │  └─ commands/
│  ├─ migrations/{content,user}/
│  ├─ resources/core.db
│  ├─ capabilities/
│  └─ tauri.conf.json
├─ dev-data/
│  └─ dev-content.db             # small sample data for development (no pipeline needed)
├─ website/                      # Astro + Starlight: landing page + docs site (§16.2)
│  └─ src/content/docs/{en,tr}/
├─ docs/
│  ├─ spec/{spec.md,spec.tr.md}  # this document and its Turkish translation
│  ├─ adr/                       # Architecture Decision Records
│  └─ assets/                    # screenshots, diagrams
├─ .github/
│  ├─ workflows/{ci,release,website}.yml
│  ├─ ISSUE_TEMPLATE/            # bug, feature, new source proposal, translation error
│  └─ PULL_REQUEST_TEMPLATE.md
├─ .devcontainer/
├─ README.md · README.tr.md
├─ CONTRIBUTING.md · CONTRIBUTING.tr.md
├─ ARCHITECTURE.md
├─ CHANGELOG.md                 # CoC and SECURITY come from the organization's .github repo
└─ LICENSE-MIT · LICENSE-APACHE
```

### 4.2 `siyak-data`

```
/
├─ sources/<source>/             # raw data + LICENSE + README (origin, version)
├─ pipeline/                     # Python
│  ├─ normalize.py               # diacritics stripping (identical to the Rust version; shared test vectors)
│  ├─ build_core.py              # Tanzil → core.db
│  ├─ build_translation.py
│  ├─ build_tafsir.py            # includes paragraph splitting
│  ├─ build_morphology.py        # morphology → word/segment tables
│  ├─ build_vectors.py
│  ├─ export_model.py            # ONNX + quantization
│  ├─ build_pricing.py           # pricing.json
│  └─ validate.py
├─ tests/normalize_vectors.json  # test set shared by Python and Rust
├─ docs/
│  ├─ pack-format.md             # pack format specification
│  ├─ adding-a-source.md         # guide to adding a translation/tafsir (incl. license checklist)
│  └─ sources.md                 # all sources with origin and license
├─ manifest.template.json
├─ README.md · README.tr.md · CONTRIBUTING.md · CONTRIBUTING.tr.md
└─ .github/
   ├─ workflows/{ci,data-release}.yml
   └─ ISSUE_TEMPLATE/            # new source proposal, data error report
```

---

## 5. Data model

Two databases live in the user's application data directory (`app_data_dir()`):

- **`content.db`**: the Quran text and the content of installed packs. Never written by anything outside the app, and can be rebuilt from scratch at any time.
- **`user.db`**: the user's own data. Pack operations never touch this file.

On first launch, `resources/core.db` is copied to `content.db`. When the app ships a newer `core.db`, the core tables are refreshed in place (§6.2).

### 5.1 `content.db`: core

```sql
CREATE TABLE surahs (
  number          INTEGER PRIMARY KEY,         -- 1..114
  name_ar         TEXT NOT NULL,
  name_tr         TEXT NOT NULL,
  name_en         TEXT NOT NULL,
  revelation_type TEXT NOT NULL CHECK (revelation_type IN ('meccan','medinan')),
  ayah_count      INTEGER NOT NULL,
  bismillah       TEXT                         -- Tanzil's basmala for the surah header; NULL for 1 and 9
);

CREATE TABLE verses (
  id            INTEGER PRIMARY KEY,           -- global 1..6236
  surah         INTEGER NOT NULL REFERENCES surahs(number),
  ayah          INTEGER NOT NULL,
  juz           INTEGER NOT NULL,
  text_uthmani  TEXT NOT NULL,                 -- display
  text_clean    TEXT NOT NULL,                 -- without diacritics; search
  UNIQUE (surah, ayah)
);

CREATE TABLE packs (
  id            TEXT PRIMARY KEY,              -- e.g. 'en.pickthall'
  type          TEXT NOT NULL CHECK (type IN
                  ('translation','tafsir','morphology','vectors','model')),
  lang          TEXT,
  title         TEXT NOT NULL,
  author        TEXT,
  version       TEXT NOT NULL,                 -- semver
  license       TEXT NOT NULL,
  attribution   TEXT,
  citation_json TEXT,                          -- citation metadata (§11.2)
  meta_json     TEXT,                          -- type-specific fields
  installed_at  TEXT NOT NULL,
  bundled       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE translations (
  pack_id   TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_id  INTEGER NOT NULL REFERENCES verses(id),
  text      TEXT NOT NULL,
  PRIMARY KEY (pack_id, verse_id)
) WITHOUT ROWID;

CREATE TABLE content_meta (
  key    TEXT PRIMARY KEY,                     -- e.g. 'core_version', 'tanzil_version'
  value  TEXT NOT NULL
);

CREATE TABLE notices (                       -- notices that must ship with the data (About → Sources)
  id      TEXT PRIMARY KEY,                    -- e.g. 'tanzil.text.uthmani' or a pack id
  source  TEXT NOT NULL,                       -- source id, e.g. 'tanzil'
  title   TEXT NOT NULL,
  body    TEXT NOT NULL,                       -- reproduced verbatim
  url     TEXT
);
```

**Quran text (Tanzil):** taken from Tanzil's XML files, not the plain-text export (the plain-text export prepends the *basmala* to verse 1 of every surah).
- `text_uthmani` (display): Tanzil Uthmani **with** pause marks and sajdah signs, rub' al-hizb marks **off**; other options at Tanzil's defaults. The exact options are recorded in `siyak-data/sources/tanzil/source.toml` and in the app's dev data, and must be identical in both.
- `text_clean` (search): Tanzil Simple Clean, used as-is (never derived with our own normalizer, because Tanzil's terms do not allow changing the text).
- **Basmala:** it is verse 1 of al-Fatiha only. For the other surahs it is not a verse and is not stored in `verses`; the reader shows it as a surah header (except at-Tawba, surah 9) from `surahs.bismillah`, which keeps Tanzil's own basmala text. Search and morphology are unaffected.

### 5.2 `content.db`: tafsir

A tafsir passage may cover a range of verses. For partial translation and citation, passages are split into paragraphs in the pipeline.

```sql
CREATE TABLE tafsir (
  id          INTEGER PRIMARY KEY,
  pack_id     TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_from  INTEGER NOT NULL REFERENCES verses(id),
  verse_to    INTEGER NOT NULL REFERENCES verses(id),
  kind        TEXT NOT NULL DEFAULT 'commentary' CHECK (kind IN ('commentary','introduction')),
  CHECK (verse_to >= verse_from)
);
CREATE INDEX tafsir_lookup ON tafsir(pack_id, verse_from, verse_to);

CREATE TABLE tafsir_paragraphs (
  tafsir_id  INTEGER NOT NULL REFERENCES tafsir(id) ON DELETE CASCADE,
  idx        INTEGER NOT NULL,                 -- starts at 0
  text       TEXT NOT NULL,                    -- small safe HTML subset (see below)
  part       TEXT,                             -- volume of the printed edition, if known
  page       INTEGER,                          -- page of the printed edition, if known
  UNIQUE (tafsir_id, idx)
);                                             -- a rowid table: WITHOUT ROWID roughly doubles disk use for long text rows
```

The pack's `lang` field states the language of the tafsir. For tafsirs where `lang` differs from the UI language, the interface shows the summary/translation buttons.

**Verse groups:** Source data is keyed per verse, but classical tafsirs often explain several verses together. The pipeline merges consecutive verses that share the same text into a single `tafsir` row (`verse_from`–`verse_to`); the text is stored once. When the tafsir of a verse is requested, the covering range is returned and the interface shows "This commentary covers verses 2:153–157".

**Paragraph text** uses a fixed, small HTML subset: `b`, `i`, `br`, `span.ayah` (quoted Quran), `span.ref`, and `span.poem > span.hemistich` for verse. The app sanitizes it again before rendering. `part`/`page` enable citations such as "al-Ṭabarī 1/113".

**Editor footnotes are dropped in v1.** Footnotes of modern critical editions (*taḥqīq*) are contemporary work and fall under the no-contemporary-works rule; packs carry only the classical text. Each pack has a `footnotes = "drop" | "keep"` setting in the pipeline, so classical footnote material (e.g. Ibn al-Munayyir's *al-Intiṣāf* in al-Kashshāf) can be enabled later case by case.

**Author's introduction (*muqaddima*):** stored as a row with `kind = 'introduction'` anchored at 1:1. The tafsir panel shows it as a separate "Introduction" entry, not as commentary on 1:1, and the chat (§10) never adds it to the context.

**Edition material** (editor studies, indexes, colophons) and untagged duplicate fragments of the same text are excluded by documented per-book rules; every excluded character is accounted for by the pipeline's text-fidelity gate.

**Range boundaries** follow the source's grouping; where the source data links groups incorrectly, boundaries are drawn from the printed edition's own range headings (e.g. al-Shawkānī's "[سورة … : الآيات a الى b]"). The text itself is never changed.

**Gaps:** a verse without its own record is added to the preceding range only if that range's text verifiably contains the verse; ranges never cross surah boundaries. Unverified gaps stay empty and are reported; the tafsir panel then says "This tafsir has no separate commentary for this verse".

**Tafsir text is not searchable** (no FTS index). This greatly reduces pack size for large tafsirs; tafsir is always accessed through a verse.

### 5.3 `content.db`: morphology

Morphology is a separate pack type; the v1 source is the Quranic Arabic Corpus (§5.3.1). Word positions are stored as `surah:ayah:word`, segments as the prefix/stem/suffix parts within a word.

```sql
CREATE TABLE words (
  id         INTEGER PRIMARY KEY,
  verse_id   INTEGER NOT NULL REFERENCES verses(id),
  position   INTEGER NOT NULL,                 -- word order within the verse, from 1
  text_ar    TEXT NOT NULL,                    -- Uthmani
  translit   TEXT,
  gloss_en   TEXT,                             -- if present in the source
  UNIQUE (verse_id, position)
);

CREATE TABLE roots (
  id    INTEGER PRIMARY KEY,
  root  TEXT NOT NULL UNIQUE                   -- in Arabic letters, e.g. 'ق و ل'
);

CREATE TABLE lemmas (
  id       INTEGER PRIMARY KEY,
  lemma    TEXT NOT NULL UNIQUE,
  root_id  INTEGER REFERENCES roots(id)
);

CREATE TABLE word_segments (
  word_id   INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
  idx       INTEGER NOT NULL,
  kind      TEXT NOT NULL CHECK (kind IN ('prefix','stem','suffix')),
  text_ar   TEXT NOT NULL,
  pos       TEXT NOT NULL,                     -- part-of-speech tag
  lemma_id  INTEGER REFERENCES lemmas(id),
  root_id   INTEGER REFERENCES roots(id),
  features  TEXT,                              -- JSON: person, number, gender, mood, case, etc.
  PRIMARY KEY (word_id, idx)
) WITHOUT ROWID;

CREATE INDEX seg_root  ON word_segments(root_id);
CREATE INDEX seg_lemma ON word_segments(lemma_id);
```

- **Alignment:** The morphology source's word segmentation must match the Tanzil Uthmani text exactly. `validate.py` compares word counts and text for every verse; on any mismatch the pack is not built.
- **Reader:** If a morphology pack is installed, the Arabic text is rendered word by word from `words`, so every word is clickable. Otherwise `verses.text_uthmani` is shown as plain text.
- **Concordance:** Found by root or lemma via `word_segments → words → verses`; returned with a per-surah frequency summary.

#### 5.3.1 Source: Quranic Arabic Corpus (QAC)

**Rationale:** Of the evaluated sources (QAC, MASAQ, Birzeit Quran Corpus/QuranMorph, QUL), QAC is the one that contains all of root, lemma, part of speech and inflectional features, has been in use the longest and has gone through volunteer review (v0.4, 77,429 words).

**Terms of use and how we comply:**

| QAC term | Compliance |
|---|---|
| GNU license; the file may only be copied and distributed verbatim, changing it is not allowed | The morphology pack contains the original QAC file **unmodified**, with its copyright notice (`qac/quranic-corpus-morphology-0.4.txt` + `qac/NOTICE`). The SQLite tables are derived from this file; the conversion changes only the format — no tag is added, removed or corrected. |
| The source must be clearly indicated | "Source: Quranic Arabic Corpus" in the word card, the Word panel, the concordance page and in exports. |
| A link to corpus.quran.com must be provided | In the same places and on the "About → Sources" screen. |
| The copyright notice must be reproduced in derived works containing a substantial portion of the file | `qac/NOTICE` ships with the pack and is shown in full on the Sources screen. |
| FAQ: research-oriented, non-commercial use of the data | Siyak is non-commercial. This term is stated clearly on the pack screen and in `siyak-data/docs/sources.md`; the app's code (MIT/Apache) is not affected by it because the data is downloaded as a separate pack. |

**Consequences:**
- **No error corrections.** Tag errors reported by users are not applied to the pack; they are collected in `siyak-data/docs/qac-errata.md` and forwarded to QAC where possible. The interface may show "There is a known note for this tag" on words listed in the errata; the note comes from a separate file and does not modify QAC data.
- **Turkish labels** (human-readable Turkish/English names for parts of speech and inflectional features) live in the app's own i18n files; they are a presentation layer, not QAC data.
- **Maintenance:** QAC has not been updated since 2011. This is stability, not risk: the version is pinned as `0.4` and recorded in the pack's `meta`.
- **Transliteration:** QAC's Buckwalter transliteration is converted to Arabic script in the pipeline; this conversion is tested together with its mapping table.

**Alignment:** QAC word positions (`surah:ayah:word:segment`) are matched to the Tanzil Uthmani text. `validate.py` checks, for every verse, the word count and that the concatenation of segments matches the Tanzil word (using documented normalization rules for diacritics and orthographic differences). On any mismatch the pack is not built.

**Future: MASAQ evaluation.** MASAQ (CC BY 3.0, Tanzil-based) will be evaluated after M7 as a permissively licensed alternative and as an additional layer with traditional *i'rab* tags: review of its fields (especially lemmas), Tanzil alignment, and comparison with QAC on a sample of a few hundred words. Because the schema is source-agnostic, MASAQ could replace QAC as the morphology pack without changes in application code (v1 allows only one morphology pack at a time; an additional *i'rab* layer would be a separate pack type).

**Notes for the app (from M1b):** roots keep QAC's letters, so `ءامن` has the root `ا م ن` — root search must pass hamza forms through normalization; four words merged by alignment rule M keep two tokens in `words.text_ar`; QAC lemmas are reproduced as-is, including quirks such as the assimilation shadda in `صَّمَد` (112:2) or `ٱسْتَعِينُ` (2:153) — these go to the errata list, never into the pack.

### 5.4 `content.db`: search indexes

```sql
CREATE VIRTUAL TABLE verses_fts USING fts5(
  text_norm                                    -- normalize(text_clean): a search index, not distributed text
);
CREATE VIRTUAL TABLE translations_fts USING fts5(
  text, pack_id UNINDEXED, verse_id UNINDEXED,
  tokenize = 'unicode61 remove_diacritics 2'
);
```

The vector table is created when the first vector pack is installed, not by a migration (packs are checked against the schema version, which a migration would raise for every published pack):

```sql
CREATE TABLE verse_vectors (
  pack_id    TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_id   INTEGER NOT NULL REFERENCES verses(id),
  embedding  BLOB NOT NULL,             -- dim little-endian float32
  UNIQUE (pack_id, verse_id)
);
```

The active pack's vectors are loaded into memory once and scanned per query (6,236 dot products take milliseconds). A plain table needs no SQLite extension, so `content.db` stays readable by every tool, and no `unsafe` code is needed to load one (ADR 0012).

At most one vector pack is active at a time (`semantic.vector_pack` setting, §7.4).

### 5.5 `user.db`

```sql
CREATE TABLE bookmarks (
  id          INTEGER PRIMARY KEY,
  verse_id    INTEGER NOT NULL UNIQUE,
  created_at  TEXT NOT NULL
);

CREATE TABLE notes (
  id          INTEGER PRIMARY KEY,
  verse_from  INTEGER NOT NULL,                -- for a single verse, from = to
  verse_to    INTEGER NOT NULL,
  body        TEXT NOT NULL,                   -- Markdown
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX notes_range ON notes(verse_from, verse_to);

-- Research collections
CREATE TABLE collections (
  id           INTEGER PRIMARY KEY,
  title        TEXT NOT NULL,
  description  TEXT,                           -- Markdown
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

CREATE TABLE collection_items (
  id             INTEGER PRIMARY KEY,
  collection_id  INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  position       INTEGER NOT NULL,             -- user-defined order
  kind           TEXT NOT NULL CHECK (kind IN ('verse_range','note','root','lemma','tafsir')),
  ref_json       TEXT NOT NULL,                -- e.g. {"from":262,"to":263} / {"root":"ر ح م"}
  annotation     TEXT,                         -- short item-specific note
  created_at     TEXT NOT NULL
);
CREATE INDEX collection_items_order ON collection_items(collection_id, position);

-- Tafsir summary/translation cache
CREATE TABLE tafsir_translations (
  pack_id        TEXT NOT NULL,
  tafsir_id      INTEGER NOT NULL,
  paragraph_idx  INTEGER NOT NULL DEFAULT -1,  -- -1: summary of the whole passage (not NULL: NULLs are distinct in SQLite primary keys)
  mode           TEXT NOT NULL CHECK (mode IN ('summary','full')),
  target_lang    TEXT NOT NULL,
  pack_version   TEXT NOT NULL,                -- cache is stale if the source changes
  text           TEXT NOT NULL,
  provider       TEXT NOT NULL,
  model          TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  PRIMARY KEY (pack_id, tafsir_id, paragraph_idx, mode, target_lang)
);

-- AI usage log
CREATE TABLE llm_usage (
  id             INTEGER PRIMARY KEY,
  feature        TEXT NOT NULL CHECK (feature IN ('chat','translate')),
  provider       TEXT NOT NULL,
  model          TEXT NOT NULL,
  input_tokens   INTEGER NOT NULL,
  output_tokens  INTEGER NOT NULL,
  cost_usd       REAL,                         -- NULL if the price is unknown
  created_at     TEXT NOT NULL
);

CREATE TABLE chat_sessions (
  id          INTEGER PRIMARY KEY,
  title       TEXT,
  provider    TEXT NOT NULL,
  model       TEXT NOT NULL,
  created_at  TEXT NOT NULL
);

CREATE TABLE chat_messages (
  id            INTEGER PRIMARY KEY,
  session_id    INTEGER NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
  role          TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content       TEXT NOT NULL,
  cited_verses  TEXT,                          -- JSON: [262, 263]
  sources_json  TEXT,                          -- sources that went into the RAG context
  created_at    TEXT NOT NULL
);

CREATE TABLE settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL                         -- JSON
);
```

Verse references in `user.db` cannot be foreign keys into `content.db` (separate databases). Integrity is enforced in the application layer.

### 5.6 Migrations
Ordered SQL migrations based on `PRAGMA user_version` for both databases. `user.db` is backed up to `user.db.bak` before it is migrated.

---

## 6. Pack system

### 6.1 Pack types

| Type | Format | Contents |
|---|---|---|
| `translation` | `.sqlite.zst` | A table matching the `translations` schema |
| `tafsir` | `.sqlite.zst` | `tafsir` + `tafsir_paragraphs` |
| `morphology` | `.tar.zst` (`.sqlite` + `qac/`) | `words`, `roots`, `lemmas`, `word_segments`; for QAC also the original file and `NOTICE` (§5.3.1) |
| `vectors` | `.sqlite.zst` | `(verse_id INTEGER, embedding BLOB)`, little-endian float32; `meta` names the embedding space and backend (below) |
| `model` | `.tar.zst` | `model.sqlite` (only `pack_info`), `model/model.onnx`, `model/tokenizer.json`, `model/LICENSE` (the model's license, shown on About → Sources); the model's settings are in `meta`. Only for vector packs with the `local` backend |

**Vector pack metadata** (ADR 0012). A vector pack defines its embedding space: `embedding_model`, canonical with a backend prefix (`local/…`, `openai/…`, `voyage/…`, `gemini/…`, `compat/…`), `dim` and `normalized`. `backend` says where queries are embedded:
- `local`: `model_pack` names the model pack, which is also in `requires`. Its `embedding_model` and `dim` must match.
- `api`: `provider` (`openai`, `voyage`, `gemini`, `openai_compatible`), `api_model` (the provider's model id) and optionally `api_dimensions` (equal to `dim`, for models that shorten their vectors). No model pack. The vectors are built once in the pipeline with the maintainers' key.

A vector pack with invalid metadata is listed as unsupported; the rest of the manifest stays usable.

All SQLite packs are distributed zstd-compressed and carry a `pack_info` table; the detailed format is `siyak-data/docs/pack-format.md`. Archive packs keep their extra files under `<app_data>/packs/<pack-id>/` (e.g. QAC's `NOTICE`, shown in full on About → Sources). At most one morphology pack is installed at a time in v1.

### 6.2 Bundled content
`core.db` is bundled with the installer and contains: the Tanzil Arabic text (Uthmani + Simple Clean) and metadata, a Turkish translation, and Pickthall (English). The app is therefore usable on first launch without internet access. The Turkish translation is planned to be Elmalılı Hamdi Yazır's, pending verification of a clean, properly licensed source of the original text (§18).

**Core refresh.** `core.db` carries `content_meta.core_version` (an integer, incremented by `siyak-data` whenever core content changes). On startup, if the bundled `core.db` has a higher `core_version` than `content.db`:
1. `content.db` is copied to `content.db.bak`.
2. In a single transaction, with `core.db` attached: `surahs`, `verses`, `verses_fts`, `notices` and the rows of bundled packs (`packs.bundled = 1` with their `translations` and `translations_fts` rows) are replaced from `core.db`; `content_meta` is updated.
3. Packs the user installed (`bundled = 0`) and all their tables are left untouched. This is safe because verse ids (1..6236) never change.
4. On any error the transaction is rolled back and the backup is kept; the app keeps working on the old core and reports the error code.


### 6.3 Manifest
Addresses (embedded in the app, tried in order):
1. **Primary:** `https://quran.kemalozyon.com/data/manifest.json`
2. **Fallback:** `https://github.com/siyak-app/siyak-data/releases/latest/download/manifest.json`

The primary address is independent of the GitHub organization and repository names, so a future rename never breaks older app versions. On every new data release (triggered via `repository_dispatch`) and on every site deploy, the website workflow copies `manifest.json` and `pricing.json` from the latest `siyak-data` release into `website/public/data/`. The pack `url`s inside the manifest are immutable release-asset addresses; pack files are downloaded from GitHub Releases.

```json
{
  "schema_version": 1,
  "generated_at": "2026-10-21T00:00:00Z",
  "pricing_url": "https://quran.kemalozyon.com/data/pricing.json",
  "packs": [
    {
      "id": "en.pickthall",
      "type": "translation",
      "lang": "en",
      "title": "The Meaning of the Glorious Koran",
      "author": "Marmaduke Pickthall",
      "version": "1.0.0",
      "min_app_version": "0.1.0",
      "size": 1900000,
      "sha256": "…",
      "url": "https://github.com/siyak-app/siyak-data/releases/download/data-2026.10.0/en.pickthall.sqlite",
      "license": "Public domain",
      "license_terms": null,
      "attribution": null,
      "citation": { "type": "translation", "author": "…", "title": "…", "publisher": "…", "year": "…" }
    },
    {
      "id": "vectors.<model>",
      "type": "vectors",
      "requires": ["model.<model>"],
      "meta": { "embedding_model": "local/<model>", "dim": 384, "source_text": "Arabic (Tanzil Simple Clean), Turkish (Diyanet Vakfı), English (Pickthall)", "normalized": true,
                "backend": "local", "model_pack": "model.<model>" }
    },
    {
      "id": "model.<model>",
      "type": "model",
      "meta": { "embedding_model": "local/<model>", "dim": 384, "query_prefix": "query: ", "passage_prefix": "passage: ",
                "max_tokens": 512, "pooling": "mean" }
    },
    {
      "id": "vectors.openai-3-small",
      "type": "vectors",
      "terms_ack": true,
      "meta": { "embedding_model": "openai/text-embedding-3-small", "dim": 512, "source_text": "Arabic (Tanzil Simple Clean), Turkish (Diyanet Vakfı), English (Pickthall)", "normalized": true,
                "backend": "api", "provider": "openai", "api_model": "text-embedding-3-small", "api_dimensions": 512 }
    }
  ],
  "bundles": [
    {
      "id": "recommended",
      "title": { "tr": "Önerilen paketler", "en": "Recommended packs" },
      "description": { "tr": "Kök analizi, anlamsal arama ve bir tefsir", "en": "Root analysis, semantic search and a tafsir" },
      "packs": ["morphology.qac", "vectors.<model>", "model.<model>", "ar.jalalayn"]
    }
  ]
}
```

**Bundles:** sets installed with one click instead of making users pick packs one by one. A casual user sees a "Recommended packs" card on first launch and installs it with one click, with the total size shown. They never meet concepts like "vector pack" or "model pack"; picking individual packs stays under "Advanced" on the Packs screen. Pack and bundle titles may be given as `{ "tr", "en" }` objects.

### 6.4 Installation flow
1. Fetch the manifest: on launch and on the Packs screen, cached for 1 hour. Without network, the last successful manifest is used.
2. Check `min_app_version`; incompatible packs are listed with an "update the app" notice.
3. Resolve `requires` dependencies and show the user the total size.
4. Stream the download to `<app_data>/downloads/<id>.part`. Progress is reported through the `pack://progress` event. Downloads can be cancelled and resume via HTTP Range.
5. **Verify sha256.** On mismatch, delete the file and report an error.
6. Install:
   - SQLite packs: in a single transaction, `ATTACH` → delete existing rows for the `pack_id` → `INSERT … SELECT` only from the expected tables/columns → update FTS → upsert `packs` → `DETACH`. A vector pack is checked first: one vector per verse of `content.db`, each `dim` finite values, of unit length if `normalized`.
   - `model` (and archive packs): extract to a temporary directory, then move atomically (rename) to `<app_data>/packs/<id>/`.
7. Delete the downloaded file.

**Update:** a reinstall through the same flow. If a tafsir is updated, entries in `tafsir_translations` with the old `pack_version` are marked "source updated"; the user may translate again.
**Removal:** packs with `bundled = 0` can be removed; a `VACUUM` is suggested afterwards.

### 6.5 Licensing
- Every pack's `license`, `attribution` and source link are listed on the "About → Sources" screen, and are shown on the Packs screen before download.
- Attributions required by a source (e.g. Tanzil, CC BY 3.0) are always visible.
- The code license (MIT/Apache) does not change the license of data packs. Packs with more restrictive terms (e.g. the QAC morphology pack: no-modification and research-use terms) are distributed as separate packs and clearly marked on the Packs screen. The manifest has a `license_terms` field (a short list of terms, TR/EN) which the Packs screen shows before download.

### 6.6 Tafsir packs: sources and selection

**Source: Quranpedia data dumps** (`quranpedia.net/dumps`). 149 tafsir books, keyed per verse, one `.json.gz` per book, with SHA-256 hashes, versioned (`built_at`), and with a changes API.

**How we comply with the Quranpedia license:**

| Term | Compliance |
|---|---|
| Classical tafsirs are shared heritage; Quranpedia's compilation work is free to use in apps and research tools | Classical tafsir packs are distributed on this basis |
| Translations and contemporary works belong to their authors and publishers | No contemporary works in v1 (rule below) |
| Redistribution as a downloadable dataset must credit Quranpedia.net with a link and the dump version | Each tafsir pack's `attribution`: "Source: Quranpedia.net · dump <version>" + link; shown on the Packs screen, in the tafsir panel and on the Sources screen. Also listed in `siyak-data/docs/sources.md` |
| Distributing outdated text is the distributor's responsibility | The data pipeline queries the changes API on every release; packs for changed books are published with a new version. The app notifies users of updates for installed tafsir packs |
| Data is provided as-is | The UI notes "The text is the source's edition; please report errors"; errors are forwarded to Quranpedia |

**Selection rule (v1):** only pre-modern classical works (author died in the 19th century or earlier). Works from the 20th century onwards (Elmalılı's tafsir, al-Maraghi, al-Manar, al-Sha'rawi, al-Sabuni, al-Zuhayli, al-Tafsir al-Muyassar, etc.) are not included in v1; later versions evaluate them work by work, depending on their license status.

**Verification:** The same title can appear under several Quranpedia IDs (a different work or a different edition). In the pipeline, author, work and edition are verified one by one from the book metadata for every pack and recorded in `siyak-data/sources/tafsir/<pack-id>/README.md`. No pack is built until its ID mapping is verified.

**v1 tafsir list:**

| Pack ID | Work | Author (died AH/CE) | Note |
|---|---|---|---|
| `ar.tabari` | Jāmiʿ al-Bayān | al-Ṭabarī (310/923) | The primary source of transmission-based (*riwāya*) tafsir |
| `ar.ibn-kathir` | Tafsīr al-Qurʾān al-ʿAẓīm | Ibn Kathīr (774/1373) | The most widely read classical tafsir; **default chat tafsir** |
| `ar.razi` | Mafātīḥ al-Ghayb | Fakhr al-Dīn al-Rāzī (606/1210) | Reason-based (*dirāya*) and theological; very large |
| `ar.zamakhshari` | al-Kashshāf | al-Zamakhsharī (538/1144) | Language and rhetoric; the Muʿtazilī approach is stated in the pack description |
| `ar.baydawi` | Anwār al-Tanzīl | al-Bayḍāwī (685/1286 approx.) | The standard tafsir of Ottoman madrasas |
| `ar.jalalayn` | Tafsīr al-Jalālayn | al-Maḥallī (864/1459) & al-Suyūṭī (911/1505) | Concise; **default chat tafsir** |
| `ar.baghawi` | Maʿālim al-Tanzīl | al-Baghawī (516/1122) | Mid-sized transmission-based tafsir |
| `ar.nasafi` | Madārik al-Tanzīl | al-Nasafī (710/1310) | Ḥanafī-Māturīdī tradition |
| `ar.maturidi` | Taʾwīlāt Ahl al-Sunna | al-Māturīdī (333/944) | Māturīdī theological tradition |
| `ar.abussuud` | Irshād al-ʿAql al-Salīm | Ebussuud Efendi / Abū al-Suʿūd (982/1574) | Ottoman Shaykh al-Islām |
| `ar.bursevi` | Rūḥ al-Bayān | İsmail Hakkı Bursevî (1137/1725) | Ottoman; mystical (*ishārī*) interpretation |
| `ar.alusi` | Rūḥ al-Maʿānī | Shihāb al-Dīn Maḥmūd al-Ālūsī (1270/1854) | The last great synthesis of the classical tradition; very large. Quranpedia's metadata names his grandson Maḥmūd Shukrī al-Ālūsī (d. 1924); corrected in the pack README and reported to Quranpedia |
| `ar.suyuti-durr` | al-Durr al-Manthūr | al-Suyūṭī (911/1505) | Compilation of transmitted reports; does not comment on every verse (~4,400 covered) |
| `ar.ibn-atiyya` | al-Muḥarrar al-Wajīz | Ibn ʿAṭiyya (541/1147) | Andalusian tradition |
| `ar.abu-hayyan` | al-Baḥr al-Muḥīṭ | Abū Ḥayyān (745/1344) | Grammar-focused |
| `ar.shawkani` | Fatḥ al-Qadīr | al-Shawkānī (1250/1834) | Combines transmission and reason |
| `ar.qurtubi` | al-Jāmiʿ li-Aḥkām al-Qurʾān | al-Qurṭubī (671/1273) | Legal (*aḥkām*) tafsir; source QUL, publication depends on Tarteel's reply (see below) |

Pack descriptions (TR/EN) state the work's method and focus in one sentence (transmission, reason, language, law, mystical), so that a casual user also understands which tafsir to choose and why.

**al-Qurtubi — source: QUL (Tarteel), resource #23.** Since al-Qurtubi is not in the Quranpedia tafsir dumps, it is taken from QUL. QUL data comes grouped by verse (`from_ayah`, `to_ayah`, `group_ayah_key`), so it converts directly to the range schema in §5.2; HTML tags in the text (`<b>`, `<i>`) are reduced to a safe subset in the pipeline, the content is not changed.

- **Permission:** QUL states no origin or license for this resource; its FAQ asks users to check the license of each resource. A written request is sent to Tarteel asking for the origin of the text (edition/*taḥqīq*) and for permission to redistribute it unmodified in an open-source, non-commercial application.
- **Technical integration does not wait for permission:** `build_tafsir_qul.py` is added to the pipeline; the pack is built and tested in development.
- **Publication gate:** the `ar.qurtubi` pack is not included in a data release (it does not appear in the `manifest`) until Tarteel replies. If the reply is positive, it ships with the next data release; no app release is needed. The reply (permission text, requested attribution) is recorded in `siyak-data/sources/tafsir/ar.qurtubi/PERMISSION.md`.
- **Attribution:** "Source: Quranic Universal Library (Tarteel)" + link, plus the original edition if stated in the reply.
- If a removal request arrives, the pack is removed from the manifest in the next data release.

---

## 7. Search, word analysis and cross-references

### 7.1 One search box and modes
The app has a single search box. Modes: **All** (default) · **Text** · **Root** · **Meaning**. In the reader (simple) defaults the mode chips are hidden and only the All behaviour runs; "see all" links on the results page switch to the relevant mode.

**All — query classification and grouped results.** `classify_query` (Rust) classifies the query in this order:

| Input | Example | Behaviour |
|---|---|---|
| Verse reference | `2:255`, `2:153-157`, `Bakara 255`, `Baqarah 255` | No search; the reader jumps to that verse |
| Arabic letters | `صبر`, `الصبر`, `ص ب ر` | A root/lemma card on top, Arabic text matches below |
| Latin-script root | `s-b-r` | Root card |
| Other text | `sabır`, `patience in hard times` | First exact matches in translations, then a "Similar in meaning" group |

Each group shows its first 3–5 results; "see all" opens the full results page of that mode. If a required pack is not installed, that group shows a pack download card instead of results.

**Text mode:** §7.2. **Root mode:** §7.3. **Meaning mode:** §7.4.

**Modes that require packs:** The Root (morphology) and Meaning (model + vectors) chips are neither hidden nor shown as disabled when their pack is missing. On click they show a card: "This search needs the X pack (size) · Download". The user thus learns that the feature exists.

### 7.2 Text search (FTS5)
- **In translations:** `translations_fts`, limited to the selected translation packs.
- **In Arabic:** `verses_fts`. The query passes through the Rust counterpart of the pipeline's diacritics-stripping rules. Both implementations are tested against `tests/normalize_vectors.json`. Diacritics are ignored (the query `الصبر` finds `الصَّبْرِ`).
- Exact phrase search in quotes: `"sabredenlerle beraberdir"`.
- Filters: translation packs, surah range, Meccan/Medinan.
- Results come back highlighted via `snippet()`.

### 7.3 Root and dictionary-form (lemma) search
- Input: an Arabic root (`ص ب ر`, `صبر`), a dictionary form, or a Latin-script spelling (`s-b-r`).
- The result is a concordance: the dictionary forms derived from the root with the count of each, the distribution per surah, and the list of verses in context.
- The result can be narrowed by picking a dictionary form from the list.
- The "Where this root occurs" link in the Word panel opens the same page.

### 7.4 Semantic search

```rust
pub enum TextKind { Query, Passage }
pub enum Locality { Local, Remote(ApiProvider) }

pub trait Embedder: Send + Sync {
    fn space(&self) -> &EmbeddingSpace;   // embedding_model, dim, normalized
    fn locality(&self) -> Locality;
    fn embed(&self, texts: Vec<String>, kind: TextKind) -> EmbedFuture<'_>;  // boxed future
}
```

- **Two backends** (ADR 0012), chosen by the active vector pack's metadata (§6.1):
  - **Local (default):** `OnnxEmbedder`, configured from the model pack's metadata (prefixes, pooling, token limit). Works offline and without a key; inference runs on a blocking thread. It loads `model/model.onnx` (int8 weights) and `model/tokenizer.json` from the installed model pack, runs ONNX Runtime (statically linked through the `ort` crate, no separate library to install), feeds `input_ids` and `attention_mask` plus `token_type_ids` or `position_ids` when the model has them, embeds in batches of 16, pools (`mean`, `cls` or `last`) and normalizes. Verse vectors are computed in the pipeline the same way, from the same files, so queries and verses share one space.
  - **API:** OpenAI, Voyage, Gemini or an OpenAI-compatible server (Ollama, LM Studio). Uses the provider's key from the keychain (§14.2), shared with the AI layer (§8); an OpenAI-compatible server needs the address the user entered (`semantic.base_url`). Claude is not listed because Anthropic offers no embedding model.
- **Queries and passages** are embedded differently where the model expects it (e5 `query: `/`passage: ` prefixes, Voyage `input_type`, Gemini task types); each backend maps `TextKind` itself.
- **Active pack:** at most one vector pack is active: the one named by `semantic.vector_pack`, else the first installed one.
- **Status:** `get_semantic_status` returns why semantic features are or are not available: `no_vector_pack`, `invalid`, `model_pack_missing`, `mismatch`, `key_missing`, `base_url_missing`, `not_built` or `ready` (with the provider if queries leave the computer). The interface turns each into an action (install, add key, enter address).
- **Lazy loading:** the embedder is built on first use and kept until either pack's version or the address changes.
- **Compatibility:** if the embedder's `embedding_model` and `dim` differ from the active vector pack's, semantic features are disabled and the interface explains why.
- **Privacy:** with an API pack, queries (and in chat the ranked tafsir paragraphs, §10) are sent to the provider through `net.rs` (§14.1). The pack is under "Advanced" and says so before download (`terms_ack`).
- **Query:** cosine similarity of the query vector with every verse vector of the active pack, in memory (§5.4); the `k` best, optionally leaving out verses (the verse a cross-reference starts from).
- Embeddings are computed **per verse**. Tafsir is never embedded; it reaches the chat through the verses it belongs to (§10).
- **Verse text:** each verse is embedded as its Arabic text (Tanzil Simple Clean), Turkish translation (Diyanet Vakfı) and English translation (Pickthall), one per line, as one vector. Whatever the model, this scored best on the evaluation set: Arabic alone was weakest for Turkish and English queries, and separate vectors per language (averaged or the best of three) were worse than one vector of the combined text. The text is recorded in each vector pack (`source_text`).
- **Evaluation set:** `siyak-data/data/eval/semantic.toml`, drafted by the maintainers: Turkish, English and Arabic queries and cross-references (a verse and related verses elsewhere), each with the verses a good result list should contain, checked against the Diyanet Vakfı translation. `siyak-data eval-embeddings` reports hit@10, recall@10 and MRR@10 per language, for models and for built packs exactly as the app runs them. A new model or API pack is measured against the same set.

### 7.5 Word panel
Opens when a word is clicked in the reader:
- Morphology segment by segment: prefix/stem/suffix, part of speech, root, lemma, features (with human-readable Turkish/English labels).
- "Where this root occurs (N)" and "Where this lemma occurs (N)" links lead to the concordance.
- The word, its root or its lemma can be added to a collection.

### 7.6 Cross-references
Related verses for a verse are merged from two sources:
1. **Semantic similarity:** KNN with the verse's own vector (k=10, excluding the verse itself). Requires a vector pack.
2. **Shared roots/lemmas:** an overlap score weighted by the inverse frequency (IDF) of the verse's roots/lemmas. Requires a morphology pack. This keeps very frequent roots (e.g. "قول") from dominating.

Results are shown in two separate sections, with their reasons ("similar in meaning", "shared roots: ر ح م, غ ف ر"). The user always sees why a verse is considered related. If a curated cross-reference dataset is found, it is added as a third source (§18).

---

## 8. AI provider layer

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream(&self, req: LlmRequest, out: Channel<LlmEvent>) -> Result<Usage>;
}
```

| Provider | Note |
|---|---|
| Anthropic | Messages API, streaming |
| OpenAI | streaming |
| Google Gemini | streaming |
| OpenAI-compatible | The user enters a `base_url`. Fully offline with Ollama/LM Studio; many models with one key through aggregators such as OpenRouter. The key is optional. |

**Model roles** (chosen separately in settings):
- `chat_model`: for chat.
- `translation_model`: for tafsir summaries/translations. By default the cheapest model of the selected provider is suggested.

**Model capability flags** (user-adjustable, with defaults for known models):
- `arabic_sources`: can the model reliably read classical Arabic sources? Off by default for local/small models.

---

## 9. Tafsir summaries and translations

### 9.1 Modes
- **Summary (default):** a short Turkish (UI-language) summary of the passage. Cheap, because the output is short.
- **Full translation:** paragraph by paragraph. The first paragraph is translated; the rest follows on "translate more".

### 9.2 Flow
1. Check the cache (`tafsir_translations`). If present, show it for free.
2. Otherwise show the **estimated cost** next to the button: input tokens are estimated locally with a tokenizer approximation, output with a per-mode estimate, multiplied by `pricing.json`. E.g. "Summarize (≈ 0.3 ¢)".
3. If the user confirms, the request goes to `translation_model` and the result is streamed.
4. The result is cached, and the actual token counts are written to `llm_usage`.

### 9.3 Display rules
- A translation/summary is always shown **next to the original**, never in place of it.
- Persistent label: "Machine translation · <provider>/<model> · <date>". Clicking it highlights the corresponding original paragraph.
- System prompt: when translating technical (legal, theological) terms, keep the original Arabic term in parentheses; add no commentary; flag uncertain phrases.

### 9.4 Spending control
- Settings → AI → Spending: spending this month and in total (per feature and model), computed from `llm_usage`.
- Optional monthly limit: when reached, a warning is shown before the action (it does not block; it asks for confirmation).
- `pricing.json` lives in the `siyak-data` release and can be updated without an app release. For models with unknown prices (e.g. local) cost is shown as "unknown/free".

```json
{
  "updated_at": "2026-10-21",
  "currency": "USD",
  "models": {
    "anthropic/<model>": { "input_per_mtok": 0.0, "output_per_mtok": 0.0 }
  }
}
```

### 9.5 Out of scope for v1: community translations
Users submitting a translation as a contribution to `siyak-data`, and reviewed translations being distributed as a "community translation" pack. Not implemented until a review process is defined.

---

## 10. AI chat (RAG)

1. Embed the user's message and fetch the top-k (default 8) verses from the active vector pack.
2. For each verse add the Arabic text and the selected translation(s).
3. **Automatically** add the tafsirs of the retrieved verses, within a context budget:
   - **Chat tafsirs** are selected separately (`settings.chat.tafsirs`, at most 3). All installed tafsirs are available in the reader; only this selection goes to the chat. Default: al-Jalalayn (concise) + Ibn Kathir (mid-sized). If a selected tafsir's pack is not installed, it is skipped and the interface says so.
   - **Budget by verse rank:** the total tafsir budget (default ~6,000 tokens, adjusted to the model's context window) is spread over the 3 most relevant verses; the remaining verses get only the Arabic text and translation.
   - **Paragraph selection for long passages:** if a passage exceeds the per-verse budget, its `tafsir_paragraphs` are ranked against the question on the fly with the active embedder (§7.4; with an API pack the paragraphs are sent to its provider), and the most relevant paragraphs are added, in their original order, until the budget is full. No persistent tafsir vectors are stored; only the few dozen paragraphs of that passage are embedded at query time. Skipped paragraphs are marked "[…]" in the context.
   - **Language:** tafsirs in the UI language can always be added. Arabic tafsirs are added only if `chat_model.arabic_sources = true`; the model then quotes the original wording when it cites them. If the flag is off, the interface warns: "The selected model cannot read Arabic tafsirs; only translations were used."
   - **Cost:** if the context exceeds a user-defined threshold, the estimated cost is shown next to the send button (§9.4).
4. System prompt:
   - Rely only on the provided context and give a `[surah:ayah]` citation for every claim; name the tafsir when quoting one.
   - Say so explicitly when the context is insufficient.
   - Do not issue legal rulings (*fatwā*); refer such questions to qualified scholars.
5. `[surah:ayah]` references in the answer are parsed, stored in `cited_verses` and shown as clickable links. All sources that went into the RAG context are stored in `sources_json` and can be opened as "sources used".
6. Chat does not work without a vector pack; the interface suggests downloading one.
7. A chat answer, or part of it, can be added to a collection as a note (labelled "AI answer").

Persistent notice: "Answers are generated by AI and may contain errors; they are not a religious ruling."

---

## 11. Collections and export

### 11.1 Collections
- One research topic = one collection (e.g. "Patience in the Quran").
- Items: verse/range, note, root, lemma, tafsir passage. Ordered by drag and drop; each item can have a short annotation.
- The reader, search, concordance, cross-reference and chat screens all have an "Add to collection" action.

### 11.2 Export
- **Markdown:** collection title and description; for each item the Arabic text, selected translation(s), the user's notes and a bibliography. The save dialog is opened from Rust.
- **Citation text:** copyable citations for verses and translations/tafsirs, generated from the packs' `citation` metadata. Templates live in separate files (`export/citation/*.tmpl`), so adding a style needs no code change. v1 styles: İSNAD and APA (provisional, §18).
- **JSON backup:** bookmarks, notes, collections and the tafsir translation cache. Chat history and API keys are not included.

```json
{
  "format": "siyak-userdata",
  "version": 1,
  "exported_at": "…",
  "bookmarks": [{ "surah": 2, "ayah": 255, "created_at": "…" }],
  "notes": [{ "from": "2:255", "to": "2:255", "body": "…", "created_at": "…", "updated_at": "…" }],
  "collections": [{ "title": "…", "description": "…", "items": [{ "kind": "verse_range", "ref": { "from": "2:153", "to": "2:157" }, "annotation": "…" }] }],
  "tafsir_translations": [{ "pack_id": "…", "surah": 2, "ayah": 255, "paragraph_idx": 0, "mode": "summary", "text": "…", "provider": "…", "model": "…", "pack_version": "…" }]
}
```

For portability, exports use `surah:ayah` instead of `verse_id`. A `note` item refers to a note by its position in the file's `notes` (`{ "note": 3 }`), as notes have no portable id. Notes, collections and items also carry their timestamps, and tafsir translations `target_lang` and `created_at`, so a backup restores what it saved. Import runs in `merge` or `replace` mode and returns a report; merging the same file twice adds nothing, and `replace` first keeps a copy of `user.db` beside it.

---

## 12. IPC contract

All commands return `Result<T, AppError>` (`{ code, message }`). Types are exported automatically to `src/lib/bindings.ts` with `specta` + `tauri-specta`.

| Area | Commands |
|---|---|
| Content | `list_surahs`, `get_verses(surah, translation_ids[])` (with the words of each verse when a morphology pack is installed), `get_verse_words(verse_id)`, `get_compare(from, to, translation_ids[], tafsir_ids[])` |
| Tafsir | `list_tafsirs`, `get_tafsir(verse_id, pack_id)`, `get_tafsir_introduction(pack_id)`, `estimate_tafsir_translation(tafsir_id, paragraph_idx?, mode)`, `translate_tafsir(…, channel)` |
| Morphology | `get_morphology_pack`, `get_word(word_id)`, `concordance(root_id \| lemma_id, translation_id?, limit, offset)`, `search_roots(query)` |
| Search | `classify_query(query)`, `search_text(query, scope, pack_ids[], filters, limit)`, `search_semantic(query, limit)`, `semantic_status()` |
| Cross-refs | `related_verses(verse_id)` → `{ semantic[], shared_roots[] }` |
| Packs | `fetch_manifest(force?)`, `install_pack(id)`, `install_bundle(id)`, `cancel_install(id)`, `uninstall_pack(id)` |
| User | `toggle_bookmark`, `list_bookmarks`, `upsert_note`, `delete_note`, `list_notes(range?)` |
| Collections | `list_collections`, `get_collection`, `upsert_collection`, `delete_collection`, `add_collection_item`, `update_collection_item`, `move_collection_item`, `remove_collection_item` |
| Export | `export_collection_markdown(id, translation_ids[], labels, style)`, `list_citation_styles()`, `format_citation(ref, style)`, `export_user_data()`, `import_user_data(mode)` |
| AI | `set_api_key`, `delete_api_key`, `has_api_key`, `list_models(provider)`, `chat_send(…, channel)`, `list_chat_sessions`, `get_chat_session`, `delete_chat_session`, `get_usage_summary(period)` |
| System | `get_settings`, `set_setting`, `get_network_log(limit)` |

Events: `pack://progress`, `pack://done`, `pack://error`. Streaming (chat, translation) uses `ipc::Channel`.

---

## 13. Frontend

### 13.1 Stack
React + TypeScript + Vite, Tailwind CSS + shadcn/ui, TanStack Router (deep links: `/read/2/255`, `/compare/2/153-157`, `/root/رحم`), TanStack Query, TanStack Virtual, Zustand, react-i18next, react-markdown (HTML disabled), dnd-kit (collection ordering).

### 13.2 Main layout and progressive disclosure
One interface, three panes:
- **Left:** navigation (surahs, collections, search history).
- **Center:** main content (reader / comparison / search results / concordance).
- **Right:** context panel with tabs: Tafsir · Word · Related · Notes · Chat. It follows the selected verse or word.

Panes are collapsible and resizable; the layout is persisted in `settings`.

**Experience profile.** On first launch a single question is asked: "How would you like to use the app? Read and understand / Do research". The answer is stored as `settings.experience_profile` (`reader` | `researcher`) and **only sets defaults**. No feature is locked; every default can be changed individually in Settings.

| Default (`settings` key) | `reader` | `researcher` |
|---|---|---|
| Right panel (`layout.right_panel`) | Closed | Open, Word tab |
| Number of translations (`reader.translations`) | 1 | 2 |
| Verse actions (`reader.actions`) | Icons on hover | Always visible, labelled |
| Word click (`word.click`) | Small card (meaning, root, occurrence count, "Detailed analysis") | Full analysis in the right panel |
| Word underline (`word.underline`) | None | Dotted underline |
| Technical terms (`ui.technical_terms`) | Hidden (in tooltips) | Shown in parentheses |
| Search mode chips (`search.mode_chips`) | Hidden (All only) | Visible |
| Copy citation (`reader.citation_action`) | Hidden | Visible |

**Component principle:** both profiles use the same components; no separate screens are written. For example, "Detailed analysis" in the small word card opens the Word tab in the right panel. The PR template asks "How does this look / hide in the reader (simple) defaults?" for every new feature.

**Terms:** i18n files keep technical terms as plain/technical pairs (`term.lemma.plain` = "sözlük biçimi" / "dictionary form", `term.lemma.technical` = "lemma"). Other examples: concordance → "geçtiği yerler" / "where it occurs"; semantic search → "anlamına göre ara" / "search by meaning".

Reference mockup: the "Okuma Ekranı Mockup" canvas (reader and researcher defaults side by side).

### 13.3 Screens
1. **Reader:** virtualized verse list; a basmala header for every surah except al-Fatiha (where it is verse 1) and at-Tawba; Arabic (clickable word by word when morphology is installed) + selected translations; bookmark, note, compare and add-to-collection actions on every verse.
2. **Compare:** the user's chosen translations and tafsirs for a verse/range in parallel columns; add/remove/reorder columns; rows aligned per verse; summary/translation buttons in tafsir columns.
3. **Concordance:** root/lemma header, derived forms, distribution per surah (small bar chart), verse list in context (keyword highlighted).
4. **Search:** All (grouped) · Text · Root · Meaning (§7.1).
5. **First launch:** experience profile question, then the "Recommended packs" card (§6.3 bundles).
6. **Collections:** list + detail; drag-and-drop ordering; export.
7. **Chat:** sessions, messages, clickable citations, "sources used".
8. **Packs:** grouped by type; size, license, version, progress.
9. **Settings:** appearance (theme, fonts, sizes), language, AI (providers, model roles, capability flags, spending), privacy (network log), About → Sources.

### 13.4 Typography and direction
- Fonts are bundled; no CDN. Default Arabic font: Amiri Quran (SIL OFL). Licenses of alternative fonts are verified separately.
- Arabic blocks use `dir="rtl" lang="ar"`; translation blocks use the pack's `lang`.
- Only logical Tailwind utilities are used (`ms-`, `me-`, `ps-`, `pe-`, `start-`, `end-`).
- Separate font size and line height settings for Arabic and translations (Arabic default ≥ 2.0).

### 13.5 Accessibility and keyboard
↑/↓ verse navigation, `b` bookmark, `n` note, `t` tafsir, `w` word panel, `r` related, `c` compare, `Ctrl/Cmd+K` command palette. Visible focus rings.

---

## 14. Trust and security

### 14.1 Network
- All HTTP requests go through a single client in `net.rs`, which has a fixed host allow-list:
  - `quran.kemalozyon.com` (manifest, pricing, update metadata)
  - `github.com`, `objects.githubusercontent.com` (packs, updates)
  - Provider API hosts (Anthropic, OpenAI, Gemini; Voyage for embeddings)
  - The `base_url` the user explicitly entered for an OpenAI-compatible provider
- **Connection log:** for each request the host, time, purpose (`pack`, `update`, `chat`, `translate`, `embed`) and status are recorded; no content is logged. The last 500 entries are kept in memory and in `user.db`, viewable under Settings → Privacy.

### 14.2 API keys
- Stored in the OS keychain (service: `com.kemalozyon.siyak`, account: `<provider>`). Never returned to the webview; the interface only learns "saved / not saved".
- Guidance on the add-key screen:
  1. "Create a separate key for this app."
  2. "Set a spending limit for this key/project in your provider's dashboard."
  3. "Your key is only sent from this computer to the provider you chose. If in doubt, revoke it in your provider's dashboard."
  4. Links to the provider dashboard and to the relevant source code in the app (`secrets.rs`, `net.rs`).

### 14.3 Application
- Tauri v2 capabilities: only the app's own commands are exposed to the webview; no `fs`, `http` or `shell` plugin permissions.
- Strict CSP: `default-src 'self'`; no external scripts.
- Packs are verified with sha256, updates with the Tauri updater signature.
- Pack `.sqlite` files are treated as untrusted input: data is copied only from the expected tables/columns; no triggers, views or SQL from a pack are ever executed.
- Markdown (notes, chat) is rendered with HTML disabled.

### 14.4 Verifiable builds
- Releases are built only in GitHub Actions; no binaries are uploaded by hand.
- Build provenance is published for every installer with `actions/attest-build-provenance`. The README contains the verification command.

---

## 15. Build, release and updates

### 15.1 Application
- Pushing a `v*` tag runs `tauri-apps/tauri-action`, building for Windows (NSIS/MSI), macOS (universal `.dmg`) and Linux (`.AppImage`, `.deb`); a draft release is created, and `latest.json` and provenance are uploaded.
- The updater signing key is generated with `tauri signer generate`; the private key is stored in the app repository's secrets (not as an organization secret).
- Updater endpoints (tried in order): `https://quran.kemalozyon.com/updates/latest.json`, then `https://github.com/siyak-app/siyak/releases/latest/download/latest.json`. The website copies `latest.json` when the release workflow completes.
- OS code signing is optional in v1 (§18). If builds are unsigned, the download page includes SmartScreen/Gatekeeper instructions.
- The ONNX Runtime native library is bundled for each platform.

### 15.2 Data
- Pushing a `data-YYYY.MM.N` tag runs the pipeline: packs are built → `validate.py` → sha256 → `manifest.json` + `pricing.json` → release.
- **Validation gates:**
  - 114 surahs, 6,236 verses.
  - 6,236 rows in every translation pack.
  - Valid tafsir ranges, no empty paragraphs.
  - Morphology word segmentation matches the Tanzil text for every verse.
  - For the QAC pack: the sha256 of the original file equals the known v0.4 hash (proof it is unmodified); the number of derived rows equals the number of segments in the file; `NOTICE` is present.
  - 6,236 rows with the correct dimension in the vector pack.
  - Diacritics-stripping test vectors pass.
- Vector and model packs are built in the same pipeline run, with the same model version.

### 15.3 Download page
Part of the landing page (§16.2).

### 15.4 Versioning
- App: SemVer; during `v0.x`, breaking changes may land in minor versions.
- `CHANGELOG.md` and release notes are generated automatically from Conventional Commits (release-please or git-cliff). Release notes are generated in English; a Turkish summary is added to the release notes page on the website.
- Data: `data-YYYY.MM.N` tags; every pack has its own SemVer version.

---

## 16. Open source, documentation and contribution

### 16.1 Language policy

| Area | Language |
|---|---|
| Code: identifiers, comments, doc comments, log and error-message keys | **English only** |
| Commit messages, branch names, PR titles and descriptions | **English only** |
| Test names, CI output, `ARCHITECTURE.md`, `docs/adr/`, `docs/pack-format.md` | **English only** |
| This specification | English is the source of truth; Turkish translation in `docs/spec/spec.tr.md` |
| `README`, `CONTRIBUTING` | English original, Turkish counterpart (`*.tr.md`) |
| Website: landing page, user guide, developer guide | Turkish + English |
| App interface | Turkish + English (i18n files) |
| Issues and Discussions | Both languages accepted; tagged with labels (`lang:tr`, `lang:en`) |

**Rules:**
- No user-visible text is hard-coded; every string is an i18n key (enforced by an ESLint rule). The Rust side returns error **codes**, not user-facing text (`AppError.code`).
- Islamic terms are written in code with consistent English transliteration and defined in a single glossary (`docs/glossary.md`: `surah`, `ayah`, `tafsir`, `lemma`, `root`…). Turkish words (`sure`, `meal`, `ayet`) are never used in code.
- In bilingual documents the English version is the source of truth. Turkish pages start with a line "Last synced with the English version: <commit/date>"; Starlight automatically shows the English page where no translation exists.

### 16.2 Website (landing + documentation)
- **Technology:** Astro + Starlight in `website/` of the app repository. Hosting: GitHub Pages (`.github/workflows/website.yml`, on every merge to `main` and on every data release).
- **Domain:** `quran.kemalozyon.com`. A DNS CNAME record for `quran` pointing to `siyak.github.io`; custom domain and "Enforce HTTPS" in the app repository's Pages settings.
- The site also publishes the fixed addresses the app uses: `/data/manifest.json`, `/data/pricing.json`, `/updates/latest.json`.
- **i18n:** Starlight's built-in multilingual support: `/tr/…` and `/en/…`; redirect based on browser language.
- **Pages:**
  - **Home:** what it is, screenshots (reader and researcher defaults), core principles (offline, free, open source, no key needed), download button.
  - **Download:** OS detection; `releases/latest/download/<fixed-name>` links (no version numbers in installer names); version label from the GitHub API with a fallback to the releases page; SmartScreen/Gatekeeper instructions for unsigned builds; build provenance verification command.
  - **User guide:** getting started, packs, search modes, root analysis, collections, AI features (with guidance on getting a key and setting a spending limit).
  - **Sources and licenses:** compiled from `siyak-data/docs/sources.md`.
  - **Developer guide:** setup, architecture (`ARCHITECTURE.md`), ADRs, pack format, contribution guide.
  - **Release notes.**
- The website contains no telemetry or third-party scripts; apart from the GitHub API it makes no external requests.

### 16.3 Making contribution easy

**The 15-minute goal:** a new contributor should be able to run the app within 15 minutes of cloning the repository.
- One command: `pnpm install && pnpm tauri dev`.
- `dev-data/dev-content.db`: sample data with a few surahs and translations; it grows with the milestones (partial morphology in M5, a small vector set in M7). Used in dev builds with `SIYAK_DEV_DATA=dev`; by default they use the committed `core.db`, so published packs install. Either way, contributors never need to run the Python pipeline. Size target < 5 MB.
- `.devcontainer/`: Rust, Node, pnpm and the Linux webview dependencies ready to go.
- Platform prerequisites in `CONTRIBUTING.md` (Windows: WebView2/MSVC; macOS: Xcode CLT; Linux: WebKitGTK packages).

**Ways to contribute** (including for people who don't write code):

| Path | Where | Guide |
|---|---|---|
| Code | App repository | `CONTRIBUTING.md`, `good first issue` labels |
| New translation/tafsir sources, license research | `siyak-data` | `docs/adding-a-source.md`, "New source proposal" issue form (license field required) |
| UI translation (new language) | `src/i18n/` | `docs/translating.md` |
| Reviewing Turkish labels for morphology tags and term translations; reporting QAC tag errors (errata) | Issues/Discussions | "Term correction" and "QAC tag error" issue forms |
| Documentation | `website/` | "Edit this page" link on every page |

**Project management:**
- GitHub Issues + a GitHub Project board (milestones match §17).
- Labels: `good first issue`, `help wanted`, `area:*` (`reader`, `search`, `morphology`, `packs`, `llm`, `website`, `data`), `lang:*`, `needs-license-check`.
- GitHub Discussions for ideas and questions; issues for actionable work.
- Architectural changes are first discussed as an ADR proposal (a PR adding `docs/adr/NNNN-*.md`).

### 16.4 Code quality and CI
Rules are enforced by CI, not just written down. `ci.yml` runs on every PR:

| Area | Checks |
|---|---|
| Rust | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo doc` without warnings |
| Frontend | ESLint (incl. the i18n rule), Prettier, `tsc --noEmit`, Vitest |
| Bindings | Check that the `tauri-specta` bindings are up to date (fail on diff) |
| Commits | Conventional Commits (`feat:`, `fix:`, `docs:`…) checked on the PR title; squash merge |
| Build | Builds on all three platforms (without release signing) |
| Website | Astro build + broken-link check |
| `siyak-data` | `ruff`, `pytest`, `validate.py` on sample data |

**Readable code:**
- Every Rust module starts with a `//!` comment explaining what it does and why it exists; every public function has a `///` doc comment. `cargo doc` builds without warnings in CI.
- TSDoc on public hooks and components in TypeScript.
- Complex logic (diacritics stripping, IDF scoring, the pack install transaction, the RAG context budget) gets a "why" comment and a reference to the relevant ADR/spec section. "What it does" should be readable from the code itself.
- Module boundaries match the "to change X, look at Y" map in `ARCHITECTURE.md`.

**The PR template** asks: what changed and why; linked issue; how it was tested; screenshots (for UI changes); "How does this look in the reader (simple) defaults?" (§13.2); were new user-facing strings added in both languages; was documentation updated.

**Definition of done (for every milestone and PR):** code + tests + relevant documentation (English) + a bilingual guide update if user-facing.

### 16.5 Repository files

| File | Contents |
|---|---|
| `README.md` / `README.tr.md` | What it is, screenshot, download link, development setup in 5 steps, links to ways of contributing |
| `CONTRIBUTING.md` / `.tr.md` | Setup, branch/commit conventions, PR process, running tests, ways to contribute |
| `ARCHITECTURE.md` | Layers, data flow, module map, core invariants (e.g. "the webview never touches the network") |
| `docs/adr/` | Decision records. Initial set: 0001 Tauri v2 + React; 0002 two databases; 0003 tafsir is never embedded; 0004 on-demand Arabic tafsir translation; 0005 progressive disclosure; 0006 `MIT OR Apache-2.0`; 0007 QAC morphology, unmodified; 0008 classical tafsirs from Quranpedia, no contemporary works in v1; 0009 English-only code, bilingual docs; 0010 Rust `xtask` for dev tooling (dev DB, bindings) |
| `docs/glossary.md` | Glossary (English code name · Turkish · Arabic) |
| `docs/spec/spec.md` / `spec.tr.md` | This specification and its Turkish translation |
| `CODE_OF_CONDUCT.md` (org `.github`) | Contributor Covenant |
| `SECURITY.md` (org `.github`) | Vulnerability reporting (GitHub private vulnerability reporting); API keys and pack integrity in scope |
| `CHANGELOG.md` | Generated automatically |

### 16.6 GitHub organization settings
- Plan: free. Owner: the project owner (sole owner). New maintainers get per-repository `maintain`/`write` access.
- 2FA required for all members.
- `main` is protected: PRs required, no merge without green CI, at least one approval (with an exemption for the owner's own PRs while there is a single maintainer), squash merge.
- Release and updater signing keys are repository secrets, not organization secrets; only the release workflow can access them.
- Private vulnerability reporting enabled.
- Organization profile page (`.github/profile/README.md`): project introduction (TR + EN), website, download and contribution links.

---

## 17. Milestones

| # | Scope | Done when |
|---|---|---|
| M0 | Skeleton + open-source foundation | Tauri + React + Tailwind + shadcn + i18n + specta; three-pane layout; `ci.yml` with all checks; `dev-content.db`; README, CONTRIBUTING (bilingual), ARCHITECTURE, glossary, initial ADRs, issue/PR templates, devcontainer |
| M1 | Data pipeline | `core.db`, translation packs, normalization test vectors, validation gates, manifest, `siyak-data` docs (`pack-format`, `adding-a-source`, `sources`) |
| M2 | Reader | Surah list, virtualized verses, translation selection, fonts, RTL, experience profile and its defaults |
| M3 | Packs | Manifest, bundles, download/verify/install/remove, progress UI, Sources screen |
| M4 | Tafsir + compare + search | Tafsir panel (with paragraphs), compare screen, single search box (All + Text), FTS5 |
| M5 | Morphology | Morphology pack, clickable words, word card and panel, Root mode, concordance |
| M6 | Collections | Bookmarks, notes, collections, Markdown/citation/JSON export |
| M7 | Semantic layer | Model choice, local (ONNX) and API embedders, vector pack, Meaning mode, cross-references |
| M8 | AI | Provider layer, keychain, network allow-list + log, tafsir summary/translation + cost, RAG chat |
| M9 | Release + website | Updater, provenance, landing page, user guide (both languages), v0.1.0 |

Ordering rationale: the open-source foundation comes first (M0), because adding it later is expensive and loses early contributors. Value without AI comes first (M2–M6); at the end of M6 the app is a publishable research tool that needs no API key.

---

## 18. Open questions

| # | Question | Impact |
|---|---|---|
| 1 | Embedding model: ~~the default local model~~ **answered (M7):** BGE-M3 (ADR 0012). Still open: which API models get a pack | Pack size, search and cross-reference quality |
| 2 | ~~Text to embed~~ **Answered (M7):** Arabic, Turkish and English combined, one vector per verse (§7.4) | Vector pack design |
| 3 | ~~Evaluation set~~ **Answered (M7):** drafted by the maintainers, 144 items: 58 Turkish, 30 English and 20 Arabic queries, 36 cross-references (§7.4) | M7 acceptance criteria |
| 4 | **MASAQ evaluation** (§5.3.1): does it have lemmas, Tanzil alignment, sample comparison with QAC; clarify the license wording of the latest version | Second morphology pack, *i'rab* layer |
| 5 | **al-Qurtubi permission:** origin and redistribution permission for QUL #23 from Tarteel (§6.6). Request sent? Reply received? | Publication of the `ar.qurtubi` pack |
| 6 | **Turkish translation source:** a clean, digital, clearly licensed source of Elmalılı Hamdi Yazır's original (non-simplified) translation for `core.db`, and of his tafsir *Hak Dini Kur'an Dili* | Bundled Turkish translation (§6.2), Turkish tafsir layer |
| 7 | v1 citation styles: İSNAD and APA are implemented provisionally; to be reviewed against the style guides. Chicago and the `citation` fields each pack needs are still open | §11.2 |
| 8 | Is there a curated cross-reference dataset with a suitable license? | Third source in §7.6 |
| 9 | Will OS code signing be done in v1? (Apple Developer Program has a yearly fee; a Windows certificate costs extra) | Installation experience |
| 10 | Embedding licenses of alternative Arabic fonts such as KFGQPC Uthmanic Script HAFS | Font options |
