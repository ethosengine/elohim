---
id: "backlog-orchestrator-measure-tightening"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Tighten orchestrator measure — require lastBuild commit==HEAD AND non-NOT_BUILT result before reading green"
slug: "orchestrator-measure-tightening"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI/orchestrator"
recurrence: 2
source_shifts:
  - "2026-04-30"
  - "2026-05-14"
domain: "code"
relatedNodeIds:
  - "memory:project_pre_dispatch_hard_fail_post_dispatch_unstable"
  - "memory:project_orchestrator_predictive_vision"
  - "memory:feedback_understand_orchestrator_substrate_before_changes"
tags: [ci, orchestrator, measure, baseline, code-domain, recurring]
shift_objective: |
  The orchestrator's "is the downstream green?" measure reads NOT_BUILT / superseded results
  as a pass. A child that was superseded (abortPrevious) or never built returns NOT_BUILT, and
  the measure treats absence-of-FAILURE as success — so a pipeline that never actually ran for
  the current commit is read as green (observed 2026-04-30 and 05-14).
  Tighten the measure: a pipeline counts as green ONLY when its lastBuild commit == the HEAD
  being measured AND its result is a real terminal success (not NOT_BUILT / ABORTED / a
  superseded run). This is code-domain — the measure logic lives in the orchestrator helper
  methods (read genesis/orchestrator/strategy.mjs + the orchestrator README before touching;
  do NOT edit any Jenkinsfile body — root Jenkinsfile is near the 64KB CPS cap, so the helper
  belongs in a .mjs/groovy helper, not inline). Done when a NOT_BUILT or
  stale-commit downstream no longer reads as green, and a test pins the commit==HEAD +
  non-NOT_BUILT requirement.
---

# Tighten the orchestrator success measure

## Why this matters

Code-domain. A measure that reads NOT_BUILT as green is the root of the "phantom success"
class — it lets the orchestrator advance past a pipeline that never ran for the current
commit, and downstream baseline logic then trusts that lie. This pairs with the baseline
state-machine item; fixing the measure is the upstream half.

## The failure shape

- A child is superseded (abortPrevious) or skipped → result NOT_BUILT / ABORTED.
- The measure checks for FAILURE and, finding none, reads green.
- A pipeline whose lastBuild is for an *older* commit also reads green — staleness is invisible.

## Shape of the fix (code-domain)

A pipeline is green only when **lastBuild commit == HEAD** AND **result is a real terminal
success** (exclude NOT_BUILT, ABORTED, superseded). The logic lives in the orchestrator
helper layer (`strategy.mjs` / orchestrator helpers), NOT inline in any Jenkinsfile — read
`genesis/orchestrator/README.md` + `strategy.mjs` first
(`feedback_understand_orchestrator_substrate_before_changes`), and respect the post-dispatch
UNSTABLE contract (`project_pre_dispatch_hard_fail_post_dispatch_unstable`).

## Acceptance

A NOT_BUILT or stale-commit downstream no longer reads as green; a test pins the
commit==HEAD + non-NOT_BUILT requirement.
