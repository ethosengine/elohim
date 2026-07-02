---
id: "backlog-minimal-cast-story-retune-three-epics"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retune resiliency/delivery/projection epic stories to the minimal 4-cast (household trio + adam)"
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
tags: [stories, scope, spine:sync-scale-honesty]
---

# Retune the three epics' stories to the minimal 4-cast

Operator directive (2026-07-02, on reading the starvation ceiling menu): rather
than asking shem for capacity, scale the deployed cast — and the STORIES — to
the minimal footprint that virtualizes the resiliency, delivery, and projection
epics. The deployment half landed (deployments.json: 10 shem personas
suspended, active cast = matthew/jessica/james/adam). The rails auto-pend
scenarios referencing suspended personas (isHumanDeployed → 'pending').

This item is the story half: sweep the three epics' a2o features + fixtures for
scenarios that hard-reference suspended personas or assume the 14-cast, and
retune them to the 4-cast (or tag scenario-level @requires with the wider-cast
need) so epic coverage runs ACTIVE instead of accumulating silent pendings.
Judge-owned surface (a2o) — this is deliberate follow-up work, not a shift
side-edit. Check especially: resilience/ (placement-diversity scenarios that
may want >4 peers — those get @requires tags naming the need), federation
peer-mesh counts, and any seeded-fixture persona lists.
