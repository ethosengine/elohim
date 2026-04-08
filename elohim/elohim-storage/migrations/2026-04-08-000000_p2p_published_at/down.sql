-- Reverses p2p_published_at migration. Category C (operational).
-- Requires SQLite >= 3.35.0 (2021-03-12) for ALTER TABLE ... DROP COLUMN.
-- System SQLite on Ubuntu 22.04+ and all bundled builds satisfy this.
DROP INDEX IF EXISTS idx_content_p2p_unpublished;
ALTER TABLE content DROP COLUMN p2p_published_at;
