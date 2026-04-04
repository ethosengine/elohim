-- Source of truth: Local SQLite (peer shard tracking)
-- Classification: C (Operational) — rebuilt from shard protocol ack events
-- No dht_anchor_hash: ephemeral tracking data

CREATE TABLE shard_locations (
    shard_hash TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'lamad',
    status TEXT NOT NULL DEFAULT 'announced',
    first_seen TEXT NOT NULL DEFAULT (datetime('now')),
    last_verified TEXT,
    PRIMARY KEY (shard_hash, peer_id)
);

CREATE INDEX idx_shard_locations_peer ON shard_locations(peer_id);
CREATE INDEX idx_shard_locations_status ON shard_locations(status);
