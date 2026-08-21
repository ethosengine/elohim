---
id: "backlog-seam-registry-schema-invalid-row-silently-dropped"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Nothing validates seam-registry.yaml against its schema before the census reads it — doorway's registry is schema-invalid today and the row degrades to a silently-dropped decision point"
slug: "seam-registry-schema-invalid-row-silently-dropped"
written: "2026-08-21"
author: "claude (freshness-verdict landing; seam reported by the implementing agent, verified by the orchestrator)"
status: "backlog"
priority: "medium"
jobs: [elohim-genesis]
nodes: []
relatedNodeIds:
  - "memory:project_epr_meta_compose_gate"
tags: [seam-registry, epr-meta, census, placement-audit, p2p-design-gate, honest-absence, C4]
cites:
  - doorway/doorway-service/seam-registry.yaml
  - elohim/sdk/schemas/v1/manifest/seam-registry.schema.json
  - .claude/scripts/memory-kit/placement-audit.py
  - .claude/skills/p2p-design-gate/SKILL.md
---

# seam-registry.yaml is never schema-validated before the census reads it

**Chain:** per-crate `seam-registry.yaml` → `seam-registry.schema.json` → `placement-audit.py --epr-meta` census.
**Between A→C:** every registry declares it conforms to the schema; the census is declared to be a derived
read-model over the registries.
**Missing node B:** nothing validates a registry against the schema before the census reads it.
**Assertion + probe:** "every in-tree `seam-registry.yaml` validates against
`elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`" — probe: `jsonschema.validate` per registry as
a gate / pre-push step (or the first thing `placement-audit.py --epr-meta` does, failing loud).
**Current state (2026-08-21, verified):** RED and invisible. 8 registries in tree; 7 validate;
`doorway/doorway-service/seam-registry.yaml` fails — `decisionPoints[12]` (`install_steward_routes`)
carries `kind: state-transition`, which is not in the schema's `kind` enum
(`pure-decision-predicate | verdict-fn | boundary-answer-type | reason-outcome-enum`). The census is
deliberately row-at-a-time fail-per-row (the EprRouter poisoned-scope lesson: one bad row must not empty
the set), so the invalid row degrades to a silently-dropped decision point instead of a loud failure —
the C4 honest-absence class applied to the instrument itself.

## Task (atomic)

1. Add the validation probe where the census runs (`placement-audit.py --epr-meta`) AND as a gate clause
   for any tree that owns a `seam-registry.yaml`; a schema-invalid registry must be a visible failure,
   while the census keeps its per-row degradation for *semantic* problems.
2. Resolve the doorway row deliberately — either reclassify `install_steward_routes` into one of the four
   kinds or extend the schema enum with `state-transition` (and say why a fifth kind is a kind, not a
   costume). Changing another agent's registered `kind` is a classification judgment, not a typo fix —
   the implementing agent correctly left it alone.
