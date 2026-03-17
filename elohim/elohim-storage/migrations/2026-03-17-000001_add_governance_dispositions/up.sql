-- Source of truth: agent source chain (private governance profile).
-- Classification: B (Agent-Scoped). NOT published to DHT.
CREATE TABLE IF NOT EXISTS governance_dispositions (
    id TEXT PRIMARY KEY NOT NULL,
    human_id TEXT NOT NULL UNIQUE,
    risk_tolerance REAL NOT NULL DEFAULT 0.5,
    change_openness REAL NOT NULL DEFAULT 0.5,
    consensus_preference REAL NOT NULL DEFAULT 0.5,
    priority_values TEXT NOT NULL DEFAULT '[]',
    voting_pattern_summary TEXT NOT NULL DEFAULT '{}',
    total_votes_cast INTEGER NOT NULL DEFAULT 0,
    total_challenges_filed INTEGER NOT NULL DEFAULT 0,
    total_signals_recorded INTEGER NOT NULL DEFAULT 0,
    dht_anchor_hash TEXT,
    last_computed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_governance_dispositions_human ON governance_dispositions(human_id);
