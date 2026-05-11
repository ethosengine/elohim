-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'governance-action:%')
-- Category A — every row carries dht_anchor_hash NOT NULL.

CREATE TABLE IF NOT EXISTS governance_actions (
    id TEXT PRIMARY KEY,
    dht_anchor_hash BLOB NOT NULL,
    governance_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    proposer_cid TEXT NOT NULL,
    threshold_json TEXT NOT NULL,
    eligibility_predicate_json TEXT,
    ballot_format TEXT NOT NULL,
    closes_at TEXT NOT NULL,
    parameters_json TEXT,
    title TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS governance_actions_subject ON governance_actions(subject_cid);
CREATE INDEX IF NOT EXISTS governance_actions_kind ON governance_actions(governance_kind);
CREATE INDEX IF NOT EXISTS governance_actions_closes ON governance_actions(closes_at);
