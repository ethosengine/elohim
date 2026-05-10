-- Phase 12 of iroh parallel stack — peer_transport_manifest graduation.
-- Replaces 2026-05-08-045024_cross_stack_peer_map (transition-bridge schema).
-- Per genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
-- lines 440-505: this is permanent structural schema, not transition-bridge.
--
-- Source-of-truth notes (Category C operational projection):
-- - agent_cid is the canonical DHT-anchored identity (imagodei); PK so
--   writes from either transport converge.
-- - libp2p_peer_id and iroh_node_id are NULL-able; CHECK enforces that
--   at least one is populated (a row with neither is a protocol violation).
-- - libp2p_addrs_json, iroh_relays_json, libp2p_supports_json,
--   iroh_supports_json are TEXT containing JSON arrays of strings.
--   Encoding/decoding lives in src/p2p_iroh/peer_map.rs.
-- - discovery_methods_json is a TEXT JSON array of method names
--   ("pkarr", "kademlia", "mdns", "doorway-introduction").
-- - capability_level is 0-5 (per device archetypes — 5 = hub default,
--   4 = always-on full node, 3 = laptop, 2 = phone, 1 = wearable, 0 = unknown).
-- - last_observed is unix epoch seconds.

-- Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C).
-- No dht_anchor_hash column: agent_cid is a string reference to the DHT-canonical
-- imagodei identity (anchored via the existing AgentPeerBinding entry type +
-- peer_identity_bindings projection). This table is rebuildable from observed
-- handshake arrivals; if it disagrees with the DHT, the DHT wins.
CREATE TABLE peer_transport_manifest (
    agent_cid              TEXT NOT NULL PRIMARY KEY,  -- Category C operational projection key; DHT-anchor lives in AgentPeerBinding
    libp2p_peer_id         TEXT NULL,
    iroh_node_id           TEXT NULL,
    libp2p_addrs_json      TEXT NULL,
    iroh_relays_json       TEXT NULL,
    libp2p_supports_json   TEXT NULL,
    iroh_supports_json     TEXT NULL,
    discovery_methods_json TEXT NOT NULL,
    capability_level       INTEGER NOT NULL,
    last_observed          INTEGER NOT NULL,
    CHECK (libp2p_peer_id IS NOT NULL OR iroh_node_id IS NOT NULL)
);

CREATE UNIQUE INDEX idx_peer_transport_manifest_libp2p_peer_id
    ON peer_transport_manifest(libp2p_peer_id) WHERE libp2p_peer_id IS NOT NULL;
CREATE UNIQUE INDEX idx_peer_transport_manifest_iroh_node_id
    ON peer_transport_manifest(iroh_node_id) WHERE iroh_node_id IS NOT NULL;

-- Mechanical migration of existing rows from the bridge table.
-- Spec lines 502-504: capability_level defaults to 5 (hub default);
-- discovery_methods_json defaults to '["kademlia"]' for libp2p-only rows
-- and '["pkarr","kademlia"]' if iroh_node_id is present.
-- last_seen_at (ISO 8601 string) is converted to unix epoch via strftime.
-- libp2p_addrs_json / iroh_relays_json / *_supports_json default to NULL —
-- they will populate as observations land.
INSERT INTO peer_transport_manifest (
    agent_cid,
    libp2p_peer_id,
    iroh_node_id,
    libp2p_addrs_json,
    iroh_relays_json,
    libp2p_supports_json,
    iroh_supports_json,
    discovery_methods_json,
    capability_level,
    last_observed
)
SELECT
    agent_cid,
    peer_id,
    node_id,
    NULL,
    NULL,
    NULL,
    NULL,
    CASE
        WHEN node_id IS NOT NULL THEN '["pkarr","kademlia"]'
        ELSE '["kademlia"]'
    END,
    5,
    CAST(strftime('%s', last_seen_at) AS INTEGER)
FROM cross_stack_peer_map;

DROP INDEX IF EXISTS idx_cross_stack_peer_map_peer_id;
DROP INDEX IF EXISTS idx_cross_stack_peer_map_node_id;
DROP TABLE cross_stack_peer_map;
