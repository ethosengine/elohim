---
id: "backlog-gherkin-prepush-lint"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pre-push gherkin/cucumber grammar linter — catch empty-alternation and bare-continuation before an AST-abort drops the whole E2E run"
slug: "gherkin-prepush-lint"
written: "2026-06-02"
author: "cartographer"
status: "done"
priority: "high"
area: "genesis/a2o"
recurrence: 2
source_shifts:
  - "2026-05-05"
  - "2026-05-17"
domain: "code"
relatedNodeIds:
  - "memory:feedback_a2o_is_human_experience_not_dev_bugs"
  - "memory:feedback_cascade_halt_masks_failures"
tags: [genesis, a2o, gherkin, cucumber, lint, prepush, code-domain, recurring]
shift_objective: |
  A single malformed Gherkin file aborts the ENTIRE E2E run at parse time: an unescaped `/`
  produces an empty-alternation, and a bare continuation line is rejected by the AST parser.
  When that happens the cucumber report comes back empty, which the pipeline reads as UNSTABLE
  with a blank body — so one typo in one feature file silently drops every scenario's result
  (observed 2026-05-05, 05-17).
  Resolve it with a pre-push gherkin/cucumber grammar linter that catches empty-alternation
  (unescaped `/` in scenario-outline alternation), bare-continuation lines, and other
  AST-abort triggers BEFORE the run, so the author fixes the typo locally instead of blanking
  CI. This is code-domain (a pre-push lint over genesis/a2o/features/**). Pair it with the
  "read the E2E log FIRST" discipline (a blank cucumber report means a parse abort, not a
  feature failure). Done when a feature file with an empty-alternation or bare-continuation
  fails a committed pre-push lint instead of aborting the E2E run.
---

# Pre-push gherkin/cucumber grammar linter

## Why this matters

Code-domain. The grammar-abort is a maximally-lossy failure: one bad character blanks the
entire E2E result and the pipeline reports UNSTABLE-with-empty-body, which reads like an
environment flake rather than a syntax error. A pre-push lint moves the fix to where the typo
was made.

## The failure shape

- Unescaped `/` → empty-alternation; bare continuation line → AST reject.
- The cucumber report comes back empty.
- The pipeline reads empty-report as UNSTABLE with a blank body — every scenario's real result
  is gone.

## Shape of the fix (code-domain)

A pre-push grammar lint over `genesis/a2o/features/**` that catches empty-alternation,
bare-continuation, and other AST-abort triggers locally. Keep the failure boundary clean:
a2o is human experience, and a *syntax* error is a dev bug, not a scenario change
(`feedback_a2o_is_human_experience_not_dev_bugs`). Document the "read the E2E log first —
blank report = parse abort, not feature failure" rule alongside it.

## Acceptance

A feature file with an empty-alternation or bare-continuation fails a committed pre-push lint
rather than aborting the E2E run.

## Completion (2026-07-31)

Already landed in commit `b0150a4cc`:
`genesis/a2o/scripts/gherkin-prepush-lint.mjs`, its committed `node:test`
regressions, the `lint:gherkin` package script, and `.husky/pre-push.bash`
integration. Reverified here: both malformed-fixture tests pass and the linter
parses all 160 current feature files.
