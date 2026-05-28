-- Source of truth: derived projection of GovernanceState × Content.contentType
-- × Proposal? × Manifest(kind: pillar-projection, pillar: qahal)
-- Category C operational projection — reconstructable from substrate entries.
-- Triggers: Signal::GovernanceStateUpdated, Signal::ContentCreated/Updated,
--           Signal::ProposalCreated/Closed,
--           Signal::ManifestUpdated{kind: "pillar-projection", pillar: "qahal"}
-- See: elohim/sdk/schemas/v1/views/mechanism-selection.schema.json

CREATE TABLE IF NOT EXISTS mechanism_selection (
    entity_type               TEXT NOT NULL,
    entity_id                 TEXT NOT NULL,
    -- Mechanism ladder level 0-7
    level                     INTEGER NOT NULL DEFAULT 1,
    -- Name: 'none'|'reactions'|'graduated-feedback'|votingMechanism
    mechanism                 TEXT NOT NULL DEFAULT 'reactions',
    -- 'angular' (levels 0-2) or 'psephos' (levels 3-7)
    render_target             TEXT NOT NULL DEFAULT 'angular',
    -- boolean flags stored as 0/1
    context_menu_only         INTEGER NOT NULL DEFAULT 0,
    allow_reactions           INTEGER NOT NULL DEFAULT 1,
    allow_graduated_feedback  INTEGER NOT NULL DEFAULT 0,
    -- For levels 3-7: active proposal details; NULL when no active proposal
    active_proposal_id        TEXT,
    active_proposal_mechanism TEXT,
    -- CID of pillar-projection Manifest used; NULL = protocol defaults
    policy_manifest_cid       TEXT,
    computed_at               TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_mechanism_selection_level
    ON mechanism_selection (level);

CREATE INDEX IF NOT EXISTS idx_mechanism_selection_render_target
    ON mechanism_selection (render_target);
