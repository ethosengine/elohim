---
id: "backlog-ci-app-wasm-cache-core-sha-pin-blocks-nondna-deploys"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "App build pulls wasm-cache-core by current-commit version → every NON-DNA commit fails to deploy (chronic app-pipeline red; blocks all app-side validation)"
slug: "ci-app-wasm-cache-core-sha-pin-blocks-nondna-deploys"
written: "2026-06-08"
author: "agentic-developer (overnight shift)"
status: "wip"
priority: "high"
ci_status: blocked
jobs: [elohim, elohim-edge]
tags: [ci, app, edge, wasm-cache-core, build-graph, artifact-versioning, deploy, chronic, escalation]
cites:
  - Jenkinsfile
  - elohim/holochain/dna/Jenkinsfile
---

# App build pulls wasm-cache-core by current-commit version → non-DNA commits can't deploy

## Symptom (chronic — the `elohim` app pipeline window is `FUFFFUU`)
- App build (`elohim/dev`, e.g. #1512): `oras pull harbor…/elohim-wasm-cache-core:1.0.0-dev-<sha> — not found`,
  then the elohim-service vitest suite fails: `Cannot find module '/wasm/elohim-cache-core/elohim_cache_core.js'`
  → build UNSTABLE.
- Edge build (`elohim-edge/dev` #1049) then reports `Images Pushed: Skipped`, `hApp Available: No`,
  `Edge Node Healthy: Unknown` — **it holds its deploy pending the app build**, so NOTHING deploys to alpha.

## Root cause (confirmed)
- The app build pulls the WASM by the **current commit's** version: `Jenkinsfile:612`
  `oras pull harbor.ethosengine.com/ethosengine/elohim-wasm-cache-core:${happVersion}` (happVersion =
  `1.0.0-dev-<commit-sha>`).
- But the `elohim-wasm-cache-core` image is built + pushed ONLY by the **DNA pipeline**
  (`elohim/holochain/dna/Jenkinsfile`), which the orchestrator triggers **only when
  `elohim/holochain/dna/` or `elohim/elohim-cache-core/` change** (`dna/Jenkinsfile:5`). Its recent window is
  `ABORTED, SUCCESS` — it rarely runs.
- So for ANY commit that does **not** touch the DNA/cache-core (the overwhelming majority — app, doorway,
  storage, a2o, genesis changes), the DNA pipeline does not run → no `wasm-cache-core:1.0.0-dev-<that-sha>`
  exists → the app build's sha-pinned `oras pull` 404s → the app's WASM is missing → vitest fails → app
  UNSTABLE → edge skips deploy → **the change never reaches alpha.**
- The app build's existing 404 fallback (Jenkinsfile ~619-633) only warns + continues; it does NOT recover a
  usable WASM, so the test still hard-fails on the missing module.

## Impact
This is the chronic cause of the `elohim` pipeline being red on nearly every push, and it **blocks all
app-side validation** — e.g. tonight's EPR Slice-0/1 app changes (raw-node viewer, PathViewer composite
renderer, Open-in-pillar) are committed + locally green but could NOT be CI-validated because the app never
deployed (genesis a2o tested the old deploy). The doorway-side EPR (edge) is also blocked because edge holds
its push on the app build.

## Fix direction (escalation — operator/CI-owned; cross-cutting, near root-Jenkinsfile CPS limit, unvalidatable without a push)
The app/edge builds must resolve the wasm-cache-core to the **last DNA-built (baseline) version**, not the
current commit's happVersion. Options:
1. Have the DNA pipeline ALSO push a moving tag (e.g. `:latest-dev` or `:<branch>`), and have the app build
   pull that when the sha-pinned tag is absent (or always).
2. Resolve the wasm-cache-core version via the orchestrator's **baseline** mechanism (the last successful DNA
   build's sha), the same way other cross-pipeline artifact baselines are resolved — so a non-DNA commit reuses
   the last-built WASM instead of demanding a fresh sha-tagged one.
3. Make the app build's 404 fallback actually recover: pull the latest available wasm-cache-core tag.

Not landed this shift: it's a chronic, cross-cutting build-graph/artifact-versioning change in the root
Jenkinsfile (near the 64KB CPS limit), separate from this shift's EPR+seeding scope, and unvalidatable without
a push + an iteration cycle. Escalated with the confirmed root cause so the operator can fix it with the
version-resolution context + CI validation. **This is the top blocker to validating any app-side change.**

## Diagnosis provenance
Overnight agentic-developer shift 2026-06-08 — genesis #1107 (built c762aae4) deployed nothing because the
app build #1512 failed on the missing wasm-cache-core; Jenkinsfile:612 + dna/Jenkinsfile:5 confirm the
sha-pin-vs-DNA-only-build mismatch.
