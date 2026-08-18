DROP INDEX IF EXISTS idx_peer_identity_bindings_cross_signed;
ALTER TABLE peer_identity_bindings DROP COLUMN proof_status;
ALTER TABLE peer_identity_bindings DROP COLUMN signature;
