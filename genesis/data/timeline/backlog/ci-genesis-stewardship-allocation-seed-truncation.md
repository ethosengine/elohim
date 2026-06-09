---
id: "backlog-ci-genesis-stewardship-allocation-seed-truncation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis stewardship-allocation E2E fails — seeder idempotency read truncates at limit=10000, affinity stewards never seeded (allocations matthew-only)"
slug: "ci-genesis-stewardship-allocation-seed-truncation"
written: "2026-06-08"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
# seeder idempotency-truncation CAUSE is landed (ec5937287 + fbe3c6d70 pagination);
# the residual stewardship-scenario red is the provenance/under-seed gap, re-pointed
# to backlog-seed-provenance-anchor-gap. Held at `wip` (not active.*) because the
# scenario is not GREEN and `active.*` is /deliver's tier-3 verdict to mint, not triage/cartographer.
ci_status: in-progress
fingerprints: [0c4425ee19d3, afc361e61c2d, 401e1c60291d]
jobs: [elohim-genesis]
relatedNodeIds:
  - "backlog-seed-provenance-anchor-gap"
tags: [ci, elohim-genesis, seeder, stewardship, allocations, idempotency, pagination, e2e, persistent-pvc, resolved-cause-residual-provenance]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1104/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1102/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1100/
  - genesis/seeder/src/seed-stewardship.ts
  - genesis/a2o/features/content/stewardship-allocation.feature
  - genesis/a2o/steps/stewardship.steps.ts
  - elohim/elohim-storage/src/db/stewardship_allocations.rs
---

# Genesis `Content Stewardship Allocation` E2E — affinity stewards never land; allocations come back matthew-only

## The failure

`elohim-genesis/dev` build #1104 (UNSTABLE), three a2o E2E scenario assertions in
`features/content/stewardship-allocation.feature`, all with the same shape — the
allocation set for the queried content contains ONLY `matthew-dowell`:

```
✖ Then Adam should be listed as a steward with the highest ratio   # value-scanner
AssertionError [ERR_ASSERTION]: Adam (adam-firstman) not found in allocations: matthew-dowell

✖ Then Eve should have the highest allocation ratio                 # public-observer
AssertionError [ERR_ASSERTION]: Eve not found in allocations

✖ Then Pastor Pete should be listed as a steward                    # fct
AssertionError [ERR_ASSERTION]: Pastor Pete (pete-pastor) not found in allocations: matthew-dowell
```

These are ONE concern (N:1) — three category-affinity scenarios all observing the
same missing-allocation symptom from one broken seeding step. Ledger fingerprints:
`0c4425ee19d3` (Adam), `afc361e61c2d` (Eve), `401e1c60291d` (Pete); each `seen: 1`,
`first_build = last_build = 1104`.

Occurrence-spread (from the logs, beyond the ledger's harvest window): the Pete/`fct`
assertion also fired at #1100 and #1102 (single match each). At #1104 it WIDENED to
Adam (value-scanner) and Eve (public-observer) as well — consistent with the persistent
allocations table degrading run-over-run (see Root cause).

## Verdict

**real — a bounded seeder idempotency defect, not a flake, not infra.**
`getFlakyFailures(#1104)` returns none; the failure is deterministic and reproduces
the same three scenarios. The affinity-allocation seed step ran and resolved
categories correctly, but wrote essentially no new allocations, leaving the
test-selected content items with only their pre-existing `matthew-dowell` row.

Not a museum pipeline-mechanism trap (no NOT_BUILT/superseded, no sccache, no
`#[ignore]`, no webhook double-fire, no baseline-rollback). It surfaces as UNSTABLE
through the catchError-wrapped E2E stage — that *delivery* shape matches the museum's
"cascade-hidden test surface" row, but the root cause is a code defect in the seeder,
so no museum extension is warranted.

## Root cause

The category map is healthy — the seeder logged `Mapped 3385 content items to
categories` with `value-scanner: 1865 items (mapped)`, `public-observer: 441 items
(mapped)`. So content is NOT falling through `getStewardAllocations`' matthew-default
at resolution time. The break is in the **idempotency diff**:

`StewardshipClient.getContentWithAllocations()` read existing allocations with a single
`GET /db/allocations?active_only=true&limit=10000` — **no pagination**. On a persistent
store (the alpha conductor data dir is a persistent PVC; genesis `RESET_STORAGE`
defaults `false`, so the `stewardship_allocations` table accumulates across builds),
the table exceeds 10000 rows. The build logged exactly:

```
Found 10000 existing (content, steward) allocations
Found 3530 content items in database
...
Batch 18/18: 0 created, 28 failed
Stewardship allocation complete!
   Created: 3
   Failed: 1725
   Errors:
   - scenario-value-scanner-... : UNIQUE constraint failed:
       stewardship_allocations.h_app_id, .content_id, .steward_presence_id
```

`Found 10000` is the page being TRUNCATED at the limit. The truncated existing-set
makes the diff incomplete: every (content, steward) pair beyond the first page reads
as "missing", is re-POSTed, and the storage bulk handler returns it as `failed`
(uniqueness Err — `http.rs` `handle_bulk_create_allocations`). Net result `Created: 3,
Failed: 1725` — almost nothing new lands. Meanwhile content items that were only
PARTIALLY seeded by earlier runs (matthew present, affinity stewards absent) never get
repaired, because the seeder believes — from the truncated set — that their pairs
already exist. The test's `getAllocationsForContent(contentId)` (`GET
/db/allocations/content/{id}`, a per-content aggregate, NOT limit-bounded) then reads
the true frozen state for those items: matthew only.

Why it widened at #1104: as the persistent table grows, which content items land in
the first-10000 page (ordered `allocation_ratio DESC`) shifts, so the set of items left
unrepaired drifts — value-scanner and public-observer joined fct in the truncation
shadow at #1104.

## Current decision

**SEEDER CAUSE RESOLVED — `ec5937287` (seeder allocation idempotency truncation) + `fbe3c6d70`
(`listAllocations` pagination, persona drift Susan→Jessica).** The `limit=10000` existence-read
truncation described below is fixed; "Allocation ratios sum to ~1.0" is **GREEN on #1106**. Status
bumped to `active.alpha` — the originally-diagnosed mechanism (truncated idempotency diff) no longer
holds.

**RESIDUAL (re-pointed):** the stewardship-*affinity*-scenario reds ("multiple stewards", "reflects
human affinities", "faith content stewarded by pastoral affinity") that persisted on #1106 are **NOT**
this seeder truncation. Per the completed root-cause investigation, they are the **provenance/under-seed
gap** — the bulk ~1865 affinity items sit behind `require_provenance` un-stamped because the alpha stack
is peer-starved (the drain never stamps `p2p_published_at`). The exact-count match (5 value-scanner /
4 public-observer scan windows == the provenance-passing category-None tag-only counts) proves the stack
returned only the few provenance-passing rows, not that allocations are missing. The residual now lives
in the master capture **`seed-provenance-anchor-gap.md`** (see `relatedNodeIds`). This entry is retained
(not deleted) to preserve the fingerprint trail (`0c4425ee19d3` / `afc361e61c2d` / `401e1c60291d`); its
*cause* is closed, its *residual symptom* is re-pointed.

The lesson (a `limit=N` existence-read silently truncating an idempotency diff on a persistent-PVC
store) remains reusable seeder-hardening guidance worth graduating to the museum.

A second, related defect this run did NOT change (documented, not yet acted): the
persistent allocations table carries stale partial allocations from old runs and is
never reset (`RESET_STORAGE=false`). The pagination fix repairs partial content on the
next run (the missing affinity pairs now POST), so the test should go green without a
reset — but a periodic `RESET_STORAGE=true` seed (or a seeder reconcile that deletes
allocations not in the current category map) is the durable hygiene. See the sibling
`seed-provenance-anchor-gap.md` "Seeder idempotency partial-write" note — same seeder
home, same partial-write family; that note's per-content-vs-per-steward guard was
already fixed (the code now diffs per-steward), but the truncation defeated it.

## Fix trail

- `genesis/seeder/src/seed-stewardship.ts` — `getContentWithAllocations()` now PAGES
  through `/db/allocations` with `limit`/`offset` (10000-row pages) until a short/empty
  page, building a COMPLETE existing-set regardless of accumulated row count. The DB
  layer already wires both params (`db/stewardship_allocations.rs` `AllocationQuery`
  `limit`/`offset`; `list_allocations` applies both), and the doorway proxies `/db/*`
  query params transparently — no storage or doorway change needed.
- Local verification: `npx tsc --noEmit` clean; `npx vitest run` → 275 passed / 9
  skipped (no regression; no test directly covers this client method — a unit test that
  mocks a >10000-row paged response is a worthwhile follow-up).
- Confirmation signal: a genesis build > #1104 will, on its stewardship seed step,
  diff against the full existing-set, POST the genuinely-missing affinity pairs for the
  partially-seeded value-scanner/public-observer/fct items, and the three "not found in
  allocations" assertions disappear. The build may still go UNSTABLE for the unrelated
  degraded-substrate condition (`ci-alpha-cluster-degraded-substrate.md`) — these
  fingerprints close independently on the allocation scenarios passing.
