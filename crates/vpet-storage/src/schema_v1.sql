-- One row: the single pet this install is raising.
CREATE TABLE pet (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    seed          INTEGER NOT NULL,
    rng_state     INTEGER NOT NULL,
    stage         TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    hatched_at    INTEGER,
    age_secs      REAL    NOT NULL,
    incubation    REAL    NOT NULL,
    generation    INTEGER NOT NULL,
    care_count    INTEGER NOT NULL,
    last_seen     INTEGER NOT NULL,
    needs_json    TEXT    NOT NULL,
    behavior_json TEXT    NOT NULL,
    day_log_json  TEXT    NOT NULL,
    critical_json TEXT    NOT NULL
);

CREATE TABLE personality (
    trait TEXT PRIMARY KEY,
    value REAL NOT NULL
);

CREATE TABLE relationship (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    trust       REAL NOT NULL,
    affection   REAL NOT NULL,
    familiarity REAL NOT NULL
);

-- Genome per generation; the previous row is the parent genome.
CREATE TABLE creature_genome (
    generation      INTEGER PRIMARY KEY,
    genome_json     TEXT    NOT NULL,
    visual_dna_json TEXT    NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE TABLE evolution (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    generation          INTEGER NOT NULL,
    from_stage          TEXT    NOT NULL,
    to_stage            TEXT    NOT NULL,
    genome_changes_json TEXT    NOT NULL,
    at                  INTEGER NOT NULL
);

CREATE TABLE memory (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       TEXT    NOT NULL,
    tag        TEXT    NOT NULL,
    summary    TEXT    NOT NULL,
    importance INTEGER NOT NULL CHECK (importance BETWEEN 1 AND 5),
    created_at INTEGER NOT NULL
);
CREATE INDEX memory_created ON memory (created_at);

CREATE TABLE events (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    at           INTEGER NOT NULL,
    name         TEXT    NOT NULL,
    payload_json TEXT    NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
