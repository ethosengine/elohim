DROP INDEX IF EXISTS idx_economic_events_scope_collab_cid;
-- SQLite does not support DROP COLUMN on older versions; this is a best-effort rollback.
-- For production use diesel migration redo to apply up.sql again.
