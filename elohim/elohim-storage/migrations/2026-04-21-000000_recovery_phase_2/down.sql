DROP INDEX IF EXISTS idx_key_rotations_new_agent;
DROP INDEX IF EXISTS idx_key_rotations_human;
DROP TABLE IF EXISTS key_rotations;

DROP INDEX IF EXISTS idx_recovery_quorum_requests_commitment;
DROP INDEX IF EXISTS idx_recovery_quorum_requests_human;
DROP TABLE IF EXISTS recovery_quorum_requests;

DROP INDEX IF EXISTS idx_recovery_seed_commitments_active;
DROP INDEX IF EXISTS idx_recovery_seed_commitments_human;
DROP TABLE IF EXISTS recovery_seed_commitments;
