-- Recovery Protocol Phase 2 — M4 RevocationVote Projection
-- Source of truth: DHT (imagodei RevocationVote entries)
-- Projection: rebuildable via signal replay on RecoveryV2Signal::RevocationVoteSubmitted
-- The UNIQUE(revocation_id, steward_id) constraint enforces no-double-vote at the projection layer.

CREATE TABLE IF NOT EXISTS revocation_votes (
    dht_anchor_hash             TEXT    PRIMARY KEY NOT NULL,
    id                          TEXT    NOT NULL UNIQUE,
    revocation_dht_anchor_hash  TEXT    NOT NULL,
    revocation_id               TEXT    NOT NULL,
    steward_id                  TEXT    NOT NULL,
    approved                    INTEGER NOT NULL,  -- 0/1 (false/true)
    attestation                 TEXT    NOT NULL,
    voted_at                    TEXT    NOT NULL,
    UNIQUE (revocation_id, steward_id)
);

CREATE INDEX IF NOT EXISTS idx_revocation_votes_revocation
    ON revocation_votes(revocation_id);

CREATE INDEX IF NOT EXISTS idx_revocation_votes_steward
    ON revocation_votes(steward_id);
