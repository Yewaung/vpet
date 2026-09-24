-- MVP 2: language, learning and activity.
ALTER TABLE pet ADD COLUMN conversations INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pet ADD COLUMN language_stage INTEGER NOT NULL DEFAULT 0;

CREATE TABLE vocabulary (
    word       TEXT PRIMARY KEY,
    exposures  INTEGER NOT NULL,
    learned_at INTEGER,
    source     TEXT    NOT NULL
);

-- One row per activity category; stats are small and change together.
CREATE TABLE interest (
    category  TEXT PRIMARY KEY,
    stat_json TEXT NOT NULL
);

-- Structured sessions only: app stem (or "unknown") + category. No titles.
CREATE TABLE activity (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at    INTEGER NOT NULL,
    duration_secs REAL    NOT NULL,
    app           TEXT    NOT NULL,
    category      TEXT    NOT NULL,
    confidence    REAL    NOT NULL
);
CREATE INDEX activity_started ON activity (started_at);

-- Rolling window of recent chat lines.
CREATE TABLE conversation (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    at        INTEGER NOT NULL,
    from_user INTEGER NOT NULL,
    text      TEXT    NOT NULL
);
