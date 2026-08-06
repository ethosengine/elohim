---
id: "backlog-minimal-cast-story-retune-three-epics"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retune epic stories to the coordination-ladder cast (7 peers, one instance per tier)"
slug: "minimal-cast-story-retune-three-epics"
written: "2026-07-02"
author: "shift-genesis-verdicts-green"
shift_objective: "post-close operator directive 2026-07-02: scale stories to the minimal footprint that virtualizes the epics"
status: "backlog"
priority: "medium"
themes: [a2o, scope, resiliency, delivery, projection, minimal-footprint]
relatedNodeIds:
  - "genesis/data/timeline/backlog/cluster-to-shem-p2p-request-starvation-11-peer-blackout.md"
  - "genesis/orchestrator/data/deployments.json"
tags: [stories, scope, habits:sync-scale-honesty]
---

# Retune the epics' stories to the coordination-ladder cast

Operator directive (2026-07-02, refined live): the minimal footprint is not a
pod count — it is a SCALE MODEL of the coordination ladder, one live instance
per tier, so the stories model HOW the protocol scales:

| Tier | Cast instance |
|---|---|
| intimacy | matthew↔jessica (within-household relationship) |
| family | the Dowell household trio (matthew/jessica/james, on-prem) |
| community / local | gertrude (stewarded elder household) + susan (neighbor household) in relationship with the family household — household topology is DATA-layer, not k8s placement |
| regional | on-prem region {m,j,j} vs shem region {adam,gertrude,susan,eve} — real membership for the regional_distribution folds + shem-side replication (RS across 4) |
| global | adam (federation anchor / elohim.host) ↔ matthew (doorway-alpha): the two-doorway convergence proven live 2026-07-02 |

Deployment half landed (deployments.json: active 7 = adam/matthew/jessica/
james/gertrude/susan/eve; suspended 7 = pete/terrance/frank/caleb/daniel/emma/
nancy; shem 11→4 pods). The rails auto-pend scenarios referencing suspended
personas (isHumanDeployed → 'pending').

This item is the story half: sweep the epics' a2o features + fixtures and
retune each scenario to the ladder cast — every scenario should NAME the
coordination tier it exercises (the cast table above is the legend), scenarios
hard-referencing suspended personas re-cast or take a scenario-level @requires
naming the wider-cast need (frank = the raspberry-pi pantry-tier candidate for
tiered-quilt stories; terrance = the 6Gi chromebook-edu heavyweight).
Judge-owned surface (a2o) — this is deliberate follow-up work, not a shift
side-edit. Check especially: resilience/ (placement-diversity scenarios that
may want >4 peers — those get @requires tags naming the need), federation
peer-mesh counts, and any seeded-fixture persona lists.
