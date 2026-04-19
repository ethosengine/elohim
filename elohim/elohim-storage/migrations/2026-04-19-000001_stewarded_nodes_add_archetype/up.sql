-- Source of truth: node-registry DNA NodeRegistration entry (dht_anchor_hash
-- already links to the DHT). This migration indexes archetype + household for
-- fast visibility joins powering /shefa/devices.
-- Operational Category C columns — no new DHT entry types introduced.
ALTER TABLE stewarded_nodes ADD COLUMN device_archetype_id TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN household_id TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN hostname TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN node_role TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN capability_level INTEGER;
ALTER TABLE stewarded_nodes ADD COLUMN can_steward INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN can_infer INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN can_doorway INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN signature TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN signed_at TEXT;
CREATE INDEX IF NOT EXISTS idx_stewarded_nodes_household ON stewarded_nodes(household_id);
CREATE INDEX IF NOT EXISTS idx_stewarded_nodes_archetype ON stewarded_nodes(device_archetype_id);
