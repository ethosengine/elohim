-- Recovery Protocol Phase 2 — DHT projection tables
-- Source of truth: DHT (imagodei DNA). These tables are read-optimized projections
-- populated via post-commit signals from RecoveryV2Signal events.

-- Source of truth: DHT (imagodei RecoverySeedCommitment entry)
-- Projection populated from RecoveryV2Signal::SeedCommitmentCreated
CREATE TABLE recovery_seed_commitments (
    dht_anchor_hash      TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey   TEXT NOT NULL,
    seed_public_half     BLOB NOT NULL,
    threshold_n          INTEGER NOT NULL,
    total_m              INTEGER NOT NULL,
    commitment_nonce     BLOB NOT NULL,
    superseded_by        TEXT,
    created_at           TEXT NOT NULL
);
CREATE INDEX idx_recovery_seed_commitments_human
  ON recovery_seed_commitments(human_agent_pubkey);
CREATE INDEX idx_recovery_seed_commitments_active
  ON recovery_seed_commitments(human_agent_pubkey)
  WHERE superseded_by IS NULL;

-- Source of truth: DHT (imagodei RecoveryQuorumRequest entry)
-- Projection populated from RecoveryV2Signal::RecoveryQuorumRequestCreated
CREATE TABLE recovery_quorum_requests (
    dht_anchor_hash        TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey     TEXT NOT NULL,
    seed_commitment_hash   TEXT NOT NULL,
    new_agent_pubkey       TEXT NOT NULL,
    hosting_doorway_pubkey TEXT NOT NULL,
    recovery_mode          TEXT NOT NULL,
    stewarded_grant_hash   TEXT,
    request_nonce          BLOB NOT NULL,
    created_at             TEXT NOT NULL
);
CREATE INDEX idx_recovery_quorum_requests_human
  ON recovery_quorum_requests(human_agent_pubkey);
CREATE INDEX idx_recovery_quorum_requests_commitment
  ON recovery_quorum_requests(seed_commitment_hash);

-- Source of truth: DHT (imagodei KeyRotation entry)
-- Projection populated from RecoveryV2Signal::KeyRotationCommitted
-- The `dht_anchor_hash` is the authoritative reference; this projection exists
-- only for fast lookup of "what is agent X's current key?" across the protocol.
CREATE TABLE key_rotations (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    superseded_agent_pubkey  TEXT NOT NULL,
    seed_commitment_hash     TEXT NOT NULL,
    recovery_request_hash    TEXT NOT NULL,
    quorum_signature         BLOB NOT NULL,
    rotated_at               TEXT NOT NULL
);
CREATE INDEX idx_key_rotations_human
  ON key_rotations(human_agent_pubkey);
CREATE INDEX idx_key_rotations_new_agent
  ON key_rotations(new_agent_pubkey);
