-- Human relationships with intimacy levels and consent tracking
-- Supports the Imagodei identity layer for custody and emergency protocols
CREATE TABLE human_relationships (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'imagodei',
    party_a_id TEXT NOT NULL,
    party_b_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    intimacy_level TEXT NOT NULL DEFAULT 'recognition',
    is_bidirectional INTEGER NOT NULL DEFAULT 0,

    -- Consent and custody flags
    consent_given_by_a INTEGER NOT NULL DEFAULT 0,
    consent_given_by_b INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_a INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_b INTEGER NOT NULL DEFAULT 0,
    auto_custody_enabled INTEGER NOT NULL DEFAULT 0,
    emergency_access_enabled INTEGER NOT NULL DEFAULT 0,

    -- Relationship metadata
    initiated_by TEXT NOT NULL,
    verified_at TEXT,
    governance_layer TEXT,
    reach TEXT NOT NULL DEFAULT 'private',
    context_json TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);

-- Unique constraint: one relationship of each type between parties per app
CREATE UNIQUE INDEX idx_human_rel_unique ON human_relationships(app_id, party_a_id, party_b_id, relationship_type);

-- Indexes for relationship queries
CREATE INDEX idx_human_rel_app_id ON human_relationships(app_id);
CREATE INDEX idx_human_rel_party_a ON human_relationships(app_id, party_a_id);
CREATE INDEX idx_human_rel_party_b ON human_relationships(app_id, party_b_id);
CREATE INDEX idx_human_rel_type ON human_relationships(relationship_type);
CREATE INDEX idx_human_rel_intimacy ON human_relationships(intimacy_level);
CREATE INDEX idx_human_rel_custody ON human_relationships(auto_custody_enabled) WHERE auto_custody_enabled = 1;
