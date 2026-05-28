-- Source of truth: derived projection of FeedbackSignal × Manifest(kind: standing-policy)
-- Category C operational projection — reconstructable from FeedbackSignal entries +
-- standing-policy Manifest payload at any time.
-- Triggers: Signal::FeedbackSignalCreated, Signal::ManifestUpdated{kind: "standing-policy"}
-- See: elohim/sdk/schemas/v1/views/accumulation-status.schema.json

CREATE TABLE IF NOT EXISTS accumulation_status (
    entity_type          TEXT NOT NULL,
    entity_id            TEXT NOT NULL,
    total_signals        INTEGER NOT NULL DEFAULT 0,
    unique_participants  INTEGER NOT NULL DEFAULT 0,
    consensus_strength   REAL NOT NULL DEFAULT 0.0,
    -- status: 'pending' | 'ready_for_sensemaking' | 'controversy_detected' | 'settled'
    status               TEXT NOT NULL DEFAULT 'pending',
    ready_for_sensemaking  INTEGER NOT NULL DEFAULT 0, -- boolean: 0 or 1
    controversy_detected   INTEGER NOT NULL DEFAULT 0, -- boolean: 0 or 1
    settled                INTEGER NOT NULL DEFAULT 0, -- boolean: 0 or 1
    -- CID of the standing-policy Manifest whose thresholds were applied; NULL = protocol defaults
    policy_manifest_cid  TEXT,
    computed_at          TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_accumulation_status_status
    ON accumulation_status (status);
