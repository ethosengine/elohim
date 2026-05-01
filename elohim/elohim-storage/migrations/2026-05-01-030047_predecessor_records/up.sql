-- Source of truth: libp2p trust-compute gradient (EPR Phase 3.5).
-- This table is an operational (Category C) durable store for predecessor
-- back-propagation records received from gossip. sealed_blob holds the
-- dryoc-encrypted 2-of-2 predecessor payload (T11 lights up the crypto).
-- Durable disk persistence ensures peer-restart does not truncate back-prop chains.

CREATE TABLE predecessor_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_cid TEXT NOT NULL,
    predecessor_peer_id TEXT NOT NULL,
    received_at TEXT NOT NULL,
    sealed_blob BLOB NOT NULL,
    UNIQUE(target_cid, predecessor_peer_id)
);
CREATE INDEX idx_predecessor_target ON predecessor_records(target_cid);
