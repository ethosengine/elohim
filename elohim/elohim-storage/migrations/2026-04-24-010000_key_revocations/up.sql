-- Recovery Protocol Phase 2 — M4 KeyRevocation Projection
-- Source of truth: DHT (imagodei KeyRevocation entries)
-- Projection: read-optimized; rebuildable via signal replay on
--   RecoveryV2Signal::KeyRevocationRequested / KeyRevocationEffective

CREATE TABLE IF NOT EXISTS key_revocations (
    dht_anchor_hash  TEXT    PRIMARY KEY NOT NULL,
    id               TEXT    NOT NULL UNIQUE,
    human_id         TEXT    NOT NULL,
    revoked_key      TEXT    NOT NULL,
    reason           TEXT    NOT NULL,
    trigger_type     TEXT    NOT NULL,
    initiated_by     TEXT    NOT NULL,
    required_votes   INTEGER NOT NULL,
    current_votes    INTEGER NOT NULL,
    threshold_reached INTEGER NOT NULL,  -- 0/1 (false/true)
    effective_at     TEXT,               -- NULL while pending
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_key_revocations_human
    ON key_revocations(human_id);

CREATE INDEX IF NOT EXISTS idx_key_revocations_revoked_key
    ON key_revocations(revoked_key);

CREATE INDEX IF NOT EXISTS idx_key_revocations_pending
    ON key_revocations(threshold_reached);
