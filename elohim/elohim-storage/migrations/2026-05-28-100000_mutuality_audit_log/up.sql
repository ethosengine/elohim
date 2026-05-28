-- Operational log of mutuality_audit_service sweep results.
-- Source of truth: local SQLite operational projection; rebuildable by re-running
-- the sweep over current Mishpat::Commitment DHT entries. No dht_anchor_hash —
-- this is sweep telemetry, not notarized.
-- Per spec §6.2: 2026-05-28-mutual-storage-replication-dwelling-hub-design.md.

CREATE TABLE mutuality_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    commitment_cid TEXT NOT NULL,
    provider_dwelling_hub_id TEXT NOT NULL,
    recipient_dwelling_hub_id TEXT NOT NULL,
    reciprocity_status TEXT NOT NULL,
    days_since_authored INTEGER NOT NULL,
    grace_period_days INTEGER NOT NULL,
    signaled_at TEXT,
    swept_at TEXT NOT NULL
);
CREATE INDEX idx_mutuality_audit_commitment ON mutuality_audit_log(commitment_cid);
CREATE INDEX idx_mutuality_audit_recipient ON mutuality_audit_log(recipient_dwelling_hub_id);
CREATE INDEX idx_mutuality_audit_swept ON mutuality_audit_log(swept_at);
