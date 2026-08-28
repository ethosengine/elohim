---
id: "backlog-relationships-table-per-host-rides-no-sync-plane"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The content relationship graph is per-host — 8,920 relationships on the seeding peer, 0 on the two peers whose content rows converged — it rides no sync plane"
slug: "relationships-table-per-host-rides-no-sync-plane"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
---

## Measured (household mesh, dual transport, 2026-08-28T04:45Z, binaries at 6e4fa4389)

After `just seed apply local content` through matthew (A): content rows converged A=3,452 → B=3,441 / C=3,449 within ~12 min, but `/db/relationships` reads **A=8,920, B=0, C=0** and stays there. The relationship table is written only by the seeder's extractor on the seeding host (bulk create) and by nothing else; no sync plane (Automerge docs, inventory gossip, iroh manifest board) carries it, and the reconcile controller has no relationship stream.

## Why it matters

The content graph is native Rust in storage (`ContentGraphResolver`, memory `project_content_graph_native_rust_not_cozo_apollo`) — so a peer's graph is exactly what it seeded itself. A doorway fronting B or C serves related-content, paths-by-prerequisite and graph queries from an EMPTY graph while its content is complete. On the fleet this is masked because the genesis seeder writes every peer directly; on any peer that adopts content from the network rather than from a seeder (the P1 direction the dataplane is moving toward) the graph never arrives.

## Cure shape (design decision, p2p-design-gate)

Relationships are attributes of two notarized content nodes (Linked, A2). Either derive them on adoption (the extractor already runs from `metadata.relationships` / `relatedNodeIds` — run it on projected rows, not only on seeded ones, so the graph is a projection of content and needs no plane of its own), or carry them on the content Automerge doc. The first is cheaper and keeps the graph derivable; it also removes the seeder as the only writer.
