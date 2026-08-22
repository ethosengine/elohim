---
index: false
id: feedback-sprint-dod-includes-prepush-gates
name: sprint-dod-includes-prepush-gates
title: Sprint DoD includes pre-push gates
description: "Task DoD must run the touched tree's gate clauses (lint/format:check/typecheck), not just unit tests; graphos sprint went 142-green with a red a2o gate."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e5328acc-ec5f-4701-8d4f-17dc78dd9c5b
---

When a sprint touches a lint-gated tree (any project the `.husky/pre-push` graph covers), every task's definition-of-done — and the final review — must run that project's actual gate clauses (`pnpm run lint && pnpm run format:check && pnpm run typecheck`), not only the unit suite.

**Why:** 2026-06-11 graphos sprint: five tasks, per-task spec+quality reviews, visual smoke, 142/142 unit tests green — and the a2o gate was red the whole time (23 lint errors: prettier reflow, sonarjs cognitive-complexity/slow-regex/duplicate-string). Only the FINAL holistic reviewer ran the gate commands. A red gate blocks the integrator's dev push or forces `--no-verify`. Third instance of the "quality loop green, gate red" shape (cf. [[lint-autofix-string-scan-poison]], [[reviewer-issue-admissibility]]).

**How to apply:** Put the gate clauses in the plan's per-task verify steps for any task touching a gated tree, and in the final-review checklist. lint-fixer (sonnet tier) cleans up well after the fact, but the cheap move is catching it per-commit.
