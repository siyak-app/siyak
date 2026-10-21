-- content.db schema v2: spec §5.1 (surahs.bismillah, notices, content_meta), §5.2 (tafsir.kind,
-- tafsir_paragraphs.part/page as a rowid table) and §5.4 (verses_fts over normalized text).
-- Matches siyak-data/schema/content.sql (SCHEMA_VERSION = 2); the shapes are compared by tests.

-- §5.1 core

-- Tanzil's basmala for the surah header; NULL for 1 (where it is verse 1) and 9.
ALTER TABLE surahs ADD COLUMN bismillah TEXT;

-- Notices that must ship with the data, shown in full on About → Sources.
CREATE TABLE notices (
  id      TEXT PRIMARY KEY,                    -- e.g. 'tanzil.text.uthmani' or a pack id
  source  TEXT NOT NULL,                       -- source id, e.g. 'tanzil'
  title   TEXT NOT NULL,
  body    TEXT NOT NULL,                       -- reproduced verbatim
  url     TEXT
);

-- Versions of the core data: 'core_version' (an integer as text, compared as an
-- integer; drives the core refresh of spec §6.2) and 'tanzil_version'.
CREATE TABLE content_meta (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL
);

-- §5.2 tafsir

ALTER TABLE tafsir ADD COLUMN kind TEXT NOT NULL DEFAULT 'commentary'
  CHECK (kind IN ('commentary','introduction'));  -- introduction: the author's, at 1:1

-- A rowid table: WITHOUT ROWID roughly doubles disk use for long text rows.
-- SQLite cannot change a table's primary key in place, so the table is rebuilt.
CREATE TABLE tafsir_paragraphs_v2 (
  tafsir_id  INTEGER NOT NULL REFERENCES tafsir(id) ON DELETE CASCADE,
  idx        INTEGER NOT NULL,                 -- starts at 0
  text       TEXT NOT NULL,                    -- small safe HTML subset (spec §5.2)
  part       TEXT,                             -- volume of the printed edition, if known
  page       INTEGER,                          -- page of the printed edition, if known
  UNIQUE (tafsir_id, idx)
);
INSERT INTO tafsir_paragraphs_v2 (tafsir_id, idx, text)
  SELECT tafsir_id, idx, text FROM tafsir_paragraphs;
DROP TABLE tafsir_paragraphs;
ALTER TABLE tafsir_paragraphs_v2 RENAME TO tafsir_paragraphs;

-- §5.4 search indexes
-- verses_fts holds normalize(text_clean) with rowid = verses.id (siyak-data
-- docs/normalization.md); it is filled by the data pipeline, never displayed.

DROP TABLE verses_fts;
CREATE VIRTUAL TABLE verses_fts USING fts5(text_norm);
