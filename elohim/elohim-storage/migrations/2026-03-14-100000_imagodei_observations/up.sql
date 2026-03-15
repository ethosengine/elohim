CREATE TABLE IF NOT EXISTS imagodei_observations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    content TEXT NOT NULL,
    structured_signals_json TEXT,
    trust_delta REAL NOT NULL DEFAULT 0.0,
    visibility_layer TEXT NOT NULL DEFAULT 'individual',
    originating_elohim TEXT NOT NULL,
    relevance_decay REAL NOT NULL DEFAULT 0.0,
    superseded_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_imagodei_obs_human ON imagodei_observations(human_id);
CREATE INDEX IF NOT EXISTS idx_imagodei_obs_type ON imagodei_observations(observation_type);

-- Add session intent columns to local_sessions
-- Using separate statements; ALTER TABLE ADD COLUMN is idempotent-safe
-- in SQLite (fails silently if column exists when wrapped in a migration).
ALTER TABLE local_sessions ADD COLUMN session_intent_json TEXT;
ALTER TABLE local_sessions ADD COLUMN intent_set_at TEXT;
