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
