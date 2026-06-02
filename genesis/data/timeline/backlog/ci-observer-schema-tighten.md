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
recurrence: 2
source_shifts:
  - "2026-05-11"
  - "2026-05-16"
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

## Shape of the fix (code-domain)

Tighten the observer's output schema + validator: forbid specific test names in
`primary_failure.evidence`; reject (or null) `estimatedDuration` on cascade-classified builds.
This keeps the tier boundary (`feedback_haiku_observe_only_no_specifics`) and the
cascade-ratio discipline (`feedback_cascade_hidden_test_surface`) machine-enforced.

## Acceptance

An observer output with a specific test name in evidence, or an `estimatedDuration` on a
cascade build, fails schema validation.
