---
epr-meta-version: 1
id: elohim-orchestrator-governance
covers: subtree
purpose: >
  The CI/CD orchestrator. It is the one pipeline that receives GitHub webhooks: it walks every
  build-manifest.json into a dependency graph, selects and orders the pipelines a change needs,
  and dispatches them, and it keeps the per-run predicted/actual build-graph artifacts. It also
  hosts the local gate runner behind `just gate` and pre-push, the manifest schema and validator,
  the Jenkins helper scripts, the rendered k8s manifests (manifests/) and the per-human deployment
  records (data/). It hosts the habit atom for end-to-end delivery, the promise that a push reaches
  every pipeline it planned within a bounded wall clock. It carries no author-time rule: the drift
  this tree has suffered is arithmetic, not a lexical pattern — a downstream budget growing past the
  parent's, or a dependency edge that serialises levels. Tests guard that (pipeline-budget.test.mjs,
  validate-only-pipeline.test.mjs) and so does the habit's live check, not an edit-time predicate.
---
# genesis/orchestrator — CI controller and governance package

What belongs here: the orchestrator's Node modules, each with a `*.test.mjs` beside it
(graph-walker, pipeline-registry, gate-runner, commit-tag-parser and the rest), the
`Jenkinsfile` and `build-graph.groovy`, `manifest.schema.json`, `environments/`, `scripts/`, and
the red-first `design-tests/`. `README.md` is the contributor guide. Commit tags, preview and
troubleshooting are documented there, not restated here.

The habit atom here is projected into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.

`covers: subtree` claims the directory. The two children keep their own manifests and rules:
`manifests/.epr-meta` (the rendered fleet manifests) and `data/.epr-meta` (deployments.json and
its resource-budget policies). No rule is added at this level. The orchestrator's drifts are
caught structurally, by its tests, `validate-manifests.mjs` and the Jenkinsfile size check.
`CI_RELIABILITY.md` is a single dated note; if dated incident notes recur at this root, they take
a dispatch route to the timeline backlog.
