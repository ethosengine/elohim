-- Source of truth: the DHT. A row says this node found, on the DHT (its own
-- conductor's AgentPeerBinding arrival signal, or a `get_bindings_for_peer` read), a
-- current AgentPeerBinding entry whose action hash is `dht_anchor_hash` and whose
-- peer, agent and validity window are exactly these. Only a cross-signed binding
-- with a matching confirmation is AUTHORITY (`p2p::transport_authority`); a binding
-- that only arrived by gossip or handshake is a hint that requests confirmation.
-- Kept apart from `peer_identity_bindings` so no writer of that table (REPLACE on
-- (peer_id, dht_anchor_hash)) can forge or erase it. Category C, re-derivable.
CREATE TABLE binding_confirmations (
    peer_id         TEXT NOT NULL,
    dht_anchor_hash TEXT NOT NULL,
    agent_cid       TEXT NOT NULL,
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    confirmed_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (peer_id, dht_anchor_hash)
);
