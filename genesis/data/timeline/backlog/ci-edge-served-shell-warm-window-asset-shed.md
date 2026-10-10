---
id: "backlog-ci-edge-served-shell-warm-window-asset-shed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Right after a roll, doorway-alpha serves its warm last-reconciled shell while shedding that shell's assets 503, and the served-shell scenarios read the shed as 'this doorway does not hold its entry script'"
slug: "ci-edge-served-shell-warm-window-asset-shed"
written: "2026-10-09"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [a529817acb61, ff1999d4c7f7]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, dataplane-validation, served-shell-boots, doorway-failover, warm-shell, catching-up-shed, shed-not-verdict, validate-only, post-roll]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1591/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1590/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1637/
  - genesis/a2o/features/dataplane/served-shell-boots.feature
  - genesis/a2o/steps/dataplane.steps.ts
  - genesis/a2o/steps/dataplane/epr-app-deliverability.steps.ts
  - genesis/a2o/src/framework/dataplane/surfaces.ts
  - doorway/doorway-service/src/render/warm_shell.rs
  - scripts/ci/verify-served-shell.sh
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# Served-shell scenarios read the post-roll asset shed as a missing bundle

## The failure

elohim-edge/dev #1591 (Dataplane Validation, the VALIDATE_ONLY sibling of timer run #1590, commit b4a391115) failed two `served-shell-boots.feature` scenarios on alpha-A:

- fp `a529817acb61`, "The site root names the declared entry script and reachable assets":
  `alpha-A: the page served at "/" names 7 asset(s) this doorway does not hold — script "polyfills-X2TQPNDQ.js" -> HTTP 503; script "main-B6EVYVJ3.js" -> HTTP 503; … [shell provenance, re-read: x-elohim-bundle=last-reconciled x-content-address=absent x-elohim-freshness=amber x-ssr-skipped=absent]`
- fp `ff1999d4c7f7`, "Client bootstrap completes and renders through each doorway" (alpha-A row):
  `alpha-A: the app threw while starting at https://doorway-alpha.elohim.host/ — navigation/boot: TimeoutError: page.waitForFunction: Timeout 30000ms exceeded.`

Occurrence: seen 1 each, first_build = last_build = 1591.

## Verdict

**A transient, read as a verdict.** The doorway's behaviour is as designed; the test is the site at fault. This is museum trap #19, "an error that establishes nothing about the work is surfaced as a verdict", at one more site that answers for itself.

Evidence:

1. #1591 started about 8 seconds after #1590's roll finished, so it measured the warm-up window.
2. The shell carried `x-elohim-bundle: last-reconciled` and freshness `amber`. That is the warm shell `warm_shell.rs` serves before the upstream answers. Its module doc says the cold-and-unavailable path sheds 503 under the catching-up contract.
3. In the same build, a few minutes later, `verify-served-shell.sh` polls with a deadline. It reported `https://doorway-alpha.elohim.host/ boots at head sha256-10e1f51d… (entry main-B6EVYVJ3.js, x-elohim-bundle: confirmed) after 2 attempt(s)`. That is the same entry script that had answered 503.
4. A read-only probe on 2026-10-09 got 200 for `/`, `/main-B6EVYVJ3.js` and `/polyfills-X2TQPNDQ.js`.
5. The bootstrap timeout follows directly from (1): a browser whose entry script answers 503 never sets `data-app-ready`. Both fingerprints are one concern.

The genesis red that the dispatch asked about is **not this class**. elohim-genesis/dev #1637 and #1639 fail the bootstrap scenario at `browserType.launch: Executable doesn't exist at /root/.cache/ms-playwright/chromium_headless_shell-1217/…`. The browser was never installed in the genesis API-level run, so that red came before the roll and is unrelated to it. Edge #1450/#1452 hit and closed the same shape in `scripts/ci/run-dataplane-validation.sh`. The harvester did not capture it, because the line is not an `AssertionError`.

## Root cause

The page read in the static scenario rides the documented catching-up shed (`getRawRidingCatchUp`). The asset reads in `findUnresolvedAssets` did not; they used a single `getRaw`. The browser scenario cannot ride a shed at all. A doorway that has just restarted serves its warm shell from its own archive at once, and for a few seconds sheds that shell's assets until its upstream answers. Both scenarios turned those seconds into a "does not hold" or "threw while starting" verdict.

## Current decision

Fixed in a2o, pending disappearance. Asset reads now ride the catching-up shed under one shared budget (`CATCHUP_RIDE_TIMEOUT_MS`), and only the documented shed body is ridden: a 404, a plain 503 or a connect error still fails on the first read. The browser visit first rides the shell and its same-doorway assets past the shed (`rideShellPastCatchUp`), then opens the browser. A shed that outlasts the budget still fails, with the ride time named.

Residual question, owned by the doorway lane and not by CI: should a doorway with a warm shell but cold assets serve the catching-up page instead of a shell its own assets cannot back? Today a person arriving in that window gets a blank page for seconds. The trust-contract runbook's restart-churn invariant covers the window. It does not cover the blank page.

## Fix trail

- `genesis/a2o/steps/dataplane.steps.ts`: `findUnresolvedAssets` rides the shed with a shared deadline. New export `rideShellPastCatchUp`. The step ceiling is derived as `CATCHUP_RIDE_STEP_TIMEOUT_MS + 60s`.
- `genesis/a2o/steps/dataplane/epr-app-deliverability.steps.ts`: the browser-visit step rides first. Its ceiling is `CATCHUP_RIDE_STEP_TIMEOUT_MS + 120s`.
- `genesis/scripts/ci/e2e-verify-api.sh`: installs Playwright chromium before the API-level cucumber run. This fixes the separate genesis class.
- Local verification: `tsc --noEmit` and `eslint` are clean. Run against live alpha with `cucumber-js --tags '@e2e and @dataplane' --name …`: 3 scenarios, 24 steps passed. The fleet was warm, so the ride path was not exercised live; its ride semantics are covered by `src/framework/dataplane/__tests__/surfaces.test.ts`.
