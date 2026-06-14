---
id: "backlog-self-heal-doorway-startup-conductor-mint-serialization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-alpha residual crash-loop on intel-nuc: per-conductor pool startup serializes N synchronous token mints before the HTTP listener binds, blowing the startupProbe budget on a slow/flaky-DNS node (NOT the warm_stream firehose, which is cured)"
slug: "self-heal-doorway-startup-conductor-mint-serialization"
written: "2026-06-14"
author: "runtime-triage"
status: "wip"
priority: "high"
self_heal_status: in-progress
severity: high
fingerprints: []
nodes: [doorway-alpha, intel-nuc]
relatedNodeIds: []
tags: [self-heal, render-degenerate, doorway, startup, conductor, liveness, crash-loop, intel-nuc, admission-shed]
cites:
  - doorway/doorway-service/src/main.rs
  - doorway/doorway-service/src/worker/conductor.rs
  - doorway/doorway-service/src/worker/pool.rs
  - doorway/doorway-service/src/server/http.rs
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - elohim/holochain/Jenkinsfile
  - doorway-freeze-incident-2026-06-13/WARM-STREAM-RESIDUAL-DIAGNOSIS.md
  - SPRINTER-HANDOFF-2026-06-14.md
  - genesis/data/timeline/backlog/ci-genesis-doorway-503-seed-phase-wedge.md
---

# doorway-alpha residual crash-loop — startup conductor-mint serialization (intel-nuc)

## What is exhausted

The self-healing here is kubelet's restart-on-hang: doorway-alpha's pod
`elohim-doorway-alpha-55c65b664c-8qncr` on node **intel-nuc** (ns
`elohim-alpha`) crash-looped repeatedly (8 restarts to ~17:10 UTC; ~13
Killing/BackOff events in the preceding 3h). This is the RECURRENCE of the
sprinter handoff's VERIFY #2 — the pod crashes **even with the warm_stream
firehose cure (`4dc862748`/`54d2bb737`) confirmed working**. doorway-alpha-b on
shem (adam backend) stayed 0 restarts on the identical image, so the cause is
node/startup-specific, not the cold-start projection path.

Kill-log evidence (Loki, doorway container's own stderr via
`terminationMessagePolicy: FallbackToLogsOnError`):

- Exit reason `Error` (NOT `OOMKilled`). Memory working set ~79 MB against a
  1Gi limit (7.7%) — **not OOM**. CFS throttling rate `0` across all samples —
  **not CPU-throttled** (the cpu 1→2 bump in `ece274734` is doing its job for
  the burst class). Panic/fatal/SIGSEGV scan over 15,804 lines: **0 matches** —
  **not a crash**.
- The decider — last lines before each kill:
  ```
  ERROR doorway::worker::conductor — "Failed to connect to conductor: Holochain error:
    WebSocket connect failed: IO error: failed to lookup address information:
    Name or service not known"
  WARN  doorway::worker::conductor — "Reconnecting to conductor in 12.8s..."
  ...(repeats)...
  WARN  doorway::projection::store — "ProjectionStore running in memory-only mode (no MongoDB)"
  Startup probe failed: Get "http://10.1.58.11:8080/health": dial tcp ...: connect: connection refused
  ```
  `connection refused` (not deadline-exceeded) ⟹ **the HTTP listener had not
  bound :8080 yet** when the startup probe fired. The conductor URL
  `ws://elohim-matthew-alpha:4445` was intermittently unresolvable on intel-nuc
  (NXDOMAIN), and the prewarm cure DID fire once Mongo connected
  (`Hot cache pre-warmed from MongoDB ... prewarmed=3661`) — confirming the
  firehose is gone and is NOT the residual.

## Root-cause inventory

The doorway startup runs the entire conductor-connection sequence **serially,
inside `async fn main`, BEFORE the HTTP listener binds**:

- `doorway/doorway-service/src/main.rs` — `server::run(state)` (the TCP bind) is
  the LAST thing called (`main.rs:1136`); everything above it is blocking
  startup.
- `doorway/doorway-service/src/server/http.rs:1088` — `TcpListener::bind` (the
  :8080 bind, i.e. the moment `/health` becomes answerable) lives inside
  `server::run`, gated behind all of `main`.
- The N-multiplier: the **per-conductor pool loop** (`main.rs:386-447`) iterates
  `CONDUCTOR_URLS`, which the pipeline builds from **every env human**
  (`elohim/holochain/Jenkinsfile` `computeConductorUrls`) — 14 humans declared
  in `deployments.json`. For EACH conductor it `await`ed
  `mint_app_auth_token(...)` synchronously (`main.rs:402-404`).
- `mint_app_auth_token` (`main.rs:1289`) retries **5×** with exponential backoff
  capped at 5s (0.5+1+2+4 ≈ up to ~12.5s of sleep, plus per-attempt
  `TypedAdminClient::connect`) when a conductor admin URL is unresolvable.
- Serialized across N conductors with flaky DNS that is up to **N × ~12.5s** of
  startup blocking before `:8080` binds — comfortably past the startupProbe
  budget (24 × 5s = 120s, `genesis/orchestrator/manifests/doorway/alpha.yaml`)
  → probe `connection refused` → SIGKILL → loop.
- This was **redundant**: `WorkerPool::new` is already non-blocking (spawns
  workers, returns immediately — `pool.rs:104-152`); the per-conductor pool is
  passed a `token_minter` (`main.rs:411-415`); and the background
  `connection_loop` mints on the first unstable (unauthenticated) app-interface
  session and re-mints on conductor restart (`conductor.rs:320-321`,
  `remint_if_due` at `conductor.rs:329-348`, mints whenever a minter exists). So
  auth heals in the background without the upfront serial mint.

Why alpha and not alpha-b: alpha-b is shem-pinned (adam backend) and did not hit
the intel-nuc conductor-DNS flakiness; both manifests run the same per-conductor
loop, so alpha-b is latently exposed to the same class if its node's conductor
DNS ever flaps during a rollout.

## Fix path

Drop the upfront synchronous `mint_app_auth_token` in the **per-conductor pool
loop** only (`main.rs:402-404`); pass `auth_token: None` and rely on the
already-configured `token_minter` + background `connection_loop` to mint on first
connect. This removes the N×(up to ~12.5s) serial cold-start blocking so the HTTP
listener binds :8080 promptly and `/health` answers within the startupProbe
budget regardless of conductor DNS state. The default pool (`state.pool`) serves
every request until each per-conductor pool authenticates; per-conductor routing
is an affinity optimization, never a request precondition. The app + admin pool
upfront mints (`main.rs:215`, `main.rs:254` path) are left intact — they are the
2 bounded, necessary default-fallback connections, not the N-multiplier.

Not done here (the actuation arm, deliberately out of scope for ELEVATE): a true
`/health` runtime isolation onto a dedicated tokio runtime
(`DOORWAY_HEALTH_PORT` exists but is intentionally unset so a partial wedge still
trips the kill) and the warm_stream closed-loop backpressure (proven pacing;
do not re-touch). Those are the durable items in
`doorway-freeze-incident-2026-06-13/WARM-STREAM-RESIDUAL-DIAGNOSIS.md` §3.

## Current decision

FIX APPLIED (bounded, repo-surface, code-only): removed the per-conductor-loop
synchronous token mint in `doorway/doorway-service/src/main.rs`. Local
`RUSTFLAGS="" cargo build --release` + `clippy -D warnings` + `fmt --check` green
(see Verification). The integrator owns push; the operator's pipeline reconciles
the rebuilt image onto the live pod. No manifest change was required — the
startupProbe budget and the cpu 1→2 bump stay as-is (they are correct; the bug
was the listener bind being gated behind serial conductor mints, not the probe
being too tight).

## Verification

- Local gates on the changed crate (commit pending): build, clippy, fmt — see
  the closing commit message for exact results.
- Live closure (post-deploy, operator/poller-owned): the exhaustion is
  considered resolved when doorway-alpha on intel-nuc binds :8080 and serves
  `/health` 200 within the startup budget through a conductor-DNS flap, and the
  restart counter
  (`kube_pod_container_status_restarts_total{pod=~"elohim-doorway-alpha.*",container="doorway"}`)
  stays flat across a rollout. The kill signature to watch for regression:
  startup probe `connection refused` on :8080 co-occurring with
  `Failed to connect to conductor ... Name or service not known`. Until the
  rebuilt image is deployed, this entry stays `wip` / `in-progress`.
