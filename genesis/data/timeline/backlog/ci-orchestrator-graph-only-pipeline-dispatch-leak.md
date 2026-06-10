---
id: "backlog-ci-orchestrator-graph-only-pipeline-dispatch-leak"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Graph-only pipeline (elohim-doorway-app) leaks into orchestrator dispatch — net.sf.json.JSONNull truthiness defeats the jenkinsPath filter"
slug: "ci-orchestrator-graph-only-pipeline-dispatch-leak"
written: "2026-06-10"
author: "ci-failure-triage"
status: "backlog"
priority: "medium"
ci_status: triaged
fingerprints: [97d7fb9c085c]
jobs: [elohim-orchestrator]
relatedNodeIds: []
tags: [ci, elohim-orchestrator, dispatch, jenkinsPath, graph-only-pipeline, jsonnull-truthiness, cps-method-size, doorway-app]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1197/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1197/artifact/predicted-build-graph.json
  - genesis/orchestrator/Jenkinsfile
  - genesis/orchestrator/pipeline-registry.mjs
  - genesis/orchestrator/pipeline-registry.test.mjs
  - genesis/orchestrator/build-graph.groovy
  - doorway/doorway-app/build-manifest.json
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Graph-only pipeline leaks into dispatch — orchestrator UNSTABLE on every doorway-app-touching push

## The failure

```
97d7fb9c085c  elohim-orchestrator — red build, stage:elohim-doorway-app   (1185–1197, seen 4)
```

Build is **UNSTABLE** (harvester "red build" is its classifier token). The
signature, from #1197 (`No item named` is the verbatim Jenkins-core error):

```
▶️ Triggering elohim-doorway-app/dev [wait-for-result]...
⏭️ elohim-doorway-app: SKIPPED — Jenkins job not provisioned (No item named elohim-doorway-app/dev found)
WARNING: elohim-doorway-app: registered in strategy.mjs but Jenkins multibranch job missing — create it to restore full delivery
```

Occurrence evidence: seen 4, first_build 1185, last_build 1197 (job
elohim-orchestrator). Recurs on every dev push whose changeset touches
`doorway/doorway-app/**` — on this window, the native-content-graph-seam work
regenerating `doorway/doorway-app/src/app/generated/*.ts` (#1197 changeset line
`└─ doorway/doorway-app/src/app/generated/content-graph-view.ts`).

## Verdict

**Real — an orchestrator dispatch-filter bug, NOT the museum echo class and
NOT an operator-missing-job.** `elohim-doorway-app` is a **graph-only pipeline
by design**: its `build-manifest.json` intentionally omits `jenkinsPath` (it is
a build-graph node for change-detection / decision-matrix, not a dispatchable
Jenkins job). `pipeline-registry.test.mjs:34` pins the contract:
`assert.ok(!names.includes('elohim-doorway-app'), 'graph-only pipeline should
be excluded')`. The pure-JS `dispatchablePipelines` (pipeline-registry.mjs:73)
honors it with a robust guard: `typeof p.jenkinsPath === 'string' &&
p.jenkinsPath.length > 0`. The bug is that the **Jenkinsfile Groovy** dispatch
path does NOT replicate that robust guard, so the graph-only pipeline survives
the exclusion filter and is dispatched, hitting a job that (correctly) does not
exist.

## Root cause — net.sf.json.JSONNull is Groovy-truthy

The exclusion filter in `applyBuildGraphRouting` (Determine Build Plan stage)
is `genesis/orchestrator/Jenkinsfile:846`:

```groovy
graphPipelines.removeAll { name -> !getPipelineMetadata(name).jenkinsPath }
```

`getPipelineMetadata` (Jenkinsfile:65) reads from `env.PIPELINE_REGISTRY_JSON`,
which `runBuildGraph` populates by `writeJSON`-serializing the registry that
`build-graph.groovy:751-759` builds with `jenkinsPath: manifest.jenkinsPath`.
For doorway-app, `manifest.jenkinsPath` is absent → null at build-graph time.
But the round-trip is the trap: **`readJSON` yields `net.sf.json` objects, and
a null value deserializes as `net.sf.json.JSONNull.getInstance()` — a non-null
OBJECT, hence Groovy-truthy.** So `getPipelineMetadata('elohim-doorway-app')
.jenkinsPath` returns `JSONNull`, `!JSONNull` is `false`, and `removeAll` does
NOT remove it. (The `readJSON returns net.sf.json` fact is already noted at
Jenkinsfile:1654 for a JSONArray case — same library, same gotcha class.)

The same truthy-JSONNull then defeats the dispatch-site guard at
Jenkinsfile:620 (`if (!config.jenkinsPath) { error … }`), so instead of a clean
pre-dispatch error the orchestrator proceeds to `build(job:
"elohim-doorway-app/dev")` (Jenkinsfile:651), which throws
`hudson.AbortException: No item named …` → the soft-skip branch
(Jenkinsfile:694) → `recordPipelineResult` marks UNSTABLE (Jenkinsfile:583-584).

Evidenced end-to-end in #1197: predicted-build-graph.json `pipelines` array
contains `elohim-doorway-app` (it survived the filter); the Execute Builds log
shows `▶️ Triggering elohim-doorway-app/dev` (line 620 guard passed —
JSONNull truthy) then the `No item named` soft-skip. The plan summary even
prints `Plan: [elohim + elohim-storybook + elohim-doorway-app + …]`.

This is a **genuinely new recurring-trap class** (JSONNull surviving a
truthiness filter after a writeJSON/readJSON registry round-trip) — graduated
into the CI museum record (`…-museum.md`) rather than forked.

## Current decision

**BLOCKED — bounded fix specified, but landing it is gated behind the
orchestrator CPS helper-region refactor. Two unblock paths:**

**(a) The bounded in-tree fix (preferred root fix, gated).** Make the Groovy
dispatch path JSONNull-safe so it mirrors the JS `dispatchablePipelines`
contract. Minimal form: normalize `jenkinsPath` to a real String-or-null where
it enters `getPipelineMetadata` (return-map line 92), collapsing
`JSONNull`/`null`/`''`/the literal `'null'` string to `null`, e.g.
`jenkinsPath: (m.jenkinsPath?.toString()?.trim()) ?: null` plus a `== 'null'`
guard — so the line-846 filter, the line-853 force-include, and the line-620
dispatch guard all behave like the JS contract. **This fix is ~1-3 lines and is
verifiable by reading** (no Jenkins run needed — the mechanism is fully
determined and the JS contract is the proven oracle). **But ANY edit to
`genesis/orchestrator/Jenkinsfile` is currently REJECTED by the
`jenkinsfile-method-size.py` PreToolUse/pre-push hook**: the helper region above
`pipeline {}` is already ~46.3KB comment-stripped vs the hook's 12KB hard limit
(the hook measures the whole helper region as one unit; the empirical
per-method CPS cap that broke #1519/#1520 is the underlying constraint). So the
one-line fix cannot land until the helper region is refactored (bash bodies →
`scripts/ci/*.sh`, per the hook's own guidance) — a pipeline-architecture sprint
(>20-file class), out of a single triage run's scope. Carry this fix INTO that
refactor sprint.

**(b) Operator provisions the multibranch job (lighter, alternate unblock).**
If `elohim-doorway-app` is *meant* to be a real dispatchable pipeline (it has a
full `gate.projects` + `build-doorway-app` step and the warning text invites
"create it to restore full delivery"), the operator creates the
`elohim-doorway-app/dev` multibranch job AND adds `jenkinsPath` to its
build-manifest.json. Then the dispatch succeeds and the UNSTABLE clears — and
the JSONNull bug becomes moot for doorway-app specifically (though it stays
latent for the other graph-only pipeline, `elohim-compute`, which would re-trip
the instant its sources change). For that reason (b) is a point-fix and (a) is
the durable fix; the recommendation is (a)-in-the-CPS-sprint, with (b)
available if the operator wants doorway-app actually built+deployed in CI.

Blocker, recorded in the ledger (`97d7fb9c085c` → `status: blocked`): the
CPS helper-region refactor (for path a) OR an operator job-provisioning
decision (path b). No `triaged_at_build` (nothing landed). Recurrence tracks
doorway-app-touching pushes and is expected until one path resolves.

## Fix trail

- No code landed (the bounded fix is hook-blocked behind the CPS-region
  refactor; documented above and graduated to the museum).
- Diagnosis verified by reading: registry build-path
  (`build-graph.groovy:751-759`) + filter (`Jenkinsfile:846`) + dispatch guard
  (`Jenkinsfile:620`) + soft-skip (`Jenkinsfile:694`) + the JS oracle
  (`pipeline-registry.mjs:73` / `pipeline-registry.test.mjs:34`), grounded in
  #1197's predicted-build-graph.json + Execute Builds log.
- Latent sibling (same root, not separately fingerprinted): `elohim-compute` is
  the other graph-only pipeline (`pipeline-registry.test.mjs:35`); it will
  re-trip this exact UNSTABLE the first time a push touches its sources. The
  durable fix (a) closes both; the operator point-fix (b) does not.

- 2026-06-10: UNBLOCKED and LANDED — the jenkinsfile-method-size hook's helper-region
  aggregate check (the blocker) was redesigned to per-def measurement (the CPS unit is the
  single method, never the region: the orchestrator's ~46KB region of small defs compiles
  fine). Fix applied at getPipelineMetadata's registry branch: jenkinsPath normalized to
  real-nonblank-String-or-absent (JSONNull removed), mirroring pipeline-registry.mjs.
  Verdict: next doorway-app-touching wave should dispatch without the soft-skip UNSTABLE;
  harvester confirms by disappearance.
