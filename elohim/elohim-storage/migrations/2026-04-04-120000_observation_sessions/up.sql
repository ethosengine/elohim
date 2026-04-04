-- Source of truth: SQLite (operational, Category C).
-- Sessions and entries are ephemeral working data — purgeable after report composition.
-- The composed report is persisted as a Content node (Category A, dht_anchor_hash via content table).
-- No dht_anchor_hash on these tables: observations are not notarized.

CREATE TABLE observation_sessions (
    -- Source of truth: SQLite (operational)
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    ttl_seconds INTEGER NOT NULL DEFAULT 300,
    source TEXT NOT NULL,
    metadata_json TEXT,
    report_content_id TEXT
);

CREATE INDEX idx_obs_sessions_started ON observation_sessions(started_at);
CREATE INDEX idx_obs_sessions_source ON observation_sessions(source);

CREATE TABLE observation_entries (
    -- Source of truth: SQLite (operational)
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES observation_sessions(id),
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    origin TEXT NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    method TEXT,
    path TEXT,
    status_code INTEGER,
    message TEXT NOT NULL,
    context_json TEXT
);

CREATE INDEX idx_obs_entries_session ON observation_entries(session_id);
