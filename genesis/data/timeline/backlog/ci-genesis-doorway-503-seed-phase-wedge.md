---
id: "backlog-ci-genesis-doorway-503-seed-phase-wedge"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis seed + E2E go red/503 across the whole run because the LIVE alpha doorway pod is still serving 503 (warm_stream wedge) — the cure landed in tree but had not reached the deployed pod at #1145"
slug: "ci-genesis-doorway-503-seed-phase-wedge"
written: "2026-06-14"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [5f4ae29a2941, 65d1d875c483, 4b6fe47bfdb3, a672ee4586c6, 193b7597a4cb]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, doorway, warm-stream, 503, seed-phase, live-substrate, deploy-timing, runtime-owned, measurement-by-restart, epr-router, caught-up]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1145/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1513/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1514/
  - genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts
  - genesis/a2o/features/protocol/landing-page-dogfood.feature
  - genesis/a2o/features/dataplane/inventory-convergence.feature
  - genesis/a2o/features/deployment/staging-validation.feature
  - genesis/a2o/steps/dataplane.steps.ts
  - genesis/a2o/steps/common.steps.ts
  - genesis/scripts/ci/restart-doorway-epr.sh
  - genesis/Jenkinsfile
  - doorway/doorway-service/src/warm_stream.rs
  - doorway/doorway-service/src/store.rs
  - doorway/doorway-service/src/main.rs
  - doorway/doorway-service/src/routes/health.rs
  - genesis/data/timeline/backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/manifests/cluster-state.yaml
---

# Genesis seed + E2E red on doorway 503 — the live alpha pod still wedged at #1145

## The failure

```
5f4ae29a2941  AssertionError [ERR_ASSERTION]: content fetch failed: 503        (genesis #1145, landing-page-dogfood.steps.ts:27)
65d1d875c483  AssertionError [ERR_ASSERTION]: commitments list failed: 503      (genesis #1145, landing-page-dogfood.steps.ts:92)
```

Occurrence evidence: both seen 1, first_build 1145, last_build 1145, job
`elohim-genesis`. They are two endpoints of ONE condition — the live alpha
doorway (`doorway-alpha.elohim.host`) serving **503 Service Temporarily
Unavailable** for the entire genesis run.

The two captured assertions are the cleanest E2E facets, but the 503 was
fleet-wide across the SEED phase first (build #1145 evidence, ci-investigator):

- **Seed Humans** — `exit code 1`, ~33 humans each `HTTP 503` from the doorway.
- **Seed Presences** — all 127 presences `HTTP 503`.
- **Seed Collectives** — `0 created, 62 failed` (all 503).
- **Seed Operator Bindings** / **Seed Projections** — `ERROR: Doorway not
  healthy — HTTP 503`.
- The pipeline then **pod-deleted both doorway pods** mid-run
  (`restart-doorway-epr.sh`): `pod "elohim-doorway-alpha-…" deleted`,
  recovering only after ~205s (`doorway serving 200 + conductor-connected …
  stable ×2 after ~205s`), and E2E ran against the recovering/still-503 pod —
  so the `landing-page-dogfood` content-fetch and commitments-list steps both
  got 503.

## Verdict

**infra / live-substrate + deploy-timing (runtime-owned) — NOT an in-tree code
bug for the sentinel to land.** This is the genesis-CI face of the
already-diagnosed-and-CURED **doorway warm_stream burst-starvation wedge**
(memory `project_doorway_wedge_unbounded_mongo_await`, CONFIRMED 2026-06-14 by
operator cluster-read: warm_stream replays the corpus with no backpressure →
burst-starves `/health` off the Tokio runtime under a `cpu:1` CFS quota →
liveness SIGKILL → ~109-restart crashloop → the pod serves 503 while down).
The genesis pipeline tests the **LIVE deployed** `doorway-alpha.elohim.host`,
not a freshly built image — so a wedged pod fails seed + E2E wholesale.

## Root cause — the cure had not reached the running pod at #1145

The wedge's durable cure DID land in the tree before #1145:

- `4dc862748` (2026-06-14 **12:52Z**) `fix(doorway): warm_stream pacing cure —
  bound Mongo ops + real inter-batch pace` (`DOORWAY_MONGO_OP_TIMEOUT_MS`
  default 2000ms in `store.rs`; `WARMUP_PACE_MS` default 8ms in
  `warm_stream.rs`).
- `54d2bb737` `fix(doorway): pre-warm hot cache from Mongo — eliminate the
  cold-start firehose (the cure)`.

Genesis #1145's **trigger commit** `7d27d3f5` (2026-06-14 **15:26Z**,
"doorway(alpha,alpha-b): capture crash logs + document liveness fix") is a
**descendant of both cure commits** — verified via
`git merge-base --is-ancestor 4dc862748 7d27d3f5` → true. So the cure was in
the SOURCE at #1145.

Yet #1145 (ran ~**16:48Z**) still hit the wedged doorway. The gap is
**deploy propagation, not the fix**: the cure only reaches
`doorway-alpha.elohim.host` after an **edge** pipeline builds the doorway image
and deploys it to alpha; a genesis run earlier in that window tests the
still-old (wedged) pod. The mid-pipeline `restart-doorway-epr.sh` pod-delete is
the pipeline's own band-aid — it restarts the pod but (if the pod still pulls
the pre-cure image, or the cold-start firehose still bursts during the ~205s
recovery while genesis is mid-run) the 503s persist into E2E.

This is the **same shape as museum trap "host-green ≠ CI-green / deploy-applied
≠ app-served"** and the `ci-alpha-cluster-degraded-substrate` "deploy succeeds
but app never serves" boundary — here the specific cause is a known wedge whose
fix is racing the deploy, not a degraded 6-peer soak. (Distinct from
`ci-alpha-cluster-degraded-substrate`: that one is shem-down / household-peers
crashlooping; #1145 ran with `shem=AVAILABLE, full topology — no reduced-scope
gating`, and the alpha doorway specifically was the 503 source, so these two
503 fps belong here, not there.)

## Current decision

**BLOCKED on the cure-bearing doorway image reaching the live alpha pod —
runtime/deploy-owned, confirmed by disappearance.** The fix is already in the
tree; the only remaining mover is operator/CI deploy propagation (an edge build
that ships the cure to `doorway-alpha`), after which a genesis re-run tests a
paced doorway and the seed phase + the two `landing-page-dogfood` 503 steps go
green. The sentinel cannot trigger that build (anonymous Jenkins MCP) and must
not touch the live pod (`kubectl` is operator-owned).

If the 503s RECUR on a genesis run that demonstrably tested a cure-bearing
doorway image (i.e. the deployed pod carries `4dc862748`/`54d2bb737` and
`/health` is stable pre-run), then the cure is INSUFFICIENT under genesis's
full-corpus seed burst — that residue is a NEW concern (re-open the
runtime-triage path for the wedge), not this deploy-timing one.

## Fix trail

- No tree change in this triage run — correct: the durable cure already landed
  (`4dc862748` + `54d2bb737`), and what remains is deploy propagation + observed
  confirmation, both operator/CI-owned. There is nothing bounded for the
  sentinel to add.
- Ledger: `5f4ae29a2941` + `65d1d875c483` set `status: blocked`
  (blocker: live alpha doorway wedged; cure landed, awaiting deploy
  propagation). **No `triaged_at_build` stamp** (nothing landed in THIS run; the
  fix predates the build and the build still failed). They disappear on a green
  streak the moment a cure-bearing doorway image serves alpha and genesis seeds
  cleanly.
- **Classifier note (harvester-side, not sentinel):** the two 503 lines
  fingerprint on the assertion message (`content fetch failed: 503` /
  `commitments list failed: 503`), which is stable and good — but they are facets
  of one doorway condition that ALSO produced ~33+127+62 seed-stage 503s the
  harvester did not separately fingerprint (those are stage-level, not
  assertion-level). The two captured fps are a faithful sample of the condition;
  collapsing them to one concern here is correct (N:1).

## 2026-08-28 — a second shape with the same symptom: genesis bounces doorway-alpha, then measures it

genesis #1512 (UNSTABLE, 6 scenarios failed) collapsed to three fingerprints, all doorway-alpha
unavailability: `ca6d43e8e0a8` `GET /health returned 503: <html>`, `cb05864906ff`
`DoorwaySessionError: <html>`, plus the by-design `1761c1e94b9b` (E2E_SHEM_HOST — see
`a2o-reconnect-storm-needs-process-control-pending`). The `<html>` body is the ingress answering
for a pod that is not ready. Cause is the pipeline's own ordering: `seedProjectionsStage()`
(genesis/Jenkinsfile:1024, invoked from Seed REA Commitments) runs `restart-doorway-epr.sh`, which
deletes the doorway-alpha pod and waits only for one `/health` 200 (conductor-connected) at the
public boundary — the pod restarted at 13:12Z and the E2E stage ran against it minutes later,
while it was still catching up (circuit `open`, `errorStreak` climbing). Not the warm-stream
wedge above; a measurement-by-restart anti-pattern inside genesis itself, the same shape the
edge pipeline documents for `[build:edge]`-to-measure.

Cure shape (pipeline, bounded): after the restart, gate the E2E stage on the same predicate the
edge quiesce gate uses for doorway A (`/health` `status == "online"` AND `p2p.caughtUp == true`
sustained, or at minimum `conductor.connected && !catching-up` for N consecutive polls) instead
of one 200; or move the doorway restart to AFTER E2E, since the projection it re-reads is not
what E2E measures. Until then the genesis measure carries a floor of ~2 self-inflicted fingerprints
after any run that restarts the doorway.

## 2026-08-28 (second pass, #1513/#1514) — the restart leg is now CURED IN TREE; the substrate leg is not

The prior section predicted "a floor of ~2 self-inflicted fingerprints after any
run that restarts the doorway." #1514 paid it exactly, in two new fingerprints:

```
a672ee4586c6  AssertionError [ERR_ASSERTION]: alpha-A /health: p2p.caughtUp is undefined (expected true)
              features/dataplane/inventory-convergence.feature:42 · steps/dataplane.steps.ts:451
193b7597a4cb  AssertionError [ERR_ASSERTION]: Doorway "alpha" at https://doorway-alpha.elohim.host
              is not healthy: status=degraded
              features/deployment/staging-validation.feature:28 · steps/common.steps.ts:870-873
```

Both in **E2E Verification**, both seen 1, first/last build 1514. A third,
older fingerprint belongs to this concern and had no canonical home until now:

```
4b6fe47bfdb3  AssertionError [ERR_ASSERTION]: alpha-A /health: p2p.caughtUp is false (expected true)
              seen 8, builds 1499..1513 — the SAME scenario/step as a672ee4586c6
```

Occurrence evidence: #1513 (13 failing scenarios) and #1514 (19) ran the
**same application code** — `git log 394f052cb..c4a148db6` is exactly one
docs-only commit (a habit atom, this backlog file, the generated
`habits.yaml`; 3 files, 27 insertions, zero application code).

### Verdict — infra + in-tree pipeline ordering, and the two are separable

**`undefined` and `false` are NOT the same failure**, and the split is
diagnostic rather than noise (`routes/health.rs`):

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub caught_up: Option<bool>,   // None when storage's reconcile task is not spawned
```

- `caughtUp: false` → a snapshot **was** cached with `Some(false)`: storage's
  projection-reconcile ran and reported *behind*. A live-but-behind substrate.
- `caughtUp` **absent** → the doorway had **no** p2p snapshot at all
  (`state.p2p_health` empty → `P2PHealth::default()`): a doorway too young, or
  one that could not reach storage, to have received its first snapshot.

`status=degraded` resolves the same way. Alpha runs `DEV_MODE=true`, so the
`conductor_connected || dev_mode` arm always reads `online`; the only reachable
path to `degraded` is the override

```rust
status: if serving.shedding || serving.degrading { "degraded" } else { status },
```

i.e. **the circuit breaker on this doorway's DECLARED PRIMARY storage peer**
(matthew) was shedding or in the slow regime. Root cause of that condition is
already canonical at `backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md`.

**What made #1514 worse than #1513** (Prometheus,
`kube_pod_container_status_restarts_total`, ns `elohim-alpha`, 13:30–15:00Z):

| pod | evidence |
|---|---|
| `elohim-matthew-alpha-0` | 0 → **1 restart at 14:36:00Z** — inside #1514's E2E window (build ended 14:38:06Z) |
| `elohim-susan-alpha-0` | 3 → 10 restarts, crash-looping across BOTH build windows |
| `elohim-gertrude-alpha-0` | 1 → 5 · `elohim-eve-alpha-0` 1 → 4 |
| `elohim-doorway-alpha-*` containers | **0** in-place restarts — the churn is pod RECREATION by this pipeline, not container crashes |
| doorway pod identity | `6bm8z` (already at restarts=3 before #1513 deleted it) → `tf5dd` 13:52Z (deleted by #1514) → `d6lts` **14:32Z**, i.e. ~6 min before #1514 ended |

So: matthew — doorway-alpha's declared primary — **restarted mid-E2E on
#1514 and not on #1513**. That is the substrate difference, and it explains
both the extra failures and the harder `undefined` reading.
(`process_start_time_seconds` returned no data for the window; that is an
unqueried gap, NOT evidence of no restart.)

### Root cause of the pipeline leg — a refresh scaffold that outlived its cause

`seedProjectionsStage()` deletes the doorway-alpha pod on **every** genesis run.
Its justification, written into `genesis/Jenkinsfile`, was:

> The router only refreshes at boot OR via SSE 'projection.registered' events
> … A pod-delete is the deterministic refresh.

**That claim was already stale when it was written.** `doorway-service`'s
`main.rs` runs a *Periodic EPR-router self-heal refresh (operator-free
recovery)* every `DOORWAY_EPR_REFRESH_SECS` (default **30**), whose sequence is
byte-for-byte the boot fetch — `resolve_epr_storage_pool` →
`fetch_projections_with_fallback` → `apply_epr_fallback_outcome` →
`prewarm_projected_shells` — with last-good preservation on failure. Its own
comment says it: *"the router self-populates once storage recovers — **no
kubectl restart needed**."* It landed in `379668123` on **2026-05-30**; the
Jenkinsfile comment was written on **2026-06-10**, eleven days later, carrying
the pre-refresh rationale forward unexamined.

The pod-delete is therefore not merely redundant — it is the **source** of the
measurement noise: deleting the pod resets the doorway's p2p snapshot cache
(→ `caughtUp` absent) and its upstream breakers (→ a cold `serving` view), and
the E2E stage measures that recovering pod ~6 minutes later. This is the
in-tree twin of the `[build:edge]`-fired-just-to-measure anti-pattern the root
`CLAUDE.md` documents.

## Current decision

**Split disposition — the pipeline leg is fixed in tree and awaits
disappearance; the substrate leg stays blocked on the operator.**

- Pipeline leg (`a672ee4586c6`): cured by the fix below. Confirmed by
  disappearance on a genesis green streak.
- Substrate leg (`193b7597a4cb`, `4b6fe47bfdb3`): **blocked**. What unblocks
  it is a stable alpha conductor fleet — matthew not restarting mid-run, and
  susan/gertrude/eve out of crash-loop — plus the matthew anchor divergence
  tracked in `self-heal-doorway-alpha-storage-breaker-matthew-rekey.md`. The
  sentinel must not touch the live cluster (`kubectl` is operator-owned) and
  cannot trigger builds (anonymous Jenkins MCP).

**Correction to this entry's earlier cure sketch:** the arm that proposed
*"move the doorway restart to AFTER E2E, since the projection it re-reads is
not what E2E measures"* is **unsafe and should not be taken** — E2E measures
`/` and `/lamad` rendering, which is exactly what the EprRouter repopulation
serves. The correct cure was never a reordering; it was noticing the restart
had become unnecessary. The other arm (gate E2E on the quiesce predicate) is
also rejected: post-restart churn is ~20 min, which genesis's 90-min clock
cannot absorb, and it would have cured the symptom by waiting out damage the
pipeline inflicted on itself.

## Fix trail (2026-08-28)

- `genesis/scripts/ci/restart-doorway-epr.sh` — **two paths, cheapest first.**
  Path 1 (default, no churn): wait `EPR_REFRESH_WAIT_SECS` (default 70 =
  2 × the 30s refresh cadence + slack, chunked with a progress line per 10s so
  the call site's `timeout(activity: true)` contract stays honest), then verify
  at the PUBLIC boundary that `/health` is 200 with `"connected":true` **and**
  a projected route answers 200 carrying `x-epr-router: dispatched` — the
  header `server/http.rs` sets only on an EPR-dispatched response, so its
  presence *is* proof the routing table is populated. Two consecutive OK polls,
  then exit 0 having deleted nothing. Path 2 (fallback, unchanged bytes): the
  original pod-delete + public-boundary rollout wait, taken only when path 1
  does not converge or when `EPR_FORCE_POD_RESTART=1`. Worst case is the old
  behaviour plus ~130s; best case replaces ~205s of churn with ~80s of waiting.
- `genesis/Jenkinsfile` — comment-only: the stale "only refreshes at boot OR
  via SSE" claim is corrected in place with the commit that falsified it, and
  the #1039 observation is re-scoped (it establishes that *a* handoff is
  needed, not that a pod-delete is the only one available). No CPS bytecode
  added; comments are stripped.
- Local verification available to the sentinel: `bash -n` clean on the script;
  the genesis gate carries no shell-lint step, and the real proof is a genesis
  run, which the sentinel cannot trigger. **Not fleet-confirmed.**
- Ledger: `a672ee4586c6` → `triaged`, `triaged_at_build: 1514`.
  `193b7597a4cb` → `blocked`. `4b6fe47bfdb3` → `blocked`, and given a canonical
  home here for the first time (it had been open and unowned since #1499).
- Museum: the class graduated as row **#16 measurement-by-restart** in
  `2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`.
