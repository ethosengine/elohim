-- Source of truth: Local SQLite (per-peer encoding state)
-- Classification: C (Operational) — rebuilt from local blob store, not shared via DHT
-- No dht_anchor_hash: this is a projection of local state

CREATE TABLE shard_manifests (
    content_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'lamad',
    blob_hash TEXT NOT NULL,
    blob_cid TEXT,
    encoding TEXT NOT NULL DEFAULT 'none',
    data_shard_count INTEGER NOT NULL DEFAULT 1,
    parity_shard_count INTEGER NOT NULL DEFAULT 0,
    shard_hashes_json TEXT NOT NULL DEFAULT '[]',
    total_size_bytes INTEGER NOT NULL,
    shard_size_bytes INTEGER NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    reach TEXT NOT NULL DEFAULT 'commons',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (content_id, h_app_id)
);

CREATE INDEX idx_shard_manifests_blob ON shard_manifests(blob_hash);
CREATE INDEX idx_shard_manifests_encoding ON shard_manifests(encoding);
