---
id: "backlog-ci-edge-a2o-steps-glob-redeploys-fleet"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "An a2o step-definition-only change dispatches a FULL elohim-edge build + alpha fleet redeploy — the dataplane-validation source glob `genesis/a2o/steps/**` triggers deploy, not validate-only"
slug: "ci-edge-a2o-steps-glob-redeploys-fleet"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "backlog"
priority: "high"
ci_status: open
fingerprints: []
jobs: [elohim-edge, elohim-orchestrator]
relatedNodeIds: []
tags: [ci, elohim-edge, change-detection, over-build, measurement-by-deploy, a2o, dataplane-validation, principle-7, fleet-churn]
cites:
  - elohim/holochain/build-manifest.json
  - genesis/orchestrator/graph-walker.mjs
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1383/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1726/
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/data/timeline/backlog/onpush-eager-debt-inventory.md
---

## What happened (2026-08-25, orchestrator #1726 → edge #1383)

Push 887da4de3..54dedb119 touched `app/lamad`, `app/elohim-app`, `genesis/a2o/features/elohim-core/**`,
`genesis/a2o/steps/elohim-core/epr-link-hypercard.steps.ts`, `genesis/manifests/habits.yaml` and one
backlog doc. Predicted dispatch: `elohim` (app) + `elohim-genesis`. Actual: **`elohim-edge` #1383 as
well** — a full doorway/storage/edge-node build, Harbor push and `Deploy Edge Node - Alpha`, i.e. the
7-pod fleet restart (~20 min churn + hours of catch-up) the museum names the measurement-by-deploy
anti-pattern. During the churn both doorways went `degraded` (conductor.connected=false, upstream
circuits open) and, because the app deploy's browser head author had just succeeded while the shell
stayed `last-reconciled`, elohim.host and alpha.elohim.host served an index pointing at a `main-*.js`
that 404'd — a visible outage caused by a step-definition edit.

## Why

`elohim/holochain/build-manifest.json` → `dataplane-validation.inputs.sources` lists
`genesis/a2o/steps/**` (plus `features/dataplane/**`, `features/resilience/**`). The entry's own
description says its only job is to make sure the PIPELINE gets triggered when only the a2o test
surface changes so the suite is *measured* — but a triggered edge run also *deploys*, unless the
commit carries `[edge:validate-only]`. The glob is also far wider than the suite it guards: every
step file under `steps/` (elohim-core, lamad, auth, delivery …) matches, not just
`steps/dataplane/**` + the shared `common.steps.ts`/fixtures.

## Fix shape (bounded)

1. Narrow the sources to what the dataplane suite actually loads: `genesis/a2o/steps/dataplane/**`,
   `genesis/a2o/steps/common.steps.ts`, `genesis/a2o/steps/mode-aware.steps.ts`,
   `genesis/a2o/steps/fixture-*.steps.ts`, `genesis/a2o/src/framework/**` (verify against the
   `run-dataplane-validation.sh` cucumber load path before editing).
2. Make an a2o-only match dispatch edge in **validate-only** mode (the `[edge:validate-only]`
   behaviour) rather than build+deploy — graph-walker/orchestrator change: a source match whose
   only contributing entry has `no build/deploy dependency` should set the validate-only flag.
3. Regression: a2o-only commit → predicted set `{elohim-edge (validate-only)}`, no deploy stage.

Out of this shift's path scope (`app/lamad`, `app/elohim-app`, `genesis/a2o`); filed rather than
fixed mid-outage.
