---
id: "backlog-genesis-pipeline-seeds-on-lint-scoped-changes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The genesis pipeline seeds the live fleet on lint-scoped changes — an a2o script/doc commit runs Seed Substrate"
slug: "genesis-pipeline-seeds-on-lint-scoped-changes"
written: "2026-08-30"
author: "shift 2026-08-30T03-25-workspace-peer-native-content-sync"
status: "open"
priority: "high"
jobs: [elohim-genesis, elohim-orchestrator]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds: ["habit:dataplane-convergence", "backlog-conductor-pin-ships-base-binary"]
tags: [ci, orchestrator, genesis, change-detection, native-content-path, museum-candidate]
---
## Measured 2026-08-30 23:40Z
Commit `1dfece801` touched only `genesis/a2o/scripts/device-ceremony.ts` (a test script) and backlog
notes. Orchestrator #1759 dispatched `elohim-genesis`; genesis #1530 ran through **Seed Database
SUCCESS → Seed Substrate** against the live alpha fleet. The trigger is correct at the STEP level
(the script matches the `lint-a2o` step's `genesis/a2o/**` input glob), but the pipeline runs its
seed-to-fleet stages regardless of WHICH step's inputs changed — so a lint-scoped a2o edit re-seeds
content. This is the monolithic-pipeline reflex the device-native content path exists to retire: a
doc/script/lint change must never seed the fleet.

## Why it matters now
We just proved a content update (the manifesto EPR) can move device-native with no pipeline. Every
incidental re-seed contradicts that and churns the fleet mid-convergence. (It did NOT clobber tonight's
manifesto head: the seed authors on matthew's per-root chain and cannot declare an earned canonical —
progenitor is null on alpha — so W2's earned cross-root canonical head still wins the election.)

## Cure (options)
1. Genesis Jenkinsfile gates Seed Database/Seed Substrate behind seed-relevant input changes
   (`genesis/data/{humans,lamad,...}/**`, `genesis/seeds/**`, `cluster-state.yaml`), skipping them
   when only lint-scoped inputs (`genesis/a2o/**`, docs, scripts) changed — per-step change detection
   the build-manifest already expresses but the pipeline ignores at stage granularity.
2. Or the orchestrator dispatches only the matched steps rather than the whole genesis job.
Deeper: as content moves to the device-native path, the genesis seed becomes fixture-only; its
fleet-seed stages should be the exception (explicit), not the default on any genesis/** change.
