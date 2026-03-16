-- ranked_votes: Source of truth: private source chain (agent-scoped ballot).
-- Classification: B2 (Agent-Scoped + Attestation) — same pattern as votes;
-- dht_anchor_hash populated when tally Attestation is issued.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- governance_signals: Source of truth: private source chain (agent-scoped reaction).
-- Classification: B2 — raw signal is private; aggregate Attestation is notarized.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.

CREATE TABLE IF NOT EXISTS ranked_votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    rank INTEGER,
    score INTEGER,
    dots INTEGER,
    approved INTEGER,
    reasoning TEXT,
    proxy_elohim_id TEXT,
    proxy_justification TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id, option_id)
);

CREATE TABLE IF NOT EXISTS governance_signals (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    signal_value TEXT NOT NULL,
    mechanism_level INTEGER NOT NULL,
    proxy_elohim_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_governance_signals_entity
    ON governance_signals(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_ranked_votes_proposal
    ON ranked_votes(proposal_id);
