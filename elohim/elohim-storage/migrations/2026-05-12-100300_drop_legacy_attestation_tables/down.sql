-- Reverse is intentionally not supported. The post-consolidation tree has no path
-- back to these per-type tables; engineers reaching for `down` should restore from
-- backup or revert the migration's commit.
SELECT 1;
