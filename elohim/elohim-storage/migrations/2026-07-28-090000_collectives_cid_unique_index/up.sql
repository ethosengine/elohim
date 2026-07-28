-- Review finding (commit 33225a8ae, LOW): project_collective's read-then-branch
-- (id_for_collective_cid check, then insert/update) is not transactional
-- against a concurrent writer racing the same collective_cid — both callers
-- can pass the "no row carries this cid yet" check before either commits,
-- each minting its own cid-bearing row. idx_collectives_cid
-- (2026-05-29-000000_hub_identity_cid_slug) is a NON-unique index: it
-- accelerates the lookup but never enforced it.
--
-- The uniqueness scope is (h_app_id, collective_cid), NOT collective_cid
-- alone: h_app_id is this table's multi-tenant boundary (every lookup here —
-- id_for_collective_cid, collective_cid_of_id, list_collective_cid_inventory —
-- filters by it), and two different app scopes legitimately CAN carry the
-- same cid string (they address different DHT networks). A bare
-- collective_cid unique index would wrongly forbid that — proven by the
-- existing `inventory_excludes_null_empty_and_dissolved_cids` regression,
-- which seeds the same cid under two different h_app_id scopes on purpose to
-- pin cross-tenant isolation.
--
-- Exact-duplicate cleanup MUST run before the unique index is created, or
-- CREATE UNIQUE INDEX itself fails on any node that already raced into a
-- duplicate. Keep only the lowest-rowid row per (h_app_id, collective_cid).
-- This is deliberately conservative: it removes ONLY rows that are
-- exact-duplicate on (h_app_id, collective_cid), and NEVER touches NULL-cid
-- rows (pre-coherence seed rows are excluded by design — see
-- project_collective's step-3 doc comment and list_collective_cid_inventory's
-- doc comment in db/collectives.rs).
DELETE FROM collectives
WHERE rowid NOT IN (
    SELECT MIN(rowid) FROM collectives WHERE collective_cid IS NOT NULL GROUP BY h_app_id, collective_cid
)
AND collective_cid IS NOT NULL;

-- Partial unique index: only NOT NULL collective_cid values are constrained,
-- so the many un-anchored (NULL) pre-coherence seed rows stay legal.
CREATE UNIQUE INDEX IF NOT EXISTS idx_collectives_cid_unique ON collectives(h_app_id, collective_cid) WHERE collective_cid IS NOT NULL;
