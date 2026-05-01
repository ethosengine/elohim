-- Source of truth: Holochain source-chain AttentionTending entries (Visibility::Private, T5/T9).
-- This table is a Category C operational cache (recomputable from source-chain subgraph).
-- Fast-read surface for aggregator (T16) and TTL sweep (services::tending::enforce_ttls).
CREATE TABLE attention_tending (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tending_cid TEXT NOT NULL UNIQUE,
    signer_pubkey BLOB NOT NULL,
    classification TEXT NOT NULL,
    filter_subject_json TEXT NOT NULL,
    context_json TEXT NOT NULL,
    reason TEXT,
    ttl_seconds INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_tended_at INTEGER NOT NULL,
    tended_at_history_json TEXT NOT NULL
);
CREATE INDEX idx_tending_signer ON attention_tending(signer_pubkey);
CREATE INDEX idx_tending_classification ON attention_tending(classification);
-- idx_tending_expiry intentionally omitted: SQLite does not use indices on
-- computed expressions such as `last_tended_at + ttl_seconds < ?`, so an
-- index on (last_tended_at, ttl_seconds) would never be consulted by the
-- sweep query in delete_expired and would be dead weight.
