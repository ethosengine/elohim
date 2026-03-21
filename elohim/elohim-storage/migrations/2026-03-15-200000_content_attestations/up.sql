-- Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — content quality attestations
-- dht_anchor_hash added by 2026-03-16-100000_lamad_provenance
CREATE TABLE content_attestations (
    id TEXT PRIMARY KEY NOT NULL,
    content_id TEXT NOT NULL,
    attestor_presence_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    attestation_type TEXT NOT NULL,
    evidence TEXT,
    grantor TEXT,
    is_revoked INTEGER NOT NULL DEFAULT 0,
    revocation TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_content_attestations_content ON content_attestations(content_id);
CREATE INDEX idx_content_attestations_attestor ON content_attestations(attestor_presence_id);
CREATE INDEX idx_content_attestations_type ON content_attestations(attestation_type);
