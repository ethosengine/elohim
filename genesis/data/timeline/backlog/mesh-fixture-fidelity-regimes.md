---
id: "backlog-mesh-fixture-fidelity-regimes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Three fleet regimes the household mesh cannot model made the fleet a discovery instrument at ~45x mesh cost: single-plane transport (dual-source selection bug invisible), arc-Empty conductor churn, and late-joiner membership"
slug: "mesh-fixture-fidelity-regimes"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "habit:dataplane-convergence"
tags: [mesh, fixtures, simulacra, dev-loop, dual-transport]
---

The carried-election arc (2026-08-30/31) proved end-to-end on the mesh in ~2
minutes per hypothesis, then spent ~6 hours and 5 fleet rolls discovering
three gates the mesh could not have surfaced, one per roll:

1. **Single-plane fixture**: the mesh ran libp2p-only, so the reconcile
   peer-source selection bug (libp2p-if-present-ELSE-iroh — a dual fleet never
   polls iroh-only peers) was structurally invisible. Cure landed 2026-08-31
   (CompositeReconcilePeers) but was discovered BY the fleet.
2. **Arc-Empty churn**: stock mesh conductors converge arcs quickly; the
   fleet's restart-cadence-vs-arc-regrowth regime (all 32 agent-infos
   storageArc None) cannot be reproduced locally.
3. **Late joiner**: mesh peers boot together; a peer joining a running mesh
   between "rolls" is not a scenario the harness can stage.

Per the operator's 2026-08-31 course-set, the doctrine is "the fleet CONFIRMS,
never discovers" and the durable fix is the modeled test-fixture / simulacra
network that the system can deploy and drive WITHOUT an agent embedded in the
runtime. These three regimes are that simulacra's first scenario families.
