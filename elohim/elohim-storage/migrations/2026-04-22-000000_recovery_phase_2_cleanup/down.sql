-- Down migration: reverse M1-cleanup by restoring the prior M1 shape.
-- Not expected to run in practice (dev-only clean slate) but provided for completeness.

DROP INDEX IF EXISTS idx_key_rotations_authority_kind;
DROP INDEX IF EXISTS idx_key_rotations_new_agent;
DROP INDEX IF EXISTS idx_key_rotations_human;
DROP TABLE IF EXISTS key_rotations;

DROP INDEX IF EXISTS idx_recovery_requests_kind;
DROP INDEX IF EXISTS idx_recovery_requests_human;
DROP TABLE IF EXISTS recovery_requests;

-- Restore the M1 seed-commitment table (for completeness; content is lost).
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
CREATE INDEX idx_recovery_seed_commitments_human ON recovery_seed_commitments(human_agent_pubkey);
CREATE INDEX idx_recovery_seed_commitments_active ON recovery_seed_commitments(human_agent_pubkey) WHERE superseded_by IS NULL;

-- Restore the M1 recovery_quorum_requests and key_rotations (prior shapes).
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
CREATE INDEX idx_recovery_quorum_requests_human ON recovery_quorum_requests(human_agent_pubkey);
CREATE INDEX idx_recovery_quorum_requests_commitment ON recovery_quorum_requests(seed_commitment_hash);

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
CREATE INDEX idx_key_rotations_human ON key_rotations(human_agent_pubkey);
CREATE INDEX idx_key_rotations_new_agent ON key_rotations(new_agent_pubkey);
