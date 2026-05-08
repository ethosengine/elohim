-- Phase 10 of iroh parallel stack — cross-stack peer identity mapping.
--
-- During the libp2p ↔ iroh hybrid window, the same logical peer is
-- reachable under TWO transport identities: a libp2p PeerId (multibase-
-- encoded multihash) and an iroh NodeId (32-byte ed25519 pubkey, base32-
-- encoded for display). This table records both for the same logical
-- agent so receivers on either transport can resolve "is this the peer
-- I know on the other stack?"
--
-- Category C operational projection. Rebuildable from observed handshake
-- arrivals on both stacks. Post-cutover (Phase 11), once libp2p deps
-- are removed, this table is dropped.
--
-- Source-of-truth notes:
-- - peer_id is captured on identity-handshake arrival on the libp2p side
--   (existing peer_identity_bindings flow).
-- - node_id is captured on identity-handshake arrival on the iroh side
--   (new IrohIdentityHandshakeProtocol; Phase 11 wires this to the same
--   handshake service).
-- - agent_cid is the canonical human-stable identity from imagodei;
--   used as the primary key so writes from either stack converge.

CREATE TABLE cross_stack_peer_map (
    agent_cid    TEXT NOT NULL PRIMARY KEY,
    peer_id      TEXT NULL,
    node_id      TEXT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_cross_stack_peer_map_peer_id
    ON cross_stack_peer_map(peer_id) WHERE peer_id IS NOT NULL;
CREATE UNIQUE INDEX idx_cross_stack_peer_map_node_id
    ON cross_stack_peer_map(node_id) WHERE node_id IS NOT NULL;
