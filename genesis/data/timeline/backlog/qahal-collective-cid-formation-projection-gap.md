---
id: "backlog-qahal-collective-cid-formation-projection-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "household-formation E2E — collective_cid never stamped on family-dowell: CollectiveProjected signal path needs ceremony/peer-subscription verification"
slug: "qahal-collective-cid-formation-projection-gap"
written: "2026-06-07"
author: "tackle-top-three investigation (wf_e3cc3753-f1a + follow-up agent)"
status: "open"
priority: "high"
jobs: [elohim-genesis]
tags: [qahal, household-formation, collective-cid, reconcile-controller, dna-signal, e2e, app-scope]
cites:
  - genesis/a2o/features/qahal/household-formation.feature
  - genesis/seeder/src/seed-household-formation.ts
  - genesis/seeder/src/seed-collectives.ts
  - elohim/elohim-storage/src/reconcile/controller.rs
---

# `collective_cid not stamped — formation projection has not run` is a SIGNAL-PATH gap, not the FK storm

Two scenarios fail in `features/qahal/household-formation.feature`: "All three
members are affirmed participants" and "The household collective is coherent —
family-layer, CID-stamped". The 2026-06-07 investigation proved these do NOT
trace to the jessica-alpha FK storm (fixed by the stub-materialize change in
`do_account_import` — see `participation_without_parent_fk_fails_and_stub_materializes`).

## Mechanism (evidence-backed, medium confidence on the live trigger)

- `collective_cid` is stamped ONLY by the reconcile controller's
  `on_collective_projected` handler (`controller.rs:760-783` slug-alias merge;
  `:816-837` fallback create+stamp), driven by a `CollectiveProjected` DNA
  signal from the **imagodei conductor** the storage peer subscribes to
  (`main.rs:1861,1898`).
- The signal originates when `seed-household-formation.ts:510-524` calls
  `create_collective` on the FOUNDER's (matthew's) conductor.
- The stamp targets **`family-dowell`** (the charter's `slugAlias`), a
  DIFFERENT row from the `household-dowell`/community ids the account-import
  joins use. Fixing the joins cannot stamp the CID.

## To verify in the next CI run (live observation needed)

1. Does the household-formation ceremony actually run in CI, against
   matthew's conductor?
2. Does `E2E_STORAGE_URL` point at the storage peer that SUBSCRIBES to
   matthew's imagodei conductor (signal delivery), and did
   `seed-collectives.ts` seed `family-dowell` to that same peer (the
   slug-alias merge needs the row present)?
3. Does the ceremony's affirmation choreography project participations under
   the post-slug-merge `family-dowell` id (not a transient `collective:uhCkk…`
   id)? Otherwise the triad-participants scenario stays red even with the FK
   fixed.

## Riding coherence cleanup

Collectives app-scope is split: `/db/collectives` seed POSTs land under the
legacy `h_app_id="lamad"` (`http.rs` `extract_app_context` legacy-prefix
branch) while account-import participations use `qahal`. The FK is on bare
`collectives(id)` so this doesn't break inserts, but scoped reads
(`get_collective`) see inconsistent worlds. Decide ONE scope (tables default
`'qahal'`) and reconcile the seeder + legacy-prefix branch. The stub
materialization deliberately uses "lamad" today so the projection
controller's upsert converges instead of PK-colliding — flip both together.
