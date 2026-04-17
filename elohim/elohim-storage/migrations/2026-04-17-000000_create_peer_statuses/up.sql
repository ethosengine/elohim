-- Source of truth: DHT (infrastructure DNA PeerStatus entry, Category A notarized).
-- This table is a read-optimized projection populated by the post-commit signal
-- handler in src/signals.rs. Do not write here directly — all writes flow from
-- InfrastructureSignal::PeerStatusRecorded. If this projection and the DHT
-- disagree, the DHT wins (rebuild the projection from it).

CREATE TABLE peer_statuses (
    peer_id TEXT PRIMARY KEY,                        -- AgentPubKey (base64) of the peer
    status TEXT NOT NULL,                            -- starting|online|degraded|maintenance|leaving
    general_pool_member INTEGER NOT NULL,            -- 0/1
    accepting_stewardship_reserves INTEGER NOT NULL, -- 0/1
    archetype_class TEXT,                            -- optional archetype id (e.g. "home-nuc")
    timestamp BIGINT NOT NULL,                       -- micros since epoch (from PeerStatus.timestamp)
    dht_anchor_hash TEXT NOT NULL,                   -- ActionHash (base64) of the upstream DHT entry
    updated_at BIGINT NOT NULL                       -- local insert/update time, micros since epoch
);

CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
