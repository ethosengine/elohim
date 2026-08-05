---
id: "backlog-sophia-upstream-graph-types-features-sync-epic"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia fork is missing five interactive-graph types and two graph features landed upstream post-fork — consolidated sync epic"
slug: "sophia-upstream-graph-types-features-sync-epic"
written: "2026-07-30"
author: "surfaced by a read-only evaluation of upstream/main vs the sophia fork merge-base (2026-07-30)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "backlog-security-jquery-2-1-1-shipped-in-sophia-umd-bundle"
tags: [sophia, perseus, upstream-sync, interactive-graphs, mafs, accessibility, a11y, epic, jquery-3-migration-adjacent, submodule]
cites:
  - sophia/packages/sophia/src/widgets/interactive-graphs/mafs-graph.tsx
  - https://github.com/Khan/perseus/pull/3354
  - https://github.com/Khan/perseus/pull/3514
  - https://github.com/Khan/perseus/pull/3697
  - https://github.com/Khan/perseus/pull/3696
  - https://github.com/Khan/perseus/pull/3673
  - https://github.com/Khan/perseus/pull/3672
  - https://github.com/Khan/perseus/pull/3669
  - https://github.com/Khan/perseus/pull/3643
---

## What this captures

A read-only evaluation of `upstream/main` against the sophia fork's merge-base
(`cbef3cb967`, fork HEAD `f0157adbb`, 2026-07-30) found the fork is missing a
coherent block of upstream work in `packages/sophia`'s interactive-graphs
widget: **five graph types** and **two supporting features**, all landed
upstream *after* the fork point. This entry consolidates them into one epic
because they share a root cause (the fork never absorbed this slice of
upstream's mafs-graph work) and because several *other* upstream fixes and an
accessibility wave are blocked on the same gap.

### Missing graph types

`packages/sophia/src/widgets/interactive-graphs/mafs-graph.tsx`'s dispatch
table lacks render functions for five graph types that exist upstream:
**absolute-value, exponential, logarithm, tangent, vector.** Upstream added
these after the merge-base — e.g. tangent via upstream PR #3354 (commits
`dbbde31e74`, `4b2a7c85db`). The other four types were not individually
traced to PR numbers this pass; treat as "added post-merge-base, same
mafs-graph.tsx surface" pending a per-type harvest (see Working notes below).

### Missing features

- **pointLabels** — the custom point-label feature (`usePointAriaLabel`,
  `build-point-aria-label.tsx`). Landed upstream across PRs #3697, #3696,
  #3673, #3672, #3669, #3643.
- **ClipToGraphBounds** — edge-point placement clipping. Landed upstream PR
  #3514, commit `08fe06e533` (21 files touched).

## Why this is one epic, not five+two isolated diffs

This single gap is the blocker for several *other* pieces of upstream work the
fork cannot cleanly take yet:

- Upstream tangent fix `a4e51fc66a` (depends on the tangent graph type
  existing in the fork first).
- Upstream axes-thickness fix `01c43fd16b`.
- Upstream input-number a11y flag `3d2c95c680` — which additionally requires
  the `InputNumber` → `NumericInput` migration (`dd984a32ff`, `dca55701d8`,
  `e2c97852f2` — roughly 80 files) as its own prerequisite.
- **4 of the 12 commits** in the WB-Announcer accessibility wave — the
  vector/tangent/logarithm/exponential announcers — are blocked because they
  announce state for graph types the fork doesn't render.

The **portable 8/12 of the Announcer wave** (plus its prerequisites — infra
`1f56144658`, circle `e2e2132ab1`, point `625791bd32`, polygon `29041d51fe`)
do **not** depend on this gap and are being handled separately on branch
`feat/jquery-3`. When this epic lands, the 4 blocked announcer commits should
ride along in the same pass rather than being re-scoped from scratch.

## Working notes for the harvest

- The `upstream` remote is already configured in the sophia submodule.
- Upstream commit messages are `[WidgetName]`-bracketed, so scoping a future
  harvest per graph type is cheap:
  `git -C sophia log upstream/main --grep='\[Tangent\]'` (swap in
  `Vector`, `Logarithm`, `Exponential`, `AbsoluteValue` etc.) surfaces the
  full commit set for that type without re-deriving it by hand.
- Suggested build shape: port **per-graph-type** (each type = graph
  render function + strings + editor wiring + tests), and land
  **pointLabels first** — several of the upstream graph-patch and announcer
  diffs carry `pointLabels`/`usePointAriaLabel` in their context lines, so
  having it in the tree first will make those patches apply cleanly instead
  of needing manual reconciliation.

## Current decision

**Backlog — no blocker other than scheduling/scale.** This is a
multi-graph-type, multi-feature port into a Perseus-derived widget surface;
size and de-risking work needed (per-type tests, eyes-verification of each
rendered graph via `pnpm look`/a2o) put it past a quick fold, but nothing is
waiting on upstream or on another in-flight branch except the `feat/jquery-3`
Announcer-wave coordination noted above. Priority is `medium`: real a11y and
upstream-parity value, but no security exposure, so it does not compete with
the `high`-priority jQuery 2→3 migration for scheduling.

## Verification

No fix landed this pass — read-only evaluation only. Confirmed by diffing
`upstream/main` against merge-base `cbef3cb967` (fork HEAD `f0157adbb`) and
reading `packages/sophia/src/widgets/interactive-graphs/mafs-graph.tsx`'s
dispatch table against the upstream equivalent. Closure requires: all five
graph types rendering and editable in the fork with tests, both features
(pointLabels, ClipToGraphBounds) present, the 4 blocked Announcer commits
landed, and eyes-verified rendering of each new graph type (`pnpm look` /
a2o Sophia widget paths) since Perseus widget regressions are typically
visual, not test failures.
