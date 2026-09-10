---
id: "backlog-ci-observer-schema-tighten"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Tighten ci-observer (Haiku) schema — forbid specific test names in primary_failure.evidence; reject estimatedDuration on cascade builds"
slug: "ci-observer-schema-tighten"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "CI/tooling"
recurrence: 3
source_shifts:
  - "2026-05-11"
  - "2026-05-16"
  - "2026-09-10"
domain: "code"
relatedNodeIds:
  - "memory:feedback_haiku_observe_only_no_specifics"
  - "memory:feedback_cascade_hidden_test_surface"
tags: [ci, tooling, observer, haiku, schema, code-domain, recurring]
shift_objective: |
  The ci-observer (Haiku tier) is meant to report API-grounded facts only, leaving specific
  test-name attribution to the Sonnet ci-investigator. But the observer's output schema is
  loose enough that it (a) puts specific test names into `primary_failure.evidence` (hallucinated
  specificity Haiku isn't licensed for) and (b) misuses `estimatedDuration` on cascade builds
  where the field is meaningless (observed 2026-05-11, 05-16).
  Resolve it by tightening the observer's output schema: forbid specific test names in
  `primary_failure.evidence` (the schema enforces "no specifics" per
  feedback_haiku_observe_only_no_specifics) and reject / null `estimatedDuration` on
  cascade-classified builds. This is code-domain (the observer's JSON schema + validator).
  Done when an observer output containing a specific test name in evidence, or an
  estimatedDuration on a cascade build, fails schema validation.
---

# Tighten the ci-observer output schema

## Why this matters

Code-domain. The Haiku observe-only boundary is a known, load-bearing rule
(`feedback_haiku_observe_only_no_specifics`) — but a rule that isn't schema-enforced gets
violated. Schema enforcement is the mechanism that keeps Haiku's tier honest and reserves
specificity for the Sonnet investigator.

## The failure shape

- The observer emits a specific test name in `primary_failure.evidence` — specificity Haiku
  is not licensed to assert.
- The observer emits `estimatedDuration` on a cascade build, where the field has no meaning.
- **Third shape (2026-09-10, recurrence 3): a confident false negative on job resolution.**
  Dispatched to summarize `elohim-orchestrator` #1845, the observer reported the job "does not
  appear in the current Jenkins instance" and offered "build pruned / job no longer exists /
  never completed on this controller" — with `status: not_found` and `confidence: low`. All
  three suggestions were wrong; the build existed and was UNSTABLE. Two mechanical causes,
  neither of which the observer is equipped to notice:
  1. **Multibranch path.** `getBuild(jobFullName: "elohim-orchestrator")` returns empty; the
     real path is `elohim-orchestrator/dev`. Every elohim pipeline is a multibranch job, so the
     bare name never resolves.
  2. **Paginated discovery read as exhaustive.** The observer fell back to `getJobs`, which
     caps at 10 items and returns them alphabetically — page 1 is `bootcamp*`/`devspaces*` and
     never reaches `e`. It read "10 jobs, none matching elohim" as "the job does not exist."

  This is worse than over-specificity: the observer's *absence* claims are unbounded by the
  API, so a null result gets narrated as substantive infrastructure loss. A triage agent that
  trusted it would have chased a phantom pruned-job incident instead of the real defect.

## Shape of the fix (code-domain)

Tighten the observer's output schema + validator: forbid specific test names in
`primary_failure.evidence`; reject (or null) `estimatedDuration` on cascade-classified builds.
This keeps the tier boundary (`feedback_haiku_observe_only_no_specifics`) and the
cascade-ratio discipline (`feedback_cascade_hidden_test_surface`) machine-enforced.

For the third shape the fix is in the observer's *agent definition*, not the schema:

- State the multibranch contract explicitly — elohim Jenkins jobs resolve as
  `<job>/<branch>` (`elohim-orchestrator/dev`, `elohim-genesis/dev`, `elohim-edge/dev`,
  `elohim-holochain/dev`); a bare job name resolving empty means *wrong path*, not *absent job*.
- Forbid inferring absence from `getJobs`: it is a paginated, alphabetical, 10-cap listing and
  is never evidence that a job does not exist.
- Schema side: a `not_found` context must carry the exact queried paths that returned empty,
  and may not be accompanied by narrative causal suggestions ("pruned", "removed", "never
  completed") — those are investigator-tier claims the observer cannot ground.

## Acceptance

An observer output with a specific test name in evidence, or an `estimatedDuration` on a
cascade build, fails schema validation.
