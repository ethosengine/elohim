-- Initial Diesel migration with multi-tenant app scoping
-- This migration creates the complete schema with app_id on all app-scoped tables

-- Schema version tracking (global, no app_id)
CREATE TABLE schema_version (
    version INTEGER NOT NULL
);
INSERT INTO schema_version (version) VALUES (2);

-- App registry for validation and future features (quotas, permissions)
CREATE TABLE apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    enabled INTEGER NOT NULL DEFAULT 1
);

-- Default apps: lamad for learning content, elohim for shared infrastructure
INSERT INTO apps (id, name, description) VALUES
    ('lamad', 'Lamad', 'Learning platform content - paths, concepts, quizzes, assessments'),
    ('elohim', 'Elohim', 'Shared infrastructure - resources, sensemaking, attestations, coordination');

-- Content metadata table
-- Actual content body is stored in blob_store, referenced by blob_hash
CREATE TABLE content (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    title TEXT NOT NULL,
    description TEXT,
    content_type TEXT NOT NULL DEFAULT 'concept',
    content_format TEXT NOT NULL DEFAULT 'markdown',

    -- Blob reference (content body in blob_store)
    blob_hash TEXT,
    blob_cid TEXT,
    content_size_bytes INTEGER,

    -- Metadata as JSON (flexible schema)
    metadata_json TEXT,

    -- Visibility and status
    reach TEXT NOT NULL DEFAULT 'public',
    validation_status TEXT NOT NULL DEFAULT 'valid',

    -- Authorship
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Tags stored separately for efficient querying
-- app_id included in composite PK for app isolation
CREATE TABLE content_tags (
    app_id TEXT NOT NULL DEFAULT 'lamad',
    content_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (app_id, content_id, tag),
    FOREIGN KEY (content_id) REFERENCES content(id) ON DELETE CASCADE
);

-- Learning paths
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
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

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

    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

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

    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE SET NULL
);

-- Attestations granted upon path completion
CREATE TABLE path_attestations (
    app_id TEXT NOT NULL DEFAULT 'lamad',
    path_id TEXT NOT NULL,
    attestation_type TEXT NOT NULL,
    attestation_name TEXT NOT NULL,
    PRIMARY KEY (app_id, path_id, attestation_type),
    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- App scoping indexes (CRITICAL for query performance)
CREATE INDEX idx_content_app_id ON content(app_id);
CREATE INDEX idx_paths_app_id ON paths(app_id);
CREATE INDEX idx_chapters_app_id ON chapters(app_id);
CREATE INDEX idx_steps_app_id ON steps(app_id);

-- Unique constraints for app-scoped IDs (safety)
CREATE UNIQUE INDEX idx_content_app_unique ON content(app_id, id);
CREATE UNIQUE INDEX idx_paths_app_unique ON paths(app_id, id);

-- Content indexes
CREATE INDEX idx_content_type ON content(content_type);
CREATE INDEX idx_content_format ON content(content_format);
CREATE INDEX idx_content_reach ON content(reach);
CREATE INDEX idx_content_created_at ON content(created_at);
CREATE INDEX idx_content_blob_hash ON content(blob_hash);

-- Tag indexes (include app_id for scoped queries)
CREATE INDEX idx_content_tags_app_tag ON content_tags(app_id, tag);
CREATE INDEX idx_path_tags_app_tag ON path_tags(app_id, tag);

-- Path indexes
CREATE INDEX idx_paths_type ON paths(path_type);
CREATE INDEX idx_paths_difficulty ON paths(difficulty);
CREATE INDEX idx_paths_visibility ON paths(visibility);

-- Step indexes
CREATE INDEX idx_steps_path_id ON steps(path_id);
CREATE INDEX idx_steps_chapter_id ON steps(chapter_id);
CREATE INDEX idx_steps_resource_id ON steps(resource_id);
CREATE INDEX idx_steps_order ON steps(path_id, order_index);

-- Chapter indexes
CREATE INDEX idx_chapters_path_id ON chapters(path_id);
CREATE INDEX idx_chapters_order ON chapters(path_id, order_index);
