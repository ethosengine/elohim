-- migrations/2026-05-11-110000_projection_events/up.sql
-- OBSERVATION-LAYER NOTE (Observation/Event Layer spec — Stage 8):
-- This table remains operational (Category C) as-is — it is NOT migrated.
-- The post-commit projector ack log is its own concern, distinct from the
-- new observation primitive. Cross-reference only.
-- See: genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §10 Stage 8.
--
-- Phase 4 — append-only operational log of doorway projector acks.
-- Source of truth: DHT (EconomicEvent entry, content_store zome,
-- action='ack-projection'). This table is rebuildable from any peer's
-- content_store storage projection by replaying the rea_projection signal
-- stream filtered to action='ack-projection'. P2P design gate Category C.

CREATE TABLE projection_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blob_hash TEXT NOT NULL,
    projector_agent_cid TEXT NOT NULL,
    emitted_at TEXT NOT NULL,
    source_action_hash TEXT NOT NULL UNIQUE  -- dedup by source DHT action
);

CREATE INDEX idx_projection_events_blob_emitted
    ON projection_events (blob_hash, emitted_at DESC);
