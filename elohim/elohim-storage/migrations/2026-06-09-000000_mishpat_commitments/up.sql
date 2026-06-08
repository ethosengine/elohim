-- Mishpat::Commitment projection — the read-optimized cache of the compute-bounds
-- commitment (policy envelope) the bounds_validator checks. Source of truth: Holochain
-- DHT (mishpat DNA Commitment entry); this row is the P1 reconciliation projection,
-- populated from the Mishpat create_commitment post-commit signal. A NULL dht_anchor_hash
-- means un-notarized/storage-only — the ProjectionCommitmentFetcher refuses a bounds-pass
-- on it. Columns mirror the CommitmentRecord contract (commitment_fetcher.rs).
-- Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §6.5.
CREATE TABLE mishpat_commitments (
    cid TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    scope TEXT NOT NULL,
    provider TEXT NOT NULL,
    recipient TEXT NOT NULL,
    bounds_json TEXT NOT NULL,
    valid_from TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    revoked_at TEXT,
    state TEXT NOT NULL DEFAULT 'proposed',
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_mishpat_commitments_action ON mishpat_commitments(action);
CREATE INDEX idx_mishpat_commitments_provider ON mishpat_commitments(provider);
