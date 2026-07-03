---
id: "backlog-a2o-playwright-device-hardfail-topology"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steps/ui/topology.steps.ts hard-fails (not skips) when no Playwright device — 3 blob-durability scenarios red in non-browser CI runs"
slug: "a2o-playwright-device-hardfail-topology"
written: "2026-07-03"
author: "blob-durability-suite-green shift"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [elohim-edge]
relatedNodeIds: [blob-durability]
tags: [ci, a2o, playwright, device-mode, resilience, topology]
cites:
  - genesis/a2o/steps/ui/topology.steps.ts
  - genesis/a2o/features/resilience/observable-distribution.feature
  - scripts/ci/run-dataplane-validation.sh
---

# `requirePwDevice`/`requirePwDevice`-style helpers in `steps/ui/topology.steps.ts` `assert.fail` instead of returning `'pending'`

## The failure

Discovered 2026-07-03 (edge #1144, first live run of the blob-durability suite past the
`E2E_STORAGE_URL` harness gate). `scripts/ci/run-dataplane-validation.sh` runs plain
`cucumber-js` — no `E2E_DEVICE_MODE=playwright` — so any scenario that opens a browser
page via `Matthew opens "/shefa/cluster"` etc. hits `requirePwDevice` in
`genesis/a2o/steps/ui/topology.steps.ts:46`, which `assert.fail`s
(`"Matthew has no Playwright device. Is E2E_DEVICE_MODE=playwright?"`) instead of
returning `'pending'` the way the rest of the resilience-suite's browser-gated steps do
(e.g. `requirePlaywright` elsewhere per `genesis/a2o/CLAUDE.md` → "Guard:
`requirePlaywright(this)` returns null in non-Playwright mode").

Three `@concern:blob-durability` scenarios in `observable-distribution.feature` (not
`@wip`, not `@browser-only`-tagged) go red for this reason every non-browser CI run:
"Operator can see their household device cluster", "Peer-topology page aggregates by
household, not by peer", "Doorway operator dashboard topology tab is reachable".

## Why not fixed in the same shift

Different root-cause class from the seeding/data bugs fixed alongside this entry
(commit `f0c295a58`) — `requirePwDevice`/similar helpers in `topology.steps.ts` are
shared across other feature suites beyond blob-durability, so the right fix (skip
gracefully vs. tag these scenarios `@browser-only` vs. run the dataplane validation
stage in playwright mode) is a scoped design decision, not a one-line mechanical fix.

## Proposed next step

Either (a) tag these 3 scenarios `@browser-only` (they genuinely need a rendered page)
so they cleanly HELD-skip on the non-browser `@dataplane` runner, or (b) make
`topology.steps.ts`'s device-requirement helper return `'pending'` like the rest of the
suite. (a) is safer/narrower; (b) is more broadly correct but needs auditing every
other consumer of that helper first.
