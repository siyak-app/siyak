-- content.db schema v3: packs.type 'audio' (recitation packs, spec §6.1, ADR 0014).
-- Matches siyak-data/schema/content.sql (SCHEMA_VERSION = 3).
--
-- SQLite cannot change a CHECK constraint in place, so packs is rebuilt with the
-- procedure of https://sqlite.org/lang_altertable.html#otheralter. This migration runs
-- with foreign keys off (`foreign_keys_off` in db/content.rs): dropping packs would
-- otherwise cascade into every installed translation and tafsir. The runner checks
-- the foreign keys before it commits.

CREATE TABLE packs_v3 (
  id            TEXT PRIMARY KEY,              -- e.g. 'en.pickthall'
  type          TEXT NOT NULL CHECK (type IN
                  ('translation','tafsir','morphology','vectors','model','audio')),
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

INSERT INTO packs_v3 (id, type, lang, title, author, version, license, attribution,
                      citation_json, meta_json, installed_at, bundled)
SELECT id, type, lang, title, author, version, license, attribution,
       citation_json, meta_json, installed_at, bundled
FROM packs;

DROP TABLE packs;
ALTER TABLE packs_v3 RENAME TO packs;
