DROP TABLE IF EXISTS imagodei_observations;

-- SQLite does not support DROP COLUMN; session_intent_json and intent_set_at
-- columns on local_sessions are left in place on rollback.
