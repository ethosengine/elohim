-- Recovery Protocol Phase 2 — M3 Witness Projection
-- Source of truth: DHT (imagodei DNA HumanityWitness entry linked from RecoveryRequest
--                       via RecoveryRequestToHumanityWitness).
-- This table is a read-optimized projection off RecoveryV2Signal::IntimateWitnessSubmitted.

CREATE TABLE IF NOT EXISTS recovery_witnesses (
    dht_anchor_hash       TEXT PRIMARY KEY NOT NULL,
    recovery_request_hash TEXT NOT NULL,
    witness_agent_id      TEXT NOT NULL,
    human_id              TEXT NOT NULL,
    note                  TEXT,
    submitted_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recovery_witnesses_request_hash
    ON recovery_witnesses(recovery_request_hash);

CREATE INDEX IF NOT EXISTS idx_recovery_witnesses_human_id
    ON recovery_witnesses(human_id);
