-- migrations/2026-05-11-110000_projection_events/down.sql
DROP INDEX IF EXISTS idx_projection_events_blob_emitted;
DROP TABLE IF EXISTS projection_events;
