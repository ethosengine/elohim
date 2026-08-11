---
id: "backlog-adam-pull-loop-wedged-at-boot"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam's projector pull loop is wedged at boot (total=0, fetched=0, caughtUp=false for hours) — doorway B answers catching-up on every head-record/EPR read"
slug: "adam-pull-loop-wedged-at-boot"
written: "2026-08-10"
author: "batch-3 ghost-declaration diagnosis session"
status: "backlog"
priority: "high"
tags: [dataplane, projector, pull-loop, catching-up-shed, adam, doorway-b, self-heal-exhaustion, saga]
cites:
  - genesis/data/timeline/backlog/2026-07-10-server-side-epr-read-path-catching-up-shed.md
  - genesis/data/timeline/backlog/adopt-before-author-evidence-starvation.md
---

# adam's pull loop never completes its first pass — B-plane reads shed forever

## Evidence (2026-08-10, live)

- `https://elohim.host/p2p/status` → `pull: {total: 0, fetched: 0, pending: 0,
  failed: 0, caughtUp: false}` — hours after the last restart, the puller has
  not completed (or started) a first pass. Not backlog lag: **zero** items
  have ever entered the window.
- Every `GET /db/content/{id}/head-record` on doorway B answers
  `{"status":"catching-up","retryAfter":30}` — the catching-up shed is
  permanent, so adam cannot serve head records over HTTP, and B-caughtUp read
  false for the whole of validate-only run #1339's 45-minute gate window
  (telemetry-only for the gate, but a standing self-heal exhaustion).
- Contrast matthew: `pull: {total: 2, fetched: 2, caughtUp: true}`.

## Why it matters

adam is the genesis-pair supplier peer with the largest anchored corpus; a
permanently-shedding B plane removes one of two doorway testimonies (the
guide-star convergence bar is two doorways testifying the same footprint) and
hides adam's conductor behind a shed for every HTTP-path read. This is a
distinct defect from the ghost-declaration deadlock (cured on branch
feat/angular22-node24, 2026-08-10) — the p2p-plane view-federation responder
still answers, so record supply is not gated on this, but the HTTP surface and
the B-side caughtUp telemetry are.

## Causal correction (operator Codex probe + live check, 2026-08-10 evening)

- **`pull.caughtUp=false` does NOT cause the doorway shedding.** Source
  tracing (read-only Codex probe): the doorway's catching-up shed reads
  `projectionReconcile.caughtUp`, not `pull.caughtUp`. The two symptoms may
  share startup pressure, but that relationship is unproven — do not fold
  them into one defect.
- **The startup-hydration suspect is FALSIFIED live.** Hypothesis was that
  hydration (full content rows + a per-row tag query where only IDs are
  needed) blocks creation of the acquisition ticker. Loki, adam, every boot
  in the last 24h: "P2P node started" → "Loaded local content IDs for
  replication state" (count≈4454–4462) lands **~1–2 seconds** later
  (05:01:34→05:01:36, 09:50:52→09:50:54, 12:09:00→12:09:01). Hydration
  completes fast; the ticker is not materially delayed. (The
  IDs-only-but-loads-full-rows inefficiency is still real as a cleanup, just
  not this wedge's cause.)
- **Also observed:** adam restarted 4× in ~10h (boots 02:09, 05:01, 09:50,
  12:09) — restart cadence itself deserves an explanation before any
  single-boot theory.

## Next probes

1. Why does the pull loop report total=0 — never scheduled (init-order gate
   waiting on a bridge/condition that never fires on adam), or first query
   hung on the conductor with no timeout? Find the pull loop's boot path in
   `elohim/elohim-storage` and its gating condition; check adam's boot logs
   for the loop's first log line vs its absence. (Hydration is NOT the
   blocker — see the causal correction above.)
2. If the first pull query hangs on the conductor: does it use a bounded call
   (cf. the head-record responder's 5s budget) or an unbounded
   `HcClient::call_zome` (~60s ws timeout, retried forever)?
3. What flips `projectionReconcile.caughtUp` on adam, and why it stays false —
   that (not `pull`) is the doorway-shed input.
4. Why adam restarts every few hours (crashloop? OOM? deploys?) — read the
   pod's restart reason before theorizing about any single boot.

## Scoped follow-ups (separately claimable)

- **Status honesty (`pull: null`)**: publish the already-schema-valid
  `pull: null` until the first reconcile completes, instead of presenting
  default zeroes as if work had begun and finished empty (C4: "never started"
  vs "started and behind" are currently collapsed).
- **Direct `/head-record` HTTP budget**: the HTTP route lacks the p2p
  responder's response budget and can exceed doorway's 12s request limit.
  Note: merely wrapping `HcClient::call_zome` in a timeout is NOT a complete
  fix — it does not cancel conductor work (same reason
  `HealPacing::batch_extern_budget` stays strictly below `attempt_timeout`).

## Read-only adjudication (Codex Task 3, 2026-08-10)

### Verdict

The title's **pull-loop-wedged** diagnosis is not supported by the source or by
the current live status. `pull:{total:0,...,caughtUp:false}` is an ambiguous
default/empty-set representation, not evidence that the ticker never ran. The
remaining live defect is the projection-reconcile backlog and the doorway
upstream breaker it helps keep unhealthy; those are distinct from acquisition.

Two additional causal corrections are decisive:

1. The acquisition first query is local SQLite, not a conductor zome call. No
   unbounded `HcClient::call_zome` exists on this path.
2. `projectionReconcile.caughtUp` is health telemetry, not a doorway shed
   decision input. The generic proxy sheds when its per-upstream circuit is open,
   or when storage actually answers 429/503. A false projector bit can coexist
   with and help explain upstream pressure, but the proxy never branches on it.

### 1. Pull boot path and the meaning of zero

- `P2PNode::run` refreshes status, hydrates local replication ids, constructs a
  60-second acquisition ticker whose first tick is immediate, then enters the
  main swarm `select!` (`p2p/mod.rs:2915-2939`).
- The ticker arm calls `run_acquisition_reconcile` whenever `sync_paused` is
  false (`p2p/mod.rs:3073-3078`). Reconcile loads active pins and diffs their
  `head_ref`s against the local lamad projection (`p2p/mod.rs:7939-8000`). It
  performs no network or conductor call.
- Reconcile and dispatch are silently skipped while `sync_paused=true`. DB-pool
  acquisition also returns silently, and pin/presence query failures log only at
  debug. Therefore log absence cannot currently prove which path occurred.
- More importantly, every status refresh publishes
  `pull: Some(acquisition.rollup())` even before a reconcile has established an
  observed desired set (`p2p/mod.rs:8411-8415`). `rollup()` deliberately computes
  `caught_up = total > 0 && fetched == total`; zero trackers, zero active pins,
  a never-run loop, and an early-returning loop all serialize identically as
  `{total:0,fetched:0,pending:0,failed:0,caughtUp:false}`
  (`p2p/acquisition.rs:268-286`). The existing
  `resolved_empty_desired_set_is_not_caught_up` test confirms this is intended
  empty-set behavior, not a progress signal.

Live at approximately 19:13 UTC, Adam reported six connected peers,
`syncPaused:false`, drain `4462/4462`, and the same zero pull rollup. The same
snapshot reported **82 projection-reconcile sweeps**, proving the P2P runtime is
polling; it does not prove an active acquisition pin exists. `GET /api/v1/pins`
could not close that last ambiguity because doorway's upstream circuit shed the
request before a trustworthy pin list was returned.

**Finding:** the actionable acquisition defect is status honesty/observability,
not a demonstrated boot wedge. Publish `pull:null` (or an explicit
`initialized/lastAttempt/lastSuccess/activePins` block) until the first local
reconcile completes, and count/log early returns. Only then can `total=0`
distinguish “observed empty” from “not measured.”

### 2. Fetch timeout question

The hypothetical conductor hang is falsified for acquisition:

- `run_acquisition_reconcile` is a local DB operation.
- Missing bytes are dispatched as libp2p `ShardRequest::GetContent`, not through
  `HcClient` (`p2p/mod.rs:8291-8338`).
- The shard request-response behavior has a configured request timeout; the
  default is 30 seconds (`p2p/mod.rs:541`, `p2p/behaviour.rs:372-375`).
- `OutboundFailure` removes the request from
  `pending_acquisition_fetches`, records a `transport_failure`, and marks the
  content failed (`p2p/mod.rs:5212-5227`). It cannot remain pending forever on an
  unbounded conductor call.

### 3. What actually flips `projectionReconcile.caughtUp`

The projection stream is separately spawned in `main.rs`. After a 30-second
boot settle, discovery runs every configured tick without the conductor. Once
the lamad bridge is present, a single-flight `run_heal` consumes that discovery
plan. Only the completed heal publishes the status snapshot.

`ProjectionReconcileState::publish_sweep` assigns `caught_up` from the folded
`GapCounts` (`projection_reconcile.rs:1163-1177`). The fold ANDs the REA,
content, and collectives arms, so every arm must drain its pending work and call
its end-of-arm `update_caught_up` before the published bit can become true
(`projection_reconcile.rs:1320-1367`). This bit means “the sweep ended,” not
convergence; failed/actionable divergence and the measured precondition govern
the stricter `converged` bit.

Adam's live snapshot explains the false bit without invoking the pull loop:
`pending:3089`, `completed:0`, `failed:3`, `peersAsked:6`,
`divergentAnchor:2932`, `sweeps:82`. At least one folded arm is still carrying
thousands of pending gaps, so `caughtUp:false` is the expected result of the
published formula. The source does not support a causal edge from
`pull.caughtUp` into this state.

### 4. What actually causes the HTTP catching-up response

Doorway polls storage's `projectionReconcile` block every 30 seconds and copies
`caughtUp`, `converged`, and `divergentAnchor` into its health snapshot
(`doorway-service/src/main.rs:581-615`; the alternate server composition has the
same code at `server/http.rs:1807-1846`). No serving gate reads those fields.

The generic storage proxy instead consults `UpstreamBreakers` immediately before
the send. An open circuit returns the catching-up response **without calling
storage**; an upstream 429/503 is also translated into catching-up and recorded
as a breaker failure (`routes/storage_proxy.rs:245-323`). The live
`GET /admin/self-healing` snapshot at approximately 19:14 UTC showed the Adam
storage upstream `half-open`, `errorStreak:24`, `lastGood:null`, while projector
telemetry separately showed `caughtUp:false`. That is the direct explanation for
`GET /api/v1/pins` returning catching-up at the same moment.

**Finding:** diagnose the breaker/backpressure cause from the upstream snapshot
and storage admission logs. Do not use `projectionReconcile.caughtUp=false` as
proof that the proxy intentionally gated a read on projector progress.

### 5. Restart cadence

The four observed boots align one-for-one with successful `elohim-edge/dev`
deployments, not with an unexplained crash cadence:

| Adam boot (UTC) | Edge build window | Jenkins rollout evidence |
|---|---|---|
| ~02:09 | #1334, started 01:14 | Adam StatefulSet rollout completed at revision `5f477b8c55` |
| ~05:01 | #1335, started 04:11 | completed at revision `6cb595cf8b` |
| ~09:50 | #1337, started 09:01 | completed at revision `6cdd95c4bf` |
| ~12:09 | #1338, started 11:19 | completed at revision `565bdb94d5` |

Each boot falls inside its build's deployment window, and each build log names a
distinct completed Adam rolling revision. This is strong affirmative evidence
for deploy-driven restarts. No OOM/crashloop explanation is needed for these
four events; a Kubernetes last-termination read would only be required if an
additional boot appears outside an edge/holochain rollout window.

### Remaining uncertainty

The read-only external surfaces still do not reveal Adam's active pin count
while its upstream circuit is open. Thus the narrow unresolved question is
whether the observed-empty pull set reflects zero active pins or a local DB
early return. It is **not** whether a conductor call hung. Instrumenting the
first-pass state and early-return counters closes that uncertainty directly.

## Scoped follow-ups landed locally (Codex Task 3, 2026-08-10)

The two Adam pull-loop follow-ups are implemented on the existing Category C
acquisition projection:

- `/p2p/status.pull` remains `null` until the first local active-pin × presence
  reconcile completes. A completed pass with no active pins publishes the
  distinct observed value `{total:0,fetched:0,pending:0,failed:0,caughtUp:false}`.
- Every scheduled reconcile tick now records exactly one typed outcome:
  `completed`, `sync_paused`, `db_pool_missing`, `db_pool_unavailable`,
  `pin_load_failed`, or `presence_query_failed`. The first completed pass logs
  its active-pin, desired-item, and local-item census and publishes
  `elohim_acquisition_reconcile_initialized` plus
  `elohim_acquisition_active_pins`. Early returns warn with branch context.

Focused proof is green:

- `metrics::tests::acquisition_reconcile_outcomes_are_stable_pretouched_and_incrementable`
- `p2p::acquisition::tests::pull_is_null_until_a_reconcile_observes_the_desired_set`
- `schema_contract::p2p_status_view_with_null_drain_and_uninitialized_pull`
- `cargo test export_bindings` — 77 export tests passed; the generated
  `P2PStatusInfo.ts` carries the same null-vs-observed-zero contract.

The first live scrape after deployment can now settle the remaining Adam
question without inference: `initialized=1, active_pins=0` is a genuinely empty
desired set; `initialized=0` plus the reconcile outcome series identifies the
exact pre-census stop.
