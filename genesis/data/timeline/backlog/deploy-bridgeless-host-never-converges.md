---
id: "backlog-deploy-bridgeless-host-never-converges"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A persistently bridgeless serving host never converges the SPA head — heal leg is conductor-gated (SkipNoBridge) with no peer-pull fallback"
slug: "deploy-bridgeless-host-never-converges"
written: "2026-07-10"
author: "substrate-cure sprint (code-review P0.4 disposition)"
status: "open"
priority: "medium"
area: "substrate/replication-heal"
domain: "operator"
jobs: [elohim, elohim-edge]
relatedNodeIds:
  - "memory:feedback_reach_head_replication_distinct_planes"
  - "memory:project_prod_main_lag_vs_alpha_dev"
cites:
  - genesis/data/timeline/backlog/bulk-seed-witness-bootstrap-single-head.md
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
tags: [substrate, replication, projection-reconcile, skip-no-bridge, spa-blob, deploy, single-head, peer-pull-heal]
---

# Persistently bridgeless host never converges the deployed head

## Context

The per-host `?deployTier=amber` diesel-direct write was deliberately retired
(`9f9c4aec4`, 2026-07-07): it minted divergent un-witnessed heads per backend —
the root cause of the "elohim.host stuck" incident. That retirement is correct:
trust is derived (DHT-witnessed or not), never a write mode; there is ONE
notarized head, authored through a conductor by `authorHeadOnce` and gossiped.

## The residual gap (replication plane, not head election)

A serving host whose OWN conductor bridge is persistently down never runs its
heal leg: `projection_reconcile.rs` `heal_decision(bridge_up, …)` returns
`SkipNoBridge` unconditionally when the bridge is down, and `run_heal`
(including `witness_bootstrap`) requires the own conductor. The DHT-witnessed
head exists and has gossiped — but this host never re-projects it into its
local SQLite row, so `lookup_slug_blob_hash` keeps serving stale bytes or
404ing new slugs until its conductor returns AND a heal tick completes.

This is availability/custody (replication plane) — the head election is fine.

## Fix direction (design needed, rust-architect scope)

Do NOT resurrect a diesel-direct write. The discovery leg (`run_discovery`)
already runs conductor-free from boot; the design question is whether the
reconcile loop can pull an already-DHT-witnessed head projection from a
healthy peer (P2P/HTTP fan-out) when the local bridge is down — the row would
carry the witnessed head's provenance, not a locally-minted one, so the
single-head invariant is preserved. Sibling concern to the ingest-side
witness bootstrap ([[bulk-seed-witness-bootstrap-single-head]]).
