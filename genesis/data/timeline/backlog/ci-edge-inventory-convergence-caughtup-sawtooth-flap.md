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
tags: [ci, elohim-edge, dataplane, a2o, caughtUp, projection-reconcile, quiesce, flake, measurement]
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
