-- SQLite doesn't support DROP COLUMN directly in older versions
-- For schema rollback, we need to recreate the table
-- This is a simplified rollback - in production, would need to preserve data

-- Note: SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN
-- For older versions, this requires table recreation

ALTER TABLE content DROP COLUMN content_body;

-- Revert schema version
UPDATE schema_version SET version = 2;
