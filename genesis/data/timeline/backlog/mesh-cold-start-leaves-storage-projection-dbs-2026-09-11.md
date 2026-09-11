---
id: "backlog-mesh-cold-start-leaves-storage-projection-dbs"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Household mesh cold start wipes the conductor sandboxes and the doorway archive but leaves every storage peer's projection DBs — the mesh restarts with commitments, roots and collectives minted by agents that no longer exist"
slug: "mesh-cold-start-leaves-storage-projection-dbs-2026-09-11"
written: "2026-09-11"
author: "doorway-federation sprint — serving-receipt attempts 1–3 and Task 17b (mesh agent)"
status: "open"
priority: "medium"
tags: [hc-mesh, cold-start, elohim-storage, projection, fixtures, deliverability, D8]
relatedNodeIds: []
cites:
  - app/elohim-app/scripts/hc-mesh.sh
  - genesis/a2o/features/dataplane/epr-app-deliverability.feature
  - genesis/orchestrator/scripts/serving-receipt.mjs
---

# Cold start is not cold for storage

## Observed (2026-09-11)

- `just mesh stop && just mesh start` (peer 0's admin port silent) regenerates the conductor sandboxes and, since
  bceeb0424, drops the doorways' Mongo archive — but `/tmp/elohim-local-mesh/<peer>/{content.db*, graph.db*, cache,
  blobs, blobs_iroh}` persist for matthew, jessica and james.
- Consequences measured the same day: `seed-household-formation` / `seed-spool-custody` red with *"caller is not a
  current Steward of collective:…"* on a collective minted by a destroyed agent (Task 17b report); and the
  serving-receipt lane `epr-app-deliverability.feature` refused twice after a cold start — *"alpha-A: a root commitment
  already exists; fixture refuses to replace it"* — because the Prologue's root commitment from the previous mesh life
  was still in storage's projection. Two receipt attempts lost (07:47, 07:49) before the DBs were wiped by hand.

## Why it matters

A "cold" start that keeps projected truth from a dead network is the local analogue of the fleet's stale-shell class:
the dataplane's own honesty rules (heal fills, never moves; the DHT is the manifest) cannot hold when the projection
outlives the DHT it projected. Every station that borrows a root, mints a collective, or counts commitments reads a
ghost.

## Fix (bounded)

In `hc-mesh.sh`'s cold-start branch (the block that drops `$MONGO_DIR`, ~:3169), also remove each peer's projection
state — `content.db*`, `graph.db*`, `cache/`, `blobs/`, `blobs_iroh/` — under the same `MESH_KEEP_*` discipline
(`MESH_KEEP_STORAGE_DB=1` to study the skew deliberately). Log one line per peer. Update the `just mesh start` gospel
line and the hc-dev-orchestrator package in the same pass (`dev-lifecycle-script-sync`). Verify: cold start →
`epr-app-deliverability.feature` 5/5 without a manual wipe.
