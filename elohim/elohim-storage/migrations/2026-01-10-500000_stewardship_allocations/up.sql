-- Stewardship allocations - one-to-many content stewardship with allocation ratios
-- Philosophy: Content isn't "owned" but "stewarded" by multiple contributors
-- Ratios are computed from contribution history and negotiated by Elohim governance
-- Stewards can object with valid concerns (hostile takeover, rights violations)
CREATE TABLE stewardship_allocations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',

    -- What is being stewarded
    content_id TEXT NOT NULL,

    -- Who is stewarding (ContributorPresence ID)
    steward_presence_id TEXT NOT NULL,

    -- Allocation ratio (0.0 - 1.0, all allocations for content should sum to 1.0)
    allocation_ratio REAL NOT NULL DEFAULT 1.0,

    -- How ratio was determined
    allocation_method TEXT NOT NULL DEFAULT 'manual', -- 'manual' | 'computed' | 'negotiated'

    -- Contribution evidence (why this steward has this allocation)
    contribution_type TEXT NOT NULL DEFAULT 'original_creator',
    -- 'original_creator' | 'editor' | 'translator' | 'curator' | 'maintainer' | 'inherited'
    contribution_evidence_json TEXT, -- Details of contribution

    -- Governance state
    governance_state TEXT NOT NULL DEFAULT 'active',
    -- 'active' | 'disputed' | 'pending_review' | 'superseded'

    -- Dispute handling
    dispute_id TEXT, -- References a dispute record if governance_state = 'disputed'
    dispute_reason TEXT,
    disputed_at TEXT,
    disputed_by TEXT, -- Presence ID of objector

    -- Elohim negotiation reference
    negotiation_session_id TEXT, -- References path-negotiation if ratio was negotiated
    elohim_ratified_at TEXT, -- When Elohim approved this allocation
    elohim_ratifier_id TEXT, -- Which Elohim agent ratified

    -- Temporal validity
    effective_from TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until TEXT, -- NULL means currently active
    superseded_by TEXT, -- ID of replacement allocation

    -- Recognition accumulation snapshot
    recognition_accumulated REAL NOT NULL DEFAULT 0.0,
    last_recognition_at TEXT,

    -- Metadata
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for allocation queries
CREATE INDEX idx_alloc_app_id ON stewardship_allocations(app_id);
CREATE INDEX idx_alloc_content ON stewardship_allocations(content_id);
CREATE INDEX idx_alloc_steward ON stewardship_allocations(steward_presence_id);
CREATE INDEX idx_alloc_governance ON stewardship_allocations(governance_state);
CREATE INDEX idx_alloc_active ON stewardship_allocations(content_id, governance_state, effective_until);
CREATE INDEX idx_alloc_disputed ON stewardship_allocations(governance_state) WHERE governance_state = 'disputed';

-- Unique constraint: one steward per content per active period
CREATE UNIQUE INDEX idx_alloc_unique_active ON stewardship_allocations(
    app_id, content_id, steward_presence_id
) WHERE effective_until IS NULL AND governance_state = 'active';
