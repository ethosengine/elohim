---
id: feedback-pipeline-work-separate-from-sprint
name: feedback-pipeline-work-separate-from-sprint
description: "When iterating on CI/CD pipelines, do NOT stash or touch in-flight sprint work. Push the sprint to its target, then check out a feature branch off the target (e.g. dev) in a separate worktree for pipeline-fix iteration. Use Jenkins itself as the test surface — don't run local gates that block on a dirty sprint tree."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 5ed7452d-de73-43b1-814f-3b1742a3b1b8
cites:
  - .claude/commands/shift.md
---

When the operator says "use the pipelines to shake out fixes" or asks for CI/CD iteration, do NOT stash or otherwise disturb in-flight sprint work (uncommitted scaffolding on a long-running sprint branch).

**Why:** Sprint branches carry multi-day in-progress work (e.g. M-REA-3 Phase B + M-AGGR-3 continuation scaffolding on `sprint/cross-pillar-cleanup` during the 2026-05-28 pipelines-unstable-or-better shift). Stashing it during a pipeline-iteration shift creates context that has to be popped back later, breaks the trail of what's actively being authored, and confuses the operator's mental model when they switch back to sprint work. The sprint tree should stay exactly as the operator left it; pipeline iteration is its own track, not a stop-the-world detour through sprint.

**How to apply:** When the shift's Objective is pipeline health (not sprint feature work):

1. Push the sprint to its landing target as a normal fast-forward — the push transmits committed history only, so the dirty working tree is irrelevant to what lands. The sprint workspace stays untouched throughout.
2. For pipeline-fix iteration, create a feature branch off the landing target (`dev`) in a **separate worktree** — `git worktree add /projects/elohim-worktrees/<shift-slug> -b fix/<shift-slug> origin/dev`. That worktree has a clean tree, can run the readiness check, and is where every pipeline fix commit lives.
3. Push fix branches to their own remote refs and use Jenkins as the test surface — `[build:<pipeline>]` commit tags drive dispatch. Don't rely on local pre-push gates to validate pipeline-touching changes; CI is the gate by design here.
4. When a fix verifies green, merge the feature branch → dev (fast-forward or PR per the operator's preference).
5. Never touch the sprint workspace from the pipeline-iteration shift. When the shift closes, the sprint workspace is still in the state the operator left it in.

Anti-pattern: stashing the sprint workspace to pass a readiness check, then popping it back at shift close. The stash is a load-bearing context the operator was actively authoring against — disturbing it is a regression in the operator's working state even if everything gets restored.

Related: [[feedback_pause_sprint_when_substrate_in_flight]] (substrate-in-flight ≠ sprint-pause; pipeline iteration runs alongside sprint work on its own branch).
