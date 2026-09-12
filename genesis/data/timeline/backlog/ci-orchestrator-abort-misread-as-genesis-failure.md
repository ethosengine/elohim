---
id: "backlog-ci-orchestrator-abort-misread-as-genesis-failure"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "An operator pressing Stop on a downstream build turns the orchestrator UNSTABLE with 'Genesis failed' — triggerPipeline flattens FlowInterruptedException into ERROR, so a deliberate abort is indistinguishable from a real failure and manufactures a CI finding"
slug: "ci-orchestrator-abort-misread-as-genesis-failure"
written: "2026-09-10"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [9b7f3c58a51a]
jobs: [elohim-orchestrator, elohim-genesis]
relatedNodeIds: []
tags: [ci, orchestrator, abort, misclassification, not-built-is-lossy, elohim-genesis, findings-sentinel]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1845/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1574/
  - genesis/orchestrator/Jenkinsfile
  - genesis/orchestrator/pipeline-results.mjs
  - genesis/orchestrator/orchestrator-integration.test.mjs
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/data/timeline/backlog/ci-orchestrator-supersede-aborts-in-flight-edge-rolls.md
  - genesis/data/timeline/backlog/ci-jenkins-controller-restart-orphans-wave.md
---

## The failure

Ledger fingerprint `9b7f3c58a51a` — `elohim-orchestrator`, `red build, stage:Execute Builds`,
seen 1, first_build 1845, last_build 1845.

`elohim-orchestrator/dev` #1845 (result `UNSTABLE`, description `auto: elohim-genesis | P2P:
alpha=ok`). The Execute Builds stage console, lines 611-620:

```
▶️ Triggering elohim-genesis/dev [wait-for-result]...
Scheduling project: elohim-genesis » dev
Starting building: elohim-genesis » dev #1574
Aborted by Matthew Dowell
Build elohim-genesis » dev #1574 completed: ABORTED
❌ elohim-genesis: ERROR
WARNING: Genesis failed - seeding or tests may have issues
```

`elohim-genesis/dev` #1574's `InterruptedBuildAction` names the cause exactly:
`UserInterruption: Aborted by mbd06b` plus `BuildTriggerCancelledCause: Calling Pipeline was
cancelled`. Nothing in genesis failed. A human pressed Stop.

## Verdict

**Real — but the defect is in the measure, not in genesis.** This is a self-inflicted
instance of museum trap #1: the orchestrator reads ABORTED as a failure. Note the two
existing ABORTED shapes are both ruled out by evidence:

- **Not trap #1's supersede shape** — no `abortPrevious` preemption; #1574 was not superseded
  by a newer genesis build number.
- **Not trap #15's restart-orphaned shape** — a search of #1574's 2274-line console for
  `Resuming build|after Jenkins restart|Waiting for reconnection` returns exactly one match,
  and it is `Aborted by Matthew Dowell`. No controller restart.

This is a **third ABORTED shape: the operator manual abort.** Its remedy is neither "ignore
the red" (#1) nor "retrigger" (#15) — it is *there should never have been a red*. The
harvester behaved correctly (`RED = {"FAILURE","UNSTABLE"}` matched a genuinely-UNSTABLE
build); the orchestrator lied to it.

## Root cause

Four-step chain, all in `genesis/orchestrator/Jenkinsfile`:

1. `triggerPipeline` calls `build(job:…, wait: true, propagate: false)`. **`propagate: false`
   suppresses downstream RESULT propagation but NOT interruption** — an abort of the
   downstream still throws `FlowInterruptedException` into the waiting parent's step.
2. `triggerPipeline`'s `catch (Exception e)` special-cases only `'No item named'` and
   flattens *everything else* to `[success: false, result: 'ERROR']`. **The abort's identity
   is destroyed here.**
3. The genesis handler branches on `!success` and calls
   `unstable('Genesis failed - seeding or tests may have issues')` — asserting a seeding/test
   defect that was never observed.
4. Orchestrator ends UNSTABLE → `ci-harvest` captures it → new fingerprint → a triage agent
   is dispatched at 23:25Z for a human pressing Stop.

The sharp edge: **the repo already declares the correct semantics and the Groovy layer never
consumed them.** `genesis/orchestrator/pipeline-results.mjs` is documented as the "single
source of truth for what counts as success, failure, or waste" and says verbatim that
`ABORTED` is `WASTED_RESULTS`, not `TERMINAL_FAILURE_RESULTS` — "ABORTED is NOT a failure of
the work… Persistent waste signals supersede-thrash or **operator-aborts**." The Jenkinsfile
cannot import an `.mjs`, so the contract drifted silently in the one consumer that most needed
it.

## 2026-09-12 — the "recurrence" at #1854 is a FINGERPRINT COLLISION, not this concern

The sweep reopened `9b7f3c58a51a` on `elohim-orchestrator/dev` **#1854**
(`last_build 1854 > triaged_at_build 1845`). It is not this failure.

- #1845 was `UNSTABLE`, description `auto: elohim-genesis`, and its Execute Builds console
  read `Aborted by Matthew Dowell` → `❌ elohim-genesis: ERROR`.
- #1854 is **`FAILURE`**, description `auto: elohim-edge`, and a regex sweep of its console
  for `ABORTED|Aborted|⏹️` returns **no abort of any kind**. The stage log reads
  `Build elohim-edge » dev #1454 completed: FAILURE` → `❌ elohim-edge: FAILURE` →
  `[Pipeline] error`.

So #1854 is an ordinary **downstream echo** — the class already canonicalized at
`ci-orchestrator-downstream-drift-echo.md` (`ci_status: blocked`) — of a genuine
`elohim-edge/dev` #1454 red (the `blob-durability` habit's `coverageShortfall`-ABSENT
scenario, owned by that habit's own ledger, not by CI triage).

Both builds mint the SAME fingerprint because the harvester's unclassified fallback keys
only on the failing stage name, and **`stage:Execute Builds` is where every downstream
verdict lands** — so it is the maximally-colliding identifier in this job. Recorded as a
third instance of the over-coarse polarity at
`ci-harvest-fingerprint-granularity-banner-collision.md`.

**Action taken:** ledger `triaged_at_build` re-stamped `1845 → 1854`, resetting the sweep's
recurrence reference to the current point. `status` stays `triaged`. **The ABORTED fix is
untested, not disproven** — no operator abort has occurred since it landed, so the
green-streak clock simply restarts.

Second observation, recorded and deliberately NOT fixed here: #1854's summary printed
`BUILDS: ✅ 0 succeeded │ ⚠️ 0 unstable │ ❌ 0 failed │ ⏹️ 0 aborted` for a build whose
Execute Builds stage had just reported a downstream FAILURE. The `error` step aborts the
stage before `results` is populated, so the post-block summarises an empty map. That is
pre-existing behaviour (not introduced by this entry's fix — `failCount` excludes only
`wasted`, and `wasted` is false here), and it is museum trap #1's lossy-measure family one
layer in. It belongs with the echo concern, not here.

## Current decision

**Fixed and landed, locally verified; awaiting disappearance confirmation.** Ledger entry
stamped `status: triaged`, `triaged_at_build: 1854` (re-stamped 2026-09-12 — see the
collision note above; it was 1845 at landing). The harvester's green-streak sweep
(≥3, no recurrence) confirms and closes. `decompose_on_confirm` is deliberately **not** set —
the lesson graduates into the anti-patterns museum as trap #1's third discriminator, so the
entry should be graduated-then-decomposed rather than silently deleted.

## Fix trail

`genesis/orchestrator/Jenkinsfile` — ABORTED is now classified as *waste*, per
`pipeline-results.mjs`:

- `triggerPipeline` catch: a `FlowInterruptedException` returns
  `[success: false, wasted: true, result: 'ABORTED']` *before* the generic ERROR fallthrough.
  Deliberately not rethrown — this catch already swallowed the interruption before the branch
  existed, so classifying it changes no control flow.
- `dispatchResult`: carries `wasted: result.result == 'ABORTED'` for the non-throwing path
  (a downstream that ends ABORTED without interrupting the parent).
- Genesis handler + `recordPipelineResult`: a wasted result echoes
  `⏹️ … ABORTED — waste, not a verdict; baseline held so the next push re-dispatches` and
  **never calls `unstable()`**. Baseline is deliberately not advanced — the work was never
  verdicted, so the next push touching the pipeline re-dispatches it.
- Summary + CI-summary artifact: `failCount` excludes waste, a separate `wastedCount` /
  `wasted_pipelines` keeps supersede-thrash and operator-aborts visible rather than hidden.

Verification (local, no build trigger available — Jenkins MCP is anonymous):

- 5 new regression tests in `orchestrator-integration.test.mjs` under
  `ABORTED classification (Jenkinsfile honours pipeline-results.mjs)` — confirmed red before
  the fix (5 fail), green after.
- Full orchestrator suite `npm test`: **175/175 pass** (was 155 — `pipeline-results.test.mjs`
  and `reconcile-build-graph.test.mjs` existed but were never wired into the `test` script;
  both now run, since `pipeline-results.mjs` is the authority these tests cite).
- `npm-groovy-lint` error set on this file byte-compared HEAD vs post-fix: identical
  (`DeadCode: 2, NglParseError: 3, UnusedVariable: 7` — all pre-existing, none introduced).
- Jenkinsfile CPS method-size guard: `pipeline {}` block 55578 → 56058 bytes (+480). WARN
  before and after (threshold 55000), well under HARD 65000; largest `script {}` body
  unchanged at 10413.

## Done when

- [x] ABORTED classified as waste, not failure, across dispatch + reporting + summary
- [x] Regression tests hold the Jenkinsfile to `pipeline-results.mjs`'s contract
- [x] Lesson graduated into the anti-patterns museum (trap #1, third discriminator)
- [ ] `elohim-orchestrator` green streak ≥3 with no recurrence of `9b7f3c58a51a`
- [ ] Not addressed here: `reconcile-build-graph.mjs`'s `UNSUCCESSFUL_TERMINAL` still groups
      ABORTED with FAILURE. That is **correct for a reconciler** — it asks "did what we
      dispatched actually deliver?", and an aborted pipeline did not. Left deliberately
      unchanged; the distinction is *delivered?* (ABORTED = no) vs *did the work fail?*
      (ABORTED = unknown).
