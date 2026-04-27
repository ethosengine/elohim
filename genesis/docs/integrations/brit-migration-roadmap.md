# Brit Migration Roadmap

**Source spec:** `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md`
**Companion specs:** `elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md`, `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md`

The Jenkins → brit-attestation-producer migration is staged so each stage independently delivers value and can be rolled back without disturbing later stages.

## Stage 1a — Pre-push + orchestrator advisory (LANDED — this plan)

**Plan:** `genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md`

**Delivered:**
- `genesis/orchestrator/scripts/brit-helper.sh` — safe-fallback wrapper
- `.husky/pre-push` — advisory `brit verify` + `brit plan --since origin/dev`
- `genesis/orchestrator/Jenkinsfile` — advisory `Brit Plan` stage

**Behavior:** warn-only everywhere. No CI behavior change. Developers gain a 5-second local plan preview.

**Rollback:** revert the three commits.

## Stage 1b — ci-builder image rebuild (OPERATOR ACTION — separate repo)

**Repo:** `ee-jenkins-ci-builder` (or wherever `harbor.ethosengine.com/ethosengine/ci-builder:latest` is sourced from).

**Steps:**
1. Add brit submodule (or pinned-version tarball download) to the ci-builder Dockerfile build steps.
2. `RUN cargo build --release -p brit-cli -p brit-build-ref` during image build.
3. Place binaries on `$PATH` (e.g., `/usr/local/bin/brit` and `/usr/local/bin/brit-build-ref`). Note: the brit-cli binary is currently named `rakia` in the local build — install it as `brit` on the ci-builder image (symlink or rename) for consistency with how Stage 2 will reference it.
4. Push new ci-builder tag to Harbor (e.g., `ci-builder:1.x-brit`).
5. Update `genesis/orchestrator/Jenkinsfile` (line 702) and `genesis/Jenkinsfile` (line 123) image references to the new tag.

**Validation:** orchestrator's `Brit Plan (advisory)` stage now produces real plan output instead of `WARN: brit not installed`.

**Rollback:** revert ci-builder tag references in the two Jenkinsfiles.

## Stage 1c — Per-pipeline post.success attestation writes (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-1c-attestation-writes.md` (TO WRITE after Stage 1b stabilizes)

**Scope:** add `post { success { } }` blocks to one downstream pipeline (recommend: `elohim/holochain/Jenkinsfile`, the elohim-edge pipeline) that shell `brit-helper.sh build-ref build put ...` after each successful build stage. Notes accumulate in `refs/notes/brit/builds/<pipeline>:<step>`. Push notes refs at end of pipeline.

**Behavior:** still no CI dispatch behavior change; notes accumulate as advisory data.

**Validation:** after a few builds, `git fetch origin refs/notes/brit/builds/*:refs/notes/brit/builds/*` followed by `git notes --ref=refs/notes/brit/builds/elohim-edge:cargo-build-doorway list` shows accumulating notes.

**Rollback:** revert the post-block additions; existing notes are harmless.

## Stage 2 — Consume brit plan for dispatch (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-2-consume-plan.md` (TO WRITE after Stage 1c stabilizes 2 weeks)

**Scope:** orchestrator's planning stage replaced with `brit plan` shell-out; `env.PIPELINES_TO_RUN` derived from `BuildPlan.steps[].pipeline`. Downstream Jenkinsfiles' stage `when` blocks consult plan verdict instead of `params.STEPS` / `params.FORCE_BUILD`. Per-pipeline opt-in via env var for safe rollout.

**Behavior:** dispatch decisions become brit-driven. Manifest-only changes route to deploy steps narrowly (the originating-incident resolution).

**Rollback:** revert the consumption code; legacy `build-graph.groovy` still operational as fallback.

## Stage 3 — Retire legacy (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-3-retire-legacy.md` (TO WRITE after Stage 2 stabilizes 2 weeks across all pipelines)

**Scope:** delete `analyzePipelineRequirements`, `runBuildGraph`, `groupByDependencyLevel`, `propagateDependencies`, `archivePipelineBaselines`, `getKnownDivergences`, the `PIPELINES` map in orchestrator (currently DEPRECATED, advisory-only). Delete `pipeline-baselines.json` artifact archival. Delete `params.STEPS`, `params.FORCE_BUILD`, `params.FORCE_DEPLOY` from downstream Jenkinsfiles. Net: ~750 lines of Groovy removed.

**Behavior:** brit is sole source of truth.

**Rollback:** revert the deletion commits.

## Operator success criteria per stage

- **Stage 1a:** `.husky/pre-push` produces a `── brit advisory (Stage 1a — warn-only)` block on every push. Orchestrator log contains `Brit Plan (advisory)` stage output.
- **Stage 1b:** orchestrator's advisory stage produces real plan JSON (not `WARN: brit not installed`).
- **Stage 1c:** `git notes --ref=refs/notes/brit/builds/<pipeline>:<step> list` shows N entries after N successful builds of that step.
- **Stage 2:** for the originating incident scenario (manifest-only commit), `env.PIPELINES_TO_RUN = "elohim-edge"` only, total dispatch time ~90s instead of ~75min.
- **Stage 3:** `wc -l genesis/orchestrator/Jenkinsfile` shows ~750 lines (down from ~1532 after Stage 1a additions).
