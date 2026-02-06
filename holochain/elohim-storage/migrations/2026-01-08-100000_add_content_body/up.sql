-- Add content_body column for inline text content storage
-- This allows storing markdown, JSON quizzes, etc. directly in SQLite
-- without needing blob storage for simple text content.
--
-- For large/binary content, continue using blob_hash/blob_cid references.

ALTER TABLE content ADD COLUMN content_body TEXT;

-- Update schema version
UPDATE schema_version SET version = 3;
