---
id: "backlog-conductor-websocket-flap-breaks-deploy-write-path"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Deploy WRITE path (PATCH /db/content + blob forward) 503s on ALL hosts when the doorway↔conductor app-interface websocket is flapping — reads serve from diesel cache, so a green /health + 200 reads mask a dead write path"
slug: "conductor-websocket-flap-breaks-deploy-write-path"
written: "2026-06-29"
author: "pipeline-shakeout shift"
status: "open"
priority: "high"
ci_status: backlog
jobs: [elohim, elohim-edge]
tags: [deploy, stage-spa-blob, conductor, websocket, doorway, notarize, write-path, read-cache-masks-write, elohim-host, alpha, projection-reconcile]
cites:
  - scripts/ci/stage-spa-blob.sh
  - Jenkinsfile
  - genesis/data/timeline/backlog/deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health.md
  - .claude/data/conductor-leak-rca-diverse-eyes-synthesis-2026-06-18.md
---

# Deploy write-path 503 = doorway↔conductor websocket flap (reads from cache mask it)

## Observed (2026-06-29, app #1576 Upload SPA Blob, after the shem CPU relief 97182d06b)

elohim.host/ stayed **404 "App not found"** after FOUR app deploys (#1573–#1576). The shem
CPU relief (capping 10 remote fixtures 2000m→1000m) genuinely fixed adam's CPU-starvation
data-path flap (adam reads now 8/8 200, restart count reset on a clean restart) — but
elohim.host did NOT recover. ci-investigator + live Loki proved a SECOND, distinct layer.

## Root cause (high confidence — per-leg deploy log + live doorway Loki)

The deploy stages each SPA bundle in two steps per leg: (1) `PUT /admin/seed/blob` (blob
upload), (2) `PATCH /db/content/{slug}` (sets blobHash → the SPA mount). #1576, all 6 legs:

| host | PUT /admin/seed/blob | PATCH /db/content |
|---|---|---|
| alpha.elohim.host (matthew) | ✓ all 3 attempts | **503 all 3** |
| elohim.host (adam) | 503 (1 PUT got `forwarded_to_storage:false`, then 503) | **503** |

Uniform **HTTP 503**, zero intermittency, all retries. NOT auth (PUT proves the admin key is
accepted), NOT timeout/000, NOT a stale/invalid blobHash (the served `1c345187…` is valid —
matthew serves the app from it). The **content-row PATCH never landed on EITHER host** for
several deploys; matthew only serves because of an OLD already-staged blob.

Live doorway Loki (`{namespace="elohim-alpha", app="doorway"}`) at the failure window shows
the mechanism — a tight reconnect loop:

```
Connecting to conductor at ws://elohim-matthew-alpha-0...:8445
Connected to conductor
Conductor closed connection: None
Reconnecting to conductor in 100ms...        (repeating)
```
and `elohim_storage::p2p::projection_reconcile: conductor get failed; retry next sweep`
(`Zome call failed: Websocket error: Timeout`).

**So:** GET `/db/stats` and GET `/db/content` return 200 because reads are served from the
diesel/sqlite projection cache. A WRITE (`PATCH /db/content` = a notarize zome call; and the
blob forward to storage) needs a LIVE, stable conductor app-interface — which the conductor
keeps **closing** (`Conductor closed connection: None`). Every write 503s; every read 200s.
The shallow `/health` (admission-exempt, doorway-up) and even the read path are all green
while the write path is dead — the deploy's "is this host healthy?" signal is blind to it.

## Durable, not transient (disambiguated 2026-06-29 22:57)

Initially this looked churn-induced: edge #1130 (CPU-relief rollout) was immediately followed
by the redundant edge #1131 (a second full-mesh rollout — over-dispatch, see below). But the
disambiguation came back **DURABLE (Hypothesis B)**:

- Edge #1131 finished SUCCESS at ~22:07; edge #1132 had not yet reached its *rollout* (still
  in build phase) at 22:57 — so there was a ~50-minute window with **no active mesh rollout**.
- Across that idle window the shared doorway (`elohim-doorway-alpha-678c657688-pw85p`) logged
  `Conductor closed connection: None` on a **precise ~10-second cadence** (22:55:48, :58,
  22:56:08, :18, :28… 3 closes per cycle), with **zero successful `Connected to conductor`**
  in the window. A clockwork 10s close-only cycle ~50 min after the last rollout is a
  connection-lifecycle pathology (idle-timeout / keepalive / signal-subscription closing the
  app-interface), **not** restart settling.

So a deploy retry will NOT fix it. **⚠ The candidate roots guessed in this section (conductor
leak / `2224edbf8` late-connect bridge) were BOTH WRONG — see the corrected RCA below.**

## CORRECTED RCA (2026-06-30 — two adversarial RCA workflows + live Loki, self-corrected once)

Two further investigations (17 agents, live-Loki-grounded, adversarially verified) overturned
the single-cause framing TWICE (the "None-token else branch / mint-before-connect" guess was
also refuted by live logs showing the doorway DOES mint+authenticate) and converged on **two
genuinely independent defects**:

### Defect B — THE write blocker (operator/infra-owned): per-cell DHT-arc incoherence on the `ethosengine` conductors
matthew's conductor cannot resolve DHT `get_links` for the content zome, so every
content-notarizing zome call times out → 503:
- `real_ribosome.rs:652` / `get_links.rs:76` `Host("Other: get_links response channel dropped:
  likely response timeout")`, `zome=content_store fn=get_rea_commitment`, ~60s metronome — on
  **matthew, james, jessica** (all `node_name=ethosengine`); the other 6 conductors are CLEAN.
- `kitsune2_gossip::timeout` rounds carry `our_arc_set: ArcSet { inner: {} }` for SOME agents
  while OTHER agents on the **same conductor / same `wss://signal.elohim.host:443` relay** carry
  a full ~512-index arc. An empty-arc cell has no local holder for `get_links` → timeout.
- **NOT a relay/WAN fault** (refuted by the alternative-skeptic: the relay carries live
  sessions with zero transport errors; the node reaches `peers_asked:12-13`; conductor
  validation flows cleanly `0 awaiting deps`). It is **per-cell/per-agent arc-coherence +
  gossip-round non-completion** at the kitsune2 layer (candidate: a tx5/WebRTC post-signal
  data-path issue, or arc/bootstrap config — NOT signal reachability). The operator's "other
  conductors flapping" instinct was right in *kind*; the corroborated set is the ethosengine three.
- Write-path 503 chain (code-verified end-to-end): PATCH `/db/content` → `patch_needs_conductor`
  → storage's OWN official in-process bridge `hc_registry.lamad_client()` (`http.rs:4955-4972`)
  → `content_store` zome call (`hc_client.rs:235-259`) → `Websocket Timeout` →
  `StorageError::Conductor` → 503 (`response.rs:156`). Reads stay 200 (projection cache, not
  DHT-gated). **No client/storage/doorway code change can manufacture DHT holders** — recovery
  is gated on the ethosengine conductors re-claiming non-empty arcs / completing gossip rounds.

### Defect A — doorway worker-pool optimistic-auth storm (code-fixable, but does NOT block writes)
The "~6-10s close / awaiting authentication" storm is the **doorway worker pool**
(`worker/conductor.rs`) — a SEPARATE socket from the write path. `send_authenticate`
(`:378-412`) hand-rolls the auth via rmpv, then `sleep(50ms)` + returns `Ok` **without reading
any ack**; `debug!("Authenticated with conductor")` (`:249`) is thus OPTIMISTIC (a lie). The
conductor drops the socket at its ~10s auth-timeout (`websocket.rs:294`); `run_session` treats
the sub-10s session as unstable and re-mints (`:345-347`) → self-sustaining metronome. Proven
non-blocking + not-overload by a clean live contrast on the SAME conductor: the doorway's
OFFICIAL `ZomeCaller` (holochain_client) logs `zome call succeeded` every ~60s. Fix tracked in
its own backlog [[doorway-worker-pool-optimistic-auth-storm]] — hygiene only, NOT an
elohim.host fix.

## Concurrent-dispatcher note (2026-06-29)

The mesh did not quiesce because a **second active session** pushed a 4-commit dataplane/a2o
batch to dev at 22:25:44 (`e76057a06`…`eb809c7ff`), spawning orchestrator #1347 → edge #1132 →
another rollout. Two dispatchers on dev simultaneously is the documented concurrent-push
mutual-abort hazard; this shift stood down rather than fight it. Operator: coordinate to a
single dispatcher before the conductor-stabilization + final elohim.host deploy.

## The repo-fixable lessons (durable)

1. **A deploy host-readiness gate must probe the WRITE path, not just `/health` or a read.**
   `/health` 200 + GET `/db/content` 200 both pass while every write 503s. The deploy (and the
   operator-facing health) should gate on a cheap conductor-backed *write* readiness signal
   (e.g. a conductor-connected gauge, or the doorway M1/M5 reconnect counter being quiet), or a
   no-op authenticated write, before declaring a host deploy-ready. Extends
   [[deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health]] (which named the shallow
   /health gap; this adds: reads ALSO mask it — only a write probes the conductor).
2. **`Upload SPA Blob` catchError→UNSTABLE-swallow** still hides an all-legs-STALE deploy as a
   merely-UNSTABLE board state while elohim.host silently 404s. A persistent all-legs failure
   should fail loudly (named alerting junit), never silently leave every host STALE.
3. **Over-dispatch made it worse:** pushing `[build:app]` before the prior edge commit
   (97182d06b) had baselined kept that commit in the changeset window, so the app push
   re-triggered edge (#1131) → a second mesh churn that prolonged the conductor instability.
   After an edge deploy, let the orchestrator baseline it before the next push (or the next
   push re-churns the mesh).

## Operator-owned (live cluster, NOT repo) — PRECISE LEVER (corrected 2026-06-30)

elohim.host write recovery is gated on **Defect B** — the `ethosengine` conductors
(matthew/james/jessica) re-claiming non-empty DHT arcs and completing gossip rounds. This is
NOT a relay/WAN reachability problem (the relay carries live sessions; the node reaches
12-13 peers). Investigate the **kitsune2 arc-declaration / gossip-round-completion** layer:
why specific cells declare `our_arc_set: ArcSet { inner: {} }`, and why gossip rounds time out
even for full-arc agents — candidate is the tx5/WebRTC post-signal data path (the signal
handshake succeeds but the round response never arrives), or arc/bootstrap config on these
three co-located conductors.

**Acceptance gate (Loki, no kubectl):** `get_links response channel dropped … get_rea_commitment`
and `projection-reconcile: conductor get failed … Websocket error: Timeout` drop to ~0 on
matthew/james/jessica; `kitsune2_gossip::timeout … our_arc_set: ArcSet { inner: {} }` stops
(arc becomes non-empty). THEN a `PATCH /db/content` returns 200 and propagates to a peer — and
only then does a single clean `[build:app]` flip elohim.host 404→200. Repo side is correct
(blob builds + uploads fine; the repo-fixable lessons above — write-readiness deploy gate +
the UNSTABLE-swallow + Defect A doorway hygiene — are real but none of them recover writes).

## Delta 2026-09-02 04:4xZ (shift land-rung5-batch, evidence-only — ceiling: no kubectl from here)

Both doorways report `conductor.connected: false, connected_workers: 0/4` on `/health` (live read
04:44Z: doorway-alpha AND elohim.host). elohim-genesis #1540–#1544 all fail at `Verify Target
Health` (`verify-doorway-readiness.sh` requires `conductor.connected=true`; the doorway now
reports honestly per health.rs). Storage pods are healthy (release-adoption series 7/7 after
edge #1411's storage-only roll; conductors were NOT rolled). So the doorway↔conductor app
interface is down fleet-wide while storage↔conductor is up — the write path (seed, canonical-head
declare via doorway) is closed; the p2p dataplane and the election plane are not. Operator
action: inspect the doorway pods' conductor worker reconnect loop (app-port auth timeout class,
`app-port-4445-auth-timeout-fleet-wide`) — a doorway pod restart or the token re-mint is the
likely cure; the readiness script's dead `STORAGE_HEALTHY` variable is a separate cosmetic defect.
