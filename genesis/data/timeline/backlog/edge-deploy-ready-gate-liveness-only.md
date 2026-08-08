---
id: "backlog-edge-deploy-ready-gate-liveness-only"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "edge deploy 'N/N peers Ready' gate is a :8090 liveness probe — it cannot see mesh non-participation"
slug: "edge-deploy-ready-gate-liveness-only"
written: "2026-08-08"
author: "integrator-session"
status: "backlog"
priority: "medium"
tags: [edge-pipeline, deploy-gate, observability, dht, mesh-participation]

relatedNodeIds:
  - "backlog-susan-kitsune2-gossip-never-attempts"
---

# "7/7 peers Ready" went green while a peer sat entirely outside the DHT

Edge #1326 reported `Edge deploy result for alpha: 7/7 peers Ready` while
susan had zero kitsune2 gossip attempts (see related item). The gate's
"Ready" is the pod `/health` liveness probe on :8090 — storage-layer
liveness — and says nothing about DHT/mesh participation. The deploy gate
therefore certifies a fleet that can be partially partitioned at the
gossip layer.

Shape of a fix (when picked up): the per-deploy gate (or the fleet-quiesce
probe that already runs in Dataplane Validation) should include a
per-peer mesh-participation signal — e.g. gossip-attempt count nonzero in
the post-deploy window, or `/db/p2p/conductor-diagnostics` peer-visibility
— so Ready means "in the mesh", not "process is up". Route per the
substrate trust contract runbook (probes are the authority); this is a
probe-coverage gap, not a new register.
