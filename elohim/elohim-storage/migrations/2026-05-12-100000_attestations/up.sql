-- Source of truth: Holochain DHT (projection of Content entries with content_type LIKE 'attestation:%')
-- Category A — every row carries dht_anchor_hash NOT NULL.
-- Per spec genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §7.4.

CREATE TABLE IF NOT EXISTS attestations (
    id TEXT PRIMARY KEY,
    dht_anchor_hash BLOB NOT NULL,
    attestation_kind TEXT NOT NULL,
    subject_cid TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    issuer_cid TEXT NOT NULL,
    parent_governance_action_cid TEXT,
    vote_value TEXT,
    vote_weight TEXT,
    proof_class TEXT NOT NULL,
    proof_evidence_json TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    expires_at TEXT,
    supersedes_cid TEXT,
    revocation_reason TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    manifest_ref TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT
);

CREATE INDEX IF NOT EXISTS attestations_subject ON attestations(subject_cid, attestation_kind);
CREATE INDEX IF NOT EXISTS attestations_issuer ON attestations(issuer_cid);
CREATE INDEX IF NOT EXISTS attestations_parent ON attestations(parent_governance_action_cid);
CREATE INDEX IF NOT EXISTS attestations_kind ON attestations(attestation_kind);
CREATE INDEX IF NOT EXISTS attestations_supersedes ON attestations(supersedes_cid);
