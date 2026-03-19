-- Source of truth: SQLite (operational, Category C — no dht_anchor_hash)
-- Reconstructable from source entities. Polymorphic attachment like schedules.
-- Geospatial context for any CID-addressed entity in the protocol.
CREATE TABLE spatial_contexts (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    -- Core position (WGS84)
    latitude REAL,
    longitude REAL,
    altitude REAL,
    accuracy REAL,
    -- H3 hexagonal index (pre-computed from lat/lng for spatial queries)
    h3_res5 TEXT,   -- Municipal/community level (~253 km²)
    h3_res7 TEXT,   -- Neighborhood level (~5.2 km²)
    h3_res9 TEXT,   -- Parcel level (~0.1 km²)
    -- Place resolution (links to governed Place entity, Sprint 2)
    place_id TEXT,
    -- OpenStreetMap reference
    osm_type TEXT,
    osm_id INTEGER,
    -- Descriptive
    label TEXT,
    context_type TEXT NOT NULL DEFAULT 'point',
    geometry_json TEXT,
    metadata_json TEXT,
    -- Temporal
    observed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- One spatial context per entity (can be relaxed for historical tracking later)
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_spatial_ctx_entity ON spatial_contexts (entity_type, entity_id);
CREATE INDEX idx_spatial_ctx_h3_res5 ON spatial_contexts (h3_res5);
CREATE INDEX idx_spatial_ctx_h3_res7 ON spatial_contexts (h3_res7);
CREATE INDEX idx_spatial_ctx_h3_res9 ON spatial_contexts (h3_res9);
CREATE INDEX idx_spatial_ctx_place ON spatial_contexts (place_id);
CREATE INDEX idx_spatial_ctx_type ON spatial_contexts (entity_type, app_id);
