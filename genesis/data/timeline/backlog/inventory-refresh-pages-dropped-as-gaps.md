---
id: "backlog-inventory-refresh-pages-dropped-as-gaps"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A peer's bounded inventory refresh (77 pages on a 3.5k corpus) is received ~4 pages deep: every page that lands out of order reads as a sequence gap, is dropped, and fires a snapshot request that is a Stage-1 placeholder — so a dual-mode peer's view of a neighbour's blob inventory sits near 9 % and never converges"
slug: "inventory-refresh-pages-dropped-as-gaps"
written: "2026-08-29"
author: "shape-3 pull-leg cut (mesh measurement)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "backlog-pull-leg-drains-before-iroh-book-warms"
tags: [dataplane, inventory, gossip, dual-stack, sequence-gap, bounded-work, ratchet-lane-P]
---

## Measured (dual household mesh, 2026-08-29, three peers, 3.4k-row corpus)

- Publisher (matthew): `T22: published bounded inventory refresh via DualGossipPublisher count=3513 page_count=77
  sequence=2541 payload_budget=3500` every ~60 s — one replacement-snapshot page + 76 contiguous delta pages, on
  BOTH planes (`inventory_broadcaster::build_bounded_refresh`).
- Receiver (jessica) `peer_blob_inventory` for matthew: **184 hashes** (46 `gossip-snapshot` + 138 `gossip-delta`)
  against matthew's 2046 blob-bearing rows; distinct applied page sequences in the last refresh: 2466, 2467, 2468 —
  the snapshot page (fingerprint fast-path) plus three deltas, then every later page a gap. Cursor 2468 → 2541.
- `Inventory delta gap — requesting snapshot`: jessica 3189, james 3676, matthew 589 in ~1 h; gap-size distribution
  on jessica (last 2000): 21–80 pages ahead 1337, 6–20 433, >80 106, 2–5 98, exactly 1 only 26 — i.e. once the
  cursor falls behind, the whole remainder of the refresh reads as gaps; it is not single-page loss.
- `Inventory snapshot applied` 59 vs 3189 requests: `P2PCommand::SnapshotRequest` is `// T14 Stage-1 placeholder`
  (`p2p/mod.rs`) — the request is logged and dropped; the receiver simply waits for the next periodic refresh and
  keeps whichever pages happen to land in order. `iroh inventory fetch work dropped: bounded command queue saturated`
  47–52 per peer.

## Why it matters

`peer_blob_inventory` is what `score_and_enqueue` (blob back-fill) and `lookup_hosts` route from. A 9 % view means
a peer asks the wrong neighbour for bytes (`quilt draw: peer does not have blob (NotFound)` 3097 on jessica in the
same hour) and blob custody decisions read an inventory that is mostly absence. It did NOT cause the 2026-08-29
recovery red (that was half records — see `pull-leg-drains-before-iroh-book-warms`), but it is the next thing the
same probe will find once that is green.

## Cure shape (receive side; the publisher is already bounded)

1. Per-peer reorder window: hold out-of-order pages (bounded, e.g. 128) and apply them when the cursor catches up
   instead of dropping them; only a gap that persists past a short deadline is a real loss.
2. Dual-plane dedup by `(peer_id, sequence)` before the sequence check, so the second plane's copy of a page that
   was refused early can still apply in order.
3. Make `SnapshotRequest` real (a unicast `ProjectionInventory` pull already exists — `ProjectionInventory: serving
   local inventory` 847 on matthew) or delete the request path and its log line: a placeholder that logs
   "requesting" 3k times an hour is worse than honest absence.

Probe: after a warm restart, `count(*) from peer_blob_inventory where peer_id=<survivor>` on the recovering peer
reaches the survivor's blob-bearing row count within two refresh cadences; `Inventory delta gap` per hour < 10.
