I want to plan and implement **a `DEPLOY_ONLY` parameter on the orchestrator + edge pipelines** so we can re-deploy with existing Harbor image tags when only manifest/RBAC/storageClass changes ship — without burning ~75 min on DNA + edge rebuilds we already know are fine.

## Context (self-contained)

A previous shift (`2026-04-27T03-56-all-pipelines-green-or-unstable`) cleared the DNA integration test surface and the doorway Dockerfile drift, then bailed when edge `Deploy Edge Node - Alpha` hit kubectl `Forbidden: jenkins-deployer` in `elohim-alpha`. Operator restored RBAC + NetworkPolicy permissions cluster-side and pinned `alpha-mongodb` PVC to `storageClassName: openebs-hostpath` (commit `edfe5c57`). That commit re-triggered the orchestrator on `dev`, which kicked off **the entire build matrix** — DNA pack + DNA Integration (~50 min), edge build (~25 min), sophia, app — even though the only change was a single line in a k8s manifest. We knew the existing image tags in Harbor were good. We just needed `kubectl apply`.

This is a recurring pattern. Whenever we ship a manifest-only / RBAC-only / NetworkPolicy-only change, or when an operator restores cluster state after a k8s drift, the existing artifacts in Harbor are valid and we want to redeploy without the ~75 min build cycle.

### The two pipelines that gate deploy time

| Pipeline | Long stages | Skippable when DEPLOY_ONLY=true |
|---|---|---|
| `elohim-holochain` | DNA Pack, **DNA Integration (~50 min)**, Push DNA to Harbor | Yes — DNA artifacts already exist in Harbor |
| `elohim-edge` (`elohim/holochain/Jenkinsfile`) | Quality Gate: Doorway (~12 min), Build Doorway, Build Storage (~7 min), Build Edge Node Image, Build hApp Installer, Push to Harbor | Yes — image tags already exist in Harbor |
| `elohim-app` (root `Jenkinsfile`) | Build, lint, vitest, cypress | Yes — site image already exists |
| `elohim-sophia` | Build sophia, build:umd | Yes — submodule artifact already exists |
| `elohim-genesis` | Seed | **NO** — seeding is the deploy work |

The **Deploy Edge Node - Alpha/Staging/Prod** stages in `elohim/holochain/Jenkinsfile` already read tags from `genesis/orchestrator/environments/{env}.env` (loaded via `loadBuildVars()`) — they don't depend on the build stages of the same run. So `DEPLOY_ONLY=true` on the edge pipeline can legitimately skip every stage *before* `Deploy Edge Node - <Env>` and the deploy still works against whatever tags are recorded in the env file.

### Where the orchestrator dispatches

`genesis/orchestrator/Jenkinsfile`:
- `@Field def PIPELINES` map at line 29 — declares which pipelines exist, their changePatterns, dependsOn, deploymentCheck, triggersGenesis, manualOnly.
- Changeset analysis logic around lines 280-1042 that picks `PIPELINES_TO_RUN` based on what changed in git.
- Dispatch at line 1068+ that triggers downstream Jenkins jobs.

`elohim/holochain/Jenkinsfile`:
- Build/quality stages from line 1 through the Push to Harbor stage (~line 1300).
- `Deploy Edge Node - Alpha` at line 1332.
- `Deploy Edge Node - Staging` at line 1507.
- `Deploy Edge Node - Prod` at line 1567.
- Existing precedent for skip-style params: `SKIP_LEGACY_IMAGES`, `STEPS`, `FORCE_BUILD`, `FORCE_DEPLOY`, `DEPLOY_STRATEGY`, `HUMAN_ASSIGNMENTS`. Adding another param is a well-trodden path.

## Proposed shape (brainstorm should validate or revise)

### Parameter on both pipelines

Boolean param `DEPLOY_ONLY` (default `false`).

### Orchestrator behavior when `DEPLOY_ONLY=true`

1. Skip changeset analysis (or run it but discard the result).
2. Set `PIPELINES_TO_RUN = "elohim-edge,elohim-genesis"` (or just `elohim-edge` — see open question 2 below).
3. Pass `DEPLOY_ONLY=true` as a build parameter to each downstream pipeline triggered.

### elohim-edge behavior when `DEPLOY_ONLY=true`

Add `when { expression { params.DEPLOY_ONLY != true } }` to every stage *before* `Deploy Edge Node - Alpha`:
- Quality Gate: Doorway, Build Doorway, Build Doorway App, Build Agent SDK, Build Storage, P2P Simulation Test, Build Edge Node Image, Build hApp Installer, Push to Harbor.

Deploy stages run as normal, reading tags from `genesis/orchestrator/environments/{env}.env`.

### elohim-holochain (DNA) behavior when `DEPLOY_ONLY=true`

Skip DNA Pack, DNA Integration, Push DNA to Harbor. Effectively the whole pipeline is a no-op. Easier shape: just don't trigger this pipeline from the orchestrator when DEPLOY_ONLY is set.

### elohim-app, elohim-sophia behavior when `DEPLOY_ONLY=true`

Same as DNA — orchestrator skips triggering them entirely.

### elohim-genesis behavior

Genesis seeding **always runs** when DEPLOY_ONLY=true (since the manifest change might necessitate re-seed). Open question 2 below.

## Open questions for brainstorm

1. **Genesis re-run on DEPLOY_ONLY: opt-in or default-on?** If I'm re-applying RBAC, I probably *don't* need to re-seed (the data is in MongoDB and DHT-projected already). If I'm re-applying a manifest that recreated a namespace, I *do* need to re-seed. Add a sub-param `SEED=true|false|auto` (auto = run if alpha health fails post-deploy)? Or keep simple: DEPLOY_ONLY skips genesis by default, operator can manually trigger genesis.

2. **Auto-run vs. manual-trigger.** Should DEPLOY_ONLY ever fire from a webhook automatically, or always be a `Build with Parameters` operator action? If manifest-only changes set `DEPLOY_ONLY=true` automatically (via a path-based heuristic in the orchestrator), we get free fast paths. But that's harder to reason about. Recommendation: **manual-trigger only** for v1 — operator types `DEPLOY_ONLY=true` on the orchestrator parametrized build form. Auto-detection is a v2.

3. **Tag staleness sanity check.** Operator could fire `DEPLOY_ONLY=true` against an env file pointing at a tag Harbor garbage-collected. Should we add a `docker manifest inspect` pre-check on each tag in the env file before kubectl apply? Cheap (<5s per tag, <30s total). Worth the safety net but optional in v1.

4. **Concurrent push collision.** What if a build is in flight when an operator fires DEPLOY_ONLY? Existing Jenkins `disableConcurrentBuilds` (verify this is set on edge) handles it. Just confirm.

5. **`when` block placement on Deploy Edge Node stages.** The deploy stages have existing `when` blocks gating on branch + IMAGES_PUSHED + FORCE_DEPLOY. DEPLOY_ONLY needs to be added to the `anyOf` that triggers deploy *and* should NOT short-circuit the build stages' skip logic. Carefully merge `when` predicates.

## Scope

In-scope:
- `genesis/orchestrator/Jenkinsfile` — add `DEPLOY_ONLY` param, dispatch logic, downstream param propagation.
- `elohim/holochain/Jenkinsfile` — add `DEPLOY_ONLY` param, `when` blocks on build stages, verify deploy stages already work with env-file tags.
- `elohim/holochain/dna/Jenkinsfile` — add `DEPLOY_ONLY` param, short-circuit (or have orchestrator just not trigger it).
- `Jenkinsfile` (root, elohim-app) — same as DNA.
- `sophia/Jenkinsfile` — same.
- `genesis/Jenkinsfile` — accept `DEPLOY_ONLY` param if needed for any conditional logic.

Out-of-scope:
- Auto-detection of "manifest-only changes" (v2).
- Tag-existence pre-check via `docker manifest inspect` (consider as a follow-up).
- Restructuring the env-file format.
- Changing genesis seeding behavior beyond the run/skip decision.

## Done criteria

1. Operator can trigger `elohim-orchestrator/dev` with `DEPLOY_ONLY=true` and the run completes in ~5 min (vs. ~75 min) by skipping all build stages and going straight to deploy.
2. The deploy succeeds against whatever tags are recorded in `genesis/orchestrator/environments/dev.env` (or `alpha.env`).
3. A `DEPLOY_ONLY=false` (default) run behaves identically to today's pipeline.
4. The Jenkinsfile changes pass `npm-groovy-lint --failon error` (the pre-push gate).
5. A test run of `DEPLOY_ONLY=true` against the alpha env on `dev` actually deploys and the edge node passes its health check.

## Reference: prior context

- Shift sprint result: `.claude/shifts/2026-04-27T03-56-all-pipelines-green-or-unstable.sprint-result.md`
- Current dev tip: `edfe5c57 fix(infra): pin alpha-mongodb PVC to openebs-hostpath`
- The orchestrator/dev #727/#728 builds running the slow rebuild path right now are exactly what DEPLOY_ONLY would have skipped.
- Existing skip-style param precedent: `SKIP_LEGACY_IMAGES` (commit `e57ec1e3 feat(pipeline): add SKIP_LEGACY_IMAGES param for consolidated transition`), `STEPS` (commit `c65c3f73 feat(ci): add STEPS parameter and shouldRunStep gating to all pipelines`).
