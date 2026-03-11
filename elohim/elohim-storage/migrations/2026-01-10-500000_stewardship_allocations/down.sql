-- Drop stewardship_allocations table
DROP INDEX IF EXISTS idx_alloc_unique_active;
DROP INDEX IF EXISTS idx_alloc_disputed;
DROP INDEX IF EXISTS idx_alloc_active;
DROP INDEX IF EXISTS idx_alloc_governance;
DROP INDEX IF EXISTS idx_alloc_steward;
DROP INDEX IF EXISTS idx_alloc_content;
DROP INDEX IF EXISTS idx_alloc_app_id;
DROP TABLE IF EXISTS stewardship_allocations;
