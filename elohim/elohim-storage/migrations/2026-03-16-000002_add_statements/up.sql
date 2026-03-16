-- statements: Source of truth: DHT (Polis sensemaking entry).
-- Classification: A (Notarized) — community-visible statements for opinion clustering.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.
--
-- statement_votes: Source of truth: private source chain (agent-scoped stance).
-- Classification: B2 (Agent-Scoped + Attestation) — private stance;
-- clustered aggregate is notarized.
-- dht_anchor_hash added in migration 2026-03-16-300000_qahal_provenance.

CREATE TABLE IF NOT EXISTS statements (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    text TEXT NOT NULL,
    agree_count INTEGER NOT NULL DEFAULT 0,
    disagree_count INTEGER NOT NULL DEFAULT 0,
    pass_count INTEGER NOT NULL DEFAULT 0,
    group_id TEXT,
    is_bridging INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS statement_votes (
    id TEXT PRIMARY KEY NOT NULL,
    statement_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    vote TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(statement_id, human_id)
);

CREATE INDEX IF NOT EXISTS idx_statements_entity ON statements(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_statement_votes_statement ON statement_votes(statement_id);
