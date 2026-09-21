-- Reverting drops the carried-record cache. Nothing notarized is lost: the row
-- keeps its declared head and anchor, and every peer falls back to the
-- conductor-probe path (the re-probe ladder) exactly as before story 1.4b.
ALTER TABLE content DROP COLUMN declared_head_record_json;
