---
name: commit-only-integrator-pushes
title: Commit-only; integrator pushes
description: "Autonomous mode ends at committed-on-shift-branch; never git push or merge to dev — the integrator is the single push/merge authority."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ec566588-36bb-4cb6-a181-0caedd86b2a0
---

When the operator hands off autonomous work (e.g. "carry on until done, I'm going to bed" — 2026-06-04), the expected terminal state is **committed on the shift branch, NOT pushed**. An integrator (operator-side process) picks up local commits and handles the push/merge.

**Why:** pushing from the dev environment would trigger orchestrator/webhook paths the operator wants to control (see [[sprint-branch-not-orchestrator-indexed]]); the integrator is the single push authority.

**How to apply:** in autonomous stretches, resolve blockers without AskUserQuestion, keep commits small and scoped (shared worktree — selective staging only, see [[concurrent-sessions-shared-worktree]]), end with everything committed and a written wrap-up. Never `git push`, never merge to dev unprompted.
