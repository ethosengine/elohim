-- Reverse M2 additions to recovery_requests.
DROP INDEX IF EXISTS idx_recovery_requests_human_id;

-- SQLite does not support DROP COLUMN prior to 3.35. Use the standard
-- table-rebuild idiom if rolling back on older SQLite. On the versions
-- bundled with Diesel's vendored SQLite (3.40+), DROP COLUMN works.
ALTER TABLE recovery_requests DROP COLUMN required_witness_count;
ALTER TABLE recovery_requests DROP COLUMN human_id;
