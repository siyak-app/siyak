-- content.db schema v1: spec §5.1 (core), §5.2 (tafsir), §5.3 (morphology), §5.4 (search indexes).
-- Copied from docs/spec/spec.md. Vector tables (vec0) are created per pack at install time (M7).

-- §5.1 core

CREATE TABLE surahs (
  number          INTEGER PRIMARY KEY,         -- 1..114
  name_ar         TEXT NOT NULL,
  name_tr         TEXT NOT NULL,
  name_en         TEXT NOT NULL,
  revelation_type TEXT NOT NULL CHECK (revelation_type IN ('meccan','medinan')),
  ayah_count      INTEGER NOT NULL
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

-- §5.2 tafsir

CREATE TABLE tafsir (
  id          INTEGER PRIMARY KEY,
  pack_id     TEXT NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  verse_from  INTEGER NOT NULL REFERENCES verses(id),
  verse_to    INTEGER NOT NULL REFERENCES verses(id),
  CHECK (verse_to >= verse_from)
);
CREATE INDEX tafsir_lookup ON tafsir(pack_id, verse_from, verse_to);

CREATE TABLE tafsir_paragraphs (
  tafsir_id  INTEGER NOT NULL REFERENCES tafsir(id) ON DELETE CASCADE,
  idx        INTEGER NOT NULL,                 -- starts at 0
  text       TEXT NOT NULL,
  PRIMARY KEY (tafsir_id, idx)
) WITHOUT ROWID;

-- §5.3 morphology

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

-- §5.4 search indexes
-- verses_fts is an external-content index: it is filled with
-- INSERT INTO verses_fts(verses_fts) VALUES ('rebuild') after verses are written.

CREATE VIRTUAL TABLE verses_fts USING fts5(
  text_clean, content='verses', content_rowid='id'
);
CREATE VIRTUAL TABLE translations_fts USING fts5(
  text, pack_id UNINDEXED, verse_id UNINDEXED,
  tokenize = 'unicode61 remove_diacritics 2'
);
