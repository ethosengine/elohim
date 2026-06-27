-- Lens projection — the read-optimized cache of an `author-lens` Mishpat::Commitment
-- (the plural-Mishpat governance primitive). Source of truth: Holochain DHT (mishpat
-- DNA Commitment entry, action='author-lens'); this row is the P1 reconciliation
-- projection, populated from the create_commitment post-commit signal (plan S3). A
-- NULL dht_anchor_hash means un-notarized/storage-only — the forward index
-- (find_lenses_governing_epr, plan S4) fail-closes on it (dht_anchor_hash IS NOT NULL).
-- Classification A (Notarized). `cid` is the Commitment entry_hash (NEVER action_hash).
-- `governs_epr` is the EPR SLUG-ID scope key (plan A3), the forward-index key — NOT the
-- dag-cbor EprHead CID.
-- Spec: 2026-06-27-plural-mishpat-lenses-over-epr-design.md §8.
-- Plan: 2026-06-27-plural-mishpat-lenses-service-layer-plan.md (S2, I5).
CREATE TABLE lenses (
    cid TEXT PRIMARY KEY,                    -- = Commitment entry_hash (read/scope key)
    governs_epr TEXT NOT NULL,               -- EPR slug-id (forward-index key, plan A3)
    school TEXT NOT NULL,                     -- collective / school-of-thought label
    role TEXT NOT NULL DEFAULT 'lens',        -- lens | floor | ceiling
    rule_json TEXT NOT NULL,                  -- the deterministic predicate (the teeth)
    telos_json TEXT NOT NULL,                 -- what the lens steers toward
    version_parent TEXT,                      -- CID of the superseded lens (immutable chain), or NULL
    revoked_at TEXT,                          -- lifecycle: non-null excludes from the live set
    dht_anchor_hash TEXT,                     -- Classification A: NULL = un-notarized (fail-closed)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_lenses_governs_epr ON lenses(governs_epr);
CREATE INDEX idx_lenses_role ON lenses(role);
