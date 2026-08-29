---
name: feedback_push_branch_discipline
title: Push, branch & worktree discipline (umbrella)
description: "Commit-only (integrator pushes); one push per batch (concurrent pushes mutually abort); shared worktree — path-limited commits, never bulk-revert; check core.hooksPath; sprint/* is not CI-indexed."
metadata:
  node_type: memory
  type: feedback
---

# Push, branch & worktree discipline (umbrella)

Folds the git push/branch/worktree discipline cluster — the rules governing where autonomous work stops and how commits reach CI. Members:

- [[feedback_commit_only_integrator_pushes]] — Autonomous mode ends at committed-on-shift-branch; never git push or merge to dev — the integrator is the single push/merge authority.
- [[feedback_concurrent_push_mutual_abort]] — Dev pushes minutes apart kill each other's builds (abort-previous), even same-session; one push per batch, wait until COMPLETE; escalate silent webhook loss.
- [[feedback_concurrent_sessions_shared_worktree]] — Sessions co-commit on shift/* in ONE worktree — never bulk-revert ambient mods; commit path-limited (-m … -- paths); never amend without re-checking HEAD.
- [[feedback_worktree_push_bypasses_husky_gate]] — Whether a worktree push runs the husky gate depends on core.hooksPath — check `git config core.hooksPath` before assuming either way; verify green regardless.
- [[feedback_hook_bypass_integration_shakeout]] — The agent working ON the CI pipeline may push --no-verify during integration shakeout only if gates already ran green; CI becomes its verification surface.
- [[feedback_che_devworkspaces_direct_to_main]] — che-devworkspaces (CI/image infra) pushes straight to main, inert-by-default; elohim monorepo main is reviewed dev→main only — surface classifier blocks.
- [[feedback_work_stays_in_operator_visible_tree]] — All work lands in /projects/elohim (the operator's VS Code mount); never create sibling worktrees like /projects/elohim-wt-land — invisible work is unreviewable.
- [[feedback_partition_compile_and_stale_dist]] — Two integration anti-patterns from 2026-07-24 overnight — commit partitions must respect COMPILE deps, and local dist/ presence proves nothing about CI stage coverage
- [[project_sprint_branch_not_orchestrator_indexed]] — Orchestrator indexes only {PR-*, dev}: sprint/* and claude/* pushes never trigger CI ([build:*] inert, NOT_BUILT); auto-deploy only via dev-merge.
- **Never reset by relative ref in a shared worktree (2026-08-27 near-miss).** `git reset --soft HEAD~1` meant to drop MY commit removed the sibling session's newest commit instead — three of theirs had landed on top of mine in the minutes between. Recovered from the reflog within a minute, but the rule is: name the commit (`git reset --soft <sha>` / `git revert <sha>`), re-read `git log -5` immediately before any history op, and prefer a forward correction commit over rewriting when another session is live. Corollary: an inert `[build:*]` tag buried in history is harmless — the orchestrator reads `git log -1` only — so leave it and add a commit above it rather than rebase.

**2026-08-29 — two agents committing in ONE shared worktree collide on the index.** Sweep B's `git reset`
emptied sweep A's staged index mid-flight, and B's `git add` of its own file landed inside A's commit (A had to
amend it out). One index per worktree: serialize commits (one committer at a time) or give each integrating
agent `isolation: worktree`; never run two path-limited committers concurrently in the same checkout.
