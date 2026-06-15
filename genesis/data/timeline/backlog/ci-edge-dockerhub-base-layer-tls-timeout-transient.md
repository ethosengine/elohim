---
id: "backlog-ci-edge-dockerhub-base-layer-tls-timeout-transient"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-edge image build fails on `FROM node:20-slim` base-layer pull — Docker Hub CloudFront TLS handshake timeout (edge #1078) → orchestrator #1249 Level-1 abort; transient, self-cleared on retrigger (#1079 green), NOT a code/deploy regression and NOT the genesis-CPS cascade"
slug: "ci-edge-dockerhub-base-layer-tls-timeout-transient"
written: "2026-06-15"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [e10ac62010f1]
jobs: [elohim-orchestrator, elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-orchestrator, elohim-edge, docker-hub, cloudfront, base-image-pull, node-20-slim, tls-handshake-timeout, buildkit, transient, infra-flake, retrigger-recovers, operator-owned, host-green-not-ci-green, museum-candidate, misleading-fingerprint-label]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1249/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1078/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1079/
  - elohim/holochain/Jenkinsfile
  - genesis/manifests/
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# elohim-edge Docker Hub base-layer TLS timeout — orchestrator Level-1 abort, infra transient

## The failure

```
fp e10ac62010f1  elohim-orchestrator — "red build, stage:Level 1: elohim-edge"
                 (elohim-orchestrator/dev #1249, FAILURE, seen 1, build 1249..1249)
```

The orchestrator fingerprint's label points at the dispatch stage; the actual fault is one layer down,
in the child edge build the orchestrator waited on.

Orchestrator #1249 (dispatch plan `[elohim + elohim-storybook + elohim-holochain] → elohim-edge → genesis`)
ran the `elohim` app build to UNSTABLE (non-aborting, as designed — see cascade-deadlock note below), then
at **`Level 1: elohim-edge`** triggered `elohim-edge » dev #1078` wait-for-result. That child failed, so the
orchestrator aborted before genesis was ever reached:

```
▶️ Triggering elohim-edge/dev [wait-for-result]...
Starting building: elohim-edge » dev #1078
Build elohim-edge » dev #1078 completed: FAILURE
❌ elohim-edge: FAILURE
...
ERROR: Build(s) failed: elohim-edge - Aborting
Finished: FAILURE
```

The root-cause signature is in **edge #1078's** image-build stage — pulling the `node:20-slim` base layer
from Docker Hub's CloudFront CDN:

```
#6 [2/6] WORKDIR /app
#6 ERROR: failed to copy: httpReadSeeker: failed open: failed to do request:
  Get "https://production.cloudfront.docker.com/registry-v2/docker/registry/v2/blobs/sha256/e5/
  e54aec64.../data?Expires=...": net/http: TLS handshake timeout
------
 > [2/6] WORKDIR /app:
Dockerfile:12
  10 |     FROM node:20-slim
  ...
  12 | >>> WORKDIR /app
error: failed to solve: failed to compute cache key: failed to copy: ...
  net/http: TLS handshake timeout
time="2026-06-14T21:50:13Z" level=fatal msg="no image was built"
```

Edge #1078's build description confirms nothing shipped: `hApp:NO | Push:Skip | Deploy:?` — the Rust+DNA+app
compile all completed, the image build hit the network blip on the base-layer fetch, `Push to Harbor` and all
`Deploy Edge Node` stages were skipped-due-to-earlier-failure.

**Occurrence evidence:** seen 1, first/last build 1249 (orchestrator). Edge #1078 checked out `41bbe2e6`
(`docs(backlog): shard-manifest/shard-location write path investigation`) — a non-genesis commit. Edge #1078
failed **2026-06-14 21:50 UTC**.

## Verdict — INFRA TRANSIENT (Docker Hub / CloudFront base-layer pull), self-cleared

- **Self-recovery proven by disappearance, no code change.** The very next edge build, **`elohim-edge » dev
  #1079 = SUCCESS`** (started ~21:49→completed shortly after #1078's failure window; the operator retriggered
  via `c6243f340` `[build:edge]`). Identical inputs minus the network blip → green. The TLS handshake timeout
  is a classic transient against Docker Hub's CDN, not a build-input fault.
- **Operator already diagnosed it** in commit `c6243f340`: "retrigger edge — transient Docker Hub TLS timeout
  on node:20-slim in edge #1078 (Rust+DNA+app all passed; nothing deployed) [build:edge]".
- **Host-green ≠ CI-green family** (museum reading §"The load-bearing reading"): the gap is the CI
  environment's egress to an external registry CDN, not the code.

## Root cause

A buildkit/nerdctl base-image layer fetch (`FROM node:20-slim`) to `production.cloudfront.docker.com` returned
`net/http: TLS handshake timeout`. The CI image build has **no pull-through cache / retry** for docker.io base
images, so a single CDN TLS hiccup aborts the whole edge image build (`no image was built`), which the
orchestrator's wait-for-result reads as a hard FAILURE and aborts the level on.

## Disambiguation — this is NOT the genesis-CPS breakage, and NOT the cascade-deadlock gate

The dispatch context (2026-06-15 ~00:00–00:32 genesis CPS `MethodTooLargeException` window; commits
b49fd9906/fc7b1aad6/c1f853e0c; genesis #1151 dying at parse) is **chronologically and causally unrelated** to
this finding:

- **Timing:** edge #1078 failed 2026-06-14 **21:50 UTC** — roughly 2 hours *before* the genesis-CPS window.
  (The ledger `ts` is harvester capture time, not build time.) The edge failure literally predates the genesis
  breakage.
- **Plan order:** edge precedes genesis in the dispatch plan (`… → elohim-edge → genesis`); genesis-`seed-content`
  is Level 2, never reached because edge (Level 1) aborted first. Genesis could not have caused this.
- **Not the cascade-deadlock live-target gate** (`project_cascade_deadlock_live_target_gate`): that gate is on
  `elohim-app`'s `E2E Testing - Alpha Validation`, already wrapped `catchError→UNSTABLE` (51d16c4d4). In #1249
  the `elohim` app correctly went **UNSTABLE (not abort)** and the orchestrator proceeded to Level 1 — the abort
  came from edge's *own* image-build FAILURE, exactly the designed hard-gate path. The gate fix is working as
  intended; this is an orthogonal external-infra flake.

The sibling genesis-CPS fingerprint (6867ad3fed6d, elohim-genesis #1146→#1151) is owned by the live session's
structural `genesis/Jenkinsfile` refactor and is intentionally NOT addressed here.

## Current decision

**BLOCKED — operator-domain (CI egress / registry resilience), low priority because it self-heals on retrigger.**
No bounded code fix in the tree: the failure surface is an external Docker Hub CDN TLS timeout. The nominal
unblock is an **operator/infra move**, not a repo change of mine:

- A **pull-through cache for docker.io base images** (e.g. Harbor proxy-cache project, or registry mirror) so
  `FROM node:20-slim` resolves against a warm in-cluster cache instead of the public CloudFront edge — see the
  related `harbor-registry-spof` backlog item; OR
- a **buildkit base-image pull retry** in the edge image-build invocation (`elohim/holochain/Jenkinsfile`) so a
  single layer-pull TLS hiccup retries rather than aborting the build.

Until then this stays a known retrigger-recovers transient. **`ci_status: blocked`, no `triaged_at_build`
stamp:** I landed no fix, and stamping would point the harvester's recurrence reference at the *orchestrator*
green streak — which can independently re-fail at the genesis-CPS stage (a different fingerprint the live
session owns), and `last_build > triaged_at_build` would misread that as "the edge fix didn't take." Leaving it
`blocked` sidesteps the false recurrence reference; the orchestrator-fingerprint disappears naturally once a
clean orchestrator/dev run completes Level 1 with a warm/lucky base-layer pull.

## Fix trail

- No tree fix landed (external-infra transient; no bounded repair). Canonicalization only.
- Operator already retriggered edge via `c6243f340` `[build:edge]` → **edge #1079 SUCCESS** (recovery confirmed).
- Edge-job recovery is proven (#1079 green); the **orchestrator** fingerprint disappearance is pending the next
  orchestrator/dev run reaching Level 1 cleanly (note: #1079 was a standalone `[build:edge]` retrigger, not an
  orchestrator run, so it is not itself the orchestrator job's green-streak evidence).

## Museum

Matches the host-green ≠ CI-green / external-registry-transient family (museum §"The load-bearing reading",
2nd cluster). Mechanism-distinct from the existing `ci-jenkins-k8s-pod-exec-websocket-transient` (k8s pod-exec
websocket 404) and `ci-edge-p2p-sim-docker-compose-missing` (daemonless-runtime) entries — a **Docker Hub
base-layer pull TLS timeout** is its own surface, hence its own concern. `seen=1` (first occurrence) — tagged
`museum-candidate`, NOT graduated into the museum record (the ≥3-distinct-shift recurrence bar is unmet). If a
docker.io base-image pull TLS/network transient recurs across ≥3 shifts, graduate the lesson into
`2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` (extend; never fork).
