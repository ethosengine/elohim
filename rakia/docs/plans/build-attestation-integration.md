# Build Attestation Integration

**Status:** Design
**Date:** 2026-04-13
**Depends on:** brit Phase 2a (`elohim/brit/docs/plans/phases/phase-2a-build-attestation-primitives.md`)
**Supersedes:** rakia's internal `build-state.json` baseline scheme

## Problem

Rakia today (and the Jenkins orchestrator it will replace) computes staleness by comparing the current commit against a persisted `lastSuccessfulCommit` in `build-state.json`. When an executor fails before builds actually run:

- If the state was saved during planning, it advances past unbuilt changes → future runs see empty diffs → nothing rebuilds (we lived through this as "baseline leapfrog")
- If the state wasn't saved, every run cold-starts → full rebuild cascade
- Deployment state is invisible — rakia knows what it triggered, not what's actually live

The root cause is that rakia owns the state. If rakia dies, the state is corrupt or lost.

## Solution

Rakia stops owning build state. It consumes brit's attestation refs (Phase 7) as the sole source of truth for "what was built" and "what is deployed". Rakia's only job becomes: compare current inputs to the attestation graph, decide what needs work, dispatch it, and trigger attestation writes when work completes.

**The state lives with the artifact, not with rakia.**

## Change Detection — New Model

For each step in the build graph:

```
current_inputs_hash = hash(files matching step.inputs.sources at HEAD)
latest_build = brit build-ref build get --step {stepName}
stale = (latest_build is None) OR (latest_build.inputsHash != current_inputs_hash)
```

That's the entire algorithm. No per-pipeline baseline. No `build-state.json`. No `GIT_PREVIOUS_SUCCESSFUL_COMMIT` dance. The ref advances only when a BuildAttestation is written, which only happens after a real build completes.

**Leapfrog is structurally impossible**: the ref cannot advance past an unbuilt commit because no attestation exists for that commit.

## Deployment State — New Capability

Rakia gains a question it could never answer before: **what is actually deployed?**

```
alpha_state = brit build-ref deploy get --step {stepName} --env alpha
needs_deploy = (alpha_state is None)
             OR (alpha_state.artifactCid != latest_build.outputCid)
             OR (alpha_state.healthStatus != "healthy")
             OR (now - alpha_state.attestedAt > alpha_state.livenessTtlSec)
```

Health-check pods running in each environment write DeployAttestations on a schedule (or via webhook on pod restart). Rakia reads these refs to determine whether a fresh build needs to be promoted.

## Reach Gating — New Capability

Promotion from dev to staging to prod becomes governance-aware:

```
reach = brit build-ref reach compute --step {stepName}
can_promote_to_staging = (reach >= "community")
can_promote_to_prod    = (reach >= "public")
```

Reach is derived from the sum of attestations (build + deploys + validations) against the AppManifest's promotion rules. A solo-dev artifact with one build attestation and no validation gets `self` reach — it can be installed locally but won't auto-promote. The same artifact after a SonarQube scan, a Trivy scan, and a successful test-suite attestation can earn `community` reach without any additional peer co-signing.

## Succession and Dispatch

When rakia determines a step is stale, it consults the manifest's `stewardshipCollective` to decide who should build:

1. Query the collective's members, ordered by `succeeds` relationships (priority order)
2. Dispatch to the highest-priority available peer
3. If no response within timeout, fall through to next
4. Any steward in the collective can produce a valid BuildAttestation — the rakia-peer layer handles which peer is currently the leader

For Stage 1, "the collective" is typically just the Jenkins server. The succession machinery is designed in but trivially satisfied. Stage 2 activates real multi-peer dispatch.

## What Disappears from Rakia

Lines of code that go away:

- `build-state.json` read/write/archive (entire lifecycle)
- `lastSuccessfulCommit` tracking
- `buildProcessHashes` map (replaced by attestation `inputsHash`)
- Cold-start logic (no cold-start possible — refs either exist or don't)
- `saveBuildState` timing logic (save during planning vs. execute)
- The entire concept of "build graph baseline"

Rakia's `rakia affected` / `rakia plan` commands become thin wrappers over brit ref reads.

## Integration with Existing Rakia Phases

| Rakia Stage | Attestation Dependency |
|---|---|
| Stage 0 (current) | Jenkins writes build refs via `brit build-ref build put` after successful builds. Change detection reads them. Zero rakia code changes yet — this is a brit CLI integration. |
| Stage 1 (Root) | Rakia-cli replaces the orchestrator's change detection entirely. Reads brit refs, computes affected steps, returns the work list. Deployment verification pods deployed to each env. |
| Stage 2 (Canopy) | Rakia-peer dispatches builds to steward peers via libp2p. Each peer writes its own BuildAttestation. Reach thresholds enforce N-of-M diverse attestation before promotion. |
| Stage 3 (Forest) | Full economic flow — attestations emit REA economic events, stewardship credit accumulates, succession is live governance. |

## Migration from `build-state.json`

The current rakia prototype writes `build-state.json`. Migration is a one-time translation:

1. For each step in the current `build-state.json.stepStates`, read `lastBuiltCommit`
2. Synthesize a bootstrap `BuildAttestationContentNode` with `agentId = "migration"`, `builtAt = now`, `outputCid = "unknown"`, marked with a migration flag
3. Write to brit refs
4. Delete `build-state.json`

Future runs use brit refs exclusively. The synthetic attestations get replaced with real ones on the next successful build.

## Acceptance Criteria

- `rakia affected` returns identical results to the current `build-state.json`-based algorithm for non-edge-case inputs
- Leapfrog scenario: kill the executor mid-Execute-Builds → next run correctly identifies all unbuilt work (the bug we hit 2026-04-13)
- Cold-start scenario: empty refs, full changeset → rakia marks everything stale and dispatches all steps
- Deployment verification: artifact built but not deployed → rakia reports "deploy needed" for that env
- Reach gating: artifact at `self` reach → rakia refuses to auto-promote to staging without additional attestations

## Why This Integration, Not a Rakia-Native State System

We considered keeping state in rakia with improved persistence (git-backed, DHT-backed, etc.). That leads to rakia reimplementing what brit already needs to do for commit attestations. The primitives are the same: content-addressed claims with pillar coupling, cached in refs, durable on the DHT. Shipping them in brit means:

- Every brit repo (not just rakia-using ones) gets build legibility
- The primitives compose with brit's existing commit-attestation surface
- Rakia stays a thin consumer focused on build graph mechanics
- The engine/app boundary stays clean on both sides

## Open Question Deferred

**How rakia exposes deployment state to the elohim-app UI.** The current UI has no concept of "what commit is deployed to alpha". With DeployAttestations, this becomes queryable. But the UI integration is a separate design. For this phase, `rakia status` CLI output is sufficient.
