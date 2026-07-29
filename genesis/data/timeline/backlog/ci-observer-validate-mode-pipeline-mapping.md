---
id: "backlog-ci-observer-validate-mode-pipeline-mapping"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ci-observer validate-mode prompts false-flag under_built without the project→pipeline mapping"
slug: "ci-observer-validate-mode-pipeline-mapping"
written: "2026-07-29"
author: "deliver-the-saga morning sprint"
status: "open"
priority: "low"
tags: [ci, agents, ci-observer, delegable]
---

# ci-observer validate-mode needs the project→pipeline mapping baked in

Recorded as a watch-out in the 2026-07-29 overnight sprint result: ci-observer
validate-mode prompts (compare orchestrator dispatch vs predicted pipeline set)
false-flag `under_built` unless the dispatching prompt carries the
project→pipeline mapping (the graph-walker's manifest-driven names vs the
Jenkins job names).

## Scope (disjoint, delegable to any agent)

- Bake the canonical mapping table (build-manifest.json projects → Jenkins job
  names, per the CLAUDE.md CI/CD table) into the ci-observer agent definition's
  validate-mode section (`.claude/agents/ci-observer.md`), so callers don't
  have to re-supply it.
- Keep it status-free (gospel discipline: stable mapping only, no live counts).

## DoD / verification

- A validate-mode dispatch on a recent orchestrator build correctly names the
  dispatched pipeline set with zero false `under_built` flags.
- The projection stays in sync if the agent is epr-meta-packaged (check
  .epr-meta/elohim/packages/agents/ for an authoritative source before editing
  the .claude file directly — edit the package, regenerate the projection).
