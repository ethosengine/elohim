---
id: "backlog-ci-genesis-topology-tab-loading-race"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway topology-tab a2o step point-in-time-checks stewards-count/retry, racing the /admin/dashboard/topology fetch (reads the loading spinner as neither)"
slug: "ci-genesis-topology-tab-loading-race"
written: "2026-07-19"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [260e63ac086d]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, a2o, playwright, browser-tier, topology, doorway-dashboard, test-timing-race, wait-for-terminal-state]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1327/
  - genesis/a2o/steps/ui/topology.steps.ts
  - genesis/a2o/features/resilience/observable-distribution.feature
  - doorway/doorway-app/src/app/components/dashboard/tabs/topology-tab.component.ts
  - genesis/data/timeline/backlog/a2o-playwright-device-hardfail-topology.md
---

# `the snapshot shows known stewards and recent gossip windows` snapshots component state point-in-time instead of waiting for it to settle

## The failure

```
AssertionError [ERR_ASSERTION]: Topology panel rendered but neither stewards-count nor retry button is visible
  at E2EWorld.<anonymous> (…/genesis/a2o/steps/ui/topology.steps.ts:591:18)
```

Occurrence evidence: fingerprint `260e63ac086d`, seen 1×, first=last build **elohim-genesis #1327** (build result UNSTABLE). Scenario **"Doorway operator dashboard topology tab is reachable"**
(`features/resilience/observable-distribution.feature:116`). The step trace shows every
precondition green — the device exists and the panel renders:

```
✔ Given human "Matthew" is logged in on doorway-app "alpha" with device   # steps/fixture-humans.steps.ts:116
✔ And an operator opens the doorway admin dashboard                        # steps/ui/topology.steps.ts:130
✔ When the operator clicks the "topology" tab                             # steps/ui/topology.steps.ts:545
✔ Then the tab renders a federation snapshot from "/admin/dashboard/topology"  # steps/ui/topology.steps.ts:560
✖ And the snapshot shows known stewards and recent gossip windows          # steps/ui/topology.steps.ts:576
```

The substrate probe for this run reads `remote pool (shem): AVAILABLE — Full topology
available — no reduced-scope gating`, so this is **not** a reduced-scope / env-blocked
skip and **not** the `NO_PW_DEVICE` device-hardfail concern
(`a2o-playwright-device-hardfail-topology.md`, fp `5dba80d982e1`) — the device is present
and the failing assertion is the tolerant success-or-error check itself.

## Verdict — real (test-timing race, not a product regression)

`doorway/doorway-app/.../topology-tab.component.ts` renders the panel container
(`data-testid="operator-topology"`) in **all three** signal states — `loading()` shows a
spinner, `error()` shows a Retry button (`topology-retry`), `view()` shows the
stewards-count (`topology-stewards-count`). The prior step
(`the tab renders a federation snapshot`) waits only for the always-present container, so
it passes while `ngOnInit → refresh()` is still awaiting `GET /admin/dashboard/topology`.
The failing step then did a bare point-in-time `isVisible()` on both terminal testids with
no wait:

```ts
const stewardsVisible = await stewards.isVisible().catch(() => false);
const retryVisible = await retry.isVisible().catch(() => false);
assert.ok(stewardsVisible || retryVisible, 'Topology panel rendered but neither …');
```

The only state in which BOTH are absent is `loading()` — a resolved `view()` always shows
stewards-count and an `error()` always shows retry. So the assertion caught the component
mid-fetch: the endpoint was still in flight (plausibly slow on the degraded/UNSTABLE alpha
substrate this build ran against) and the point-in-time check read the loading spinner as
"neither present." Byte-identical assertion unchanged since 2026-05-06 (`d22e3f9540`);
first tripped at #1327 (seen 1×) — a latent race that surfaces when the endpoint is slow,
not a code change.

## Root cause

Point-in-time visibility snapshot of an asynchronously-settling component. The tolerant
`stewards-count OR retry` assertion was designed to accept both terminal paths (success and
error) but never waits for one to arrive — it races the in-flight fetch.

## Current decision

Bounded test-repair landed (below). Fix waits for whichever terminal testid appears first
rather than snapshotting, so a slow endpoint no longer false-reds; a genuine >15s hang of
`/admin/dashboard/topology` still fails (correctly — that is a real signal). Awaiting
disappearance confirmation: elohim-genesis green-streak ≥3 with no recurrence of
`260e63ac086d`. Not stamped `decompose_on_confirm` — the reusable a2o lesson (**wait for a
terminal state; never point-in-time `isVisible()` an async-loading component**) is worth
graduating into `genesis/a2o/CLAUDE.md`'s step-authoring conventions before the entry is
decomposed, alongside the sibling `requirePlaywright`-return-`'pending'` convention that
`a2o-playwright-device-hardfail-topology.md` already names.

## Fix trail

`genesis/a2o/steps/ui/topology.steps.ts` (`the snapshot shows known stewards …`, ~L576-601):
replaced the two bare `isVisible()` snapshots with a `Promise.race` of two
`locator.waitFor({ state: 'visible', timeout: 15_000 })` calls (each `.then(()=>true)
.catch(()=>false)` so neither leaks an unhandled timeout rejection). The race resolves
`true` the instant either terminal testid becomes visible within the window, `false` only
if neither settles — preserving the original assertion message.

Verified locally (no live alpha/substrate in this env — static + structural, not a live run):
- `pnpm exec tsc --noEmit -p .` (genesis/a2o) — clean.
- `pnpm exec eslint steps/ui/topology.steps.ts` — clean.
- `pnpm exec prettier --check steps/ui/topology.steps.ts` — passes.
- `npx cucumber-js features/resilience/observable-distribution.feature --dry-run --name
  "Doorway operator dashboard topology tab is reachable"` — all 7 steps resolve (no
  undefined-step / parse / wiring regression).

Ledger: `260e63ac086d` set `status: triaged`, `triaged_at_build: 1327`.
