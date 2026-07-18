---
name: feedback_worktree_push_bypasses_husky_gate
description: Pushing from a linked git worktree silently skips the husky pre-push gate — .husky/_ is gitignored and absent there; verify green another way.
title: Worktree pushes silently skip the husky gate
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 3770615f-abc2-41d9-b005-cbf326b6db5f
---

Pushing a branch from a **linked git worktree** silently bypasses the husky pre-push quality gate. `core.hooksPath = .husky/_` (husky v9 layout), and `.husky/_/` is husky's *generated* hook dir — **gitignored**, so it is NOT checked out by `git worktree add`. Git resolves the relative hooksPath against the worktree top, finds no `.husky/_/pre-push`, and runs **no hook at all**: zero output, not a `--no-verify` — a *silent* skip. A push to `dev`/`main` from a worktree therefore lands **ungated**.

**Why:** integration is often done in an isolated worktree (to avoid the shared main worktree's ambient changes). On 2026-07-16 a 7-commit integration landed on `dev` from a scratchpad worktree with the pre-push gate never firing — CI (the orchestrator) was the only actual verification, discovered only because the push log had none of the usual `[pre-push]`/gate lines.

**How to apply:** when pushing from a worktree, do NOT assume the gate ran. Either (a) push from the main `/projects/elohim` checkout where `.husky/_` exists, or (b) verify green yourself in the worktree (run the touched crates' `just`/cargo tests directly) and confirm CI on `dev` actually triggered as the server-side backstop. Distinct from [[feedback_hook_bypass_integration_shakeout]] (a *deliberate* `--no-verify` grant) — this one is an *accidental* silent skip. Related: [[feedback_commit_only_integrator_pushes]], [[project_sprint_branch_not_orchestrator_indexed]].
