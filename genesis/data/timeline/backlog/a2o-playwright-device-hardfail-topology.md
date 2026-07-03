---
id: "backlog-a2o-playwright-device-hardfail-topology"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steps/ui/topology.steps.ts hard-fails (not skips) when no Playwright device — 5 blob-durability scenarios red in non-browser CI runs"
slug: "a2o-playwright-device-hardfail-topology"
written: "2026-07-03"
author: "blob-durability-suite-green shift"
status: "resolved"
priority: "medium"
ci_status: resolved
resolved: "2026-07-03"
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

Confirmed via edge #1145 (2026-07-03, first live run past the `E2E_STORAGE_URL` +
`ALLOW_SEED_SHARD_MANIFEST` gates): **5** `@concern:blob-durability` scenarios (not 3 as
originally estimated) go red for this reason every non-browser CI run — 4 in
`observable-distribution.feature` ("Operator can see their household device cluster",
"Peer-topology page aggregates by household, not by peer", "Reciprocity page shows
inflow, outflow, and net hosting", "Doorway operator dashboard topology tab is
reachable") plus 1 in `resilience.steps.ts`'s viewport-archetype step ("Resilience
hypercard stays fully inside a phone viewport" — a distinct `assert.fail('No Playwright
device found')` site, not `requirePwDevice` itself, but the same root cause).

## Why not fixed in the same shift

Different root-cause class from the seeding/data bugs fixed alongside this entry
(commit `f0c295a58`) — `requirePwDevice`/similar helpers in `topology.steps.ts` are
shared across other feature suites beyond blob-durability, so the right fix (skip
gracefully vs. tag these scenarios `@browser-only` vs. run the dataplane validation
stage in playwright mode) is a scoped design decision, not a one-line mechanical fix.

## Proposed next step

Either (a) tag these 5 scenarios `@browser-only` (they genuinely need a rendered page)
so they cleanly HELD-skip on the non-browser `@dataplane` runner, or (b) make
`topology.steps.ts`'s device-requirement helper return `'pending'` like the rest of the
suite. (a) is safer/narrower; (b) is more broadly correct but needs auditing every
other consumer of that helper first.

**2026-07-03 follow-up:** (a) alone does NOT resolve this for the dataplane-validation
job — confirmed `scripts/ci/run-dataplane-validation.sh`'s cucumber invocation uses
`--tags '@dataplane and not @wip'`, which does not exclude `@browser-only`. The complete
(a)-shaped fix is two parts together: tag the 5 scenarios `@browser-only` in the feature
files, AND extend the `--tags` filter in `run-dataplane-validation.sh` to
`'@dataplane and not @wip and not @browser-only'`. Deliberately left undone by the
`blob-durability-suite-green` shift (2026-07-03) — changing the tag filter changes which
scenarios the measure command's own report counts, which reads closer to a measure-scope
decision than a mechanical fix; left for operator/next-shift judgment rather than made
unilaterally overnight.

## Resolution (2026-07-03, operator-authorized)

Operator authorized the recommended option (a). Part 1 (tagging) was already in
place — all 5 scenarios already carry `@browser-only` in
`observable-distribution.feature` (the 4 topology scenarios + "Resilience hypercard
stays fully inside a phone viewport"). Part 2 applied: extended the `--tags` filter in
`scripts/ci/run-dataplane-validation.sh` to
`'@dataplane and not @wip and not @browser-only'`, with an inline comment anchoring the
rationale.

**Verified via cucumber `--dry-run` (no substrate needed):** the blob-durability set
drops 20 → 10 scenarios; the full `@dataplane` suite drops 37 → 27. The entire
10-scenario delta is blob-durability's `@browser-only` scenarios — no other concern and
no non-browser scenario is affected. Coverage is preserved: `@browser-only` scenarios
still run under the playwright-mode jobs (`test:browser:e2e`, delivery-browser profile).

**Expected effect on the spine's blob-durability node:** `passed=3 failed=5` (RED) →
`passed=3 failed=0` (GREEN — the 5 structural hard-fails become clean HELD-skips). This
clears the failures and greens the concern; it does not by itself lift `passed` to ≥5
(the remaining non-browser pending scenarios flip to passed only when their live-substrate
preconditions are met — a substrate-timing condition, not a code gap). The closing CI
measurement lands when this reaches `dev` and edge re-runs the Dataplane Validation stage.
