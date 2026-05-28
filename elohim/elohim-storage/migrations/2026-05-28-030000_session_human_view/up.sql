-- Source of truth: derived projection of EconomicEvent stream × HumanProgress
-- filtered by agent (provider field) and lamadEventType values:
--   'content-view', 'contributor-presence', 'path-started', 'path-completed',
--   'step-completed', 'content-complete'.
-- Category C operational — reconstructable from the underlying EconomicEvent
-- entries in the elohim DNA content_store zome at any time.
--
-- No dht_anchor_hash column: this table holds no DHT-sourced rows of its own.
-- It is a COUNT aggregate over the existing economic_events projection table
-- (which DOES carry dht_anchor_hash per event row). Provenance can be verified
-- by querying economic_events WHERE provider = ? for the individual rows.
--
-- Keyed by (agent_id, h_app_id) — one row per agent per app.
-- Projection writer: elohim-storage/src/db/session_human_view.rs
-- Signal trigger: Signal::EconomicEventCreated with qualifying lamadEventType.
-- Schema: elohim/sdk/schemas/v1/views/session-human-view.schema.json
CREATE TABLE IF NOT EXISTS session_human_view (
    agent_id              TEXT NOT NULL,
    h_app_id              TEXT NOT NULL,
    nodes_viewed          INTEGER NOT NULL DEFAULT 0,
    nodes_with_affinity   INTEGER NOT NULL DEFAULT 0,
    paths_started         INTEGER NOT NULL DEFAULT 0,
    paths_completed       INTEGER NOT NULL DEFAULT 0,
    steps_completed       INTEGER NOT NULL DEFAULT 0,
    journey_started_at    TEXT,
    computed_at           TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, h_app_id)
);

CREATE INDEX IF NOT EXISTS idx_session_human_view_h_app_id
    ON session_human_view (h_app_id);
