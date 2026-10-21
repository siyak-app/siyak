-- user.db schema v1: spec §5.5. Copied from docs/spec/spec.md.

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
  paragraph_idx  INTEGER NOT NULL DEFAULT -1,  -- -1: summary of the whole passage
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
