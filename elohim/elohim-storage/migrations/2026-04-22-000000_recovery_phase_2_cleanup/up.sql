-- Recovery Protocol Phase 2 — M1-Cleanup
-- Source of truth: DHT (imagodei DNA).
-- Drops deleted-entry-type projections and restructures surviving ones for the revised shape.

-- Drop the dead seed-commitment projection entirely.
DROP INDEX IF EXISTS idx_recovery_seed_commitments_active;
DROP INDEX IF EXISTS idx_recovery_seed_commitments_human;
DROP TABLE IF EXISTS recovery_seed_commitments;

-- Restructure recovery_quorum_requests → recovery_requests.
-- Source of truth: DHT (imagodei RecoveryRequest entry).
-- Projection populated from RecoveryV2Signal::RecoveryRequestCreated.
DROP INDEX IF EXISTS idx_recovery_quorum_requests_commitment;
DROP INDEX IF EXISTS idx_recovery_quorum_requests_human;
DROP TABLE IF EXISTS recovery_quorum_requests;

CREATE TABLE recovery_requests (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    hosting_doorway_pubkey   TEXT NOT NULL,
    proposed_authority_kind  TEXT NOT NULL,   -- "intimate_quorum" | "community_consensus" | "governance_act" | "network_witness" | "cryptographic_quorum"
    proposed_authority_json  TEXT NOT NULL,   -- JSON blob for variant-specific fields (grant_hash, purpose, stewardship_hash)
    request_nonce            BLOB NOT NULL,
    created_at               TEXT NOT NULL
);
CREATE INDEX idx_recovery_requests_human ON recovery_requests(human_agent_pubkey);
CREATE INDEX idx_recovery_requests_kind ON recovery_requests(proposed_authority_kind);

-- Restructure key_rotations for the RecoveryAuthority enum.
-- Source of truth: DHT (imagodei KeyRotation entry).
-- Projection populated from RecoveryV2Signal::KeyRotationCommitted.
-- The dht_anchor_hash is authoritative; this projection is a fast-lookup cache.
DROP INDEX IF EXISTS idx_key_rotations_new_agent;
DROP INDEX IF EXISTS idx_key_rotations_human;
DROP TABLE IF EXISTS key_rotations;

CREATE TABLE key_rotations (
    dht_anchor_hash          TEXT PRIMARY KEY NOT NULL,
    human_agent_pubkey       TEXT NOT NULL,
    new_agent_pubkey         TEXT NOT NULL,
    superseded_agent_pubkey  TEXT NOT NULL,
    recovery_request_hash    TEXT NOT NULL,
    authority_kind           TEXT NOT NULL,   -- variant discriminator
    authority_json           TEXT NOT NULL,   -- JSON blob for variant fields (witness_hashes, challenge_hash, etc.)
    rotated_at               TEXT NOT NULL
);
CREATE INDEX idx_key_rotations_human ON key_rotations(human_agent_pubkey);
CREATE INDEX idx_key_rotations_new_agent ON key_rotations(new_agent_pubkey);
CREATE INDEX idx_key_rotations_authority_kind ON key_rotations(authority_kind);
