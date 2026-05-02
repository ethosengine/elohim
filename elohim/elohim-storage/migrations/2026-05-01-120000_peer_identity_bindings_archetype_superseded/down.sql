DROP INDEX IF EXISTS idx_peer_identity_bindings_archetype;
DROP INDEX IF EXISTS idx_peer_identity_bindings_current_per_agent;
ALTER TABLE peer_identity_bindings DROP COLUMN superseded_by;
ALTER TABLE peer_identity_bindings DROP COLUMN device_archetype;
