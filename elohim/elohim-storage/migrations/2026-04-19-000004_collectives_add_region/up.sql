-- Add region to collectives for geographic distribution bucketing.
-- Operator-declared metadata; nullable (unknown region = not yet configured).
ALTER TABLE collectives ADD COLUMN region TEXT;
CREATE INDEX IF NOT EXISTS idx_collectives_region ON collectives(region);
