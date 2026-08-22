---
id: "backlog-dataplane-fleet-lane-lost-coverage-to-local-rescope"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet Dataplane Validation now skips 76/80 scenarios — the wave's local-mesh re-scoping removed the fleet lane's fixture context; give CI a fleet-side fixture manifest or split lane profiles"
slug: "dataplane-fleet-lane-lost-coverage-to-local-rescope"
written: "2026-08-22"
author: "orchestrator (overnight pipeline-landing shift)"
status: "open"
priority: "high"
tags: [a2o, dataplane-validation, fixtures, lane-scoping, ci, measurement]
---

# The fleet confirmation lane lost its measure to the local proving ground's gain

Edge #1374 (pre-wave) ran the dataplane suite against alpha: 71 scenarios,
52 passed / 14 failed / 5 pending — a real fleet measure. Edge #1376
(post-wave validate-only, 2026-08-22 11:45Z): **80 scenarios, 76 SKIPPED,
3 passed / 1 failed** — saga-status reads all 11 chapters `pending-env`.

Cause: the wave re-grounded many dataplane/saga scenarios to the household
mesh (household-mesh fixture Givens, live-fabric floors, `@requires` caps
resolved against the act1 lane contract). Locally that took the saga to
21/22 green — mesh-is-the-proving-ground working as intended. But CI's
fleet run provides no household fixture manifest / fabric context, so the
re-scoped `Before` gates hold nearly everything: the fleet CONFIRMATION
layer silently lost the coverage the habit checks name ("edge Dataplane
Validation" backs blob-durability, dataplane-convergence, doorway-failover).

The 4 fleet-executable scenarios that remain: reconcile-inventory 3 passed;
inventory-convergence "seed-facing doorway peer catches its projection up
under sustained gossip" FAILED (measured while gertrude/susan were still in
their post-deploy catch-up curve — retest after quiesce before treating as
a defect).

## Fix shape (design-first, not a tag revert)

- Give the FLEET lane its own fixture context: a fleet-side fixture manifest
  (peer names, storage URLs, doorway pair, PIDs n/a) generated at deploy
  time — the same shape hc-mesh-prologue writes locally — so
  household-mesh-fixture Givens resolve against alpha's real topology
  instead of skipping.
- OR split profiles explicitly: `@act:i` (household mesh) vs a fleet tag
  set, with the Dataplane Validation stage running the fleet set and the
  habit checks renamed to match. Either way the skip-count of the fleet
  lane becomes a monitored number, not a silent slide (a lane that skips
  95% must fail loudly — add a coverage floor to run-dataplane-validation.sh).

## Done when

The edge Dataplane Validation stage executes a non-trivial scenario count
against alpha again (≥ the pre-wave 71-scenario order), the saga chapters
read from fleet evidence instead of `pending-env`, and a coverage floor
guards the lane against silent skip-slides.
