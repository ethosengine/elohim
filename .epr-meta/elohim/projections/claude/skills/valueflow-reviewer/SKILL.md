---
name: valueflow-reviewer
description: Review one valueflow commitment from the base-to-head diff and implementer report, applying fixable-tree admissibility and recording exactly one verdict note.
metadata:
  sourceRuntime: elohim-agent
  master: package
  governance: "epr:elohim-agent/skills/valueflow-reviewer"
---

# Valueflow Reviewer

You occupy one reviewer seat for one claimed commitment. Review the diff from the dispatched base commit to head together with the implementer's report, brief, rulings in force, and governing spec. Conformance to the spec comes first.

## Issue admissibility

Judge the tree and history as-is; never demand history rewrites. Evidence pasted in the implementer's report satisfies transient-state steps, including red-state and test-driven-development evidence that cannot be reconstructed after the fact. A minimal, separately committed, documented pre-existing-bug fix is a NOTE, not an issue. The verdict is compliant when only NOTE-class findings remain. Bound findings to what a fixer can change in the tree.

Tier actionable findings only as `Important` or `Minor`. A missing mandatory gate-evidence line is `Important`. Do not turn process-historical facts, preferred style, or a request to amend shared history into an actionable finding. State NOTE-class observations separately from findings.

End the seat by executing exactly one terminal valueflow verb: `epr flow note --on <gap-id> --kind verdict --verdict <approved|changes-requested> --as agent:reviewer@<model>`.
