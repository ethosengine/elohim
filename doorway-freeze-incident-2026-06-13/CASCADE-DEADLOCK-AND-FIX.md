# Cascade deadlock: the deploy that fixes alpha was blocked by alpha being down

**Date:** 2026-06-14 · **Branch:** `feat/frontend-eyes-sprint` (commit-only) · **Sibling of:** `README.md` (the doorway freeze itself)

## One-sentence fault

`elohim-app` is the only **waited-on** Level-0 pipeline in the orchestrator cascade, and its build can be driven to **FAILURE purely by the live target being down** (the `E2E Testing - Alpha Validation` stage opens with `timeout 60s curl https://alpha.elohim.host`) — a Level-0 FAILURE trips the orchestrator fail-fast, which never dispatches `elohim-edge` (Level 1+), the only pipeline that runs `kubectl apply`. So **a down alpha blocks the deploy that would fix alpha.**

## Evidence (orchestrator #1240, 2026-06-13 20:47 UTC, FAILURE)

- Plan: `[elohim + elohim-storybook + elohim-holochain] → elohim-edge → genesis`.
- Level 0 dispatched 3 pipelines. `elohim-storybook` + `elohim-holochain` are `longRunning` → fire-and-forget (`DISPATCHED`, outcome invisible). **`elohim-app` is NOT `longRunning` → waited-on → returned FAILURE.**
- `ERROR: Build(s) failed: elohim - Aborting` (orchestrator `Execute Builds`, level-failed throw). Post Actual Build Graph / Reconcile / Verify Deployment / Post-flight all skipped.
- **`elohim-edge` was never dispatched** (it lives in Level 1+). The self-healing/arc deploy is edge's `build-edge-image` → `deploy-manifests` (Levels 2-3), so it never reached the cluster.
- Corrections to the prior recap: the DNA/holochain pipeline has **no deploy stage** (edge deploys); edge was **never dispatched**, not aborted mid-flight; the exact app failing-stage is *inferred* (MCP can't resolve `elohim-app/dev`), but alpha was provably 503 at orchestrator preflight, so the shape holds.
- **Closing the E2E-alpha inference:** the app pipeline's only *fatal post-deploy* stage is `E2E Testing - Alpha Validation` (Verify-Holochain-Infra is echo-only; staging E2E is `when`-skipped on dev). So an observed `elohim-site-alpha` rollout ⟹ the failure was post-deploy ⟹ E2E-alpha is necessarily the gate. This rests on the operator observation that the site image rolled out. **If a future app build FAILs-and-aborts *upstream* of E2E (build/compile/Sonar), that's a different cause and this fix won't cover it.**

## The complete class of "alpha-down fails the cascade"

| Gate | Fatal? | Role |
|---|---|---|
| App `runE2ETests('alpha')` (`Jenkinsfile:1358`) | **YES → aborts cascade** | THE deadlock — Level 0, waited-on, blocks edge. |
| Genesis `Verify Target Health` `timeout 120s curl` (`genesis/Jenkinsfile:1435`) | YES → exit 124 | **Downstream** of edge. Killed genesis #1141/#1142. Not the primary deadlock. |
| Edge deploy (`kubectl apply`) | No — applies regardless of target health | **The fix carrier.** |
| Orchestrator post-flight / P2P / fed-smoke | No — all `catchError → UNSTABLE` already | Already advisory. |

## Fix (this commit) — Lever 2, surgical

`Jenkinsfile` `E2E Testing - Alpha Validation` stage: wrap `runE2ETests('alpha', …)` in `catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE')`. App goes **UNSTABLE not FAILURE** when alpha is down; `triggerPipeline` treats UNSTABLE as success (`success == result in [SUCCESS, UNSTABLE]`) → no level-abort → **edge dispatches and deploys**. App build/compile/Sonar failures upstream still hard-gate. Mirrors the orchestrator's own advisory post-flight gates.

**Why not 2a (`longRunning: true` on the app manifest):** considered, lower edit-risk, but makes *all* app failures (incl. real build breaks) cascade-invisible. 2b keeps build failures gating and makes only the live-target E2E advisory.

## What this fix does and does NOT do

- ✅ **Breaks the deadlock** — app E2E on a down alpha no longer aborts the cascade, so an edge-bearing cascade can dispatch and deploy while alpha is down.
- ⚠️ **This Jenkinsfile-only commit does NOT itself redeploy edge.** A natural landing of this commit maps to the *app* pipeline (+ genesis, which `dependsOn elohim`); **edge `dependsOn elohim-holochain`, not app**, so `propagateDependencies` won't auto-include it. To actually land the doorway/self-healing fixes on alpha, trigger an **edge-bearing cascade** (`[build:edge]` / `[build:all]`, or an edge-source change). Landing this commit alone *proves 2b works* (app E2E → UNSTABLE → no abort) but won't fix alpha by itself.
- ⚠️ **Does not make the first post-fix cascade green.** Genesis `Verify Target Health` (gate #2, downstream of edge) may still go red on exit-124 while the freshly-deployed doorway is mid-warmup. That gate *should* hold (you cannot seed a dead conductor) — it is downstream of the fix, not blocking it. A longer settle budget there is a separate, smaller decision (deferred).
- ⚠️ **Staging symmetry left untouched.** The `E2E Testing - Staging Validation` stage (`Jenkinsfile:1382`) has the same shape for `staging`/`staging-*` branches; not the current deadlock, left for a conscious follow-up.

## Lever 1 sequencing (the alpha cure rides this un-block)

The flap fixes are **already committed but undeployed**: the manifest carries `DOORWAY_WORKER_THREADS=4`, `DOORWAY_ZOME_CALL_TIMEOUT_MS`, startupProbe ft=24; the code dropped the dead `record_heartbeat`. **The running pod is still on `aa6debe6`** — they never deployed *because the cascade was deadlocked*. Correct order now:

1. This gate fix removes the abort. An **edge-bearing cascade** (`[build:edge]` / `[build:all]`) then dispatches edge → edge deploys the existing doorway fixes to alpha. (A Jenkinsfile-only landing won't pull edge in on its own — see "does NOT do" above.)
2. **Then** observe a settled pod under sustained load. The residual `warm_stream` scenario-corpus-replay wedge is corroborated by the freeze thread-dump (`Content:scenario-*` projecting at the moment of wedge) but that dump is the **pre-fix image** — it can't isolate the residual from the already-fixed cpu:1 mechanism. Diagnose eyes-on **before** writing any new `warm_stream` fix (do not pre-write against an unconfirmed RCA).

## Backlog (unchanged, surfaced again)

- Edge deploy lacks a `kubectl rollout status` wait → premature "success" + the reseed race that fails genesis at Verify-Target-Health.
- Genesis `Verify Target Health` 120s budget may be too short for a cold V8/Angular-bundle doorway start (10–60s) immediately after a deploy.
