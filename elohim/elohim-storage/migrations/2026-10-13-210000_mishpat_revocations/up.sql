-- What this peer knows about the revocation of a mishpat commitment, keyed by
-- the commitment's cid (its entry hash). Two facts share the row.
--
-- revoked_at. Source of truth: Holochain DHT (a `revokes-commitment` entry in
-- the mishpat DNA; this column is a projection of it). It is kept apart from
-- `mishpat_commitments.revoked_at` so that a revocation outlives the row it
-- names: a replayed commitment signal cannot read as live again, and a
-- revocation that arrives before its target is applied when the target lands.
--
-- issued_at / issue_outcome. Source of truth: local (this peer's own record
-- that it submitted a revocation for this cid; pending | issued | unconfirmed).
-- It is what makes a revocation once-per-row across restarts and lost signals.
-- Not rebuilt from peers and never pruned. A lost row is bounded: the provide
-- reconciler may submit that one revocation once more, with the same payload.
CREATE TABLE mishpat_revocations (
    target_cid TEXT PRIMARY KEY NOT NULL,
    revoked_at TEXT,
    issued_at TEXT,
    issue_outcome TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
