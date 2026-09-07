---
id: "backlog-test-mesh-forces-shem-unavailable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "`just test mesh` hard-exports ELOHIM_REMOTE_COMPUTE_STATUS=unavailable (justfile:81) so every @requires:shem scenario is skipped green — a household stand-in leg (delegated-sweettest with jessica as provider) can only run through the a2o escape hatch"
slug: "test-mesh-forces-shem-unavailable"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:operator-runtime-surface"
tags: [a2o, justfile, requires-cap, delegated-compute, dev-loop]
---

**Evidence (2026-09-07):** the compute authority leg ran via `cd genesis/a2o && ELOHIM_REMOTE_COMPUTE_STATUS=available pnpm exec cucumber-js features/dataplane/delegated-sweettest.feature …` because the `test mesh` recipe forces the cap unavailable and `steps/common.steps.ts:740` skips unavailable `@requires:` scenarios (a green run that measured nothing). Cure options: (a) make the export defaultable in the justfile (`${ELOHIM_REMOTE_COMPUTE_STATUS:-unavailable}`) so an explicit household stand-in can opt in; (b) add a second tag vocabulary (`@requires:owned-substrate`) for legs that a household peer can stand in for. The receipt must always name the stand-in, never a shem result. **Done when:** `just test mesh features/dataplane/delegated-sweettest.feature` can run the three scenarios on the household mesh with an explicit opt-in, and the report labels the lane.
