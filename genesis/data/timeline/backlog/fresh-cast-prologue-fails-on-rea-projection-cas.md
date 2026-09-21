---
id: "backlog-fresh-cast-prologue-fails-on-rea-projection-cas"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A fresh cast's prologue fails three MUST-SUCCEED legs on a REA-projection CAS race, then passes unchanged on a warm re-run"
slug: "fresh-cast-prologue-fails-on-rea-projection-cas"
written: "2026-09-21"
author: "serving-edge household-acceptance campaign, 2026-09-20/21 review"
status: "open"
priority: "medium"
jobs: [elohim, elohim-app]
cluster: "mesh-prologue-cast-and-env-gaps"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [mesh, prologue, rea-commitment, cas-race, readiness, cold-start]
---

**The fact.** On a fresh `MESH_RESET` cast, three MUST-SUCCEED prologue legs — `seed-base-corpus-via-A`,
`seed-commitments`, `seed-drill-custody` — failed with `HTTP 400 {"error":"REA projection changed
during authority read; deferred"}` (`F5-mesh-prologue.log`, EXIT=1 on all three). The identical
prologue re-run on the same, now-warm household passed all three unchanged (`G1-mesh-prologue-rerun.log`,
EXIT=0). The 400 originates in `rea_commitment_service.rs:787-812`'s `project_conductor_observation`:
it snapshots expected state, reads the conductor's authoritative record, then applies via
compare-and-swap (`rea_commitment_lifecycle::apply`); when the row changed between the snapshot and
the CAS (`ApplyOutcome::Deferred`, `db/rea_commitment_lifecycle.rs:43-46`), the call errors out to the
caller instead of re-reading and retrying. None of the three failing legs are among the prologue's
soft, retrying legs — `hc-mesh-prologue.sh` reserves backoff/retry for the `propagate-*-to-B` legs
(the separate stale-coordinator 503, see the sibling item below) — so a MUST-SUCCEED leg fails outright
on the first CAS race it hits.

**Evidence.** `F5-mesh-prologue.log:333-334` (`seed-base-corpus-via-A`, the `imagodei-portal`
project-epr write, `EXIT=1` at `:362`), `:2126-2128` (`seed-commitments`), `:2167-2192`
(`seed-drill-custody`) — all three carry the identical error body. `G1-mesh-prologue-rerun.log:339,2098,2159` —
same three legs, `EXIT=0`, same household, no code change between the two runs.
`elohim/elohim-storage/src/services/rea_commitment_service.rs:787-812` (`project_conductor_observation`,
the `Err(StorageError::InvalidInput(...))` arm on `ApplyOutcome::Deferred`). **Note:** the
`propagate-*-to-B` 503s visible in the same two logs are the unrelated stale-coordinator defect
(`election-ordering-503-retryable-but-permanent.md`) — cross-referenced here, not duplicated.

**Why it matters.** A CAS race on a cold household's first authority reads is expected — many seeders
write concurrently while conductors are still converging — but the prologue treats it as fatal on legs
it has itself marked non-negotiable, so a fresh cast is measurably less reliable than a warm one for
reasons that have nothing to do with the content being seeded. Nothing in `rea_commitment_service.rs`
or its callers documents `Deferred` as retryable; the caller decides once, on the first race, and gives up.

**Smallest next step.** A readiness predicate the prologue can wait on before its first authority read
on a cold household (e.g., a per-role check that the conductor is accepting authority reads without
CAS contention), rather than a re-run being the de facto cure. Failing that, the cheaper local fix:
have `project_conductor_observation`'s caller re-snapshot and retry a bounded number of times on
`Deferred` before surfacing the 400, the same bounded-retry-with-backoff shape the harness already uses
elsewhere for retryable storage backpressure.

**Links.** Evidence: `genesis/a2o/reports/recovery/serving-edge-20260920/F5-mesh-prologue.log`,
`G1-mesh-prologue-rerun.log`. Cross-reference (do not conflate):
`genesis/data/timeline/backlog/election-ordering-503-retryable-but-permanent.md`. Habit:
`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`. Cluster:
`genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md`.
