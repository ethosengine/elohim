DROP INDEX IF EXISTS idx_economic_events_bounded_by_has_point_in_time;
ALTER TABLE economic_events DROP COLUMN bounded_by;
