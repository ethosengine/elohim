-- Recreate the path tables that were dropped in the up migration.
-- These are the original CREATE TABLE statements from the initial migration
-- plus dht_anchor_hash columns added by later provenance migrations.

CREATE TABLE paths (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    title TEXT NOT NULL,
    description TEXT,
    path_type TEXT NOT NULL DEFAULT 'guided',
    difficulty TEXT DEFAULT 'beginner',
    estimated_duration TEXT,

    -- Display
    thumbnail_url TEXT,
    thumbnail_alt TEXT,

    -- Metadata as JSON
    metadata_json TEXT,

    -- Visibility
    visibility TEXT NOT NULL DEFAULT 'public',

    -- Authorship
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- DHT provenance (added by lamad_provenance migration)
    dht_anchor_hash TEXT
);

CREATE INDEX idx_paths_app_id ON paths(app_id);
CREATE UNIQUE INDEX idx_paths_app_unique ON paths(app_id, id);

-- Path tags
CREATE TABLE path_tags (
    app_id TEXT NOT NULL DEFAULT 'lamad',
    path_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (app_id, path_id, tag),
    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

-- Chapters (optional grouping within paths)
CREATE TABLE chapters (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    path_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    order_index INTEGER NOT NULL DEFAULT 0,
    estimated_duration TEXT,

    -- DHT provenance (added by lamad_provenance migration)
    dht_anchor_hash TEXT,

    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

CREATE INDEX idx_chapters_app_id ON chapters(app_id);

-- Steps (the actual learning items in a path)
CREATE TABLE steps (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    path_id TEXT NOT NULL,
    chapter_id TEXT,

    -- Step content
    title TEXT NOT NULL,
    description TEXT,
    step_type TEXT NOT NULL DEFAULT 'learn',

    -- Reference to content (optional - some steps are just text)
    resource_id TEXT,
    resource_type TEXT DEFAULT 'content',

    -- Ordering
    order_index INTEGER NOT NULL DEFAULT 0,

    -- Duration
    estimated_duration TEXT,

    -- Metadata as JSON
    metadata_json TEXT,

    -- DHT provenance (added by lamad_provenance migration)
    dht_anchor_hash TEXT,

    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE SET NULL
);

CREATE INDEX idx_steps_app_id ON steps(app_id);

-- Attestations granted upon path completion
CREATE TABLE path_attestations (
    app_id TEXT NOT NULL DEFAULT 'lamad',
    path_id TEXT NOT NULL,
    attestation_type TEXT NOT NULL,
    attestation_name TEXT NOT NULL,
    -- DHT provenance (added by imagodei_provenance migration)
    dht_anchor_hash TEXT,
    PRIMARY KEY (app_id, path_id, attestation_type),
    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

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
