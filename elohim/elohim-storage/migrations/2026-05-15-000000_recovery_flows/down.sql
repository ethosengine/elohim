DROP INDEX IF EXISTS key_revocations_effective_idx;
DROP INDEX IF EXISTS key_revocations_revoked_key_idx;
DROP INDEX IF EXISTS key_revocations_subject_idx;
DROP TABLE IF EXISTS key_revocations;
DROP INDEX IF EXISTS recovery_flows_state_idx;
DROP INDEX IF EXISTS recovery_flows_subject_idx;
DROP TABLE IF EXISTS recovery_flows;

-- Restore the pre-2026-05-15 key_revocations projection (scoped to the
-- legacy imagodei KeyRevocation DHT entry shape) so that down-migrating
-- this revision returns the DB to the state expected by Diesel models
-- compiled against the prior schema. If Task 7 has already updated the
-- models to the new schema, this rollback path will be incomplete and
-- requires a coordinated revert.
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
    threshold_reached INTEGER NOT NULL,
    effective_at     TEXT,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_key_revocations_human
    ON key_revocations(human_id);

CREATE INDEX IF NOT EXISTS idx_key_revocations_revoked_key
    ON key_revocations(revoked_key);

CREATE INDEX IF NOT EXISTS idx_key_revocations_pending
    ON key_revocations(threshold_reached);
