-- Source of truth: SQLite (operational, Category C — existing classification)
-- Enable temporal history for spatial contexts: a resource that moves,
-- a person who travels, a sensor that's relocated.

-- SQLite requires table recreation to drop UNIQUE constraint.
-- Preserve all data, add is_current flag.
CREATE TABLE spatial_contexts_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    latitude REAL,
    longitude REAL,
    altitude REAL,
    accuracy REAL,
    h3_res5 TEXT,
    h3_res7 TEXT,
    h3_res9 TEXT,
    place_id TEXT,
    osm_type TEXT,
    osm_id INTEGER,
    label TEXT,
    context_type TEXT NOT NULL DEFAULT 'point',
    geometry_json TEXT,
    metadata_json TEXT,
    observed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1
);

INSERT INTO spatial_contexts_v2 SELECT *, 1 FROM spatial_contexts;
DROP TABLE spatial_contexts;
ALTER TABLE spatial_contexts_v2 RENAME TO spatial_contexts;

-- Recreate indexes (no UNIQUE on entity_type+entity_id)
CREATE INDEX idx_spatial_ctx_entity ON spatial_contexts (entity_type, entity_id);
CREATE INDEX idx_spatial_ctx_h3_res5 ON spatial_contexts (h3_res5);
CREATE INDEX idx_spatial_ctx_h3_res7 ON spatial_contexts (h3_res7);
CREATE INDEX idx_spatial_ctx_h3_res9 ON spatial_contexts (h3_res9);
CREATE INDEX idx_spatial_ctx_place ON spatial_contexts (place_id);
CREATE INDEX idx_spatial_ctx_type ON spatial_contexts (entity_type, app_id);
CREATE INDEX idx_spatial_ctx_current ON spatial_contexts (entity_type, entity_id, is_current);
