-- Source of truth: derived projection of EconomicEvent stream filtered by
-- (content_id, lamadEventType IN ('content-view', 'content-complete')).
-- Category C operational — reconstructable from the underlying EconomicEvent
-- entries in the elohim DNA content_store zome at any time.
--
-- No dht_anchor_hash column: this table holds no DHT-sourced rows of its own.
-- It is a GROUP BY aggregate over the existing economic_events projection table
-- (which DOES carry dht_anchor_hash per event row). If you need to verify
-- provenance, query economic_events WHERE content_id = ? AND
-- lamad_event_type IN ('content-view','content-complete') for the individual
-- dht_anchor_hash values.
--
-- Keyed by (content_id, h_app_id) — one row per content item per app.
-- Projection writer: elohim-storage/src/db/content_engagement_stats.rs
-- Signal trigger: Signal::EconomicEventCreated with lamadEventType in
--   ('content-view', 'content-complete').
-- Schema: elohim/sdk/schemas/v1/views/content-engagement-stats-view.schema.json
CREATE TABLE IF NOT EXISTS content_engagement_stats (
    content_id     TEXT NOT NULL,
    h_app_id       TEXT NOT NULL,
    views          INTEGER NOT NULL DEFAULT 0,
    completions    INTEGER NOT NULL DEFAULT 0,
    unique_viewers INTEGER NOT NULL DEFAULT 0,
    completion_rate REAL NOT NULL DEFAULT 0.0,
    computed_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (content_id, h_app_id)
);

CREATE INDEX IF NOT EXISTS idx_content_engagement_stats_h_app_id
    ON content_engagement_stats (h_app_id);
