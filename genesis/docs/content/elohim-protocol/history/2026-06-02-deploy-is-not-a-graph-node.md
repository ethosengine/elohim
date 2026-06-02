---
title: "History/ADR: Deploy is not a graph node — the 2026-04-27 whole-matrix rebuild"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [orchestrator, ci, deploy, brit, attestation, baseline-drift]
# DISTILLS the 2026-04-27 incident analysis + the two-track fix. Both fixes landed
# (DEPLOY_ONLY param across pipelines; elohim/brit + elohim/rakia crates). Raw bodies
# retire to git.
distills:
  - .claude/archive/2026-05-15/genesis/docs/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md
  - .claude/archive/2026-05-15/genesis/docs/plans/2026-04-27-orchestrator-deploy-only-param-kickoff-prompt.md
  - .claude/archive/2026-05-15/genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md
# Bidirectional: the canonical orchestrator surface this gotcha points back to.
canonical:
  - ../../../../orchestrator/README.md   # genesis/orchestrator/README.md — dispatch + NOT_BUILT semantics
memory_anchors:
  - project_orchestrator_predictive_vision
  - feedback_orchestrator_abort_baseline_rollback
  - project_pre_dispatch_hard_fail_post_dispatch_unstable
  - project_redeploy_the_substrate
---

# Deploy is not a graph node — the 2026-04-27 whole-matrix rebuild (commit `edfe5c57`)

> **Hot-context pointer (the one sentence to remember):**
> A deploy stage that lives *inside* a pipeline (not as a manifest node with its own inputs/outputs)
> cannot be selectively dispatched — so any change that re-baselines the graph drags the whole matrix
> through it. If you are about to "just `kubectl apply`" and the orchestrator wants to rebuild the
> world, you have hit this.

A one-line k8s manifest change (`storageClassName: openebs-hostpath` on the alpha mongodb PVC)
re-triggered the *entire* build matrix — DNA pack + ~50min DNA integration + edge + sophia + app —
when all that was actually needed was `kubectl apply`. The Harbor image tags were already known-good.

## Three concurrent root causes (all worth remembering)

1. **Baseline drift** — `lastSuccessfulCommit` reached back across prior shifts whose work landed but
   never produced a clean run, so the graph honestly reported "lots changed since last green."
   (This is the same mechanism `feedback_orchestrator_abort_baseline_rollback` records.)
2. **Deploy isn't a graph node** — `Deploy Edge Node - Alpha` is a Jenkins *stage inside* `elohim-edge`,
   not a manifest node with its own inputs/outputs. There is no way to ask the graph to "just deploy"
   because deploy isn't addressable.
3. **Downstreams have no internal change-detection** — the orchestrator passed `FORCE_BUILD: true`
   unconditionally; once dispatched, every stage ran.

## The two-track fix (both shipped)

- **Tactical:** a `DEPLOY_ONLY` boolean param across all pipelines — skips every build stage and goes
  straight to `Deploy …`, reading image tags from `environments/{env}.env`. Turns a ~75min rebuild
  into a ~5min redeploy when only manifests/RBAC/storageClass changed. Landed; present across the
  pipeline set.
- **Strategic:** make Jenkins a thin **attestation producer / contract executor** — `brit plan --since`
  computes dispatch, `brit build-ref … put` writes attestations, `brit verify` gates pre-push; the
  bespoke `build-graph.groovy` change-detection retires over a 3-stage migration
  (keep-as-fallback → cold-start-fallback → retired). Landed in part: `elohim/brit`, `elohim/rakia`
  crates + `genesis/orchestrator/scripts/brit-helper.sh` exist; `build-graph.groovy` retained as
  fallback per the staged plan.

## Why we turned

The bespoke Groovy change-detection (1500+ lines) could not model deploy as a first-class node and
could not survive baseline drift. Moving the decision to content-addressed build attestations makes
"what actually changed since the last *green* run" the authoritative question, and makes deploy a real
node with inputs (image tags) and outputs (a DeployAttestation).

## Watch-out for future planners

Do **not** add deploy logic as another inline stage and call it addressable — it is not, until it is a
manifest node. When only manifests/RBAC/storageClass change, reach for `DEPLOY_ONLY`, not a full
dispatch. And remember the baseline-drift coupling: a "lots changed" report after a quiet shift is
usually baseline drift, not real work — confirm against the last *green* commit, not the last *landed*
one.

## Bidirectional links

- **This record → canonical:** [orchestrator README](../../../../orchestrator/README.md) (dispatch + NOT_BUILT semantics) — see the CI/orchestrator-anti-patterns museum record (`2026-06-02-ci-orchestrator-recurring-anti-patterns-museum`) for the frequency-ranked sibling lessons.
- **Distilled-from (raw bodies in git history):** Jenkins-as-brit-attestation-producer design, DEPLOY_ONLY-param kickoff, Jenkins stage-1a brit advisory (linked in frontmatter).
