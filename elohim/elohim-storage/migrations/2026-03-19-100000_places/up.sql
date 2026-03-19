-- Source of truth: DHT (Mishpat DNA, Category A — notarized)
-- This table is a read-optimized projection of Place entries from the DHT.
-- dht_anchor_hash links back to the canonical DHT entry.
CREATE TABLE places (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    dht_anchor_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    place_type TEXT NOT NULL,
    constitutional_layer TEXT NOT NULL,
    h3_index TEXT NOT NULL,
    h3_resolution INTEGER NOT NULL,
    geometry_json TEXT NOT NULL,
    centroid_lat REAL NOT NULL,
    centroid_lng REAL NOT NULL,
    parent_place_id TEXT,
    osm_reference_json TEXT,
    carrying_capacity_json TEXT NOT NULL DEFAULT '[]',
    governing_collective_id TEXT,
    status TEXT NOT NULL DEFAULT 'proposed',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_places_h3 ON places (h3_index);
CREATE INDEX idx_places_type ON places (place_type, app_id);
CREATE INDEX idx_places_layer ON places (constitutional_layer, app_id);
CREATE INDEX idx_places_parent ON places (parent_place_id);
CREATE INDEX idx_places_status ON places (status, app_id);
CREATE INDEX idx_places_collective ON places (governing_collective_id);
CREATE INDEX idx_places_dht ON places (dht_anchor_hash);
