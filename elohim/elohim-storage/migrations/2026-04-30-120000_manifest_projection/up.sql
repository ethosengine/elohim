-- Manifest projection table — Phase 3 P3.2.
-- Projected from EprKind::Manifest atoms via the projector. Local view only;
-- DHT is the source of truth.

CREATE TABLE manifests (
    cid                 TEXT NOT NULL PRIMARY KEY,
    manifest_kind       TEXT NOT NULL,         -- 'app' | 'pillar-projection' | 'standing-policy' | …
    pillar              TEXT,                  -- nullable; pillar manifests set this
    payload_json        TEXT NOT NULL,         -- the manifest payload as JSON
    schema_ref          TEXT,                  -- optional schemaRef CID for nested resolution
    signer_pubkey       BLOB NOT NULL,
    created_at          TEXT NOT NULL,         -- ISO-8601
    verified_at         TEXT,                  -- ISO-8601 when verification ran
    revision            INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_manifests_pillar ON manifests(pillar) WHERE pillar IS NOT NULL;
CREATE INDEX idx_manifests_kind ON manifests(manifest_kind);
