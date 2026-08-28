---
id: "backlog-iroh-only-content-projection-loop-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pure-iroh storage syncs documents but never projects them into content rows — the acquisition/projection loop is hosted by the libp2p P2PNode, which iroh-only mode does not construct (homo-iroh: P0 green, P1–P4 red)"
slug: "iroh-only-content-projection-loop-gap"
written: "2026-08-28"
author: "M4 row-13 cut (post M0 shift)"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-iroh-sync-round-driver-gap"
  - "habit:dataplane-convergence"
tags: [iroh, dataplane, acquisition, projection, transport-parity, ratchet-lane-P]
---

## Measured (household mesh, 2026-08-28, storage built with `p2p p2p-iroh`)

`just mesh recovery warm jessica` with `MESH_PEER_TRANSPORTS=matthew=iroh,jessica=iroh,james=iroh`:
P0 (DocStore parity) GREEN — `elohim_iroh_sync_changes_applied_total` 15 on the recovering peer, so the
iroh doc-sync driver refills a wiped DocStore on its own. P1–P4 RED for the full 900 s: `contentCount 0`.
The Automerge docs arrive, and nothing turns them into `content` rows: the acquisition queue, the
inventory→gap projection and the quilt-draw blob pull all live inside `p2p::P2PNode`, which only exists
when the libp2p swarm does (`transport_backend = Iroh` → `p2p_handle = None`; `/p2p/status` degrades to
`{"peerId"}` with no `pull` block, so P3 cannot even be read).

## What landed beside it (row 13, dual)

On **dual** the pull leg now uses iroh: `acquisition_dispatch` plans one target per peer across the
libp2p-connected set and the iroh book (iroh preferred for a dual peer), `GetContent` and the quilt-draw
`Get{hash}` go over the iroh shard ALPN, ingest is transport-neutral. Dual warm recovery: 58 s, P0–P4
PASS, `elohim_acquisition_dispatch_total{transport="iroh"}` 3 / `{libp2p}` 8, `elohim_iroh_blob_fetches_total{ok}` 3.

## Cure shape

Host the acquisition/projection loop on a transport-neutral runtime seam (the `AcquisitionIngestCtx` +
`store_acquired_record` split is the first half: it already runs from a spawned task with no `self`).
The remaining libp2p bindings are the queue owner, the inventory-gap producer, the kad head-record
publish, and the `/p2p/status.pull` projection. Until then `homo-iroh` is expected red past P0 in
`transport-recovery-measurements.feature`; the fleet (dual) is not affected.
