-- Source of truth: local (operational) — derived from governance_actions JOIN attestations, rebuildable via signal-stream replay
-- Category C — no dht_anchor_hash. Reconstruction strategy in spec §7.4.

CREATE TABLE IF NOT EXISTS governance_action_tally (
    parent_cid TEXT PRIMARY KEY,
    governance_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    threshold_m INTEGER NOT NULL,
    threshold_n INTEGER,
    threshold_percentage REAL,
    closes_at TEXT NOT NULL,
    current_approve_count INTEGER NOT NULL DEFAULT 0,
    current_reject_count INTEGER NOT NULL DEFAULT 0,
    current_abstain_count INTEGER NOT NULL DEFAULT 0,
    computed_status TEXT NOT NULL,
    last_child_at TEXT,
    rebuilt_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS governance_action_tally_status ON governance_action_tally(computed_status);
