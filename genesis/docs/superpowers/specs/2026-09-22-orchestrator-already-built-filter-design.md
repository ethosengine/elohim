---
title: The already-built dispatch filter — a pipeline is skipped only against its OWN baseline, never the frozen global one
id: orchestrator-already-built-filter-design
status: Active
class: architecture
serves: dev-system-equilibrium
date: 2026-09-22
context-tier: disclosed
steward: agent:orchestrator@claude-sonnet-5
graduation-trigger: the filter has skipped at least one further frozen-baseline wave in production with no false-skip (a producer a survivor needed was dropped) or false-dispatch (a pipeline with no changed watched input was rebuilt) incident since #1888; contested if a future wave shows rule (a)-(g) missed a case they don't already cover
cites:
  - genesis/orchestrator/README.md
  - "ci-orchestrator-recurring-anti-patterns-museum | History/ADR: CI / orchestrator recurring anti-patterns | sha256:d7703f837b3425f6 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md"
---

# The already-built dispatch filter

## 1. The problem: the frozen global baseline over-builds every pipeline (2026-09-22)

The plan computes ONE changeset, `git diff <__global__>..HEAD`, but `archivePipelineBaselines` deliberately refuses to advance `__global__` on a red orchestrator run while `recordPipelineResult` still advances each per-pipeline baseline on a non-red downstream. A streak of red orchestrator runs therefore freezes the diff base while the per-pipeline record keeps moving, and every later run re-diffs a range that is already built. Measured on #1888 (WEBHOOK, HEAD `7d27b04c`): `__global__: 82dcb5c8` (frozen — #1884–#1887 all died on the 4h timeout) beside `elohim-holochain: 86e7c220`, and the ~53-minute DNA pipeline was dispatched with `FORCE_BUILD=true` although `86e7c220..7d27b04c` touches nothing under `elohim/holochain`. `applyAlreadyBuiltFilter` now closes it on ANY trigger, in three phases with one home each:

## 2. The three-phase mechanism

1. `timer-dispatch.mjs groups` — **git and manifest reads only, no matching.** For each pipeline whose own full-40-hex baseline is a strict *descendant* of `__global__` and resolvable in the checkout, it emits `git diff --name-only <B_P>..HEAD` grouped by baseline sha, plus that baseline's provenance.
2. `walkNarrowGroups` (Groovy) — re-invokes **`build-graph.groovy::walkBuildGraph`, the same walker the plan stage used**, once per distinct baseline. Glob semantics, `buildProcess` hashing, `manualOnly` exclusion and dependency propagation are therefore identical *by construction*. There is deliberately **no second matcher**: an earlier cut re-implemented matching in JavaScript over picomatch and got two things wrong at once — picomatch excludes dot segments where Groovy's `matchesGlob()` includes them (a changed `doorway/doorway-service/src/.hidden.rs` selected edge in the plan and vanished from the narrow walk, so edge was suppressed *with changed source*), and picomatch lives in `node_modules`, which this pipeline's clean checkout never installs, so every non-exact walk would have thrown into the silent fall-through. Nothing on the planning module's import graph resolves outside the Node standard library; a test walks that graph and fails if it ever does.
3. `timer-dispatch.mjs decide` — a pipeline is skipped only when **that Groovy result excludes it**, and only after the producer-readiness rule below. A `[build:*]` tag always overrides, and genesis auto-include still runs after the filter.

`walkBuildGraph`'s `loadBuildState()` re-copies the previous build's `build-state.json` over the file `runBuildGraph` just saved, so `walkNarrowGroups` restores the authoritative bytes from `env.BUILD_STATE_JSON` in a `finally` — otherwise the post-Execute rewrite would archive the old state (#1844's shape).

## 3. What a baseline actually proves — provenance analysis

> **The skip line names what the baseline actually proves — and a producer anything still needs is never dropped.** Two provenances hide behind one number in `pipeline-baselines.json`: `recordPipelineResult` advances a baseline after a SUCCESS/UNSTABLE result and holds it on ABORTED/FAILURE (verdict-backed), but a `longRunning: true` manifest makes `triggerPipeline` fire-and-forget (`shouldWait = !(config.longRunning)`, Jenkinsfile:750 → `dispatched: true`, :622-628 → the optimistic advance, :521-529), so for `elohim-holochain`, `elohim-steward` and `elohim-storybook` (the pipeline name `app/elohim-library/build-manifest.json` actually declares — `elohim-library` itself names no dispatchable pipeline) the baseline records only that the build was *dispatched* — it may still be running, or have gone red. That matters beyond honesty: `groupByDependencyLevel` orders only *selected* pipelines and `needsDetachedDependencyBarrier` waits only for a *selected* producer, so an absent dependency reads as satisfied — drop DNA while keeping edge and edge fetches the floating `dev-latest` hApp tag (`elohim/holochain/Jenkinsfile:109`), which need not be the bytes DNA's baseline names. So a dispatch-only producer that **any surviving pipeline reaches through `dependsOn`** is KEPT. Survivors are traversal *roots* and traversal continues *through* skipped intermediates: a skipped edge contributes no fresh artifact binding either, so `app → edge → DNA` still keeps DNA.
>
> **There is no controller-evidence exception, and the filter holds no controller client.** An earlier cut allowed the skip when `lastSuccessfulBuild` reported SUCCESS at exactly `B_P`. That is unsound in this repo's own pipeline: `dev-latest` is a *floating* tag that the DNA job overwrites (`elohim/holochain/dna/Jenkinsfile:1057`) **before** its later publication steps, and other non-main branches publish it too. DNA #10 can succeed at `B_P`, #11 can overwrite the tag with different bytes and then fail or abort, and `lastSuccessfulBuild` still returns #10 — so the probe would accept #10's sha and drop DNA while edge fetches #11's bytes. A historical build query cannot establish what a floating tag names *now*; only an immutable artifact reference handed to the consumer could, and when one exists it belongs in the dispatch parameters, not here.
>
> **What this costs in practice:** because `elohim-edge` declares `dependsOn: ['elohim-holochain', …]`, the #1888 DNA dispatch is *not* eliminated on a run where edge itself is selected. The filter drops it on a wave where nothing surviving reaches it — which is the common frozen-baseline shape, and the #1886 timer shape. Skips of verdict-backed pipelines (`elohim-epr` in #1888) are unaffected.
>
> The reason is rendered from the provenance — `built green`, or `dispatched (longRunning: verdict never recorded), nothing surviving needs it` — and a pipeline whose provenance cannot be determined at all (no manifest in the checkout, e.g. `elohim-sophia` in its uninitialized submodule) is dispatched rather than skipped under a guess.

## 4. Validation

Every uncertainty dispatches: a short/missing/unparseable baseline, a missing `__global__`, an unresolvable commit, a non-ancestor, any git failure, a group the walker did not answer for, a non-zero exit from either phase, or a decision that fails validation. The decision is *validated* before it is trusted, as an EXACT PARTITION of the planned set: every member of both lists must be a string naming a planned pipeline, no name may repeat within or across the lists, and their union must cover the planned set exactly. Membership alone is not enough — for planned `["DNA","edge"]`, `{"dispatch":[],"skipped":["invented","invented"]}` would empty the wave and `{"dispatch":["edge"],"skipped":["edge"]}` would silently lose DNA. `timer-dispatch.mjs` self-checks the same predicate and exits non-zero on violation, so the rc guard and the decoder are independent barriers rather than one. Jenkins interruption is re-thrown, never swallowed.
