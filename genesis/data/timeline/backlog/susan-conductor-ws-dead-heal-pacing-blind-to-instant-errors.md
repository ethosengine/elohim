---
id: "backlog-susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "susan's storage↔conductor WebSocket is dead (shem node) — and separately, HealCircuit/pacing never engages because instant connection-closed errors reset the streak instead of tripping it"
slug: "susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors"
written: "2026-07-31"
author: "claude (Prometheus + Loki + code RCA)"
status: "open"
priority: "high"
tags: [self-heal, projection-reconcile, heal-outcomes, websocket, conductor, shem, susan, heal-circuit, pacing, dataplane]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - genesis/data/timeline/backlog/shem-conductors-signal-hairpin-suspect-dht-silent.md
  - genesis/data/timeline/backlog/alpha-a-projector-chronic-catchup-flap.md
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# susan's storage↔conductor WS is gone, and the heal loop doesn't notice it's gone

Two distinct concerns, evidenced together (Prometheus + Loki + code RCA, 2026-07-31).
(a) is a live incident on a specific node; (b) is a bounded code gap the incident
exposed in the heal-pacing machinery that `self-heal-adam-projection-catchup-exhaustion-full-arc.md`
introduced.

## (a) susan's storage→own-conductor local WebSocket is down

- **Prometheus**, susan pod, `increase(elohim_projection_heal_outcomes_total[12h])`:
  `outcome="failed"` ≈ 64,658 (content) + 1,849 (rea). Only ~5k reach the `Ok(None)`
  branch (definitive-miss, retried next sweep) — compare to 60k-155k `Ok(None)` on
  other pods in the same window. Essentially nothing on susan completes a conductor
  round-trip; almost every attempt dies before the conductor answers at all.
- **Loki**, susan, 3h window: repeating
  `projection-reconcile[content]: conductor resolve failed; retry next sweep` with
  error `Conductor error: Zome call failed: Websocket error: Websocket closed: No
  connection`, `transient:false`, `routed_to_adopt:false`.
- This is storage's **own local WS to its own conductor** (the same-pod client
  connection `projection_reconcile.rs` uses for heal calls), not a cross-node
  reachability gap — scope it distinctly from the shem-hairpin lineage (cited): that
  backlog is about shem conductors going DHT-silent (kitsune2/gossip, cross-node,
  DNS-hairpin suspected); this is susan's storage sidecar losing its own conductor
  socket. Related family (both are shem-node substrate degradation, 2026-07-28→31),
  not the same mechanism — do not merge the two without re-confirming which layer
  each failure sits at.
- Not yet root-caused to a specific trigger (conductor restart, pod recycle, resource
  pressure) — that determination is substrate/operator-owned (conductor logs, pod
  events on susan are outside this repo's evidence). **Open — needs operator/substrate
  follow-up** to confirm whether this is transient (susan needs a restart/reconnect)
  or a recurring pattern worth a poller predicate (see `alpha-a-projector-chronic-catchup-flap.md`'s
  own open question about pool-scoped reachability).

## (b) is_transient_conductor_error + HealCircuit misclassify connection-refused/closed, so pacing never engages

Code, `elohim/elohim-storage/src/p2p/projection_reconcile.rs`:

- **`is_transient_conductor_error`** (~:295-307) matches only `"timeout"` /
  `"timed out"` / `StorageError::Timeout`. `"Websocket closed: No connection"` matches
  none of these substrings, so it classifies as **non-transient** →
  `should_retry_attempt` is `false` (no in-leg retry) and — because
  `timeout_should_route_to_adopt` (~:553, used at ~:2369-2371) requires
  `transient == true` — the row **never reaches the adopt-rescue arm** either. A row
  that failed because the socket to the conductor doesn't exist is treated the same
  as a decode/logic error: no retry, no adopt, straight to `mark_failed`
  (`HealOutcomeKind::Failed`), re-discovered next sweep with the same outcome.
- **`HealCircuit::record`** (struct ~:354-398) only counts a streak of
  `is_synthetic_attempt_timeout` outcomes (our own `tokio::time::timeout` firing) —
  any other `Err` variant hits the `Err(_) => { self.consecutive_timeouts = 0; }` arm
  (:385-387), which is documented as "the conductor ANSWERED (even if with an error):
  it is responsive, so the unresponsiveness streak breaks." A closed-socket error is
  the **opposite** of an answer — the conductor (or the local WS layer) never
  responded at all — but the circuit reads it as a responsive answer and resets to
  zero. The Loki sample shows 12 such rows failing within 3ms; at that pace the
  120s leg budget and the synthetic-timeout-streak threshold both stay unreached
  forever, so susan walks its **entire** pending set (tens of thousands of ids) every
  sweep at effectively zero per-row cost and zero yield — the exact "feed an
  unresponsive conductor without pacing" failure mode `HealCircuit` was built
  (2026-07-29, per the adam backlog item cited) to prevent, just for a different
  error shape than the one it was tuned against.

### Fix direction (bounded, code-only)

1. Broaden the transient classification to include connection-refused/closed as a
   **distinct transient-equivalent class** — not identical to timeout (a closed
   socket may mean "reconnect now would help," where a timeout means "conductor is
   busy, wait") — so it can route to `timeout_should_route_to_adopt` when a peer
   hint exists, same as an answered timeout does today.
2. Make `HealCircuit::record` trip on a streak of **any never-answered** outcome
   (timeout OR connection-refused/closed), not only the synthetic-timeout marker —
   the shared property both share is "the conductor never had a chance to answer,"
   which is exactly what the circuit exists to detect and shed load from.
3. Re-check the ~5k `Ok(None)` counted on susan in the same window against the
   ~64,658 failures — confirms these ARE getting through occasionally (partial
   reconnect?) or come from a different code path (worth confirming before assuming
   the socket is permanently gone rather than flapping).

## Correction to the alpha-A projector doc (2026-07-31)

`alpha-a-projector-chronic-catchup-flap.md`'s framing of `pools_healthy 3/7` needs a
dated correction: `pools_healthy` is an **admission-routing** signal only — it does
not feed reconcile convergence directly. The projector's failure to converge is
better evidenced by `elohim_projection_reconcile_converged=0` and the heal-outcome
counters (as used in this item) than by pool-health alone. Noting the correction
here since that doc did not have a dedicated status/correction section to append to
inline without touching a file outside this item's scope.

## Status

**Open.** (a) may be operator/substrate-owned (confirm whether susan's conductor
socket is flapping or fully down, and whether a restart clears it) — needs a
runtime/operator follow-up pass, not a code fix. (b) is bounded code work: extend
the transient classification + `HealCircuit` streak predicate in
`projection_reconcile.rs` to cover connection-refused/closed, then re-verify against
susan's `elohim_projection_heal_outcomes_total` and Loki error-text distribution
post-fix.

shift_objective: |
  Fix `elohim/elohim-storage/src/p2p/projection_reconcile.rs` so instant
  connection-closed/refused conductor errors are paced like timeouts instead of
  resetting `HealCircuit`'s streak. Concretely: (1) extend
  `is_transient_conductor_error` (or add a sibling predicate) to recognize
  connection-refused/closed text as transient-equivalent for the
  `timeout_should_route_to_adopt` gate; (2) change `HealCircuit::record` to trip its
  streak on any never-answered outcome (synthetic timeout OR connection-closed), not
  only the synthetic-timeout marker; (3) add a unit test mirroring the existing
  `timeout_should_route_to_adopt` table-tests for the new error class; (4) verify
  against susan's live Prometheus/Loki signature post-deploy — `outcome="failed"`
  rate should drop as rows either retry-and-succeed or shed via the circuit instead
  of exhausting the full pending set every sweep. Sub-concern (a) — whether susan's
  conductor WS needs an operator-side restart/reconnect — is a separate,
  substrate-owned follow-up; do not fold it into this code fix's verification gate.

## Occurrence log (2026-08-03, pipeline-landing shift close)

~10:35Z: container restart, last_terminated_reason=Error (NOT OOMKilled), Ready
again post-restart — first recorded container-level crash of the saturation
pathology (prior episodes were WS-dead / per-call-timeout without termination).
Same window: her conductor memory 1m-8m alloc-bucket anomaly (18-90x peers) and
gossip-timeout-per-5s rate documented in the shift journal. Exit-code/log
detail not pulled (flagged unverified). If the crash recurs, this entry
graduates to a conductor-memory investigation with the smaps localizer.

## Addendum 2026-08-11: the circuit now trips — and shedding is what starves the ghost-decay cure

The fix this entry asked for appears to have landed in effect: `HealCircuit` is no
longer blind. It trips, fleet-wide, **8–12 times per hour** on six of seven pods
(adam 12, james 12, jessica 12, gertrude 11, eve 8, susan 8; matthew only 2).

That surfaced the NEXT link in the chain, and it re-aims a diagnosis that had been
pointed at the wrong seam all week.

**Measured 2026-08-11 ~19:45Z, post edge #1341** (which shipped
`elohim_content_ghost_decay_blocked_total{leg}` — the refusal twin of the decay
counter). The blocked-leg meter shows `leg="disabled"` = 0 on all seven pods, so
the ghost-decay flag is live fleet-wide; and yet decay-arm CONSIDERATIONS over
30 min are matthew 69, james 4, gertrude 1, susan 0, eve 0, adam 0 — against
known_divergent{content} of susan 1660, eve 647, gertrude 597.

The shem pods are not sweeping less. They sweep MORE (adam 7.1, eve/gertrude 6.1,
susan 5.1 sweeps/30min vs matthew's 3.1). The rows simply never arrive at a
`Hold`/`ContestPeer` pre-flight, which is the only place the decay arm runs.

susan's own log says why, in one sweep:

```
BATCH head resolve failed at the CALL level — every id in this batch returns to
pending ... error: "Request timeout: heal conductor call exceeded per-attempt
timeout 15s", ids=8
OPENED the unresponsive-conductor circuit on a CALL-level failure — shedding the
rest of the leg, remaining gaps resume next sweep   consecutive_timeouts=3
heal leg finished   healed=0  conductor_missing=0  to_resolve=2175  batch_size=8
```

**to_resolve=2175, batch_size=8, three consecutive timeouts, then shed.** susan
attempts ~24 of 2175 ids per sweep, heals 0, and sheds. Next sweep repeats from a
corpus that never drains. The conductor 15s per-attempt timeout fires 43–60 times
per hour on EVERY pod (fleet total 369/hr).

### Why this matters beyond susan

The ghost-declaration decay cure (2026-08-10) was never the bottleneck for the
shem stock, and neither was evidence starvation at its predicate. Decay lives
downstream of a heal leg that sheds before adjudicating. matthew is the only pod
draining phantoms **because it is the only pod whose circuit rarely opens (2/hr)**.
Tuning decay dwell, evidence windows, or advertiser diversity cannot move a row
the leg never reaches.

### Not a regression from the 2026-08-11 wave

Chronic, checked explicitly: 321 conductor timeouts fleet-wide in the hour ENDING
15:00Z (pre-deploy) vs 369 in the hour ending ~19:45Z (post-deploy, and that
window still carries restart churn). Same order of magnitude; the deploy did not
cause it.

### Open question this addendum does NOT answer

Whether shedding is correct-but-starving (the circuit is doing its job protecting a
saturated conductor, and the real lever is conductor capacity / batch sizing /
in-wasm budget) or whether the shed scope is too broad (shedding the whole leg on
3 consecutive call-level timeouts, when a smaller batch or a partial-progress
carry-forward would let a 2175-row corpus drain across sweeps). Both fit the
evidence. Note `batch_size=8` against a 15s per-attempt timeout, and that
`HcClient::call_zome` is uncancellable — a caller-side timeout abandons work the
conductor keeps doing, so retrying the same batch may be adding to the load it is
reacting to. This is the decision the next shift should make explicitly rather
than by tuning.
