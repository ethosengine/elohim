-- proposal_options: Source of truth: DHT (derived from proposal via Link).
-- Classification: A2 (Derived) — dht_anchor_hash links to parent proposal ActionHash.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- proposals extension: voting_mechanism, score_min/max, dots, quorum, threshold columns added below.
-- proposals source of truth: DHT (governance entry). Classification: A (Notarized).

CREATE TABLE IF NOT EXISTS proposal_options (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    position INTEGER NOT NULL,
    source TEXT,
    source_justification TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE proposals ADD COLUMN voting_mechanism TEXT NOT NULL DEFAULT 'consent';
ALTER TABLE proposals ADD COLUMN score_min INTEGER;
ALTER TABLE proposals ADD COLUMN score_max INTEGER;
ALTER TABLE proposals ADD COLUMN dots_per_voter INTEGER;
ALTER TABLE proposals ADD COLUMN quorum_percentage REAL;
ALTER TABLE proposals ADD COLUMN passage_threshold REAL;
