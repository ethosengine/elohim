---
name: project_clone_isolation_and_discovery_cost_findings_2026_09_07
title: Clone isolation breaks at the projection plane; feedback discovery is O(N)
description: "Measured 2026-09-07 on the household mesh: clone-cell content escapes via the role-keyed storage projection + sync into a receiving peer's BASE cell; feedback discovery sweeps 8 members/60 s over all history — bites in any group-space (D6) or correction-convergence design."
metadata:
  type: project
---

**Two substrate facts from the 2026-09-07 overnight shift (both have backlog atoms with receipts):**

1. **Clone isolation holds only in the authoring peer's DHT.** The storage `content` projection is keyed by role (`h_app_id`), not cell; a clone write lands as an unqualified projection row, crosses the Automerge sync plane, and a receiving peer's storage re-authors it into its BASE lamad cell (fresh action). Base-cell DHT delta on the author: empty on all four DHT planes; projection +2 rows, sync +2 docs; jessica's base chain gained the content under a new anchor. → `backlog/clone-content-escapes-via-shared-projection.md`. **D6 (group clone spaces) now requires cell-qualified projection + sync before any production clone** — a cell selector alone never isolates downstream. Per-clone cost with gossip: RSS +1.6 MiB, disk +2.1 MiB (~8× idle), FDs +21, threads +10.
2. **Feedback discovery is O(N) in the peer's whole history.** `feedback_projector` visits 8 subscription members per 60 s sweep with no cursor and republishes only after a clean sweep; N = every record/correction ever seen (14→127 in a day; lag 107→333 s). Contract §3's "cost ∝ subscribed set" is not the implementation. → `backlog/feedback-discovery-sweep-is-o-n-in-history.md` (slice-2 D0 row). The a2o lane pins `ELOHIM_FEEDBACK_SWEEP_SECONDS=5`; the product default is untouched.

**Also:** delegated-compute cross-peer authority PASSED on the household mesh (jessica as provider, receipt `genesis/a2o/reports/delegated-compute/mesh-20260907T094431Z/`); the shem/Adam leg needs the operator-owned Secret + worker image digest. Fresh-mesh prologue cannot seed the base corpus: seeder `create_content` lacks `reach` (+ 'step' relationship vocab).

See [[project_holochain_post_commit_signals_are_cell_local]], [[feedback_sealed_decisions_must_not_outrun_evidence]], [[project_content_sync_plane]].
