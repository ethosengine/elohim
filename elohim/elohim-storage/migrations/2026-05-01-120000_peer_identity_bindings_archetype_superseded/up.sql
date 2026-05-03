-- Add device_archetype + superseded_by columns the projection was dropping.
-- Source of truth remains the DHT entry; this migration extends the projection so
-- queries can filter to current bindings (superseded_by IS NULL) and group/aggregate
-- by archetype. Backfill device_archetype to 'node' for any existing rows; superseded_by
-- defaults to NULL (current).

ALTER TABLE peer_identity_bindings ADD COLUMN device_archetype TEXT NOT NULL DEFAULT 'node';
ALTER TABLE peer_identity_bindings ADD COLUMN superseded_by TEXT;

CREATE INDEX idx_peer_identity_bindings_current_per_agent
    ON peer_identity_bindings(agent_cid)
    WHERE superseded_by IS NULL;

CREATE INDEX idx_peer_identity_bindings_archetype
    ON peer_identity_bindings(device_archetype);
