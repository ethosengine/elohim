---
id: "backlog-ci-edge-a2o-support-load-unbuilt-storage-client-dist"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "a2o cucumber dies at support load on an unbuilt @elohim/storage-client dist — and the failure arrives as \"0 scenarios\", which reads as a measurement"
slug: "ci-edge-a2o-support-load-unbuilt-storage-client-dist"
written: "2026-09-19"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [11515c3943ed]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, a2o, cucumber, storage-client, build-order, support-load, measurement]
cites:
  - scripts/ci/run-dataplane-validation.sh
  - scripts/ci/run-dataplane-validation.test.sh
  - scripts/ci/run-mesh-quiesce-stage.sh
  - scripts/ci/tests/a2o-cucumber-sites-build-storage-client.test.sh
  - genesis/scripts/ci/install-substrate-runner.sh
  - genesis/a2o/cucumber.mjs
  - genesis/a2o/steps/dataplane/epr-app-deliverability.helpers.ts
  - elohim/sdk/storage-client-ts/package.json
---

# a2o support load fails on an unbuilt `@elohim/storage-client` dist

## The failure

`elohim-edge/dev` #1464, stage **Dataplane Validation**, result **FAILURE**
(upstream cause elohim-orchestrator/dev #1877, `VALIDATE_ONLY=true`):

```
+ bash /home/jenkins/agent/workspace/elohim-edge_dev/scripts/ci/run-dataplane-validation.sh
...
Error: Cannot find module '/home/jenkins/agent/workspace/elohim-edge_dev/genesis/a2o/node_modules/@elohim/storage-client/dist/index.js'
  code: 'MODULE_NOT_FOUND',
  path: '.../genesis/a2o/node_modules/@elohim/storage-client'
...
Findings: 0 (scenarios: 0)
declared 28 · permitted 0 · refused 0 · referred 0 · NOT MEASURED 28
ERROR: 0 dataplane scenarios ran — support load or tag filter is broken, this is NOT a pass.
ERROR: script returned exit code 3
Setting overall build result to FAILURE
```

Occurrence evidence (ledger fp `11515c3943ed`): **seen 1**, first_build 1464,
last_build 1464. Cross-build sweep of elohim-edge/dev 1460–1470 found the
fingerprint in **1464 only** (1468 unverified — two `searchBuildLog` timeouts).

## Verdict — **real**, and it predates its own build's changeset

Not a flake and not an over-capture. The error killed cucumber's support-file
load, so no scenario started; the script's own zero-scenario guard caught it
and failed the stage. Neither commit in #1464's changeset touches the runner,
`genesis/a2o/package.json`, or the SDK — the gap was standing.

Not museum trap #14 (benign-line over-capture): the lines below the capture say
the run measured nothing, not that a step succeeded.

## Root cause

`@elohim/storage-client` (`elohim/sdk/storage-client-ts`) declares
`"main": "dist/index.js"` and an `exports` map that resolves **only** into
`dist/`. Its `build` is `tsc`; there is no `prepare` script, so
`pnpm install --frozen-lockfile --filter '@elohim/a2o...'` **links the package
but never builds it**.

Cucumber loads the a2o support tree wholesale (`require: ['steps/**/*.ts']` in
`genesis/a2o/cucumber.mjs`, inherited by every profile), and
`steps/dataplane/epr-app-deliverability.helpers.ts` imports the SDK. So any CI
site that installs a2o and runs cucumber without building the SDK dies at load.

The import is recent — per the fix commit, "since `b344f533a` the a2o support
code imports `@elohim/storage-client`". A **source-side** change made a built
dist a prerequisite of every **consumer** site, and no consumer was updated.

### The shape worth keeping: a load failure wearing a measurement's clothes

Because no scenario ever starts, the failure presents downstream as
`scenarios: 0` / `NOT MEASURED 28`. Every tally that counts scenarios reads
that as a *measurement* (nothing passed) rather than as *the suite never
loaded*. `run-dataplane-validation.sh` survives this only because it carries an
explicit zero-scenario guard; the sibling site did not.

### Three sites, one required step, nothing shared

| Site | Built the SDK? |
|---|---|
| `genesis/scripts/ci/install-substrate-runner.sh` (genesis) | yes — always did |
| `scripts/ci/run-dataplane-validation.sh` (edge, Dataplane Validation) | **no** → edge #1464 |
| `scripts/ci/run-mesh-quiesce-stage.sh` (edge, mesh quiesce phase 2 saga) | **no** → same class, found by this triage |

## Current decision

Fixed at both edge sites and locked by a cross-site invariant test. Awaiting
disappearance confirmation from the harvester (job green-streak, no recurrence
of `11515c3943ed` past build 1464). The residual below is documented, not
silently carried.

## Fix trail

- `3f42852d3` (pre-existing, 2026-09-19, built as #1465) — added
  `pnpm --filter @elohim/storage-client build` to
  `scripts/ci/run-dataplane-validation.sh`, plus the harness assertion in
  `run-dataplane-validation.test.sh` (`.storage-client-built` marker, error text
  cites edge 1464). Verified live: #1465's log carries
  `> @elohim/storage-client@0.1.0 build … > tsc` and reached real scenarios
  (`permitted 4 · NOT MEASURED 24` vs #1464's `permitted 0 · NOT MEASURED 28`).
- **This triage** — `scripts/ci/run-mesh-quiesce-stage.sh`: the same build
  before the phase-2 saga cucumber run, and a `MESH-E2E: DID NOT LOAD` branch so
  a zero-scenario log is named rather than reported as `0/0 scenarios` (the
  conflation above). Phase 2 stays non-blocking by design.
- **This triage** — `scripts/ci/tests/a2o-cucumber-sites-build-storage-client.test.sh`:
  scans `scripts/ci/*.sh` + `genesis/scripts/ci/*.sh`, and for every file with a
  non-comment `cucumber-js` invocation requires either an in-file SDK build or
  an allowlist entry naming the earlier stage that builds it (the two
  `genesis/scripts/ci/e2e-verify-*.sh` sites qualify —
  `install-substrate-runner.sh` runs earlier in the same genesis workspace).

Local verification (this host):

- `bash -n scripts/ci/run-mesh-quiesce-stage.sh` → exit 0.
- `bash scripts/ci/run-dataplane-validation.test.sh` → `zero-scenario guard tests passed`, exit 0.
- `pnpm --filter @elohim/storage-client build` → exit 0, `dist/index.js` present.
- `bash scripts/ci/tests/a2o-cucumber-sites-build-storage-client.test.sh` → all
  four sites ok, exit 0.
- Negative test: a synthetic `scripts/ci/zz-negative-probe.sh` invoking
  `cucumber-js` with no SDK build made the invariant test exit 1 naming that
  file; probe removed.

## Residual (open, deliberately not taken here)

1. **The invariant test has no automatic runner.** No `gate.projects` entry owns
   `scripts/ci/**`, and `.husky/pre-push.bash` has no shell-test leg — the
   sibling `run-dataplane-validation.test.sh` and
   `scripts/ci/tests/deliverability-gate.test.sh` are in the same position. A
   guard nobody runs is museum trap #13's shape. Wiring a gate project or a
   pre-push leg for `scripts/ci/**` is the follow-up; it is pipeline wiring, not
   this fingerprint's fix.
2. **Genesis's coverage is an ordering assumption.** `e2e-verify-api.sh` /
   `e2e-verify-browser.sh` rely on `install-substrate-runner.sh` having run in an
   earlier stage of the same workspace, and that stage carries its own `when{}`
   gates (`PIPELINE_SKIPPED` + `SEED_DATA`). If those gates ever diverge from the
   E2E stage's, genesis inherits this failure. Recorded in the test's allowlist
   comment so the assumption is visible at the point of exemption.

## Evidence trail

- `elohim-edge/dev` #1464 console — Dataplane Validation stage, MODULE_NOT_FOUND
  stack, zero-scenario guard, exit 3 (quoted above).
- `elohim-edge/dev` #1465 console — the `tsc` build line and a measured run.
- ci-investigator run 2026-09-19 (this triage) — cross-build sweep 1460–1470.
- `.claude/data/ci-findings.jsonl` fp `11515c3943ed`.
