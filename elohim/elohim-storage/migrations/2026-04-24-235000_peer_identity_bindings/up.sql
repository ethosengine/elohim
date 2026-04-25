-- EPR Phase 2B — peer_identity_bindings projection table
-- Source of truth: Holochain DHT (imagodei DNA AgentPeerBinding entry — Task A.2).
-- This table is a Category C operational projection: rebuildable from DHT entries
-- + gossip replay per Principle P1. dht_anchor_hash links each row back to the
-- canonical notarized entry so provenance is auditable.
--
-- Timestamps stored as TEXT (ISO-8601) matching elohim-storage conventions.
-- valid_until NULL means the binding is open-ended (no expiry).
-- source distinguishes how the binding was learned:
--   'dht'       — projected from a DNA AgentPeerBinding signal
--   'gossip'    — received via libp2p gossip (Task A.10)
--   'handshake' — confirmed during libp2p connection handshake (Task A.9)

CREATE TABLE peer_identity_bindings (
    peer_id         TEXT NOT NULL,
    agent_cid       TEXT NOT NULL,
    dht_anchor_hash TEXT NOT NULL,
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    observed_at     TEXT NOT NULL,
    source          TEXT NOT NULL CHECK (source IN ('dht', 'gossip', 'handshake')),
    PRIMARY KEY (peer_id, dht_anchor_hash)
);

CREATE INDEX idx_peer_identity_bindings_peer_id
    ON peer_identity_bindings(peer_id);

CREATE INDEX idx_peer_identity_bindings_agent_cid
    ON peer_identity_bindings(agent_cid);
