-- Recovery flow state-machine projection.
-- Source of truth: elohim DNA Content entries with
-- content_type = 'governance-action:recovery-request'
--              | 'governance-action:identity-freeze'
-- The DHT anchor hash provides provenance back to the canonical entry.
CREATE TABLE recovery_flows (
    id                       TEXT PRIMARY KEY,
    dht_anchor_hash          BLOB NOT NULL,
    flow_kind                TEXT NOT NULL,           -- 'recovery-request' | 'identity-freeze'
    subject_human_id         TEXT NOT NULL,
    initiated_by_cid         TEXT NOT NULL,
    state                    TEXT NOT NULL,           -- 'Open' | 'Quorum' | 'Effective' | 'Closed'
    required_votes           INTEGER NOT NULL,
    current_votes            INTEGER NOT NULL DEFAULT 0,
    threshold_reached        INTEGER NOT NULL DEFAULT 0,  -- bool 0/1
    effective_at             TEXT,                    -- ISO 8601 once Effective
    closes_at                TEXT NOT NULL,           -- proposal close deadline
    metadata_json            TEXT NOT NULL,           -- full metadata for state queries
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);
CREATE INDEX recovery_flows_subject_idx ON recovery_flows(subject_human_id);
CREATE INDEX recovery_flows_state_idx ON recovery_flows(state);

-- Key revocation projection (EPR W2D — co-located per D1).
-- Source of truth: elohim DNA Content entries with
-- content_type = 'governance-action:key-revocation' and
-- the corresponding 'attestation:revocation-vote' children.
--
-- NOTE: This supersedes the 2026-04-24-010000_key_revocations schema, which
-- was scoped to the legacy imagodei KeyRevocation DHT entry shape. The new
-- column names align with the elohim-DNA Content-routed producer contract
-- (subject_human_id, initiated_by_cid, derived_compromise_at). Existing CRUD
-- in src/db/key_revocations.rs + src/api/account.rs + src/signals.rs must be
-- updated to the new column names — this is Task 7's responsibility.
DROP INDEX IF EXISTS idx_key_revocations_pending;
DROP INDEX IF EXISTS idx_key_revocations_revoked_key;
DROP INDEX IF EXISTS idx_key_revocations_human;
DROP TABLE IF EXISTS key_revocations;

CREATE TABLE key_revocations (
    id                       TEXT PRIMARY KEY,
    dht_anchor_hash          BLOB NOT NULL,
    subject_human_id         TEXT NOT NULL,
    revoked_key              TEXT NOT NULL,
    trigger_type             TEXT NOT NULL,           -- 'voluntary' | 'steward_vote' | 'challenge' | 'specialist_attestation'
    reason                   TEXT NOT NULL,
    initiated_by_cid         TEXT NOT NULL,
    required_votes           INTEGER NOT NULL,
    current_votes            INTEGER NOT NULL DEFAULT 0,
    threshold_reached        INTEGER NOT NULL DEFAULT 0,
    effective_at             TEXT,                    -- non-null when revocation is effective
    derived_compromise_at    TEXT,                    -- EPR W2D field
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);
CREATE INDEX key_revocations_subject_idx ON key_revocations(subject_human_id);
CREATE INDEX key_revocations_revoked_key_idx ON key_revocations(revoked_key);
CREATE INDEX key_revocations_effective_idx ON key_revocations(effective_at);
