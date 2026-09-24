-- Care mistakes and the family line (V-pet care; true evolution across
-- generations). Ancestors are kept when a new generation starts.
ALTER TABLE pet ADD COLUMN life_json TEXT NOT NULL DEFAULT '{}';
CREATE TABLE ancestor (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    generation   INTEGER NOT NULL,
    name         TEXT,
    stage        TEXT    NOT NULL,
    genome_json  TEXT    NOT NULL,
    personality_json TEXT NOT NULL,
    born_at      INTEGER NOT NULL,
    retired_at   INTEGER NOT NULL
);
