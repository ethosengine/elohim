---
id: "backlog-commit-tag-regex-subject-anchor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Commit-tag dispatch regexes match anywhere in %B — anchor to subject line (prose mention of a tag activates the mode)"
slug: "commit-tag-regex-subject-anchor"
written: "2026-08-01"
author: "quiescence-gated-saga-recording shift (edge #1284 silent validate-only misfire)"
status: "backlog"
priority: "medium"
jobs: [elohim-orchestrator, elohim-edge]
---

# Commit-tag dispatch regexes match anywhere in %B — anchor to subject line

## Concern

Both tag detectors scan the full `git log -1 --format=%B`:
- orchestrator `genesis/orchestrator/Jenkinsfile` (`[build:*]`, `[edge:validate-only]`, `[deploy-only]` detection)
- edge `elohim/holochain/Jenkinsfile` `computeValidateOnly()`
- node mirror `genesis/orchestrator/commit-tag-parser.mjs`

A commit whose BODY merely *mentions* a tag in prose activates the mode.
Observed live 2026-08-01: commit 53c57dd8e's body said "…via
[edge:validate-only]" explaining a future step — the intended deploy wave
(edge #1284) silently ran validation-only (SUCCESS, no build, no deploy),
costing a ~50-min wave while a live doorway fix sat unshipped.

## Cure shape

Anchor tag detection to the subject (first line) in all three homes, in one
change, with contract tests updated together (`commit-tag-parser.test.mjs`
+ `validate-only-pipeline.test.mjs` pin behavior; add a negative test: tag
string in body only → NOT detected). Document the subject-only rule in the
CLAUDE.md force-dispatch section.
