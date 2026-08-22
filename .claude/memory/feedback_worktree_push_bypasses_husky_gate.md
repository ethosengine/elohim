---
index: false
name: feedback_worktree_push_bypasses_husky_gate
description: Whether a worktree push runs the husky gate depends on core.hooksPath — check `git config core.hooksPath` before assuming either way; verify green regardless.
title: Worktree pushes and the husky gate — check hooksPath, don't assume
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 3770615f-abc2-41d9-b005-cbf326b6db5f
---

Whether a push from a **linked git worktree** runs the husky pre-push gate depends on
`core.hooksPath` (shared config across worktrees):

- `core.hooksPath = .husky/_` (husky v9 generated dir, **gitignored**) → worktree has no
  hook file → **silent skip** (observed 2026-07-16: a 7-commit dev integration landed
  ungated from a scratchpad worktree).
- `core.hooksPath = .husky` (**tracked** dir, per root CLAUDE.md) → the hook IS checked
  out in every worktree and **runs normally** (observed 2026-07-17: fresh detached
  worktree push ran the full gate; also note the gate can fail on fresh-worktree
  install-state — e.g. gitignored codegen artifacts missing, see backlog
  `constants-sync-gate-enoent-untracked-lens-market-view.md` — fix by running the
  generate step, not by bypassing).

**How to apply:** before pushing from a worktree, run `git config core.hooksPath` and
check whether that path exists in the worktree. Never *assume* the gate ran (silent skip
gives zero output) and never assume it will be skipped. Verify green yourself for the
touched trees either way; CI on `dev` is the server-side backstop. Also: the
auto-mode classifier DENIES `git push --no-verify` — resolve gate failures
legitimately. Distinct from [[feedback_hook_bypass_integration_shakeout]] (a
*deliberate* grant). Related: [[feedback_commit_only_integrator_pushes]],
[[project_sprint_branch_not_orchestrator_indexed]].
