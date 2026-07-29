---
id: "backlog-jessica-identity-fill-loop-silent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "jessica emits no identity_fill WARN loop while all other pods poll at ~5min cadence — loop dead or mis-scheduled?"
slug: "jessica-identity-fill-loop-silent"
written: "2026-07-28"
author: "heads-converge-truthful-resilience shift"
status: "resolved"
resolved: "2026-07-29"
priority: "low"
tags: [identity-fill, imagodei, alpha, observability]
cites: []
---

# jessica identity_fill loop silent — anomaly, unexplained

Observed 2026-07-28 (2h Loki window, quoted-evidence sweep): every alpha pod
except jessica emits the periodic WARN
`identity_fill: discovery found zero household cids (no memberships on DHT or
projection) — nothing to fill` at ~5min cadence (adam≈23, eve≈21, gertrude≈20,
james≈24, matthew≈24, susan≈17 in 2h). jessica: 0 WARN hits (one unrelated
INFO substring match only).

Either jessica's identity_fill loop is not running (scheduler/boot wiring), or
it is succeeding silently (unlikely — there is nothing to find fleet-wide), or
its log target/level differs. Untriaged; needs jessica's raw log tail scoped to
`elohim_storage::services::identity_fill` to confirm the loop is scheduled.

Status: OPEN (investigate). Small; good first probe for any runtime-triage
pass touching identity coherence.

## Diagnosis (2026-07-29)

Not dead, not mis-scheduled, not log-target drift: **hung**. `run_fill_loop`
(`elohim/elohim-storage/src/services/identity_fill.rs`) commits its
`tokio::select!` to the `ticker.tick() => run_once(...).await` arm. Nothing
in the `run_once` chain carries a timeout — not the client-side zome call
(`hc_client.rs:355-364`, `app_ws.call_zome(...).await` unwrapped) nor the
qahal coordinator's sequential per-link `get()`s on the zome side. If any DHT
get stalls, `run_once` never returns, so the `select!` never revisits its
arms: the ticker cannot fire again AND the shutdown branch is never observed.
Silent, permanent hang until pod restart.

jessica-alpha is the only pod whose discovery union is non-empty (226 own-chain
memberships — the other alpha pods correctly no-op on an empty union, which is
why they instead loop the "found zero" WARN every ~5min). jessica's logs
showed `identity_fill task started` exactly at pod-restart timestamps
(19:21:51Z, 21:17:02Z) with **no discovery-result line ever after** either
restart — the signature of a `run_once` that entered the DHT-read chain and
never returned, not a loop that failed to schedule.

## Fix (this commit)

`elohim/elohim-storage/src/services/identity_fill.rs`: added
`run_once_bounded`, which wraps `run_once(&pool, source)` in
`tokio::time::timeout(RUN_ONCE_BUDGET, ...)` with `RUN_ONCE_BUDGET =
Duration::from_secs(120)` — well under the 300s tick interval
(`DEFAULT_FILL_SECS`) and generous for legitimate work. `run_fill_loop` now
calls `run_once_bounded` instead of `run_once`; on elapse the `run_once`
future is dropped (cancelling any in-flight, possibly-hung conductor call)
and a `StorageError::Timeout` is returned, logged distinctly (`identity_fill:
run_once exceeded budget — abandoned this tick, retrying next tick`) from an
ordinary fetch error so operators can tell the two apart at a glance.
`MissedTickBehavior::Skip` (already in place) handles the retry cadence once
the loop is unstuck. Tests: `run_once_bounded_times_out_a_source_that_never_resolves`
and `loop_survives_a_timed_out_tick_and_ticks_again` (both
`#[tokio::test(start_paused = true)]`, deterministic via virtual time — no
real 120s wait) prove a `fetch_pairs` that never resolves is cut off by the
budget and that the ticker fires again afterward, closing the exact defect
this backlog item observed.

## Residual (not closed by this fix)

The zome-side per-link sequential reads that made a stall *slow* in the
first place were separately batched in `6a3507ae0` (landed before this fix) —
that reduces how often a stall-length read happens, but does not explain
*why* jessica's conductor specifically stalls (vs. the other five alpha pods).
Once jessica is unwedged by this fix and logs the new timeout WARN, that WARN
firing is itself the confirmation signal that a real stall is occurring on
her conductor (not a code path bug) — worth a fresh runtime-triage pass at
that point, scoped to jessica's conductor-side DHT get behavior specifically.
