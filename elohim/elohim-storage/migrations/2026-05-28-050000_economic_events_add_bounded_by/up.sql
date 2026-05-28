-- Add bounded_by column to economic_events for REA compute-commitment back-reference.
-- Nullable for backward compatibility with pre-roadmap events; new emits required to populate.
-- Source of truth: Holochain DHT (the EconomicEvent entry's bounded_by field); this column
-- is the operational projection used by bounds_validator and rate_history for fast queries.
ALTER TABLE economic_events ADD COLUMN bounded_by TEXT;
CREATE INDEX idx_economic_events_bounded_by_has_point_in_time ON economic_events(bounded_by, has_point_in_time);
