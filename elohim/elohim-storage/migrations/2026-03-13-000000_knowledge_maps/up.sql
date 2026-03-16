-- knowledge_maps: Source of truth: SQLite (operational)
-- Classification: C (Operational) — personal sensemaking, reconstructable from content relationships

-- Knowledge maps (domain, self, person, collective)
CREATE TABLE IF NOT EXISTS knowledge_maps (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    map_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    subject_name TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'private',
    shared_with_json TEXT,
    nodes_json TEXT NOT NULL,
    path_ids_json TEXT,
    overall_affinity REAL NOT NULL DEFAULT 0.0,
    content_graph_id TEXT,
    mastery_levels_json TEXT,
    goals_json TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_maps_app_id ON knowledge_maps(app_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_owner ON knowledge_maps(app_id, owner_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_type ON knowledge_maps(app_id, map_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_subject ON knowledge_maps(app_id, subject_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_visibility ON knowledge_maps(app_id, visibility);
