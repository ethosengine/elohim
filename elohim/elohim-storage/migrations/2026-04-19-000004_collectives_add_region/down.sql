DROP INDEX IF EXISTS idx_collectives_region;
-- SQLite does not support DROP COLUMN; migration is effectively irreversible.
-- To roll back: drop and recreate the collectives table without the region column.
