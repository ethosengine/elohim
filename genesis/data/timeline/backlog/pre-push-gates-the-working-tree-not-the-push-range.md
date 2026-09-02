---
id: "backlog-pre-push-gates-the-working-tree-not-the-push-range"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The pre-push gate lints the WORKING TREE, not the commits being pushed — a sibling session's untracked scratch script or half-edited file refuses the integrator's push (three trips on 2026-09-02), so a shared worktree needs a push-window protocol and the gate needs a clean-index or range-scoped mode"
slug: "pre-push-gates-the-working-tree-not-the-push-range"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch"
status: "open"
priority: "medium"
relatedNodeIds: []
tags: [pre-push, husky, gate-runner, shared-worktree, multi-session, prettier, working-tree, push-window]
cites:
  - .husky/pre-push.bash
  - genesis/orchestrator/gate-runner.mjs
  - justfile
---

## Measured

2026-09-02, shared `/projects/elohim` worktree, two sessions committing and one integrating:

- wave 4 attempt 1 refused: `genesis-a2o` format:check on an untracked `genesis/a2o/repro-follow-card.mjs`
  (the other session's Playwright probe), plus `elohim-app` deps on install-state drift.
- wave 5 attempt 1 refused after 1220 s (storage gate alone 906 s): `genesis-a2o` format:check on an
  untracked `genesis/a2o/.alpha-check.mjs`.
- earlier in the same shift: a lint leg ran over another session's uncommitted regenerated files.

Each refusal costs a full hook (15–20 min) and a re-push. The gate is right that the tree it
tests is the tree that ships only if the tree IS the push; on a shared worktree it is not.

## Interim discipline (in force)

Push windows: the integrator announces "hook starting, tree freeze ~N min"; siblings commit
freely but make no working-tree edits and keep scratch under their session scratchpad (or a
gitignored `reports/` path), never under a linted package root. Reply "window closed" when landed.

## The change

Give `gate-runner.mjs` a `--committed` mode the pre-push hook uses by default: run the project
gates against a temporary checkout of the pushed range head (`git worktree add --detach`), or at
minimum scope prettier/eslint to `git ls-files` (tracked, committed content) rather than the
directory. Untracked scratch then cannot refuse a push; uncommitted edits to tracked files still
can, which is the honest signal.

## Done when

A push from a worktree holding an untracked, unformatted `.mjs` under `genesis/a2o` passes the
`genesis-a2o` leg; a push whose committed range contains an unformatted file still fails it.
