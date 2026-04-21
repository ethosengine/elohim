-- Recovery Protocol Phase 2 — M2 Validator Implementations
-- Source of truth: DHT (imagodei DNA RecoveryRequest entry).
-- Adds two coordinator-populated fields that bridge pubkey-based rotation logic
-- to legacy String-id-based primitives (HumanityWitness, IdentityFreeze) and
-- carry the IntimateQuorum threshold computed at request time.

ALTER TABLE recovery_requests ADD COLUMN human_id TEXT;
ALTER TABLE recovery_requests ADD COLUMN required_witness_count INTEGER NOT NULL DEFAULT 2;

CREATE INDEX IF NOT EXISTS idx_recovery_requests_human_id
    ON recovery_requests(human_id);
