-- user.db schema v2 (M3): the connection log (spec §14.1) and the pack terms
-- the user has acknowledged before a first download (spec §6.5).

-- One row per HTTP request (and per redirect hop): no URL path, query or content.
CREATE TABLE network_log (
  id       INTEGER PRIMARY KEY,
  host     TEXT NOT NULL,
  at       TEXT NOT NULL,                     -- 'YYYY-MM-DDTHH:MM:SSZ'
  purpose  TEXT NOT NULL CHECK (purpose IN ('manifest','pack','update','chat','translate')),
  status   INTEGER,                           -- HTTP status; NULL if no response arrived
  bytes    INTEGER NOT NULL DEFAULT 0         -- response body bytes received
);

-- "I have read the terms" for packs whose manifest entry sets terms_ack.
CREATE TABLE pack_terms_acks (
  pack_id          TEXT NOT NULL,
  version          TEXT NOT NULL,
  acknowledged_at  TEXT NOT NULL,
  PRIMARY KEY (pack_id, version)
) WITHOUT ROWID;
