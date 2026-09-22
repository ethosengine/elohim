---
epr-meta-version: 1
id: elohim-orchestrator-governance
covers: subtree
purpose: >
  The central CI controller: change selection, dependency-level dispatch, and the per-run
  predicted/actual build-graph artifacts. It hosts the habit atom for end-to-end delivery,
  the promise that a push reaches every pipeline it planned within a bounded wall clock.
  It carries no author-time rule. The drift this tree has suffered is arithmetic, not a
  lexical pattern: a downstream budget growing past the parent's, or a dependency edge that
  serialises levels. Tests guard that (pipeline-budget.test.mjs, validate-only-pipeline.test.mjs)
  and so does the habit's live check, not an edit-time predicate.
---
# genesis/orchestrator — governance package

The habit atom here is projected into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.
