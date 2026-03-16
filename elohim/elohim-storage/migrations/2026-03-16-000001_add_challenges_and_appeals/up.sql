-- Sprint 5: Governance immune system — challenges and appeals tables
-- challenges: Source of truth: DHT (governance entry).
-- Classification: A (Notarized) — challenges are public acts, must be witnessed by the community.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- appeals: Source of truth: DHT (governance entry).
-- Classification: A (Notarized) — appeals are public acts, must be witnessed by the community.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
-- Replaces the simple challenges table with a full challenge lifecycle + appeals

DROP TABLE IF EXISTS challenges;

CREATE TABLE IF NOT EXISTS challenges (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    challenger_id TEXT NOT NULL,
    standing_basis TEXT NOT NULL,
    grounds_primary TEXT NOT NULL,
    grounds_secondary TEXT,
    evidence TEXT NOT NULL,
    requested_outcome TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    response_outcome TEXT,
    response_reasoning TEXT,
    response_actions TEXT,
    response_by TEXT,
    sets_precedent INTEGER NOT NULL DEFAULT 0,
    filed_at TEXT NOT NULL,
    acknowledged_at TEXT,
    response_deadline TEXT NOT NULL,
    responded_at TEXT,
    resolved_at TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS appeals (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    appellant_id TEXT NOT NULL,
    grounds TEXT NOT NULL,
    additional_evidence TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    escalation_level TEXT,
    decision TEXT,
    decision_reasoning TEXT,
    decided_by TEXT,
    filed_at TEXT NOT NULL,
    decided_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_challenges_entity ON challenges(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_challenges_state ON challenges(state);
CREATE INDEX IF NOT EXISTS idx_appeals_challenge ON appeals(challenge_id);
