-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — ephemeral situational data, not notarized.
-- Reconstruction: re-fetch from external APIs + re-scan economic events.
CREATE TABLE hazards (
    id                TEXT PRIMARY KEY NOT NULL,
    app_id            TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    hazard_type       TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    reported_at       TEXT NOT NULL,
    projected_onset   TEXT,
    projected_end     TEXT,
    actual_onset      TEXT,
    resolved_at       TEXT,
    affected_h3_cells TEXT NOT NULL DEFAULT '[]',
    radius_km         REAL,
    source            TEXT NOT NULL,
    source_reference  TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    status            TEXT NOT NULL DEFAULT 'active',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_hazards_place ON hazards (place_id, app_id);
CREATE INDEX idx_hazards_status ON hazards (status, app_id);
CREATE INDEX idx_hazards_type ON hazards (hazard_type, app_id);
CREATE INDEX idx_hazards_onset ON hazards (projected_onset);
