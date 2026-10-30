---
id: "backlog-ci-brit-worktree-restore-cancellation-test-races"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit cli-journey worktree_restore cancellation test races its own suspend signal — red on 2 of 3 runs at the daily-driver tip, each from a different window"
slug: "ci-brit-worktree-restore-cancellation-test-races"
written: "2026-10-29"
author: "integration shift 2026-10-29 (session dff40885)"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [ethosengine/brit ci "Tests pass"]
relatedNodeIds: []
tags: [ci, brit, flake, timing-race, github-actions, pin-attestation, not-jenkins]
cites:
  - https://github.com/ethosengine/brit/actions/runs/40941930166
  - https://github.com/ethosengine/brit/actions/runs/40964492259
  - genesis/orchestrator/.epr-meta/pin-attestation.habit.md
  - genesis/orchestrator/gate-attest.mjs
---

# brit `cancellation_before_publication_changes_nothing_and_partial_cancellation_reports_progress` races its own suspend

**Where:** brit `tests/cli-journey`, `worktree_restore::cancellation_before_publication_changes_nothing_and_partial_cancellation_reports_progress` (brit main at 7e4a83115e; the test is not touched by either branch that tripped it).

**What it does:** starts a large `brit restore --worktree`, suspends the process part-way, and asserts that nothing was published before the suspend and that a partial cancellation reports its progress. The suspend is sent on a clock, not on a signal from the restore, so the test has two windows it can miss:

| Run | Runner | Red text | Window missed |
|---|---|---|---|
| 40941930166 attempt 1 (daily-driver tip 7e4a83115e) | ubuntu | `large restore published before the process could be suspended` | suspend arrived too late |
| 40964492259 attempt 1 (S2-1a fixtures 1750f673a, one commit over that tip) | ubuntu-24.04-arm | `Error: restore never exposed its temporary write:` | suspend arrived too early |
| 40941930166 attempt 2 (re-run of the failed jobs) | ubuntu | green | — |

Two different windows on two runners in three runs is a race in the test, not a regression in restore. Both reds happened in `test-fast`; `test` (the full suite) was green on the same commits.

**Why it matters here:** `Tests pass` at the pinned brit SHA is what `genesis/orchestrator/gate-attest.mjs` reads for the `brit` gate (habit `pin-attestation`). Each red costs a re-run of the failed jobs before a pin can move (two re-runs on 2026-10-29), and the i386 `deb.debian.org` DNS failure that rides along is environmental and re-runs with it.

**Cure (brit side, out of this shift's write-set):** gate the suspend on an observable from the restore — the first temporary write appearing — instead of a sleep; or split the two assertions into two tests each driven by its own observable. Until then a red on exactly this test with one of the two texts above is re-run, not investigated.

**Closes when:** the test passes on 10 consecutive `test-fast` runs across runners without a re-run, or is rewritten to wait on the restore's own signal.
