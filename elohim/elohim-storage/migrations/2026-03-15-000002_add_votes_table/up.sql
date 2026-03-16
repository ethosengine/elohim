-- votes: Source of truth: private source chain (agent-scoped ballot).
-- Classification: B2 (Agent-Scoped + Attestation) — raw vote is private;
-- dht_anchor_hash populated when tally Attestation is issued.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- proposals extension: voting_anonymous column added below.
-- proposals source of truth: DHT (governance entry). Classification: A (Notarized).

CREATE TABLE IF NOT EXISTS votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    position TEXT NOT NULL,
    reason TEXT,
    anonymous INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id)
);

ALTER TABLE proposals ADD COLUMN voting_anonymous INTEGER NOT NULL DEFAULT 0;
