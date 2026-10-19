---
id: "backlog-ci-edge-inventory-convergence-caughtup-sawtooth-flap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "inventory-convergence asserts p2p.caughtUp as an instant on a field that sawtooths — one scenario, two peer labels, two fingerprints, and the quiesce gate said PASS minutes earlier"
slug: "ci-edge-inventory-convergence-caughtup-sawtooth-flap"
written: "2026-09-19"
author: "ci-failure-triage"
status: "backlog"
priority: "medium"
ci_status: blocked
fingerprints: [7db3e674ea84, 591656751c2b]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, dataplane, a2o, caughtUp, projection-reconcile, quiesce, flake, measurement, performance, perf-telemetry, perf-convergence, friction-blind, friction-wait, phase-steady, lane-operator]
cites:
  - genesis/data/timeline/backlog/projection-reconcile-actionable-sawtooth.md
  - genesis/data/timeline/backlog/fleet-quiesce-pass-not-convergence.md
  - genesis/data/timeline/backlog/ci-harvest-fingerprint-granularity-banner-collision.md
  - genesis/a2o/features/dataplane/inventory-convergence.feature
  - genesis/a2o/steps/dataplane.steps.ts
  - scripts/ci/fleet-quiesce-gate.sh
  - scripts/ci/run-dataplane-validation.sh
---

# `p2p.caughtUp` read as an instant, on a field that sawtooths

## The failure

`elohim-edge/dev` #1466, stage **Dataplane Validation**, result **FAILURE**:

```
1) Scenario: the seed-facing doorway peer catches its projection up under sustained gossip # features/dataplane/inventory-convergence.feature:42
   ✔ Given peer "alpha-A" at "alpha-A"
   ✔ And peer "elohim.host" at "elohim.host"
   ✖ Then peer "alpha-A" /health p2p.caughtUp is true # steps/dataplane.steps.ts:446
       AssertionError [ERR_ASSERTION]: alpha-A /health: p2p.caughtUp is false (expected true)
       false !== true
   - And peer "elohim.host" /health p2p.caughtUp is true
...
122 scenarios (1 failed, 3 pending, 109 skipped, 9 passed)
  [FAIL] inventory-convergence: passed=0 failed=1 pending=0
ERROR: script returned exit code 1
Setting overall build result to FAILURE
```

### Both fingerprints are this one scenario

`7db3e674ea84` (`alpha-A …`) and `591656751c2b` (`elohim.host …`) are the two
adjacent `Then`/`And` assertions of the **same** scenario
(`inventory-convergence.feature:42`). The peer name is interpolated into the
message, so each label mints its own fingerprint. They never co-occur: cucumber
short-circuits, so whichever assertion reaches the failing peer first is the one
that fingerprints.

Occurrence evidence:

| fp | peer label | builds | seen |
|---|---|---|---|
| `591656751c2b` | `elohim.host` | 1444, 1449, 1456, 1457 | 4 |
| `7db3e674ea84` | `alpha-A` | 1466 | 1 |

**One concern, five occurrences across 1444..1466.** Intermittent: #1467 — the
very next build, with a changeset that touches only a pipeline timeout limit and
`peer-roll-gate.sh` — passed this exact scenario (`10 passed`, no `Failures:`
entry).

## Verdict — **real substrate signal, sampled as a flake**

The assertion is not wrong and must not be "repaired". `caughtUp: false` is a
truthful read: a snapshot exists and reports behind (contrast `caughtUp`
*absent*, which means no snapshot at all — the distinction drawn in
`ci-genesis-doorway-503-seed-phase-wedge.md`). What makes it arrive as a flake
is that the scenario takes a **single sample** of a field the fleet is known to
oscillate.

### Not museum trap #16 (measurement-by-restart)

Checked explicitly, and it clears. #1466 ran `VALIDATE_ONLY=true`:

```
Stage "Deploy Edge Node - Alpha" skipped due to when conditional
VALIDATE_ONLY — skipping build/deploy stages; running Dataplane Validation only
```

`getBuild` params: `VALIDATE_ONLY:true`, `FORCE_DEPLOY:false`,
`CONDUCTOR_ROLL:false`. A full-log regex sweep for
`rollout|kubectl|pod-delete|restart|CONDUCTOR_ROLL` returned only the skip
notice, static pod-template YAML, and scenario titles. **Nothing in this run
restarted the surface it measured.** The trap-#16 diagnostic tell is absent.

## Root cause

Two layers, and only the lower one is a defect:

1. **Substrate (the real one, already canonicalized).**
   `projection-reconcile-actionable-sawtooth.md` (2026-09-19, priority high,
   cause unknown) measures ~50 actionable divergences reappearing on every
   alpha storage peer every 20–30 minutes, `healedTotal` 0, `converged` false.
   Whether a given instant reads `caughtUp: true` depends on where in that cycle
   the probe lands. That is exactly a coin toss, and it is that doc's finding.

2. **Measurement (why it costs a triage dispatch each time).** Inside the *same
   stage*, two probes of the same field disagree. The shell fleet-quiesce gate
   ran first and passed:

   ```
   fleet-quiesce[2026-09-19T09:09:50Z]: PASS A-caughtUp=True B-caughtUp=True A-quiesced=True(...) sweeps=33.0 — sustained 362s
   A-QUIESCED (gate scope: probe A converged+sustained; B-caughtUp=True excluded per 2026-08-07 plateau ruling — PASS is NOT fleet convergence)
   ```

   Minutes later the cucumber step re-read the same field and got `false`. The
   gate's own banner already disclaims that PASS is not fleet convergence
   (`fleet-quiesce-pass-not-convergence.md`) — so the gate is not lying; the
   scenario is simply asserting a stronger property than the gate establishes,
   with no settle window and no evidence in its failure message.

## Current decision — **blocked on the substrate cure**

Blocked, not fixed. The unblocking move is the root cause in
`projection-reconcile-actionable-sawtooth.md`: *an actionable anchor is either
healed (counted) or reclassified (named reason)*. Until reconcile rests, this
scenario will keep flipping, and that flipping is the honest report.

Deliberately **not** taken here, with reasons:

- **Do not add a poll/settle window to the step.** The cycle is 20–30 minutes;
  a bounded poll either fails to cross a trough (no help) or is long enough to
  mask the sawtooth (suppresses a true signal). Converting an honest red into a
  slow red buys nothing.
- **Do not `@wip` or delete the scenario.** It is the emergent-property
  assertion for the receive-side idempotency fix; quarantining it would retire
  the only check that the fleet converges at all.

Two cheap improvements are available and are *not* suppression, left for the
owner of the substrate concern so the fix and its probe land together:

1. Carry the divergence evidence into the assertion message
   (`divergentAnchor`, `healedTotal`, sweep count from the same `/health` body),
   so a failure is routable to the sawtooth without a triage dispatch.
2. Have the scenario state the property it means — reaches `caughtUp` and
   *holds* it — rather than an instant, once the substrate can satisfy it.

## Fingerprint-granularity note

This pair is a live second instance of the class in
`ci-harvest-fingerprint-granularity-banner-collision.md`, which already names
this exact assertion template (peer label + `JSON.stringify(caughtUp)`
interpolated into the message). There the variants were
`false` vs `undefined` on one peer; here they are two peers of one scenario.
Recorded as a cross-reference there in the same pass. No harvester change is
asserted by this entry — the semantic distinction between `false` and absent is
load-bearing and must survive any de-duplication.

## Evidence trail

- `elohim-edge/dev` #1466 console — scenario block, quiesce-gate PASS line,
  `VALIDATE_ONLY` skip notice, FAILURE (quoted above).
- `elohim-edge/dev` #1444 console — the same scenario with `alpha-A` passing and
  `elohim.host` failing, confirming the label swap.
- `elohim-edge/dev` #1467 — same scenario green, no relevant changeset.
- ci-investigator run 2026-09-19 (this triage) — build table 1440..1467.
- `.claude/data/ci-findings.jsonl` fps `7db3e674ea84`, `591656751c2b`.

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): PRESENT, measurement leg only. The scenario still asserts one instant sample with a message carrying only the caughtUp value (genesis/a2o/steps/dataplane.steps.ts:483-496; inventory-convergence.feature:42-45). The gate and the scenario read different surfaces: storage /p2p/status pull.caughtUp (scripts/ci/fleet-quiesce-gate.sh:50,194) versus a peer's /health p2p.caughtUp, so "same field" overstates. Same mechanism as: fleet-quiesce-pass-not-convergence. Confirming measurement: both fields polled every 5 s across two sweeps on the household.

**SEEN AGAIN 2026-10-12** (shift `2026-10-12T04-10-roll-scheduler-latch-pin-edge-green`, the first validate-only runs after the seven-conductor pin roll to `conductor-96bcd2bee215`, edge #1608 04:40–06:25Z): the fleet-quiesce gate read `A-caughtUp=False` (storage-A pull) flapping for most of 07:00–09:16Z — #1610 passed the gate only in a 6-min window, #1611 hit the 2700 s deadline (NOT_BUILT no-measure) — and #1612 (10:00Z, timer-fired) passed the gate then failed this scenario on `elohim.host /health: p2p.caughtUp is false`, now correctly surfaced as the ONE real red beside 14 undefined by the new `PENDING-ONLY refused: non-pending-step-status — failed` line (scripts/ci/run-dataplane-validation.sh). Two readings owed, not taken tonight: (1) caughtUp recovery time after THIS roll versus the 2026-10-09 roll (#1588/#1590) from the storage pods' `/health`/`/p2p/status` history — ~4 h of flapping is longer than the ~20 min documented restart churn and the first measured behaviour on the new pin; (2) whether the storage change `4d9cde774` (one read per heal-leg re-admission) moves it either way once a deploy carries it. Status unchanged.

**FIRST RUN WITH THE BOUNDED WAIT 2026-10-14** (edge #1623, validate-only, dispatched by orchestrator #2052 for
`bca0c5917`): quiesce PASS sustained 362 s, and this scenario passed (`[OK] inventory-convergence: passed=1
failed=0`). One pass is a candidate, not done: the next runs show whether the wait crosses the sweep every time.

**SEEN AGAIN 2026-10-13** (edge #1621, validate-only, no deploy that day): the quiesce gate passed in 363 s, the first pass of the day after 1,063 retired `epr:e2e-*` pins were removed from alpha matthew. Its own lines show `A-converged` going 1.0 to 0.0 at 20:10Z as sweep 237 began, and the scenario sampled `alpha-A /health p2p.caughtUp` as false at about 20:14Z. No pin changed state between 19:40Z and the sample, so this occurrence is the reconcile sweep in flight, not the pull stream. The gate requires a sweep to advance before it passes, so it hands the scenario a node that has just started one. Status unchanged.

**DECIDED 2026-10-14** (operator): the scenario waits a bounded time instead of sampling once. The 2026-09-19 objection above was to a poll against a 20-30 minute cycle. The 2026-10-13 occurrence was a single 300 s sweep in flight, and a 600 s wait crosses it. The scenario "the seed-facing doorway peer catches its projection up under sustained gossip" now uses `peer "<name>" /health p2p.caughtUp becomes true within 600 seconds` for alpha-A and elohim.host: it polls /health every 15 s and passes on the first reading where `p2p.caughtUp` is true. The instant step `... p2p.caughtUp is true` is unchanged and still used by peer-mesh.feature. A node that stays behind longer than 600 s still fails, and the message now carries the last /health body (`caughtUp` with false and absent kept distinct, `converged`, `divergentAnchor`, `observedAgeMs`, and `healedTotal` and `sweeps` where the body has them, otherwise `absent`) plus the sample count and elapsed time. The first `/health` fields are the doorway's; `healedTotal` and `sweeps` are on storage `/p2p/status`, so they read `absent` here today. Not run against the fleet from this change.

**SEEN AGAIN 2026-10-19, and not as a sawtooth** (shift `2026-10-19T03-25-finish-integration-2026-10-18`). The three validate-only siblings #1661 (started 09:42Z), #1663 (13:07Z) and #1665 (14:46Z) each failed on one step only: `peer "elohim.host" /health p2p.caughtUp becomes true within 600 seconds`. In each, alpha-A passed the same wait and the quiesce gate passed. All three ended on the same body, `caughtUp=false converged=false divergentAnchor=151`, with `observedAgeMs` of 3–8 s, so the doorway is still observing and the count does not move. Over five hours the number stayed at 151. A sweep in flight cannot explain that; the bounded wait decided on 2026-10-14 is working as intended and is reporting a plateau. These runs followed the deploys of `d37d467e4`, `c41e075a8` and `be1bdb59f` to alpha (edge #1660, #1662, #1664).

*Correction, same day.* The first version of this note said those deploys do not touch elohim.host and asked the operator for a reading. Both were wrong. elohim.host is adam-alpha's doorway, in the `elohim-alpha` namespace, and its only serving upstream is `elohim-adam-alpha:8090`. Edge #1664 rolled adam-alpha's storage to `1.0.0-dev-be1bdb59` like the others. The reading also needs no cluster access: these are public HTTP reads, taken here at 16:41Z.

- **`elohim.host /p2p/status`:**
  - `projectionReconcile`: `caughtUp=true`, `converged=false`, `divergentAnchor=151`, `healedTotal=291`, 41 sweeps, `failed=1`.
  - `provideLoop`: `deadRemainingStuck=true`, with `reanchorDeadRemaining=8`, `stuckSweeps=24` and `reanchorHeldUnanswered=4`. This is the `provide-loop-dead-remaining-stuck` runtime finding, which keeps recurring.
- **`elohim.host /db/p2p/conductor-diagnostics`** (adam's conductor, iroh `97bea7d7…`, connections opened about 03:26Z):
  - It has 6 direct connections.
  - Four of them have received **0** messages while sending 1.1M to 8.6M each.
  - The fifth, `cb879f9d…`, has received 28 326 messages, and adam's conductor blocked every one of them.
  - Only `c8087ae0…` exchanges traffic both ways (52 014 received).
- **`doorway-alpha /db/p2p/conductor-diagnostics`** (matthew's conductor) has blocked **4 034 344 of 4 034 344** incoming messages from adam's conductor `97bea7d7…`.

This is DELTA 2026-10-10a in `dataplane-convergence` (then 4 058 545 of 4 058 544), still in force nine days later. Matthew's conductor drops everything adam sends, and four peers send adam nothing. adam therefore cannot heal its 151 divergent anchors, and no wait will clear it: under the current pin a chain-fork warrant blocks forever.

So this red belongs to the warrant-exile item (`steward-key-chain-fork-warrant-exile`, decision D1, retro action S8), not to this sawtooth item and not to the pushes. It will stay red on every validate-only run until D1 is ruled and applied to adam. Status unchanged.
