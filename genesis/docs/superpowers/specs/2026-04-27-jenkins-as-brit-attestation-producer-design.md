# Jenkins as Brit-Attestation Producer — Iterative Improvement Design

**Date:** 2026-04-27
**Status:** Approved (brainstorm)
**Author:** Matthew Dowell + Claude Opus 4.7
**Companion specs:** `elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md`, `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md`

## TL;DR

The Elohim monorepo's Jenkins orchestrator (`genesis/orchestrator/Jenkinsfile`, `build-graph.groovy`, `pipeline-baselines.json`, `build-state.json` + per-pipeline downstream Jenkinsfiles) shrinks to a thin attestation producer + thin contract executor. It calls `brit plan --since` to compute dispatch, shells `brit build-ref ... put` after each successful step to write attestations into `refs/notes/brit/`, and gates pre-push with `brit verify`. The bespoke change-detection (1500+ lines of Groovy across orchestrator + downstreams) retires over a three-stage migration. This spec specifies the Jenkins-side wire-level changes, the migration order, what gets retired and when, and the originating incident's resolution path.

## 1. Problem

The 2026-04-27 incident: a one-line manifest change (`storageClassName: openebs-hostpath`, commit `edfe5c57`) triggered the entire build matrix. Diagnostic across orchestrator + downstream Jenkinsfiles surfaced three concurrent root causes (full detail in the brit harvest spec §1):

1. Baseline drift — `lastSuccessfulCommit` reached back across multiple prior shifts' work that landed but never produced a clean run; graph correctly said "lots changed since last green."
2. Deploy isn't a graph node — `Deploy Edge Node - Alpha` is a Jenkins stage inside `elohim-edge`, not a manifest node with its own inputs/outputs.
3. Downstream pipelines have no internal change detection — orchestrator passes `FORCE_BUILD: true` unconditionally; once dispatched, every stage runs.

The fix shape — per the harvest specs — is to move all three concerns to brit + rakia primitives. Jenkins becomes a thin executor of contracts brit produces. This spec details the Jenkins-side cuts, additions, and migration sequencing.

## 2. The current Jenkins surface (what changes vs. what stays)

### 2.1 Stays

- **Pipeline orchestration mechanics.** The `pipeline { ... }` blocks, `kubernetes` agent declarations, `triggerPipeline` mechanism, parallel-execution via `groupByDependencyLevel`, milestone/abort behavior, post-build artifact archiving — all remain. Jenkins is still the executor; only its decision-making shrinks.
- **Stage-level shell-out patterns.** `container('builder') { sh '...' }` blocks stay. The build commands themselves don't change.
- **Health check mechanics.** Post-deploy `runP2PValidation`, `checkHealth`, version endpoint queries — stay; they feed DeployAttestation writes.
- **Webhook trigger.** GitHub webhook → orchestrator → dispatches downstream — pattern unchanged.
- **Credentials, Harbor push, registry login.** Unchanged.

### 2.2 Changes

- **Change-detection logic.** `build-graph.groovy` walking + `analyzePipelineRequirements` advisory + `pipeline-baselines.json` per-pipeline tracking → all replaced by `brit plan --since refs/notes/brit/build-baselines/__global__`.
- **Dispatch decision.** `env.PIPELINES_TO_RUN = graphPipelines.join(',')` → derived from `BuildPlan.steps[].pipeline` extracted from `brit plan` output.
- **Per-step gating.** Downstream Jenkinsfile stage `when` blocks based on `params.STEPS`, `params.FORCE_BUILD`, etc. → `when { expression { brit_plan.steps[stage_name].verdict == 'build' } }`. The plan is authoritative.
- **Post-step bookkeeping.** `pipelineBaselines[name] = env.GIT_COMMIT_FULL` after each successful build → `sh "brit build-ref build put --step <qualified-name> --inputs-hash <hash> --output-cid <cid> --hardware-profile <profile>"` after each stage.

### 2.3 Retires (three-stage migration)

| Artifact | Stage 1 | Stage 2 | Stage 3 |
|---|---|---|---|
| `pipeline-baselines.json` | Keep + write attestations alongside | Keep as fallback for cold-start | Retired |
| `build-state.json` | Keep | Keep as fallback | Retired |
| `build-graph.groovy` | Keep, advisory only | Keep as fallback | Retired (replaced by `brit plan` shell-out) |
| `analyzePipelineRequirements` (already deprecated) | Already advisory; no change | Already advisory; no change | Retired with build-graph.groovy |
| `getKnownDivergences` allowlist | Keep | Keep | Retired |
| `PIPELINES` map (legacy, marked DEPRECATED) | Keep, advisory only | Keep, advisory only | Retired (manifests are sole source) |
| Per-pipeline `STEPS` / `FORCE_BUILD` params | Keep, default `all` | Override based on plan verdict | Retired (plan verdict is sole gate) |
| Per-pipeline `FORCE_DEPLOY` param | Keep | Override based on plan verdict for deploy nodes | Retired |
| Hardcoded `triggerPipeline` params (`FORCE_BUILD: true`) | No change | Conditional based on plan | Pass plan CID instead |

## 3. Wire-level changes per pipeline

### 3.1 Orchestrator (`genesis/orchestrator/Jenkinsfile`)

**Stage 1 (additive, behavior-preserving):**

```groovy
stage('Compute brit Plan (advisory)') {
    steps {
        container('builder') {
            script {
                sh '''
                    brit verify || echo "WARN: brit verify reported issues (advisory in stage 1)"
                    brit plan --since refs/notes/brit/build-baselines/__global__ \\
                              --registry \\
                              --output /workspace/brit-plan.json \\
                              || echo "WARN: brit plan failed (advisory in stage 1)"
                '''
                if (fileExists('/workspace/brit-plan.json')) {
                    archiveArtifacts artifacts: 'brit-plan.json', fingerprint: true
                    def britPlan = readJSON file: '/workspace/brit-plan.json'
                    echo "ADVISORY: brit plan would dispatch: ${britPlan.steps.findAll { it.verdict in ['build', 'deploy'] }.collect { it.qualifiedName }.join(', ')}"
                    echo "ADVISORY: legacy graph dispatched: ${env.PIPELINES_TO_RUN}"
                }
            }
        }
    }
}
```

Runs after the existing planning stages. Compares advisory to actual. Surfaces divergence in build log without affecting dispatch.

**Stage 2 (consume):**

The orchestrator's planning stage is replaced with `brit plan` shell-out. Dispatch is derived from BuildPlan:

```groovy
stage('Plan via brit') {
    steps {
        container('builder') {
            script {
                sh '''
                    brit verify --strict || error "brit verify failed; investigate"
                    brit plan --since refs/notes/brit/build-baselines/__global__ \\
                              --registry \\
                              --output /workspace/brit-plan.json
                '''
                def britPlan = readJSON file: '/workspace/brit-plan.json'
                
                // Group steps by pipeline (derived from qualifiedName "<pipeline>:<step>")
                def pipelinesToRun = britPlan.steps
                    .findAll { it.verdict in ['build', 'deploy'] }
                    .collect { it.qualifiedName.split(':')[0] }
                    .unique()
                
                env.PIPELINES_TO_RUN = pipelinesToRun.join(',')
                env.BRIT_PLAN_CID = britPlan.cid  // for downstream propagation
                archiveArtifacts artifacts: 'brit-plan.json', fingerprint: true
            }
        }
    }
}
```

`triggerPipeline` is updated to pass `BRIT_PLAN_CID` as a build parameter so downstreams read the same plan instead of recomputing. (Recomputation should be byte-identical for deterministic plans, but fetching by CID avoids the network round-trip.)

**Stage 3 (authoritative):** Delete `analyzePipelineRequirements`, `runBuildGraph`, `groupByDependencyLevel`, `propagateDependencies`, `archivePipelineBaselines`, `getKnownDivergences`, the entire `PIPELINES` map. The orchestrator is ~400 lines (down from ~1500) consisting of: Checkout → Plan via brit → Execute Builds (driven by plan) → Verify (post) → Archive.

### 3.2 Downstream pipelines (`elohim/holochain/Jenkinsfile`, `elohim/holochain/dna/Jenkinsfile`, root `Jenkinsfile`, `sophia.Jenkinsfile`, `genesis/Jenkinsfile`)

**Stage 1 (additive):**

After each successful build stage, write a BuildAttestation:

```groovy
stage('Build Doorway') {
    steps {
        container('builder') {
            script {
                sh 'cargo build --release ...'  // existing build logic unchanged
            }
        }
    }
    post {
        success {
            container('builder') {
                script {
                    def imageDigest = sh(returnStdout: true, script: '''
                        docker manifest inspect harbor.ethosengine.com/ethosengine/elohim-doorway:1.0.0-dev-${GIT_COMMIT_SHORT} \\
                          | jq -r '.config.digest'
                    ''').trim()
                    sh """
                        brit build-ref build put \\
                            --step elohim-edge:cargo-build-doorway \\
                            --inputs-hash \$(brit fingerprint --step elohim-edge:cargo-build-doorway --commit ${env.GIT_COMMIT_FULL}) \\
                            --output-cid ${imageDigest} \\
                            --output-kind container-image \\
                            --output-repo ethosengine/elohim-doorway \\
                            --hardware-profile-from-env \\
                            --trigger webhook
                    """
                }
            }
        }
    }
}
```

`brit fingerprint --step ... --commit ...` is an existing brit-cli command; computes the canonical inputsHash by reading the manifest and the commit's tree. `brit build-ref build put` is a Phase 2A primitive (already designed; see `elohim/brit/docs/plans/phases/phase-2a-build-attestation-primitives.md`).

The post.success block is the only addition. Build stages themselves are unchanged. Notes accumulate. Stage 1 is purely additive — no existing behavior changes.

**Stage 2 (consume):**

Stage `when` blocks consult the plan instead of `params.STEPS`:

```groovy
stage('Build Doorway') {
    when {
        expression {
            def plan = readJSON file: '/workspace/brit-plan.json'
            def step = plan.steps.find { it.qualifiedName == 'elohim-edge:cargo-build-doorway' }
            return step?.verdict == 'build'
        }
    }
    steps { /* unchanged */ }
    post { success { /* attestation write, unchanged from stage 1 */ } }
}
```

Each pipeline either fetches the plan from the orchestrator's archived artifact or recomputes via `brit plan`. The `BRIT_PLAN_CID` parameter from orchestrator carries the reference.

Deploy stages get the same treatment. `Deploy Edge Node - Alpha` runs only when its corresponding deploy node in the plan has `verdict: deploy`. The current `branch 'dev'` unconditional + `FORCE_DEPLOY` `anyOf` is replaced with plan-verdict gating.

**Stage 3 (authoritative):**

Delete `params.STEPS`, `params.FORCE_BUILD`, `params.FORCE_DEPLOY`. Delete `shouldRunStep`, `params.SKIP_LEGACY_IMAGES`, `params.DEPLOY_STRATEGY`. Each Jenkinsfile drops to ~50% of current size. Stage `when` blocks are uniform: `expression { plan.steps.find { ... }?.verdict in ['build', 'deploy'] }`.

### 3.3 Pre-push hook (`.husky/pre-push`)

**Stage 1 (additive):**

```bash
# Existing: per-project quality gates auto-detected by changed paths
# ... existing logic unchanged ...

# NEW (advisory): brit verify warns but doesn't fail
brit verify || echo "WARN: brit verify reported issues — advisory until stage 2"
```

**Stage 2 (consume):**

```bash
# Existing quality gates unchanged

# NEW: brit verify is now load-bearing
brit verify || exit 1

# NEW: plan computation against origin/dev to predict CI behavior
brit plan --since origin/dev --output /tmp/predicted-plan.json
node genesis/orchestrator/scripts/print-plan-summary.mjs /tmp/predicted-plan.json
```

`print-plan-summary.mjs` is a small (~50-line) helper that reads the plan and prints a human-readable summary: "X steps would BUILD, Y steps would SKIP, Z deploys would fire." Optional warn threshold: `BRIT_PLAN_WARN_THRESHOLD=5` flags if more than 5 steps would build.

**Stage 3 (authoritative):**

The plan summary becomes the primary push-time signal. The legacy per-project quality gates (lint, typecheck, etc.) stay — they catch issues `brit verify` doesn't (style, types, runtime errors). They're orthogonal.

## 4. Migration timeline

Stages are independently shippable. Suggested cadence:

| Stage | Scope | Risk | Validation |
|---|---|---|---|
| Stage 1 | All Jenkinsfiles + pre-push hook write attestations + advisory plan | Very low (purely additive; nothing changes) | Run for 2 weeks; compare advisory plans to actual dispatches in the build log |
| Stage 2a | Orchestrator consumes plan; downstreams still on legacy params | Low (orchestrator dispatch derived from plan; downstreams behave identically) | Run for 1 week; compare dispatch sets vs. legacy expectation |
| Stage 2b | Downstream Jenkinsfiles consume plan (one pipeline at a time) | Medium (per-pipeline plan-driven gating; possible behavior delta if plan diverges from legacy) | Per-pipeline opt-in via env var; revert if divergence; one pipeline per week |
| Stage 3 | Delete legacy code | Low (after stage 2 stabilizes for all pipelines) | All pipelines green for 2 weeks at stage 2 before stage 3 starts |

Worst-case recovery at any stage: revert the Jenkinsfile change. Notes already written are harmless (no consumer depends on them in lower stages).

## 5. The originating incident's resolution

For the storageClass change scenario specifically, after stage 3:

1. Developer makes the manifest change locally.
2. `git commit` triggers (eventually, as pre-push integration matures) `brit verify` and `brit plan`.
3. `brit plan` reports: `elohim-edge:deploy-alpha BUILD (source: genesis/orchestrator/manifests/infra/alpha-mongodb.yaml). All other steps SKIP (attestation match).`
4. Developer pushes. Orchestrator webhook fires.
5. Orchestrator runs `brit plan` — gets identical result.
6. `env.PIPELINES_TO_RUN = "elohim-edge"`. Only elohim-edge is dispatched.
7. elohim-edge's stages: `Build Doorway` (when: plan says SKIP → skipped); same for Build Storage, Build Edge Node Image, Push to Harbor.
8. `Deploy Edge Node - Alpha` (when: plan says BUILD → runs). kubectl applies the manifest. Health check passes. DeployAttestation written.
9. Total: ~90s vs. ~75min.

This is the directly measurable success criterion for stage 3.

## 6. What this spec does NOT change

- **Jenkinsfile structure / DSL.** Still pipeline-as-code Groovy. Still `pipeline { stages { stage { ... } } }`. We're not migrating off Jenkins.
- **Jenkins infrastructure.** ee-jenkins server, kubernetes agents, ci-builder image — unchanged.
- **Build commands.** `cargo build`, `pnpm build`, `cargo test`, `kubectl apply` — unchanged.
- **Credentials management.** Jenkins credentials, Harbor robot accounts, kube secrets — unchanged.
- **CI parallelism, abort, retry.** Jenkins's existing primitives (`milestone`, `disableConcurrentBuilds: abortPrevious`, `retry`, `parallel`) — unchanged.

The point is to delete decision logic, not Jenkins itself.

## 7. Failure modes & mitigations

| Failure mode | Mitigation |
|---|---|
| `brit plan` gives different result locally vs. in CI | Stage 1's advisory mode surfaces this in build log for 2 weeks before stage 2 makes it load-bearing. Determinism is the goal; any divergence is a brit bug to fix. |
| `brit build-ref build put` fails after a successful build | `post { success { } }` block has its own try/catch; failure logs but doesn't fail the build. Note never written → next walk will re-build. Self-healing. |
| `refs/notes/brit/...` push race between concurrent CI runs | Per-step ref means concurrent writes from different pipelines don't conflict. Within a pipeline, `disableConcurrentBuilds: abortPrevious` already prevents collisions. |
| Cold-start (no notes exist yet) | Phase 2A's cold-start semantics: a buildProcess file is "stale" on cold start only if it's actually in the changeset. First run after stage 3 rollout produces full rebuild + populates all notes; subsequent runs benefit. |
| Brit-cli unavailable on a Jenkins agent | Stage 1 is purely advisory — failure to invoke `brit` falls through. Stage 2+ requires brit installed on the ci-builder image. Add to Dockerfile; fail loudly at agent startup if not present. |
| Plan computation slow (large monorepo) | `brit plan` already uses `rakia_brit::changes::*` (no git shell-outs). Target: < 5s for the elohim monorepo. If it exceeds, profile + cache; this is brit-team performance work, not Jenkins-side. |
| Operator wants to force a full rebuild | `brit plan --strict-rebuild` mode (per rakia harvest spec §4.1) ignores attestations. Wire as an orchestrator param: `MODE=rebuild-all` becomes `brit plan --strict-rebuild` instead of bypassing the planner entirely. |

## 8. Open questions

- **Where does `brit` live on the ci-builder image?** Either install from a release tag in the Dockerfile, or build from the brit submodule's main branch in CI. Recommend release tag pinned per ci-builder image version, with explicit upgrades.
- **`BRIT_PLAN_CID` propagation.** Should orchestrator pass the CID and downstreams fetch the plan JSON by CID from `.git/brit/objects/`, or pass the raw JSON path? Recommend CID + fetch — keeps the plan canonical and avoids parameter-size limits.
- **Health-check pods writing DeployAttestations.** Per Phase 2A, health-check pods write DeployAttestations on schedule + on pod restart. Where do these pods live? In the alpha/staging/prod clusters as small sidecars or Deployments? Out of Jenkins-spec scope; mention as an operational gap to fill in the rakia executor work.

## 9. Cross-references

- **Brit master design:** `elohim/brit/docs/specs/2026-04-12-brit-design.md`
- **Brit Phase 2A primitives:** `elohim/brit/docs/plans/phases/phase-2a-build-attestation-primitives.md`
- **Brit harvest (Phase 2B planner + verifier):** `elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md`
- **Rakia harvest (executor + manifest schema):** `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md`
- **Brit-graph + rakia MVP:** `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`
- **Originating incident:** orchestrator-build #727/#728 over-build (storageClass commit `edfe5c57`)
- **Diagnostic report:** ci-investigator agent dispatched 2026-04-27

## 10. Done criteria

For this spec to be considered "designed":

1. Approved.
2. Stage 1 implementation plan ready (Jenkinsfile post.success blocks; pre-push advisory; orchestrator advisory plan stage).
3. Stage 2 implementation plan ready, gated on Stage 1 stability for 2 weeks.
4. Stage 3 implementation plan ready, gated on Stage 2 stability for 2 weeks (all pipelines).
5. ci-builder image update plan: `brit` binary installed at known version.
6. Rollback procedure documented for each stage (just revert Jenkinsfile commits; notes are harmless to ignore).
