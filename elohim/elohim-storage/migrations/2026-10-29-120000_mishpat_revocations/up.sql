-- What this peer knows about the revocation of a mishpat commitment, keyed by
-- the commitment's cid (its entry hash). Two facts share the row.
--
-- revoked_at. Source of truth: Holochain DHT (a `revokes-commitment` entry in
-- the mishpat DNA; this column is a projection of it). It is kept apart from
-- `mishpat_commitments.revoked_at` so that a revocation outlives the row it
-- names: a revocation that arrives before its target is applied when the
-- target lands (to the row and to its coverage mirror).
--
-- The submission record (issued_at, issue_outcome, signed_at, attempts,
-- last_attempt_at). Source of truth: local (this peer's own record that it
-- submitted a revocation for this cid; pending | issued | unconfirmed). Both
-- submitters (the provide reconciler's removed-pin arm and the HTTP un-pin
-- route) claim through it before calling the conductor, so a row is never
-- submitted by both; signed_at is fixed by the first claim, so every attempt is
-- the same entry; an unknown outcome is retried a bounded number of times and
-- then reported. Not rebuilt from peers and never pruned.
CREATE TABLE mishpat_revocations (
    target_cid TEXT PRIMARY KEY NOT NULL,
    revoked_at TEXT,
    issued_at TEXT,
    issue_outcome TEXT,
    signed_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
