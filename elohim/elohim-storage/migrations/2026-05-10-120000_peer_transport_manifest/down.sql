-- Inverse of 2026-05-10-120000_peer_transport_manifest/up.sql.
-- Recreates the cross_stack_peer_map bridge table and copies back
-- agent_cid + peer_id + node_id. last_seen_at is reconstructed from
-- the unix epoch in last_observed; first_seen_at is set equal to it
-- (we discarded first_seen_at in the up migration — accept the loss).

CREATE TABLE cross_stack_peer_map (
    agent_cid     TEXT NOT NULL PRIMARY KEY,
    peer_id       TEXT NULL,
    node_id       TEXT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_cross_stack_peer_map_peer_id
    ON cross_stack_peer_map(peer_id) WHERE peer_id IS NOT NULL;
CREATE UNIQUE INDEX idx_cross_stack_peer_map_node_id
    ON cross_stack_peer_map(node_id) WHERE node_id IS NOT NULL;

INSERT INTO cross_stack_peer_map (agent_cid, peer_id, node_id, first_seen_at, last_seen_at)
SELECT
    agent_cid,
    libp2p_peer_id,
    iroh_node_id,
    strftime('%Y-%m-%dT%H:%M:%SZ', last_observed, 'unixepoch'),
    strftime('%Y-%m-%dT%H:%M:%SZ', last_observed, 'unixepoch')
FROM peer_transport_manifest;

DROP INDEX IF EXISTS idx_peer_transport_manifest_libp2p_peer_id;
DROP INDEX IF EXISTS idx_peer_transport_manifest_iroh_node_id;
DROP TABLE peer_transport_manifest;
