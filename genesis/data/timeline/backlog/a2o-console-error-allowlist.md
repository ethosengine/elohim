---
id: "backlog-a2o-console-error-allowlist"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Strict console-error After-hook needs a per-scenario @allow-doorway-flake allowlist so env flake doesn't mask passing assertions"
slug: "a2o-console-error-allowlist"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "genesis/a2o"
recurrence: 1
source_shifts:
  - "2026-05-07"
domain: "code"
relatedNodeIds:
  - "memory:feedback_a2o_is_human_experience_not_dev_bugs"
  - "memory:feedback_cascade_halt_masks_failures"
tags: [genesis, a2o, cucumber, console-error, allowlist, code-domain]
shift_objective: |
  The strict console-error After-hook fails any scenario whose page logged a console error,
  which is the right default — but environmental doorway flake (a transient proxy/network
  error logged to console) then fails scenarios whose actual assertions passed. There's no way
  to say "this specific scenario tolerates this specific known-environmental error" (observed
  2026-05-07).
  Resolve it with a per-scenario allowlist tag — e.g. `@allow-doorway-flake` — that the
  After-hook reads to permit a narrow, named set of known-environmental console errors for that
  scenario only, while keeping the strict default for everything else. This is code-domain (the
  cucumber After-hook + a tag convention). Keep the allowlist NARROW and named so it can't
  become a blanket mute. Done when a scenario tagged @allow-doorway-flake tolerates the named
  environmental error while still failing on any other console error.
---

# Per-scenario console-error allowlist for known environmental flake

## Why this matters

Code-domain. A strict console-error gate is correct, but without an escape valve for named
environmental noise it produces false failures that erode trust in the gate — and the usual
"fix" is to weaken the gate globally, which is worse. A narrow, named, per-scenario allowlist
keeps the strict default everywhere else.

## The failure shape

- The After-hook fails any scenario whose page logged a console error.
- Transient doorway/proxy flake logs an error even when the scenario's assertions passed.
- No per-scenario tolerance → the passing scenario fails on environmental noise.

## Shape of the fix (code-domain)

A tag (e.g. `@allow-doorway-flake`) that the After-hook reads to permit a **narrow, named**
set of known-environmental console errors for that scenario only; strict default preserved
elsewhere. Keep the allowlist scoped so it can't become a blanket mute
(`feedback_cascade_halt_masks_failures` — don't bury real failures).

## Acceptance

A scenario tagged `@allow-doorway-flake` tolerates the named environmental error while still
failing on any other console error.
