-- Spatial grounding for economic events.
-- at_location references a Place ID so consumption/production events
-- can be aggregated by geography for carrying capacity enforcement.
ALTER TABLE economic_events ADD COLUMN at_location TEXT;
CREATE INDEX idx_econ_events_location ON economic_events (at_location);
