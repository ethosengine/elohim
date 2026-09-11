---
id: "backlog-self-heal-render-degenerate-cumulative-counter-false-positive"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "render-degenerate exhaustion fired on an IDLE node: /admin/render-stats counters are lifetime-cumulative, so the poller read a since-boot ratio as a live condition and filed 'sustained >= 3 polls' over a window in which zero renders happened (FIXED — delta predicate)"
slug: "self-heal-render-degenerate-cumulative-counter-false-positive"
written: "2026-09-11"
author: "runtime-triage"
status: "wip"
priority: "medium"
self_heal_status: in-progress
severity: medium
fingerprints: [afc100835f7c]
nodes: [alpha-b]
relatedNodeIds: []
tags: [self-heal, render-degenerate, sensing-gap, cumulative-counter, elevate-arm, runtime-harvest, false-positive, closure-by-disappearance, adam]
cites:
  - https://elohim.host/admin/render-stats
  - https://elohim.host/admin/self-healing
  - https://doorway-alpha.elohim.host/admin/render-stats
  - .claude/scripts/_lib/runtime_harvest.py
  - .claude/scripts/_lib/__tests__/runtime_harvest_test.py
  - elohim/elohim-render/src/stats.rs
  - elohim/elohim-render/src/traced_fetcher.rs
  - doorway/doorway-service/src/render/registry.rs
  - genesis/docs/superpowers/plans/2026-06-13-elevate-arm-runtime-harvest-plan.md
  - genesis/data/timeline/backlog/runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11.md
---

# The node was not saturated. It was idle.

## What is exhausted

Nothing. This is the first **confirmed false positive** of the elevate arm, and the
concern is the sensing defect that produced it — not a node condition.

Ledger line (fp `afc100835f7c`, filed poll 70, `2026-09-11T22:59:47+00:00`):

```
render.degenerateRate 0.36 sustained >= 3 polls (SSR stalled/timedOut saturation)
```

Re-fetch at triage, `GET https://elohim.host/admin/render-stats`:

```json
{"total": 11, "rendered": 7, "renderedEmpty": 0, "stalled": 4, "timedOut": 0,
 "errored": 0, "avgWallMs": 1046, "maxWallMs": 1869, "degenerateRate": 0.36363636363636365}
```

The condition is "live" only in the sense that the ratio is still being served. The three
samples the predicate fired on, read out of `.claude/data/runtime-cursor.json`, are
**byte-identical**:

```
alpha-b 0 render {'total': 11, ..., 'stalled': 4, 'timedOut': 0, 'degenerateRate': 0.3636...}
alpha-b 1 render {'total': 11, ..., 'stalled': 4, 'timedOut': 0, 'degenerateRate': 0.3636...}
alpha-b 2 render {'total': 11, ..., 'stalled': 4, 'timedOut': 0, 'degenerateRate': 0.3636...}
```

`total` never moved. **Zero renders occurred anywhere in the observation window.** The
poller reported sustained SSR saturation while watching a node that rendered nothing. The
4 stalls are historical — somewhere in this process's 15 965s (4.4 h) uptime — and had
already stopped.

## Root-cause inventory (scope pass)

1. **`elohim/elohim-render/src/stats.rs:10-11`** — the metric says so itself:

   > *"Cumulative (lifetime) counters for the MVP; a bounded rolling window is the natural
   > refinement once the scorer wants recency-weighting."*

   `RenderTraceStats::record` (`stats.rs:58-71`) only ever increments; `snapshot`
   (`stats.rs:74-95`) divides lifetime `stalled + timedOut` by lifetime `total`. There is
   no decay and no reset short of a process restart.

2. **`.claude/scripts/_lib/runtime_harvest.py`, `_render_degenerate` (pre-fix)** — read
   that lifetime scalar and required it to be `>= DEGEN_RATE` on `DEGEN_POLLS` consecutive
   polls. Against a monotone-ish ratio the "sustained across N polls" test carries **zero
   additional information**: once the lifetime ratio crosses 0.25 it is above 0.25 on
   every later poll until diluted by a large volume of clean renders. On alpha-b that means
   16 total renders minimum to fall under threshold — days of traffic at the observed rate.

3. **The closure contract breaks (elevate-arm plan D5).** The poller closes a finding by
   DISAPPEARANCE (`clean_poll_streak >= CLOSE_STREAK` deletes the line). A predicate over a
   lifetime ratio can never go absent promptly, so this fingerprint could not self-close —
   and because presence suppresses dispatch for ANY status, the stale line would have
   blinded the poller to a *future genuine* render exhaustion on alpha-b indefinitely.

4. **The plan specified deltas; the implementation did not.** Elevate-arm plan D2
   (`genesis/docs/superpowers/plans/2026-06-13-elevate-arm-runtime-harvest-plan.md:29`)
   names the signal as "`degenerateRate` + **`stalled`/`timedOut` deltas**", and the plan's
   own example ledger line (`:88`) reads "(stalled+timedOut **rising**)". The word "rising"
   was dropped in implementation along with the delta.

5. **The sibling predicate already had it right.** `_admission_shed` takes a strict delta
   over the equally-cumulative `admission.shedTotal`
   (`all(b > a for a, b in zip(sheds, sheds[1:]))`), and the suite even asserts
   *"admission-shed silent when shedTotal flat"*. The discipline existed in the same file
   and was applied to one cumulative counter but not the other. `_circuit_open` and
   `_projector_lag` read *state* fields (`circuit`, `caughtUp`), which are legitimately
   current-condition reads — **`_render_degenerate` was the only counter-vs-state
   confusion.**

## Fix path

Read the delta, per D2. Landed in `_render_degenerate` + new helper `_cum_render`:

- degenerate share is computed over the renders that are **NEW across the window**
  (`d_degen / d_total` between the first and last stored sample), not over lifetime;
- `d_total <= 0` → silent (nothing rendered — silence is not saturation);
- either delta negative → silent (counters reset, i.e. a pod restart) and the window refills;
- the delta baseline spans the whole stored ring buffer (`WINDOW`) so low-volume saturation
  stays visible, while still requiring `DEGEN_POLLS` observations before it may fire;
- `_cum_render` reads the `/admin/render-stats` shape (`stalled` + `timedOut`) and falls
  back to `/admin/self-healing`'s smaller `{total, degenerateRate}` sub-object, so the
  predicate works off either endpoint and stays field-presence-tolerant.

**No fingerprint churn**: `rh.fingerprint` keys on `node|class|provenance`, and
`provenance` stays `render-degenerate` — asserted in the suite so the rewrite cannot
silently re-key existing ledger entries.

**The runtime Rust is deliberately NOT changed.** A bounded rolling window in `stats.rs`
is the right eventual shape (the module says so), but it would only take effect after an
operator fleet roll, and the sensing layer must read whatever a deployed peer serves —
including older binaries. Delta-on-cumulative is correct against both.

## Current decision

**FIXED at the sensing layer; ledger line deleted as a confirmed false positive.**

Tests: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` — 44 assertions green
(was 35; the render cases were rewritten in cumulative counters and six were added,
including the alpha-b flat-counter regression and a counter-reset case). The shell
fixture `_Hot` was serving a *constant* payload — under delta semantics that describes a
node which has rendered nothing since boot, so it now serves rising counters. Sibling
suites green and unaffected: `residual_channel_test.py` (59), `harvester_blind_test.py`
(16), `findings_ledger_test.py` (17).

Verified against the real stored window, not just fixtures:

```
alpha   -> [{'provenance': 'projector:reconcile', ...}]   # genuine, still fires
alpha-b -> []                                             # false positive, now silent
```

The ledger line for `afc100835f7c` was **deleted** rather than left to age out. The rule
is manual-delete-only-on-confirmed-removed, and this qualifies twice over: the condition
never existed, and the fix demonstrably removes the signal. Leaving it would have
suppressed dispatch on a future genuine alpha-b render exhaustion for as long as it sat
there. A real saturation now re-files as NEW.

## Secondary observation — B-side SSR latency is real, but this was not its evidence

Worth recording, explicitly **not** promoted to its own concern (11 renders is too thin a
base to mint one):

| | alpha (matthew) | alpha-b (adam) |
|---|---|---|
| `total` / `stalled` | 9 / 0 | 11 / 4 |
| `avgWallMs` | 102 | 1046 |
| `maxWallMs` | 367 | 1869 |

Same image, same config. Backing out the stalls, adam's *successful* renders still average
roughly 600 ms against matthew's ~100 ms. The per-fetch soft budget is
`DEFAULT_SOFT_BUDGET_MS = 1_200` (`elohim/elohim-render/src/traced_fetcher.rs:27`), which
converts an unsettled fetch into a recorded `Stalled` plus a fast fallback
(`traced_fetcher.rs:133-147`); it is overridable per-deploy via
`DOORWAY_SSR_FETCH_SOFT_BUDGET_MS` (`doorway/doorway-service/src/render/registry.rs:357`).
Adam sitting within ~2x of that ceiling is why its stalls cluster there.

Do **not** widen the soft budget in response to this. The budget is not the defect — the
upstream read latency is, and raising the ceiling would trade a fast degenerate fallback
for a slow one. If B-side render latency needs a concern, it needs volume first: the delta
predicate will now file one honestly the moment adam actually renders and stalls.

## Follow-on named, not taken

`_projector_lag` reads `projector.caughtUp`, a live state field — sound, and it is the
predicate that correctly fired on alpha this same poll. But per the scope pass,
`ProjectionReconcileStatus` carries **no sweep timestamp**, so a *frozen* `caughtUp:false`
(heal wedged in-flight, or the lamad bridge down, so `publish_sweep` never runs) is
indistinguishable on the doorway surfaces from a live one. `observedAgeMs` does not cover
this: it measures only the doorway→storage poll hop, against a hardcoded
`P2P_HEALTH_STALE_AFTER_MS = 120_000` (`doorway/doorway-service/src/routes/health.rs:173`).
The discriminators that do exist — `projectionReconcile.sweeps` and `healedTotal` on
`/p2p/status` — are **already sampled** by the poller and simply not consulted by the
projector predicate. Naming it here; the deterministic-layer owner holds poller tuning
(same reservation the matthew-rekey record makes about a `divergentAnchor`-rising
predicate).

## Verification

- 2026-09-11 ~23:00 UTC — `/admin/render-stats` + `/admin/self-healing` re-fetched on both
  nodes (HTTP 200, quoted above); cursor window read from disk showing three identical
  alpha-b samples.
- Fixed predicate evaluated against that same on-disk window: alpha-b yields `[]`.
- Full harvester suite 44/44; three sibling suites green.
- Closure: no poller closure needed — the line is gone. Regression signature to watch: a
  `render-degenerate` fingerprint re-filing on a node whose `total` is *not* advancing
  across the window would mean the delta read has regressed.
