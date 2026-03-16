-- Governance core tables: proposals, governance_states, discussions, precedents
-- Source-of-truth classifications (P2P Coherence Sprint 4):
--
-- proposals: Source of truth: DHT (governance entry in lamad DNA).
--   Classification: A (Notarized) — proposals are public deliberative acts.
--   dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- governance_states: Source of truth: DHT (derived from proposal lifecycle via Link).
--   Classification: A2 (Derived) — dht_anchor_hash links to parent proposal ActionHash.
--   dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- discussions: Source of truth: SQLite (operational).
--   Classification: C (Operational) — reconstructable from content/proposal thread references.
--   No dht_anchor_hash needed.
--
-- precedents: Source of truth: DHT (governance memory).
--   Classification: A (Notarized) — immutable governance precedents, community relies on them.
--   dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.

-- Governance proposals: the formal deliberative act
CREATE TABLE IF NOT EXISTS proposals (
    id TEXT PRIMARY KEY NOT NULL,
    content_id TEXT NOT NULL,
    proposer_presence_id TEXT NOT NULL,
    proposal_type TEXT NOT NULL DEFAULT 'consent',
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    votes_for INTEGER NOT NULL DEFAULT 0,
    votes_against INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_proposals_content_id ON proposals(content_id);
CREATE INDEX IF NOT EXISTS idx_proposals_status ON proposals(status);

-- Governance state per entity: tracks voting lifecycle and signal accumulation
CREATE TABLE IF NOT EXISTS governance_states (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    reach TEXT NOT NULL DEFAULT 'close',
    labels TEXT NOT NULL DEFAULT '[]',
    voting_state TEXT NOT NULL DEFAULT 'none',
    signal_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_governance_states_entity ON governance_states(entity_type, entity_id);

-- Discussions: threaded conversation anchored to content or proposals
-- Operational (C): reconstructable from parent references, no DHT notarization needed
CREATE TABLE IF NOT EXISTS discussions (
    id TEXT PRIMARY KEY NOT NULL,
    content_id TEXT NOT NULL,
    author_presence_id TEXT NOT NULL,
    body TEXT NOT NULL,
    parent_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discussions_content_id ON discussions(content_id);
CREATE INDEX IF NOT EXISTS idx_discussions_parent_id ON discussions(parent_id);

-- Precedents: immutable governance memory established from resolved challenges
CREATE TABLE IF NOT EXISTS precedents (
    id TEXT PRIMARY KEY NOT NULL,
    content_id TEXT NOT NULL,
    principle TEXT NOT NULL,
    interpretation TEXT NOT NULL,
    established_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_precedents_content_id ON precedents(content_id);
