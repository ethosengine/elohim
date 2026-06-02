---
id: "backlog-jenkinsfile-heredoc-lint"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Add a heredoc-aware shellcheck pass to lint-jenkinsfiles-fast.sh (// in heredoc; CPS-scope static lint)"
slug: "jenkinsfile-heredoc-lint"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "CI/orchestrator"
recurrence: 2
source_shifts:
  - "2026-05-22"
  - "2026-05-24"
domain: "code"
relatedNodeIds:
  - "memory:feedback_understand_orchestrator_substrate_before_changes"
  - "memory:project_orchestrator_predictive_vision"
tags: [ci, orchestrator, jenkinsfile, shellcheck, heredoc, cps-scope, lint, code-domain, recurring]
shift_objective: |
  Two distinct Jenkinsfile static-analysis gaps cost shift time. (1) `lint-jenkinsfiles-fast.sh`
  doesn't lint inside heredocs, so a `//` (Groovy comment) accidentally placed inside a shell
  heredoc — where it's literal text, not a comment — slips through and breaks the embedded
  script at runtime. (2) There is no static lint for CPS-scope loss across stages: a variable
  set in one stage and read in another (without an env-bridge) compiles but is null at runtime,
  a different failure than the method-size cap (observed 2026-05-22, 05-24).
  Resolve it by extending `lint-jenkinsfiles-fast.sh` with a heredoc-aware shellcheck pass
  (lint the shell *inside* heredocs; flag `//` used as a comment there) plus a CPS-scope static
  check (flag a variable read in a stage other than the one that set it, with no env-bridge).
  This is code-domain — the lint SCRIPT, not any Jenkinsfile body (do NOT edit any Jenkinsfile;
  the existing `jenkinsfile-cps-scope.test.mjs` is the sibling surface to extend). Done when a
  `//`-in-heredoc and a cross-stage CPS-scope read each fail the committed lint.
---

# Heredoc-aware shellcheck + CPS-scope static lint

## Why this matters

Code-domain (the fix is in the lint script + test, not a Jenkinsfile). Both failure modes are
"compiles, fails at runtime" — the most expensive class, because the green local check gives
false confidence and the break only shows up in a live build.

## The failure shape

- A `//` placed inside a shell heredoc is literal text, not a comment — it breaks the embedded
  script. `lint-jenkinsfiles-fast.sh` doesn't lint inside heredocs, so it passes.
- A variable set in stage A and read in stage B (no env-bridge) is null at runtime due to CPS
  scope loss — distinct from the method-size cap, and currently uncaught statically.

## Shape of the fix (code-domain)

Extend `lint-jenkinsfiles-fast.sh` with: (1) a heredoc-aware shellcheck pass that lints the
shell inside heredocs and flags `//`-as-comment there; (2) a CPS-scope static check that flags
a variable read in a different stage than the one that set it, without an env-bridge — pairing
with the existing `jenkinsfile-cps-scope.test.mjs`. Do NOT edit any Jenkinsfile body (root is
near the CPS cap); this is lint-script work only.

## Acceptance

A `//`-in-heredoc and a cross-stage CPS-scope read each fail the committed lint.
