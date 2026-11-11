-- Source of truth: the `TransportLink` a node publishes inside its signed transport
-- manifest (`p2p::transport_link`): its libp2p identity key and its iroh node key each
-- sign the same (iroh_node_id, libp2p_peer_id, issued_at_ms) body over the elohim-epr
-- transcript. A row is written only after both halves verify, so it is a projection of
-- that two-key attestation (Category C, rebuilt from gossip).
--
-- It is the one step that lets an iroh-signed claim reach an agent:
-- iroh NodeId -> (this row) -> libp2p PeerId -> (a cross-signed AgentPeerBinding in
-- peer_identity_bindings) -> agent. `peer_transport_manifest` is never part of that
-- chain; its agent_cid stays a dial hint.
CREATE TABLE transport_links (
    iroh_node_id   TEXT PRIMARY KEY NOT NULL,
    libp2p_peer_id TEXT NOT NULL,
    issued_at_ms   BIGINT NOT NULL,
    recorded_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_transport_links_libp2p ON transport_links(libp2p_peer_id);
