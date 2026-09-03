---
id: "backlog-ci-edge-depends-on-holochain-rebuilds-dna-without-dna-change"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every storage/doorway push pays the ~60 min DNA sweettest before edge can roll — elohim-edge dependsOn elohim-holochain, and the orchestrator satisfies the edge by REBUILDING the DNA pipeline even when no DNA source changed, instead of reusing the last green hApp artifact"
slug: "ci-edge-depends-on-holochain-rebuilds-dna-without-dna-change"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch"
status: "open"
priority: "medium"
ci_status: open
jobs: [elohim-orchestrator, elohim-holochain, elohim-edge]
relatedNodeIds: []
tags: [ci, orchestrator, dispatch, principle-7, over-build, elohim-holochain, elohim-edge, artifact-reuse, cycle-time]
cites:
  - elohim/holochain/build-manifest.json
  - elohim/holochain/dna/build-manifest.json
  - genesis/orchestrator/graph-walker.mjs
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

## Measured

Wave 5 of the rung-5 batch (2026-09-02, `8181d60a8..7513654f6`) touched
`elohim/elohim-storage/src`, `doorway/doorway-service/src`, `elohim/sdk/storage-client-ts`,
`scripts/ci`, and `genesis/a2o` — no file under `elohim/holochain/dna/**`. Orchestrator #1787
dispatched `elohim, elohim-storybook, elohim-edge, elohim-holochain, elohim-genesis`.
`elohim-holochain` #1417 ran the full DNA pack + sweettest (~60 min on #1416) before
`elohim-edge` could start, because `elohim/holochain/build-manifest.json` declares
`dependsOn: ["elohim-holochain"]` and the orchestrator resolves a dependency edge by dispatching
the upstream pipeline, not by checking whether its inputs changed.

Cost this shift: three waves × ~60 min of DNA build gating three edge rolls whose diffs never
touched a zome. Worse, the in-flight edge roll of wave N is superseded/aborted by wave N+1's plan
(#1412 by #1785, #1413 by #1787), so an edge roll only completes when a wave is followed by
≥2 h of quiet — see `ci-orchestrator-supersede-aborts-in-flight-edge-rolls`.

## The change

Dependency-edge dispatch should reuse the upstream's last green artifact when the upstream's
own watch globs matched nothing in the range: `graph-walker.mjs` already knows per-pipeline
matched patterns; an edge whose upstream has `matched: []` and a green baseline artifact
(`elohim-holochain:build-happ` from the last SUCCESS) should bind the consumer to that artifact
instead of enqueueing the upstream. The `[build:dna]` tag stays the explicit override. The
"same-wave dispatch bakes the PREVIOUS happ" trap (memory `project_pipeline_dispatch_ordering`)
is the inverse hazard and must stay true when a DNA change IS in the range.

## Done when

An orchestrator run whose range touches `elohim/elohim-storage/**` and nothing under
`elohim/holochain/dna/**` prints a Build Plan line binding `elohim-edge` to the prior
`elohim-holochain` artifact and dispatches no DNA build; a range touching a zome still
dispatches both, in order.

## 2026-09-03 observation — the same over-build in a second costume (edge #1422)

A DNA-pipeline rebuild whose packed hashes are byte-identical to the baseline (the DNA Hash Guard
passed on elohim-holochain #1423) still republishes the `elohim-happ:dev-latest` OCI artifact with a
NEW digest. `resolve-happ-digest.sh` stamps that digest into the conductor pod template
(`elohim.host/happ-digest`), so the next edge deploy rolled every conductor StatefulSet
(`statefulset.apps/<prefix>-conductor configured` → `rollout status … --timeout=600s` per peer,
staggered) although nothing the conductor runs changed. Cost on 2026-09-03: the doorway fix that
was the point of the deploy waited behind a full conductor fleet roll. The digest that should
decide a conductor roll is content-derived — the packed DNA hash set (what the guard already
computes), not the artifact's OCI digest — so a rebuild that changes no DNA rolls no conductor.
