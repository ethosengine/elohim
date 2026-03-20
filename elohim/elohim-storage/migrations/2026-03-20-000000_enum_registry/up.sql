-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — extensible vocabulary managed at storage layer.
-- Seeded from JSON Schema on startup. Community types added via API.
CREATE TABLE enum_registry (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id      TEXT NOT NULL DEFAULT 'lamad',
    enum_name   TEXT NOT NULL,
    enum_value  TEXT NOT NULL,
    tier        TEXT NOT NULL DEFAULT 'extensible',
    added_by    TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE(app_id, enum_name, enum_value)
);

CREATE INDEX idx_enum_registry_name ON enum_registry (enum_name, app_id);
