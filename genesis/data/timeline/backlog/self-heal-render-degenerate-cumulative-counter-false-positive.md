---
id: "backlog-self-heal-render-degenerate-cumulative-counter-false-positive"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "_render_degenerate has twice reported a non-condition as SSR saturation: first reading a lifetime-cumulative ratio as a live one (FIXED — delta), then scoring a delta with no floor under its denominator, so one slow fetch out of four renders read as exhaustion (FIXED — event floor)"
slug: "self-heal-render-degenerate-cumulative-counter-false-positive"
written: "2026-09-11"
updated: "2026-09-12"
author: "runtime-triage"
status: "wip"
priority: "medium"
self_heal_status: in-progress
severity: medium
fingerprints: [afc100835f7c, da8bb3bdd7e1]
nodes: [alpha-b, alpha]
relatedNodeIds: []
tags: [self-heal, render-degenerate, sensing-gap, cumulative-counter, rate-without-a-base, elevate-arm, runtime-harvest, false-positive, closure-by-disappearance, adam]
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
  - genesis/data/timeline/backlog/self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall.md
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
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

---

# Second firing, 2026-09-12 — fp `da8bb3bdd7e1` (node **alpha**): a rate with no base

## What is exhausted

Still nothing. The delta fix above works exactly as designed and this finding is its
*correct* arithmetic — applied to a denominator of four.

Ledger line (fp `da8bb3bdd7e1`, filed poll 78, `2026-09-12T00:55:44+00:00`):

```
render degenerate 1/4 of NEW renders = 0.25 across last 6 polls (SSR stalled/timedOut saturation)
```

The stored window (`.claude/data/runtime-cursor.json`, read at triage) shows precisely
where that came from — **five identical samples and one transition**:

```
alpha 0..4  {'total':  9, 'stalled': 0, 'avgWallMs': 102, 'degenerateRate': 0.0}
alpha 5     {'total': 13, 'stalled': 1, 'avgWallMs': 184, 'degenerateRate': 0.0769}
```

`d_total = 4`, `d_degen = 1`, `rate = 0.25` — equal to `DEGEN_RATE`, so it fired. **One
stalled render.** Not a burst, not a trend: a single upstream fetch that crossed the soft
budget once, in one inter-poll interval.

Re-fetch at triage confirms the node is healthy, `GET https://doorway-alpha.elohim.host/admin/render-stats` (HTTP 200):

```json
{"total": 14, "rendered": 13, "renderedEmpty": 0, "stalled": 1, "timedOut": 0,
 "errored": 0, "avgWallMs": 176, "maxWallMs": 1263, "degenerateRate": 0.0714}
```

`maxWallMs` 1263 against `DEFAULT_SOFT_BUDGET_MS = 1_200` — the single stall is 5% over
budget, which is the budget doing its job (convert an unsettled fetch into a fast
fallback), not a mechanism exhausting itself. `errored: 0`, and the upstream circuit on
this node reads `"circuit": "closed", "errorStreak": 0` with `admission.shedTotal: 0`.

**The tell is the comparison.** In the very same window, alpha-b — the node with 4 lifetime
stalls, `avgWallMs` 896 and a `degenerateRate` of 0.29 — was **silent** (its new renders
were clean: `total` 11→14, `stalled` 4→4). The predicate flagged the healthy node at
176 ms and stayed quiet about the slow one at 896 ms. A sensing rule that inverts the
ranking of the two nodes it watches is mis-calibrated, whatever its arithmetic.

## Root-cause inventory (scope pass)

1. **`.claude/scripts/_lib/runtime_harvest.py`, `_render_degenerate` — no floor under the
   denominator.** The predicate gated on `rate >= DEGEN_RATE` and on `DEGEN_POLLS`
   *observations*, but never on how many degenerate renders the rate is actually made of.
   With `DEGEN_RATE = 0.25`, a 4-render delta fires on one event and a 1-render delta fires
   at `rate = 1.00`. Treating a 1-of-4 sample as evidence of a rate is a base-rate error:
   if the node's true degenerate rate were a benign 5%, a single stall would still appear
   in ~19% of 4-render windows. The predicate was sampling noise.

2. **`DEGEN_POLLS` was not buying what it appeared to buy.** It counts *stored samples*,
   not polls in which anything rendered. The 2026-09-11 fix deliberately let the delta
   baseline span the whole `WINDOW` "so low-volume saturation stays visible" — correct in
   intent, but it means five stale samples plus one moving one satisfy a "3 consecutive
   observations" test on the strength of **one** transition. The staleness the delta was
   introduced to cure re-entered through the *sustained* half of the predicate.

3. **The emitted line inherited that overstatement.** `across last {len(cums)} polls` read
   "across last 6 polls" for evidence drawn from one poll-to-poll transition, on a
   human-facing ledger line whose whole job is to tell the next reader how much to believe.

4. **Not a runtime defect, and not the sibling agent's.** `elohim/elohim-render`
   (`stats.rs`, `traced_fetcher.rs`) and the doorway render registry behaved correctly
   throughout: one fetch exceeded `DEFAULT_SOFT_BUDGET_MS` and was recorded as `Stalled`
   with a fast fallback, which is the designed behaviour.

## Not shared with the concurrent CI findings

A `ci-failure-triage` agent is holding the alpha **503 hosted-registration** and
**blob-forwarding** findings from this same window. **These do not share a root cause**,
and this record deliberately does not claim theirs:

- a 503 is a *fast* failure and would land in `errored`; alpha reports `errored: 0`, and
  the one degenerate render is a `stalled` — a 1263 ms **slow** fetch, 5% over the soft
  budget;
- the upstream breaker that fronts the storage peer never left `closed` with
  `errorStreak: 0` across the whole window, so the render path never saw the upstream fail;
- and the finding under triage here is a *sensing* defect in the poller's own Python, which
  no runtime condition could cause or cure.

The honest statement is the negative one: nothing in the render evidence corroborates a
shared upstream fault, so the two concerns stay separate.

## Fix path

Give the rate a base. Landed in `_render_degenerate`:

- new constant **`DEGEN_MIN_EVENTS = 3`** — the predicate additionally requires at least 3
  NEW degenerate renders across the window. Three is the same "3 observations before we
  believe it" discipline `OPEN_POLLS` / `SHED_POLLS` / `LAG_POLLS` already carry, applied
  to events rather than polls;
- **deliberately NOT a floor on `d_total`.** A volume floor (e.g. `d_total >= 12`) would
  blind the predicate to a genuinely saturated low-traffic peer — 3-of-3 degenerate *is*
  exhaustion — and low-traffic peers are exactly the population this poller exists to
  watch. The floor belongs on the numerator;
- the emitted line now reports `over {moving} of the last {N} polls`, where `moving`
  counts the poll transitions in which `total` actually advanced — so a mostly-idle window
  can no longer overstate its own base.

**No fingerprint churn**: `rh.fingerprint` keys on `node|class|provenance`
(`runtime-harvest.py:305`); `line` is not an input, and `provenance` stays
`render-degenerate`. Asserted in the suite.

**The soft budget is still not widened**, for the reason the 2026-09-11 record gives: the
budget is not the defect, and raising it trades a fast degenerate fallback for a slow one.

## Current decision

**FIXED at the sensing layer; ledger line deleted as a confirmed false positive.**

Tests: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` — **51 assertions green**
(was 44; +7, all seven derived from this window: the 1-of-4 regression at the exact
threshold, a 1-of-1 at rate 1.00, a 2-event case under the floor, the 3-of-3 low-traffic
case that must still FIRE, two `moving`-poll wording assertions, and a provenance-stability
assertion). Sibling suites green and unaffected: `residual_channel_test.py` (59),
`harvester_blind_test.py` (16), `findings_ledger_test.py` (17).

Verified against the real stored window, not just fixtures:

```
alpha   -> []   # false positive, now silent
alpha-b -> []   # correctly silent (its new renders are clean)
```

The line for `da8bb3bdd7e1` was **deleted** rather than left to age out — same rule and
same justification as `afc100835f7c`: manual-delete-only-on-confirmed-removed, the
condition never existed, and leaving it would suppress dispatch on a future *genuine*
alpha render exhaustion for as long as it sat there. A real saturation re-files as NEW.

## Verification

- 2026-09-12 — `/admin/render-stats` re-fetched on both nodes (HTTP 200, both quoted
  above); `/p2p/status` and the stored `upstreams`/`admission` samples read for the
  breaker and shed cross-check.
- Fixed predicate evaluated against that same on-disk window: both nodes `[]`.
- Full harvester suite 51/51; three sibling suites green.
- Regression signature to watch: a `render-degenerate` fingerprint whose line reports
  fewer than 3 degenerate renders, or one whose `moving` count is 1, would mean the event
  floor or the honest-base wording has regressed.

## What this predicate has now cost, and the standing lesson

Two false positives, zero true positives, on the only predicate in this file that derives a
**ratio** rather than reading a state field. Both failures were the same shape at different
depths: *the denominator was not what the reader assumed*. First it was lifetime when the
reader assumed recent; then it was four when the reader assumed many.

The standing rule for this file, now paid for twice: **a predicate that divides must state
what its denominator is made of, and must refuse to fire until that denominator is made of
enough.** `_circuit_open` and `_projector_lag` read state and have never misfired;
`_admission_shed` takes a strict delta and has never misfired. The ratio predicate has
misfired every time it has fired. If a third firing is also a false positive, the right
move is not a third threshold — it is to replace the ratio with a state field the runtime
publishes (a bounded rolling window in `elohim-render/src/stats.rs`, which that module
already names as its intended refinement), and let the node decide when it is saturated
instead of asking the poller to infer it from counters.

---

# 2026-09-12, same day, second firing of `da8bb3bdd7e1` — the third firing was TRUE, and this concern hands off

**The branch above resolved the other way. Do not act on the escalation it proposed.**

`da8bb3bdd7e1` re-filed at poll 82 (`12:12:45+00:00`) with an honest base — 11 new
degenerate renders over a 31-render delta across 2 moving polls, clearing the
`DEGEN_MIN_EVENTS = 3` floor this record added six hours earlier:

```
render degenerate 11/31 of NEW renders = 0.35 over 2 of the last 7 polls (SSR stalled/timedOut saturation)
```

Triage reproduced it by hand on single, cache-busted, non-concurrent requests: four
consecutive cold renders of `/` on doorway-alpha came back `x-ssr-terminal: stalled` at
1232–1256 ms — clamped at `DEFAULT_SOFT_BUDGET_MS` — while elohim.host rendered the same
route clean at 124–556 ms. **Real condition, real people served degenerate HTTP 200s.**

So the tally this record has been keeping is now superseded:

| | before | after |
|---|---|---|
| `_render_degenerate` false positives | 2 | 2 |
| `_render_degenerate` true positives | **0** | **1** |

The two fixes this record landed are what made the third firing legible: the delta cured
the lifetime ratio, and the event floor kept the predicate silent through the 1-of-4
noise. On the firing that mattered, both floors passed and the predicate was right.
**The predicate is now calibrated; leave it alone.** In particular, do NOT replace it
with a runtime-published rolling window on this evidence — that change would have made
no difference to any of the three firings, and it costs a `stats.rs` change plus a fleet
roll.

## Handoff

`da8bb3bdd7e1` stays listed in this record's `fingerprints:` because this record owns the
history of its first two (spurious) firings. Its **live** concern — and what the ledger
line cites — is now:

**`genesis/data/timeline/backlog/self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall.md`**

which carries the measured root cause (a deploy-coupled cold window after an
`elohim-host-landing` head swap, in which exactly one of 29 SSR data fetches never
settles inside the soft budget) and the blocker. `afc100835f7c` remains wholly this
record's.

## The follow-on this record named a day earlier also came due

The "Follow-on named, not taken" section above reserved a `_projector_lag` question for
the deterministic-layer owner. The same 2026-09-12 poll filed `79f357281ca5` (alpha-b,
`projector:reconcile`), and re-fetch showed `healedTotal: 0` across **209 sweeps** with
`caughtUp` flapping false↔true on windowed-scan noise — i.e. exactly the frozen-vs-live
ambiguity this record predicted, in the wild. Canonicalized in
`genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md`
(2026-09-12 section), still reserved to the owner, now with live evidence attached.
