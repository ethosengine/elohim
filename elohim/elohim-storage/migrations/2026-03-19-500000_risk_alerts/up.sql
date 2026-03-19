-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — derived from threshold crossings on operational data.
-- Reconstruction: re-evaluate thresholds against current hazards + vulnerability.
CREATE TABLE risk_alerts (
    id                TEXT PRIMARY KEY NOT NULL,
    app_id            TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    alert_type        TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    trigger_hazard_id TEXT,
    trigger_data_json TEXT NOT NULL DEFAULT '{}',
    triggered_at      TEXT NOT NULL,
    lead_time_hours   REAL,
    expires_at        TEXT,
    status            TEXT NOT NULL DEFAULT 'active',
    acknowledged_by   TEXT,
    acknowledged_at   TEXT,
    resolved_at       TEXT,
    escalated_to      TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_risk_alerts_place ON risk_alerts (place_id, app_id);
CREATE INDEX idx_risk_alerts_status ON risk_alerts (status, app_id);
CREATE INDEX idx_risk_alerts_type ON risk_alerts (alert_type, app_id);
CREATE INDEX idx_risk_alerts_dedup ON risk_alerts (place_id, alert_type, trigger_hazard_id)
    WHERE status = 'active';
