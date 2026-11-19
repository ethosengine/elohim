---
id: "backlog-brit-rebase-cost-gitlinks-and-stranding"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit rebase spools gigabytes for a two-commit replay, refuses gitlinks, and can strand a repository in 'preparing'"
slug: "brit-rebase-cost-gitlinks-and-stranding"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds: []
tags: [brit, daily-driver, rebase, switch, submodule, shallow, performance, perf-io, perf-latency, lane-operator]
cites:
  - genesis/data/timeline/backlog/brit-sprint-3-handoff-2026-11-19.md
shift_objective: |
  In this order: (1) move every rebase refusal decidable from the index and two trees ahead of
  any spooling; (2) fix the stranding cases, with the rule that a journal still in `preparing`
  can always be aborted; (3) a cost model where what is preserved scales with the replayed range
  and the paths it touches; (4) then shallow floors and gitlink moves.
---

# brit rebase: cost, gitlinks and stranding

## Measured on shem

brit 0.2.6, monorepo not shallow, a linked worktree, two docs-only commits replayed over two
upstream commits:

- `brit rebase origin/dev` ran 6 minutes at 95% of one core, RSS 160 MB;
- it wrote 29.1 GB under `/tmp/.brit-checkout-*` (29.09 GB cancelled) and read 3.5 GB;
- it then exited 1 with "Directory staging is not supported; name explicit files", leaving no state;
- `git rebase --autostash origin/dev` did the same replay in under 2 s;
- old empty `/tmp/.brit-checkout-*` directories were left behind.

## Found by reading and scratch runs (session 0e0924d5)

- Any gitlink difference between HEAD and upstream refuses ("affected gitlinks are unsupported").
- A gitlink present with its submodule directory present refuses with the directory-staging message.
- With the submodule directory ABSENT (uninitialised), `brit rebase` creates `.git/brit-rebase` in
  phase `preparing`, fails with "An object with id … could not be found" (the gitlink id treated as a
  local object), and then neither `--abort` nor `--continue` can clear it, and `brit switch` refuses
  "pending replay". The repository is stranded. The monorepo is protected only because rebase refuses
  shallow repositories first.
- Separately, a `brit switch` killed by SIGTERM just after taking the guard leaves an empty
  `<gitdir>/worktrees/<name>/brit-switch` directory. Every writer in every worktree then refuses
  "pending switch or pull", and `switch --status` / `--abort` fail with ENOENT. Cure today: remove the
  empty directory by hand.

## Shallow rebase

The selection condition "every boundary commit either walk reaches is reached by both, plus the
linear-chain rule" is necessary and sufficient. What is missing is the retained copy's format.
Design ready: a `shallow_floor` list in the journal state and a retained-manifest v2 written only
when the floor is non-empty, every history walk stopping at the recorded floor.

## Order of work (decided)

1. Every refusal decidable from the index and two trees moves ahead of any spooling.
2. The stranding fixes, and a rule that a journal still in `preparing` can always be aborted.
3. A cost model where what is preserved scales with the replayed range and the paths it touches.
4. Then shallow floors and gitlink moves.

Until (3), rebase stays a declared git fallback on both devices. No lane has started.
