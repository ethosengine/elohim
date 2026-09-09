# CI reliability: 2026-09-09

Orchestrator dev #1844 dispatched Genesis after a che-devworkspaces-only push.
The path graph matched no pipelines; the incremental build graph saw an old
`genesis/Jenkinsfile` hash in the restored build-state and requested seed-content.
The pointer was not the justification: stale build-process state was.

Planning and execution ran in the Kubernetes workspace, but `post.always`
archived a leftover controller-local `build-state.json`. That overwrote the
fresh artifact. The fix bridges the current JSON through `env.BUILD_STATE_JSON`
and writes it explicitly in post. Runs aborted before planning copy the last
completed run's artifact into a build-specific recovery directory; an unrelated
controller file is never a fallback. The first repaired run may legitimately
rebuild once to replace the already-stale baseline.

Stage View was not deleting builds: 100 remained in the job API. Dynamic
dependency-level and pipeline stages changed the column layout between runs.
Dispatch now stays within the fixed `Execute Builds` stage; dependency order,
parallel independent application jobs, and graph artifacts remain unchanged.
This is separate from image-chain serialization.

To display 15 runs, the controller operator must set
`-Dcom.cloudbees.workflow.rest.external.JobExt.maxRunsPerJob=15` in Jenkins JVM
options. No controller deployment source was found in this repository, so this
setting was not applied. See the [Stage View documentation](https://plugins.jenkins.io/pipeline-stage-view/).

Image pipelines now release their Kubernetes agents before waiting for children.
Cross-job exclusion remains pending installation of Lockable Resources on the
controller and wiring `lock(resource: 'che-image-build')` to every active image
stage, outside agent allocation. Do not put it around the downstream wait.

Regression evidence: `just gate orchestrator` (150 tests), Jenkins's native
validator (all eight active image Jenkinsfiles), image contract tests, and native
Groovy checks of fresh state, aborted-run copy-forward, missing artifact, copy
failure, and Harbor sort ordering. Live rollout still needs observation.

Story-harvest constraint:

- Chain: CI plan → execute → persist → next plan.
- Missing station: the artifact consumed by the next plan equals the producer's
  state, independent of which workspace runs post; probe the archived state hash.
- Current state: local regression checks pass; live artifact continuity unverified.
- Resource station: a waiting parent owns no BuildKit pod; global active image
  builds ≤ 1 requires the pending controller lock, not just per-job concurrency.
