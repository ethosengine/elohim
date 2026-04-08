DROP INDEX IF EXISTS idx_content_p2p_unpublished;
ALTER TABLE content DROP COLUMN p2p_published_at;
