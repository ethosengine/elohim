-- Contributor presences for stewardship lifecycle
-- Tracks recognition accumulation and claim verification for absent contributors
CREATE TABLE contributor_presences (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    display_name TEXT NOT NULL,
    presence_state TEXT NOT NULL DEFAULT 'unclaimed',

    -- External identity references
    external_identifiers_json TEXT,
    establishing_content_ids_json TEXT NOT NULL,

    -- Recognition accumulation
    affinity_total REAL NOT NULL DEFAULT 0.0,
    unique_engagers INTEGER NOT NULL DEFAULT 0,
    citation_count INTEGER NOT NULL DEFAULT 0,
    recognition_score REAL NOT NULL DEFAULT 0.0,
    recognition_by_content_json TEXT,
    last_recognition_at TEXT,

    -- Stewardship
    steward_id TEXT,
    stewardship_started_at TEXT,
    stewardship_commitment_id TEXT,
    stewardship_quality_score REAL,

    -- Claim process
    claim_initiated_at TEXT,
    claim_verified_at TEXT,
    claim_verification_method TEXT,
    claim_evidence_json TEXT,
    claimed_agent_id TEXT,
    claim_recognition_transferred_value REAL,
    claim_facilitated_by TEXT,

    -- Metadata
    image TEXT,
    note TEXT,
    metadata_json TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for presence queries
CREATE INDEX idx_presence_app_id ON contributor_presences(app_id);
CREATE INDEX idx_presence_state ON contributor_presences(app_id, presence_state);
CREATE INDEX idx_presence_steward ON contributor_presences(steward_id);
CREATE INDEX idx_presence_claimed ON contributor_presences(claimed_agent_id);
CREATE INDEX idx_presence_recognition ON contributor_presences(recognition_score DESC);
