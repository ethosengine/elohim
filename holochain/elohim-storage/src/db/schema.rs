//! Database schema definitions

use rusqlite::Connection;
use tracing::info;

use crate::error::StorageError;

/// Current schema version for migrations
pub const SCHEMA_VERSION: i32 = 6;

/// Initialize the database schema
pub fn init_schema(conn: &Connection) -> Result<(), StorageError> {
    // Check current schema version
    let current_version = get_schema_version(conn)?;

    if current_version == 0 {
        info!("Creating new database schema v{}", SCHEMA_VERSION);
        create_tables(conn)?;
        set_schema_version(conn, SCHEMA_VERSION)?;
    } else if current_version < SCHEMA_VERSION {
        info!(
            "Migrating schema from v{} to v{}",
            current_version, SCHEMA_VERSION
        );
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
    )
    .map_err(|e| StorageError::Internal(format!("Failed to create schema_version table: {}", e)))?;

    let version: i32 = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
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

    // Pillar tables (v4)
    create_pillar_tables(conn)?;

    Ok(())
}

/// Migrate schema from older version
fn migrate_schema(conn: &Connection, from_version: i32) -> Result<(), StorageError> {
    // Add migration steps here as schema evolves
    let mut current = from_version;

    // Migration: v1 -> v2: Add content_body column
    if current == 1 {
        info!("Migrating v1 -> v2: Adding content_body column");
        conn.execute("ALTER TABLE content ADD COLUMN content_body TEXT", [])
            .map_err(|e| {
                StorageError::Internal(format!("Failed to add content_body column: {}", e))
            })?;
        current = 2;
    }

    // Migration: v2 -> v3: Add relationships, knowledge_maps, path_extensions tables
    if current == 2 {
        info!("Migrating v2 -> v3: Adding graph/relationship tables");
        conn.execute_batch(RELATIONSHIPS_SCHEMA).map_err(|e| {
            StorageError::Internal(format!("Failed to create relationships table: {}", e))
        })?;
        conn.execute_batch(KNOWLEDGE_MAPS_SCHEMA).map_err(|e| {
            StorageError::Internal(format!("Failed to create knowledge_maps table: {}", e))
        })?;
        conn.execute_batch(PATH_EXTENSIONS_SCHEMA).map_err(|e| {
            StorageError::Internal(format!("Failed to create path_extensions table: {}", e))
        })?;
        conn.execute_batch(GRAPH_INDEXES_SCHEMA).map_err(|e| {
            StorageError::Internal(format!("Failed to create graph indexes: {}", e))
        })?;
        current = 3;
    }

    // Migration: v3 -> v4: Add pillar tables (presences, events, mastery, allocations, etc.)
    if current == 3 {
        info!("Migrating v3 -> v4: Adding pillar tables");
        create_pillar_tables(conn)?;
        current = 4;
    }

    // Migration: v4 -> v5: Add device_policies table
    if current == 4 {
        info!("Migrating v4 -> v5: Adding device_policies table");
        conn.execute_batch(DEVICE_POLICIES_SCHEMA).map_err(|e| {
            StorageError::Internal(format!("Failed to create device_policies table: {}", e))
        })?;
        current = 5;
    }

    // Migration: v5 -> v6: Add humans table
    if current == 5 {
        info!("Migrating v5 -> v6: Adding humans table");
        conn.execute_batch(HUMANS_SCHEMA)
            .map_err(|e| StorageError::Internal(format!("Failed to create humans table: {}", e)))?;
        current = 6;
    }

    set_schema_version(conn, current)?;
    Ok(())
}

/// Create all pillar tables (contributor_presences, economic_events, etc.)
/// Uses IF NOT EXISTS for idempotency.
fn create_pillar_tables(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(CONTRIBUTOR_PRESENCES_SCHEMA)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to create contributor_presences table: {}",
                e
            ))
        })?;
    conn.execute_batch(ECONOMIC_EVENTS_SCHEMA).map_err(|e| {
        StorageError::Internal(format!("Failed to create economic_events table: {}", e))
    })?;
    conn.execute_batch(CONTENT_MASTERY_SCHEMA).map_err(|e| {
        StorageError::Internal(format!("Failed to create content_mastery table: {}", e))
    })?;
    conn.execute_batch(STEWARDSHIP_ALLOCATIONS_SCHEMA)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to create stewardship_allocations table: {}",
                e
            ))
        })?;
    conn.execute_batch(HUMAN_RELATIONSHIPS_SCHEMA)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to create human_relationships table: {}", e))
        })?;
    conn.execute_batch(LOCAL_SESSIONS_SCHEMA).map_err(|e| {
        StorageError::Internal(format!("Failed to create local_sessions table: {}", e))
    })?;
    conn.execute_batch(COLLECTIVES_SCHEMA).map_err(|e| {
        StorageError::Internal(format!("Failed to create collectives tables: {}", e))
    })?;
    conn.execute_batch(DEVICE_POLICIES_SCHEMA).map_err(|e| {
        StorageError::Internal(format!("Failed to create device_policies table: {}", e))
    })?;
    conn.execute_batch(HUMANS_SCHEMA)
        .map_err(|e| StorageError::Internal(format!("Failed to create humans table: {}", e)))?;
    Ok(())
}

/// Content table schema
const CONTENT_SCHEMA: &str = r#"
-- Content metadata table
-- Content body can be stored inline (content_body) or in blob_store (blob_hash)
CREATE TABLE IF NOT EXISTS content (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    content_type TEXT NOT NULL DEFAULT 'concept',
    content_format TEXT NOT NULL DEFAULT 'markdown',

    -- Inline content body (for text: markdown, JSON, etc.)
    content_body TEXT,

    -- Blob reference (for large/binary content in blob_store)
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

// =============================================================================
// Schema V3: Graph/Relationship Tables
// =============================================================================

/// Relationships table schema - content graph edges
const RELATIONSHIPS_SCHEMA: &str = r#"
-- Content relationships (edges in the knowledge graph)
CREATE TABLE IF NOT EXISTS relationships (
    id TEXT PRIMARY KEY NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,  -- RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES
    confidence REAL NOT NULL DEFAULT 1.0,  -- 0.0 - 1.0
    inference_source TEXT NOT NULL DEFAULT 'explicit',  -- explicit, path, tag, semantic
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- No FK constraint on source_id/target_id as content may be seeded separately
    UNIQUE(source_id, target_id, relationship_type)
);
"#;

/// Knowledge maps table schema - user's personalized domain maps
const KNOWLEDGE_MAPS_SCHEMA: &str = r#"
-- Knowledge maps (domain, self, person, collective)
CREATE TABLE IF NOT EXISTS knowledge_maps (
    id TEXT PRIMARY KEY NOT NULL,
    map_type TEXT NOT NULL,  -- domain, self, person, collective
    owner_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    subject_name TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'private',
    shared_with_json TEXT,  -- Array of agent IDs
    nodes_json TEXT NOT NULL,  -- Graph node data
    path_ids_json TEXT,  -- Associated learning paths
    overall_affinity REAL NOT NULL DEFAULT 0.0,
    content_graph_id TEXT,  -- Reference to base content graph
    mastery_levels_json TEXT,  -- Per-node mastery data
    goals_json TEXT,  -- Learning goals
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Path extensions table schema - user customizations to paths
const PATH_EXTENSIONS_SCHEMA: &str = r#"
-- Path extensions (user customizations/forks)
CREATE TABLE IF NOT EXISTS path_extensions (
    id TEXT PRIMARY KEY NOT NULL,
    base_path_id TEXT NOT NULL,
    base_path_version TEXT NOT NULL,
    extended_by TEXT NOT NULL,  -- Agent ID who created the extension
    title TEXT NOT NULL,
    description TEXT,
    insertions_json TEXT,  -- Added steps
    annotations_json TEXT,  -- Notes on steps
    reorderings_json TEXT,  -- Step reordering
    exclusions_json TEXT,  -- Removed steps
    visibility TEXT NOT NULL DEFAULT 'private',
    shared_with_json TEXT,  -- Array of agent IDs
    forked_from TEXT,  -- Another extension this forked from
    forks_json TEXT,  -- Extensions that forked from this
    upstream_proposal_json TEXT,  -- Proposal to merge upstream
    stats_json TEXT,  -- Usage statistics
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (base_path_id) REFERENCES paths(id) ON DELETE CASCADE
);
"#;

/// Indexes for graph tables
const GRAPH_INDEXES_SCHEMA: &str = r#"
-- Relationship indexes
CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id);
CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relationship_type);
CREATE INDEX IF NOT EXISTS idx_relationships_source_type ON relationships(source_id, relationship_type);

-- Knowledge map indexes
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_owner ON knowledge_maps(owner_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_type ON knowledge_maps(map_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_subject ON knowledge_maps(subject_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_maps_visibility ON knowledge_maps(visibility);

-- Path extension indexes
CREATE INDEX IF NOT EXISTS idx_path_extensions_base ON path_extensions(base_path_id);
CREATE INDEX IF NOT EXISTS idx_path_extensions_extended_by ON path_extensions(extended_by);
CREATE INDEX IF NOT EXISTS idx_path_extensions_visibility ON path_extensions(visibility);
"#;

// ============================================================================
// Pillar table schemas (v4)
// ============================================================================

/// Contributor presences — stewardship lifecycle and recognition accumulation
const CONTRIBUTOR_PRESENCES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS contributor_presences (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    display_name TEXT NOT NULL,
    presence_state TEXT NOT NULL DEFAULT 'unclaimed',
    external_identifiers_json TEXT,
    establishing_content_ids_json TEXT NOT NULL,
    affinity_total REAL NOT NULL DEFAULT 0.0,
    unique_engagers INTEGER NOT NULL DEFAULT 0,
    citation_count INTEGER NOT NULL DEFAULT 0,
    recognition_score REAL NOT NULL DEFAULT 0.0,
    recognition_by_content_json TEXT,
    last_recognition_at TEXT,
    steward_id TEXT,
    stewardship_started_at TEXT,
    stewardship_commitment_id TEXT,
    stewardship_quality_score REAL,
    claim_initiated_at TEXT,
    claim_verified_at TEXT,
    claim_verification_method TEXT,
    claim_evidence_json TEXT,
    claimed_agent_id TEXT,
    claim_recognition_transferred_value REAL,
    claim_facilitated_by TEXT,
    image TEXT,
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_presence_app_id ON contributor_presences(app_id);
CREATE INDEX IF NOT EXISTS idx_presence_state ON contributor_presences(app_id, presence_state);
CREATE INDEX IF NOT EXISTS idx_presence_steward ON contributor_presences(steward_id);
CREATE INDEX IF NOT EXISTS idx_presence_claimed ON contributor_presences(claimed_agent_id);
CREATE INDEX IF NOT EXISTS idx_presence_recognition ON contributor_presences(recognition_score DESC);
"#;

/// Economic events — hREA/ValueFlows value tracking
const ECONOMIC_EVENTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS economic_events (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'shefa',
    action TEXT NOT NULL,
    provider TEXT NOT NULL,
    receiver TEXT NOT NULL,
    resource_conforms_to TEXT,
    resource_inventoried_as TEXT,
    resource_classified_as_json TEXT,
    resource_quantity_value REAL,
    resource_quantity_unit TEXT,
    effort_quantity_value REAL,
    effort_quantity_unit TEXT,
    has_point_in_time TEXT NOT NULL,
    has_duration TEXT,
    input_of TEXT,
    output_of TEXT,
    lamad_event_type TEXT,
    content_id TEXT,
    contributor_presence_id TEXT,
    path_id TEXT,
    triggered_by TEXT,
    state TEXT NOT NULL DEFAULT 'recorded',
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_event_app_id ON economic_events(app_id);
CREATE INDEX IF NOT EXISTS idx_event_provider ON economic_events(app_id, provider);
CREATE INDEX IF NOT EXISTS idx_event_receiver ON economic_events(app_id, receiver);
CREATE INDEX IF NOT EXISTS idx_event_action ON economic_events(action);
CREATE INDEX IF NOT EXISTS idx_event_lamad_type ON economic_events(lamad_event_type);
CREATE INDEX IF NOT EXISTS idx_event_content ON economic_events(content_id);
CREATE INDEX IF NOT EXISTS idx_event_presence ON economic_events(contributor_presence_id);
CREATE INDEX IF NOT EXISTS idx_event_path ON economic_events(path_id);
CREATE INDEX IF NOT EXISTS idx_event_time ON economic_events(has_point_in_time);
CREATE INDEX IF NOT EXISTS idx_event_state ON economic_events(state);
"#;

/// Content mastery — Bloom's taxonomy tracking with spaced repetition
const CONTENT_MASTERY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS content_mastery (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    human_id TEXT NOT NULL,
    content_id TEXT NOT NULL,
    mastery_level TEXT NOT NULL DEFAULT 'not_started',
    mastery_level_index INTEGER NOT NULL DEFAULT 0,
    freshness_score REAL NOT NULL DEFAULT 1.0,
    needs_refresh INTEGER NOT NULL DEFAULT 0,
    engagement_count INTEGER NOT NULL DEFAULT 0,
    last_engagement_type TEXT,
    last_engagement_at TEXT,
    level_achieved_at TEXT,
    content_version_at_mastery TEXT,
    assessment_evidence_json TEXT,
    privileges_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_mastery_unique ON content_mastery(app_id, human_id, content_id);
CREATE INDEX IF NOT EXISTS idx_mastery_app_id ON content_mastery(app_id);
CREATE INDEX IF NOT EXISTS idx_mastery_human ON content_mastery(app_id, human_id);
CREATE INDEX IF NOT EXISTS idx_mastery_content ON content_mastery(content_id);
CREATE INDEX IF NOT EXISTS idx_mastery_level ON content_mastery(mastery_level);
CREATE INDEX IF NOT EXISTS idx_mastery_needs_refresh ON content_mastery(needs_refresh) WHERE needs_refresh = 1;
CREATE INDEX IF NOT EXISTS idx_mastery_freshness ON content_mastery(freshness_score);
"#;

/// Stewardship allocations — content stewardship with allocation ratios
const STEWARDSHIP_ALLOCATIONS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS stewardship_allocations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    content_id TEXT NOT NULL,
    steward_presence_id TEXT NOT NULL,
    allocation_ratio REAL NOT NULL DEFAULT 1.0,
    allocation_method TEXT NOT NULL DEFAULT 'manual',
    contribution_type TEXT NOT NULL DEFAULT 'original_creator',
    contribution_evidence_json TEXT,
    governance_state TEXT NOT NULL DEFAULT 'active',
    dispute_id TEXT,
    dispute_reason TEXT,
    disputed_at TEXT,
    disputed_by TEXT,
    negotiation_session_id TEXT,
    elohim_ratified_at TEXT,
    elohim_ratifier_id TEXT,
    effective_from TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until TEXT,
    superseded_by TEXT,
    recognition_accumulated REAL NOT NULL DEFAULT 0.0,
    last_recognition_at TEXT,
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_alloc_app_id ON stewardship_allocations(app_id);
CREATE INDEX IF NOT EXISTS idx_alloc_content ON stewardship_allocations(content_id);
CREATE INDEX IF NOT EXISTS idx_alloc_steward ON stewardship_allocations(steward_presence_id);
CREATE INDEX IF NOT EXISTS idx_alloc_governance ON stewardship_allocations(governance_state);
CREATE INDEX IF NOT EXISTS idx_alloc_active ON stewardship_allocations(content_id, governance_state, effective_until);
CREATE INDEX IF NOT EXISTS idx_alloc_disputed ON stewardship_allocations(governance_state) WHERE governance_state = 'disputed';
CREATE UNIQUE INDEX IF NOT EXISTS idx_alloc_unique_active ON stewardship_allocations(
    app_id, content_id, steward_presence_id
) WHERE effective_until IS NULL AND governance_state = 'active';
"#;

/// Human relationships — identity layer with custody and consent
const HUMAN_RELATIONSHIPS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS human_relationships (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'imagodei',
    party_a_id TEXT NOT NULL,
    party_b_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    intimacy_level TEXT NOT NULL DEFAULT 'recognition',
    is_bidirectional INTEGER NOT NULL DEFAULT 0,
    consent_given_by_a INTEGER NOT NULL DEFAULT 0,
    consent_given_by_b INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_a INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_b INTEGER NOT NULL DEFAULT 0,
    auto_custody_enabled INTEGER NOT NULL DEFAULT 0,
    emergency_access_enabled INTEGER NOT NULL DEFAULT 0,
    initiated_by TEXT NOT NULL,
    verified_at TEXT,
    governance_layer TEXT,
    reach TEXT NOT NULL DEFAULT 'private',
    context_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_human_rel_unique ON human_relationships(app_id, party_a_id, party_b_id, relationship_type);
CREATE INDEX IF NOT EXISTS idx_human_rel_app_id ON human_relationships(app_id);
CREATE INDEX IF NOT EXISTS idx_human_rel_party_a ON human_relationships(app_id, party_a_id);
CREATE INDEX IF NOT EXISTS idx_human_rel_party_b ON human_relationships(app_id, party_b_id);
CREATE INDEX IF NOT EXISTS idx_human_rel_type ON human_relationships(relationship_type);
CREATE INDEX IF NOT EXISTS idx_human_rel_intimacy ON human_relationships(intimacy_level);
CREATE INDEX IF NOT EXISTS idx_human_rel_custody ON human_relationships(auto_custody_enabled) WHERE auto_custody_enabled = 1;
"#;

/// Local sessions — Tauri native identity handoff
const LOCAL_SESSIONS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS local_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    human_id TEXT NOT NULL,
    agent_pub_key TEXT NOT NULL,
    doorway_url TEXT NOT NULL,
    doorway_id TEXT,
    identifier TEXT NOT NULL,
    display_name TEXT,
    profile_image_hash TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_synced_at TEXT,
    bootstrap_url TEXT,
    UNIQUE(human_id, agent_pub_key)
);
CREATE INDEX IF NOT EXISTS idx_local_sessions_active ON local_sessions(is_active) WHERE is_active = 1;
CREATE INDEX IF NOT EXISTS idx_local_sessions_human ON local_sessions(human_id);
CREATE INDEX IF NOT EXISTS idx_local_sessions_doorway ON local_sessions(doorway_url);
"#;

/// Collectives — governance contexts with graduated participation
const COLLECTIVES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS collectives (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'qahal',
    name TEXT NOT NULL,
    description TEXT,
    governance_layer TEXT NOT NULL DEFAULT 'community',
    constitutional_parent_id TEXT,
    reach TEXT NOT NULL DEFAULT 'community',
    metadata_json TEXT,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    dissolved_at TEXT
);
CREATE TABLE IF NOT EXISTS collective_participations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'qahal',
    collective_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    intimacy_level TEXT NOT NULL DEFAULT 'recognition',
    role_context TEXT,
    governance_weight REAL NOT NULL DEFAULT 1.0,
    consent_state TEXT NOT NULL DEFAULT 'pending',
    metadata_json TEXT,
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    departed_at TEXT,
    FOREIGN KEY (collective_id) REFERENCES collectives(id),
    UNIQUE(app_id, collective_id, human_id)
);
CREATE INDEX IF NOT EXISTS idx_collectives_app ON collectives(app_id);
CREATE INDEX IF NOT EXISTS idx_collectives_layer ON collectives(governance_layer);
CREATE INDEX IF NOT EXISTS idx_participations_app ON collective_participations(app_id);
CREATE INDEX IF NOT EXISTS idx_participations_collective ON collective_participations(app_id, collective_id);
CREATE INDEX IF NOT EXISTS idx_participations_human ON collective_participations(app_id, human_id);
"#;

/// Device policies — per-subject/per-device policy rules set by stewards
const DEVICE_POLICIES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS device_policies (
    id TEXT PRIMARY KEY NOT NULL,
    subject_id TEXT NOT NULL,
    device_id TEXT,
    author_id TEXT NOT NULL,
    author_tier TEXT NOT NULL DEFAULT 'self',
    inherits_from TEXT,
    blocked_categories_json TEXT NOT NULL DEFAULT '[]',
    blocked_hashes_json TEXT NOT NULL DEFAULT '[]',
    age_rating_max TEXT,
    reach_level_max INTEGER,
    session_max_minutes INTEGER,
    daily_max_minutes INTEGER,
    time_windows_json TEXT NOT NULL DEFAULT '[]',
    cooldown_minutes INTEGER,
    disabled_features_json TEXT NOT NULL DEFAULT '[]',
    disabled_routes_json TEXT NOT NULL DEFAULT '[]',
    require_approval_json TEXT NOT NULL DEFAULT '[]',
    log_sessions INTEGER NOT NULL DEFAULT 0,
    log_categories INTEGER NOT NULL DEFAULT 0,
    log_policy_events INTEGER NOT NULL DEFAULT 1,
    retention_days INTEGER NOT NULL DEFAULT 30,
    subject_can_view INTEGER NOT NULL DEFAULT 1,
    effective_from TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_device_policies_subject ON device_policies(subject_id);
CREATE INDEX IF NOT EXISTS idx_device_policies_author ON device_policies(author_id);
CREATE INDEX IF NOT EXISTS idx_device_policies_tier ON device_policies(author_tier);
"#;

// ============================================================================
// Schema V6: Human Identity Table
// ============================================================================

/// Human identity — mutable profile data for imagodei pillar
const HUMANS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS humans (
    id TEXT PRIMARY KEY NOT NULL,
    agent_pub_key TEXT,
    display_name TEXT NOT NULL,
    bio TEXT,
    affinities TEXT NOT NULL DEFAULT '[]',
    profile_reach TEXT NOT NULL DEFAULT 'community',
    location TEXT,
    app_id TEXT NOT NULL DEFAULT 'imagodei',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_humans_agent_pub_key ON humans(agent_pub_key);
CREATE INDEX IF NOT EXISTS idx_humans_app_id ON humans(app_id);
"#;
