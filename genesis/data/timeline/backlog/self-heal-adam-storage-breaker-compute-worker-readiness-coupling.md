---
id: "backlog-self-heal-adam-storage-breaker-compute-worker-readiness-coupling"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim.host's breaker to adam storage :8090 sits Open for hours because the Adam compute-worker sidecar crash-loops inside the storage pod: one NotReady container makes the whole pod NotReady, the ClusterIP Service drops adam's only endpoint, and the doorway sees connection failures from a storage process that is itself Ready"
slug: "self-heal-adam-storage-breaker-compute-worker-readiness-coupling"
written: "2026-10-22"
author: "runtime-triage"
status: "backlog"
priority: "high"
self_heal_status: blocked
severity: high
fingerprints: [ccce437a2ecf]
nodes: [alpha-b, elohim-adam-alpha, shem]
relatedNodeIds: []
tags: [self-heal, circuit, upstream-breaker, adam, compute-worker, pod-readiness, sidecar-coupling, faithful-messenger, elevate-arm]
cites:
  - https://elohim.host/admin/self-healing
  - https://elohim.host/health
  - https://elohim.host/p2p/status
  - https://doorway-alpha.elohim.host/p2p/status
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1683/consoleText
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1684/consoleText
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1686/consoleText
  - genesis/orchestrator/scripts/render-compute-worker.mjs
  - genesis/orchestrator/data/adam-compute-worker.json
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - doorway/doorway-service/src/routes/upstream_health.rs
  - genesis/agentic/.epr-meta/measure-runs-on-a-peer.habit.md
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - .claude/data/runtime-findings.jsonl
  - .claude/data/runtime-cursor.json
---

# The adam storage breaker opens because the compute worker beside it is crashing

## What is exhausted

Ledger line (poll 232, 2026-10-22T15:58:17Z):

```
{"fp": "ccce437a2ecf", "class": "self-heal-exhaustion", "node": "alpha-b",
 "provenance": "circuit:http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090",
 "line": "upstream http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090 circuit Open >= 3 consecutive polls"}
```

The poller's window for `alpha-b` (`.claude/data/runtime-cursor.json`) shows the
upstream to adam Open across polls, with the error streak still climbing:

```
sample 2  elohim-adam-alpha  open  errorStreak 426  skipped true
sample 3  elohim-adam-alpha  open  errorStreak 428  skipped true
sample 4  elohim-adam-alpha  closed  errorStreak 0                 (a brief recovery)
sample 5  elohim-adam-alpha  open  errorStreak 21
sample 6  elohim-adam-alpha  open  errorStreak 128
sample 7  elohim-adam-alpha  open  errorStreak 292   (the filing poll)
```

Recurrence evidence, **2026-10-27** (same readiness-coupling concern as fingerprint
`ccce437a2ecf`):

- Edge [#1739](https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1739/consoleText):
  `back-off 5m0s restarting failed container=compute-worker ... CrashLoopBackOff`
  and `ERROR: Adam compute worker activation unproven` on `elohim-adam-alpha-0`.
- Edge [#1740](https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1740/consoleText):
  `https://elohim.host/epr-head/autonomous-entity-epic: HTTP 502`. Adam's storage
  process remains healthy while the failed worker removes the pod's :8090 endpoint.

While the circuit was Open, `/p2p/status` on elohim.host answered with peer
`12D3KooWPF5c…`, which is matthew (`doorway-alpha.elohim.host/p2p/status` reports
the same id). The doorway's pool upstream took over for the routes that can move to
it. The app shell has no such fallback, so `GET https://elohim.host/` returned 503
in edge #1680, #1682 and #1684 ("a visitor asking for the app was handed no page at
all"). The B-side apex served nothing for most of 2026-10-22.

## Root-cause inventory

The breaker is reporting correctly. The cause is one level below it, in how the
pod is put together.

1. `genesis/orchestrator/scripts/render-compute-worker.mjs:286-289` pushes a
   `compute-worker` container into the **storage** StatefulSet's pod spec
   (`elohim-adam-alpha`), next to `elohim-node`. It has to sit there: it reaches
   storage at `COMPUTE_API_URL=http://127.0.0.1:8090` and shares the node-local
   compute token. It has no readinessProbe, so Kubernetes counts it Ready only while
   its process is running.
2. `genesis/orchestrator/data/adam-compute-worker.json` turned the worker on
   (`enabled: true`) on dev in e1967b598 and 30e8c2197. Its image failed at the UBI10
   x86-64-v3 loader on shem's Sandy Bridge CPU (see the `measure-runs-on-a-peer`
   habit, DELTA 2026-10-22), so the container crash-looped.
3. Kubernetes counts a pod Ready only when **every** container in it is Ready. Edge
   #1683's rollout evidence: `elohim-adam-alpha-0=Running/NotReady[node=shem;
   containers=compute-worker=false/CrashLoopBackOff;elohim-node=true/;]`. Storage
   itself was Ready, but the pod was not.
4. `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml:644-668`
   is the `ClusterIP` Service `elohim-adam-alpha`, port `storage-db` 8090. It does not
   set `publishNotReadyAddresses`, so a NotReady pod has no endpoint behind it.
   Connections to `elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090` fail. The
   doorway's warm-up says so too: `lastError: "Failed to connect to
   http://elohim-adam-alpha…:8090/api/v1/cache/stream: error sending request"`.
5. `doorway/doorway-service/src/routes/upstream_health.rs:86-119`: 3 failures within
   20 s open the circuit. Each failed half-open trial (one per 30 s cooldown)
   re-opens it and adds to `errorStreak`. That is the steady climb in the samples.
   The breaker is doing its job.

The habit ledger describes the 2026-10-22 state as "CrashLoopBackOff while storage
and conductor are Ready". That holds for the containers. For the **pod** it is
wrong, and the pod is what the Service routes to. Each time the worker crashes,
elohim.host loses adam.

## Fix path

The fix belongs in the worker's render: a crashing compute worker must not take
storage out of service. The options, best first:

- **A. Supervise the worker inside its container.** Change line 289 from
  `["bash", "/opt/compute/worker-start.sh"]` to a small supervisor. It runs
  `worker-start.sh` in its own process group (`setsid`), records the exit in
  `supervisor-status.txt` on the compute PVC, kills the process group so its children cannot survive
  (a container restart gave that for free), waits with exponential backoff
  (capped near 300 s), and loops. The container then stays Running, the pod stays
  Ready, and the Service keeps adam's endpoint. Worker health stays visible through
  its own channel: `genesis/scripts/ci/runner-capture-activation.mjs` invokes
  `scripts/ci/capture-compute-worker-grant.sh`, which verifies the live worker
  process, public grant receipt and enforced capacity before writing `status.txt`
  as CI evidence. Its former PID 1 check must follow the supervised child; a
  Running supervisor or retained receipt alone cannot prove activation.
  Cost: `kubectl rollout status` stops failing on a broken worker,
  which is correct. A broken worker is a worker problem, not a storage rollout
  problem.
- **B. Put a readiness gate on the storage side.** This does not work. Kubernetes
  has no per-container Service readiness. Native sidecars (`initContainers` with
  `restartPolicy: Always`) also count toward pod Ready, and
  `publishNotReadyAddresses` would send traffic to storage that is genuinely not
  ready.
- **C. Run the worker in its own pod.** Clean separation, but it needs storage's
  compute API reachable off-loopback and the token moved to a Secret. That redesigns
  the virtual-peer contract
  (`2026-08-29-compute-envelope-virtual-peer-contract.md`), so it is a sprint, not a
  patch.

Whichever option lands needs a regression check in the render test: a rendered
compute-worker must never make an `elohim-node` pod's readiness depend on the
worker's own process lifetime.

## Current decision

**2026-10-29:** `ccce437a2ecf` fired again (poll 270), but this recurrence is NOT this concern:
edge #1757's rollout evidence reads `compute-worker=true/;elohim-node=false/CrashLoopBackOff`.
Adam's storage itself refuses to start on a full `storage-data` volume (`Disk quota exceeded
(os error 122)` since 2026-10-28T18:41Z). That recurrence is canonicalized in
`self-heal-storage-disk-quota-crashloop.md`; the ledger line cites it.

**2026-10-27:** The blocking campaign branch `feat/jenkins-native-bdd-land` is no
longer on origin; the render has been untouched since 2026-10-23 (`9dcd88757`).
Option A is implemented on `fix/compute-worker-supervise`, awaiting the steward's
landing decision. Landing rolls Adam's pod. The inline supervisor needs no image
rebuild; activation and the downstream authority probe verify the actual worker
child, preserving refusal during bootstrap, failure and retry.

The 2026-10-22 decision deferred this change to the `measure-runs-on-a-peer`
campaign while it was actively changing the render and activation evidence
(e1967b598, 30e8c2197, e73b67092). That branch conflict is gone; deployment proof
is still outstanding.

Until landing, every time the worker crash-loops on shem, this condition comes back
(`circuit:…elohim-adam-alpha…:8090` Open on `alpha-b`, and `GET https://elohim.host/`
answers 503). A recurrence is this concern, not a new one. Extend `fingerprints:`
here.

Out of scope here: `p2p.caughtUp` staying false on elohim.host (edge #1687,
`divergentAnchor` 152). That is the projection catch-up concern in
`self-heal-adam-projection-catchup-exhaustion-full-arc.md`.

## Verification

Re-fetched 2026-10-22T18:10–18:14Z. **The condition had cleared on its own** (the
worker is running, or the pod was re-created by the 17:38–17:50 deploy, and the
elohim.host doorway restarted ~17:50, `uptime 1224`):

```
GET https://elohim.host/admin/self-healing
  upstreams: elohim-adam-alpha:8090   circuit closed, errorStreak 0, recentFailures 0
             elohim-matthew-alpha:8090 circuit closed, errorStreak 0
  admission: maxInflight 256, available 256, shedTotal 0
GET https://elohim.host/p2p/status -> peerId 12D3KooWRsME… (adam, serving again)
GET https://elohim.host/                           200 1.34s
GET https://elohim.host/db/content/elohim-host-landing 200 0.04s
```

The ledger line will close when the poller stops seeing it. Nothing has fixed the
underlying coupling. Re-check after the next worker change: with the worker
deliberately failing, `elohim-adam-alpha-0` must stay `Ready`, and the elohim.host
upstream to adam must stay `closed`.

Deployment regression still to measure (proposed station in
`genesis/a2o/features/compute/measure-on-a-peer.feature`):

```gherkin
@wip @regression @requires:shem
Scenario: Adam keeps serving content when its compute worker fails
  Given Adam's storage is healthy and serves a known epr-head
  When the compute worker exits with code 1
  Then Adam's pod stays Ready and the same epr-head still returns HTTP 200
  And compute activation remains REFUSED until a live worker completes bootstrap
  # Retry delay: 10 seconds, doubling to 300; process-group TERM grace: 2 seconds.
```

The local render tests exercise the emitted command with a failing worker and
TERM-resistant children, checking retry, lock release, exit evidence and prompt
SIGTERM/SIGINT cleanup. They do not establish fleet readiness. Private-mesh
services deliberately started in separate sessions by `hc-mesh.sh` are outside
the worker's process group; their resume/cleanup behavior remains a live
deployment check.
