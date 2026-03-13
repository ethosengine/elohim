-- Path extensions (user customizations/forks)
CREATE TABLE IF NOT EXISTS path_extensions (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    base_path_id TEXT NOT NULL,
    base_path_version TEXT NOT NULL,
    extended_by TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    insertions_json TEXT,
    annotations_json TEXT,
    reorderings_json TEXT,
    exclusions_json TEXT,
    visibility TEXT NOT NULL DEFAULT 'private',
    shared_with_json TEXT,
    forked_from TEXT,
    forks_json TEXT,
    upstream_proposal_json TEXT,
    stats_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (base_path_id) REFERENCES paths(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_path_extensions_app_id ON path_extensions(app_id);
CREATE INDEX IF NOT EXISTS idx_path_extensions_base ON path_extensions(app_id, base_path_id);
CREATE INDEX IF NOT EXISTS idx_path_extensions_extended_by ON path_extensions(app_id, extended_by);
CREATE INDEX IF NOT EXISTS idx_path_extensions_visibility ON path_extensions(app_id, visibility);
