-- Source of truth: DHT
--
-- HostedAgentBinding projection (doorway-federation-failover T2.2).
--
-- The canonical truth is a Category-A2 DERIVED link on the imagodei DNA:
--   StringAnchor("hosted_at", <agent_pub_key>)  --PeerToBinding-->  <agent_pub_key>
-- with the doorway reference carried in the link tag. There is no entry type —
-- the binding is a relationship between the human's imagodei agent identity and
-- the home doorway's DoorwayRegistration (which lives in the *infrastructure*
-- DNA, so it cannot be a link target from imagodei; it travels as tag data).
--
-- This table is the read-optimized projection a SIBLING doorway consults to
-- answer "where is this agent hosted?" without a DHT round-trip on the request
-- path. It is rebuildable by signal replay per Principle P1. Writes land ONLY
-- through ReconcileController::on_hosted_agent_binding.
--
-- dht_anchor_hash is the CreateLink ActionHash — the DHT anchor for this row.
-- It is the idempotency key: replaying the same signal upserts the same row.
--
-- Timestamps are TEXT (ISO-8601), matching elohim-storage conventions.

CREATE TABLE hosted_agent_bindings (
    agent_pub_key    TEXT NOT NULL,
    doorway_id       TEXT NOT NULL,
    doorway_url      TEXT NOT NULL,
    installed_app_id TEXT NOT NULL,
    dht_anchor_hash  TEXT NOT NULL,
    bound_at         TEXT NOT NULL,
    observed_at      TEXT NOT NULL,
    PRIMARY KEY (dht_anchor_hash)
);

-- The hot read: resolve the current home doorway for an agent. `bound_at DESC`
-- is served by this index; a human who migrates between doorways accumulates
-- rows and the newest binding is the current home.
CREATE INDEX idx_hosted_agent_bindings_agent
    ON hosted_agent_bindings(agent_pub_key, bound_at DESC);
