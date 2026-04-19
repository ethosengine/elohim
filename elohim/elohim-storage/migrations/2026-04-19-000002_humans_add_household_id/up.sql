-- Source of truth: humans.json fixture + collectives DHT entries (household
-- membership). This column is the projection for fast allocations-by-household
-- joins powering /shefa/resources/category:content and /shefa/devices.
-- Category C projection; D3 seeder populates from fixture at import time.
ALTER TABLE humans ADD COLUMN household_id TEXT;
CREATE INDEX IF NOT EXISTS idx_humans_household ON humans(household_id);
