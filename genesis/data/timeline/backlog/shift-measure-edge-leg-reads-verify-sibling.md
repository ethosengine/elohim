---
id: "backlog-shift-measure-edge-leg-reads-verify-sibling"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A shift measure that keys a pipeline leg on lastCompletedBuild reads the elohim-edge verify sibling (VALIDATE_ONLY, RUN_CLASS=verify) as the pipeline's result, so a deployed-green build is scored red by its own fleet validation or NOT_BUILT by a quiesce timeout"
slug: "shift-measure-edge-leg-reads-verify-sibling"
written: "2026-10-09"
author: "claude-fable-5-1 (shift 2026-10-09T06-30-land-gradient-sprint-on-dev)"
status: "open"
priority: "low"
tags: [agentic-developer, shift, measure, judge, elohim-edge, validate-only, verify-sibling, not-built]
relatedNodeIds: []
cites:
  - genesis/local-dev/gradient-sprint-landing/measure.sh
  - genesis/local-dev/release-retention-landing/measure.sh
  - .claude/skills/agentic-developer/SKILL.md
  - genesis/orchestrator/Jenkinsfile
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# The edge leg of a landing measure must name the run class it scores

## What happened (shift 2026-10-09T06-30-land-gradient-sprint-on-dev)

The measure counted five pipelines whose `lastCompletedBuild` is SUCCESS/UNSTABLE on a commit carrying the tip. For `elohim-edge/dev` the orchestrator dispatches a build-class run and then, once it deploys, a VALIDATE_ONLY verify sibling that owns Dataplane Validation. The sibling always completes after the build, so it is always the job's last completed build:

| edge build | class | result | what the measure read |
|---|---|---|---|
| #1588 | build | SUCCESS (deployed alpha) | 1 point, for ~2 h |
| #1589 | verify | NOT_BUILT (quiesce gate never sustained; matthew `caughtUp` flapped after its conductor restart) | 0 points |
| #1590 | build (10:00 timer) | SUCCESS (re-deployed) | 1 point, for ~1 h |
| #1591 | verify | FAILURE (pre-existing apex `p2p.caughtUp` + a restart-window asset shed) | 0 points |
| #1592 | verify (`[edge:validate-only]`) | FAILURE (the apex `caughtUp` alone; the shed ride landed) | 0 points |

The measure sat at 4 of 5 for the rest of the shift while every build-class pipeline was green, and the orchestrator run that dispatched a validate-only sibling inherited its FAILURE too.

## The fix for the next measure script

Score the edge leg on the last completed build whose parameters say `RUN_CLASS=build` (or `VALIDATE_ONLY=false`), read from `api/json?tree=builds[number,result,actions[parameters[name,value]]]`, and score fleet validation as its own leg if the Objective wants it — never fold a verify sibling's fleet reading into "did the push land". A NOT_BUILT verify sibling is a no-measure (museum: NOT_BUILT ≠ regression), not a red.
