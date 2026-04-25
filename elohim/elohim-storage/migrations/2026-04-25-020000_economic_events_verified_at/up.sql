-- EPR Phase 2B — B.6/I4/I5: verified_at on economic_events projection rows
-- Source of truth: epr_atoms.verified_at (this column is a mirrored projection).
-- When the projector runs, it stamps the source EPR's verified_at into this
-- column so pillar readers can see verified-state without re-querying epr_atoms.
-- On revocation sweep, this column is cleared (set to NULL) atomically with the
-- corresponding epr_atoms.verified_at clear — within the same reconciliation pass.
-- This satisfies invariant I4 (revocation propagation) and I5 (verified-state
-- consistency): no projection row persists with stale trust state after a sweep.

ALTER TABLE economic_events ADD COLUMN verified_at TEXT;
