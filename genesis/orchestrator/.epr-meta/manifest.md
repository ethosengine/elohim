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
  records (data/). It hosts the habit atoms for end-to-end delivery (the promise that a push
  reaches every pipeline it planned within a bounded wall clock) and for submodule pin
  attestation. It carries no author-time rule: the drift this tree has suffered is arithmetic, not
  a lexical pattern — a downstream budget growing past the parent's, or a dependency edge that
  serialises levels. Tests guard that (pipeline-budget.test.mjs, validate-only-pipeline.test.mjs)
  and so do the habits' live checks, not an edit-time predicate.
rules:
  - id: orch-record-mjs
    policy: record-and-judge-are-native@1
    when:
      write: "*.mjs"
      new: true
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: orch-record-sh
    policy: record-and-judge-are-native@1
    when:
      write: "*.sh"
      new: true
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: orch-record-py
    policy: record-and-judge-are-native@1
    when:
      write: "*.py"
      new: true
      contains-any: ["appendFileSync(", "write_json_atomic", "state.json", "deadline =",
                     "monotonic() >=", ">= bound", "> bound", "age <= ", "age > "]
  - id: orch-held-mjs
    policy: held-state-is-the-incident@1
    when:
      write: "*.mjs"
      new: true
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
  - id: orch-held-sh
    policy: held-state-is-the-incident@1
    when:
      write: "*.sh"
      new: true
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
  - id: orch-held-py
    policy: held-state-is-the-incident@1
    when:
      write: "*.py"
      new: true
      contains-any: ["tool_use", "approval-evidence", "pending-approval", "held-state", "held_state",
                     "\"held\"", "'held'", ".held"]
---
# genesis/orchestrator — CI controller and governance package

What belongs here: the orchestrator's Node modules, each with a `*.test.mjs` beside it
(graph-walker, pipeline-registry, gate-runner, commit-tag-parser and the rest), the
`Jenkinsfile` and `build-graph.groovy`, `manifest.schema.json`, `environments/`, `scripts/`, and
the red-first `design-tests/`. `README.md` is the contributor guide. Commit tags, preview and
troubleshooting are documented there, not restated here.

The habit atoms here are projected into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.

`covers: subtree` claims the directory. The two children keep their own manifests and rules:
`manifests/.epr-meta` (the rendered fleet manifests) and `data/.epr-meta` (deployments.json and
its resource-budget policies). No rule is added at this level. The orchestrator's drifts are
caught structurally, by its tests, `validate-manifests.mjs` and the Jenkinsfile size check.
`CI_RELIABILITY.md` is a single dated note; if dated incident notes recur at this root, they take
a dispatch route to the timeline backlog.

The same two advisory policies are bound here for `*.mjs`, `*.sh` and `*.py` (`record-and-judge-are-native@1`, `held-state-is-the-incident@1`; they never block). The reasons and the limits of the rule are in `genesis/agentic/.epr-meta/manifest.md`, section "One home for a record, many implementors". The orchestrator-specific line: a gate script is a thin caller of a native read. It enforces a local monotonic clock and does not compute a settlement or a deadline another device must agree with; `gate-carry-claim.mjs` is the case that prompted it (the native carry decision is `epr flow intend --carry-decision`).
