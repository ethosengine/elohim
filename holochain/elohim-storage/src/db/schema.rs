//! Database schema definitions

use rusqlite::Connection;
use tracing::info;

use crate::error::StorageError;

/// Current schema version for migrations
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize the database schema
pub fn init_schema(conn: &Connection) -> Result<(), StorageError> {
    // Check current schema version
    let current_version = get_schema_version(conn)?;

    if current_version == 0 {
        info!("Creating new database schema v{}", SCHEMA_VERSION);
        create_tables(conn)?;
        set_schema_version(conn, SCHEMA_VERSION)?;
    } else if current_version < SCHEMA_VERSION {
        info!("Migrating schema from v{} to v{}", current_version, SCHEMA_VERSION);
        migrate_schema(conn, current_version)?;
    } else {
        info!("Database schema is up to date (v{})", current_version);
    }

    Ok(())
}

/// Get current schema version (0 if not initialized)
fn get_schema_version(conn: &Connection) -> Result<i32, StorageError> {
    // Create schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        [],
    ).map_err(|e| StorageError::Internal(format!("Failed to create schema_version table: {}", e)))?;

    let version: i32 = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(version)
}

/// Set schema version
fn set_schema_version(conn: &Connection, version: i32) -> Result<(), StorageError> {
    conn.execute("DELETE FROM schema_version", [])
        .map_err(|e| StorageError::Internal(format!("Failed to clear schema_version: {}", e)))?;
    conn.execute("INSERT INTO schema_version (version) VALUES (?)", [version])
        .map_err(|e| StorageError::Internal(format!("Failed to set schema_version: {}", e)))?;
    Ok(())
}

/// Create all tables
fn create_tables(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(CONTENT_SCHEMA)
        .map_err(|e| StorageError::Internal(format!("Failed to create content tables: {}", e)))?;

    conn.execute_batch(PATHS_SCHEMA)
        .map_err(|e| StorageError::Internal(format!("Failed to create paths tables: {}", e)))?;

    conn.execute_batch(INDEXES_SCHEMA)
        .map_err(|e| StorageError::Internal(format!("Failed to create indexes: {}", e)))?;

    Ok(())
}

/// Migrate schema from older version
fn migrate_schema(conn: &Connection, from_version: i32) -> Result<(), StorageError> {
    // Add migration steps here as schema evolves
    match from_version {
        // Example: 1 -> 2 migration would go here
        _ => {}
    }

    set_schema_version(conn, SCHEMA_VERSION)?;
    Ok(())
}

/// Content table schema
const CONTENT_SCHEMA: &str = r#"
-- Content metadata table
-- Actual content body is stored in blob_store, referenced by blob_hash
CREATE TABLE IF NOT EXISTS content (
    id TEXT PRIMARY KEY NOT NULL,
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
CREATE TABLE IF NOT EXISTS content_tags (
    content_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (content_id, tag),
    FOREIGN KEY (content_id) REFERENCES content(id) ON DELETE CASCADE
);
"#;

/// Paths and steps schema
const PATHS_SCHEMA: &str = r#"
-- Learning paths
CREATE TABLE IF NOT EXISTS paths (
    id TEXT PRIMARY KEY NOT NULL,
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
CREATE TABLE IF NOT EXISTS path_tags (
    path_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (path_id, tag),
    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

-- Chapters (optional grouping within paths)
CREATE TABLE IF NOT EXISTS chapters (
    id TEXT PRIMARY KEY NOT NULL,
    path_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    order_index INTEGER NOT NULL DEFAULT 0,
    estimated_duration TEXT,

    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);

-- Steps (the actual learning items in a path)
CREATE TABLE IF NOT EXISTS steps (
    id TEXT PRIMARY KEY NOT NULL,
    path_id TEXT NOT NULL,
    chapter_id TEXT,

    -- Step content
    title TEXT NOT NULL,
    description TEXT,
    step_type TEXT NOT NULL DEFAULT 'learn',

    -- Reference to content (optional - some steps are just text)
    -- NOTE: No FK constraint on resource_id because content may be seeded
    -- independently and may not exist when paths are created
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
CREATE TABLE IF NOT EXISTS path_attestations (
    path_id TEXT NOT NULL,
    attestation_type TEXT NOT NULL,
    attestation_name TEXT NOT NULL,
    PRIMARY KEY (path_id, attestation_type),
    FOREIGN KEY (path_id) REFERENCES paths(id) ON DELETE CASCADE
);
"#;

/// Index definitions for fast queries
const INDEXES_SCHEMA: &str = r#"
-- Content indexes
CREATE INDEX IF NOT EXISTS idx_content_type ON content(content_type);
CREATE INDEX IF NOT EXISTS idx_content_format ON content(content_format);
CREATE INDEX IF NOT EXISTS idx_content_reach ON content(reach);
CREATE INDEX IF NOT EXISTS idx_content_created_at ON content(created_at);
CREATE INDEX IF NOT EXISTS idx_content_blob_hash ON content(blob_hash);

-- Tag indexes
CREATE INDEX IF NOT EXISTS idx_content_tags_tag ON content_tags(tag);
CREATE INDEX IF NOT EXISTS idx_path_tags_tag ON path_tags(tag);

-- Path indexes
CREATE INDEX IF NOT EXISTS idx_paths_type ON paths(path_type);
CREATE INDEX IF NOT EXISTS idx_paths_difficulty ON paths(difficulty);
CREATE INDEX IF NOT EXISTS idx_paths_visibility ON paths(visibility);

-- Step indexes
CREATE INDEX IF NOT EXISTS idx_steps_path_id ON steps(path_id);
CREATE INDEX IF NOT EXISTS idx_steps_chapter_id ON steps(chapter_id);
CREATE INDEX IF NOT EXISTS idx_steps_resource_id ON steps(resource_id);
CREATE INDEX IF NOT EXISTS idx_steps_order ON steps(path_id, order_index);

-- Chapter indexes
CREATE INDEX IF NOT EXISTS idx_chapters_path_id ON chapters(path_id);
CREATE INDEX IF NOT EXISTS idx_chapters_order ON chapters(path_id, order_index);
"#;
