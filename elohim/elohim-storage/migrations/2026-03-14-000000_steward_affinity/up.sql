-- Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from economic_events curation acts
-- Reconstruction: SELECT steward_id, content_id, SUM(value) FROM economic_events WHERE action='curate' GROUP BY steward_id, content_id

CREATE TABLE IF NOT EXISTS steward_affinity (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    steward_id TEXT NOT NULL,
    content_id TEXT NOT NULL,
    affinity_score REAL NOT NULL DEFAULT 0.0,
    source TEXT NOT NULL DEFAULT 'genesis_seed',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_steward_affinity_unique
    ON steward_affinity(app_id, steward_id, content_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_app_id
    ON steward_affinity(app_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_steward
    ON steward_affinity(app_id, steward_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_content
    ON steward_affinity(app_id, content_id);
