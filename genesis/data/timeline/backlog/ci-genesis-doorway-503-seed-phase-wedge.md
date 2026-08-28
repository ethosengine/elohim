---
id: "backlog-ci-genesis-doorway-503-seed-phase-wedge"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis seed + E2E go red/503 across the whole run because the LIVE alpha doorway pod is still serving 503 (warm_stream wedge) — the cure landed in tree but had not reached the deployed pod at #1145"
slug: "ci-genesis-doorway-503-seed-phase-wedge"
written: "2026-06-14"
author: "ci-failure-triage"
status: "backlog"
priority: "high"
ci_status: blocked
fingerprints: [5f4ae29a2941, 65d1d875c483]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, doorway, warm-stream, 503, seed-phase, live-substrate, deploy-timing, runtime-owned]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1145/
  - genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts
  - genesis/a2o/features/protocol/landing-page-dogfood.feature
  - doorway/doorway-service/src/warm_stream.rs
  - doorway/doorway-service/src/store.rs
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
