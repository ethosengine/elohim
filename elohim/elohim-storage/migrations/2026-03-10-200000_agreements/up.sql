-- Source of truth: Holochain DHT (Agreement entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Agreement ActionHash

-- Agreement — bilateral contract anchor linking paired Commitments.
-- Thin by design: Commitments carry the terms, Agreement proves pairing.
CREATE TABLE agreements (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    name TEXT,
    note TEXT,
    dht_anchor_hash TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agreement_app_id ON agreements(app_id);
