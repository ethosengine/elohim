---
id: "backlog-inventory-refresh-pages-dropped-as-gaps"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A peer's bounded inventory refresh (77 pages on a 3.5k corpus) is received ~4 pages deep: every page that lands out of order reads as a sequence gap, is dropped, and fires a snapshot request that is a Stage-1 placeholder — so a dual-mode peer's view of a neighbour's blob inventory sits near 9 % and never converges"
slug: "inventory-refresh-pages-dropped-as-gaps"
written: "2026-08-29"
author: "shape-3 pull-leg cut (mesh measurement)"
status: "wip"
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

## 2026-08-29 cure shape 1+2 landed and MEASURED (cold dual mesh, 3.4k-row seed)

`p2p::inventory_reorder` holds pages that arrive ahead of the cursor (per publisher, bounded 160, lowest sequences
kept) and `flush_held_pages` applies them in order after every delta apply and after every snapshot
apply/dedup; only a page beyond the window warns and fires the (placeholder) snapshot request. Counted in
`elohim_inventory_pages_total{kind,outcome}` (pre-touched — the fleet can now read this without Loki).
Before (this morning, same corpus): jessica's inventory of matthew **184** of 2046 hashes, 3189 gap warnings/hour.
After (17:39, ~7 min post-seed): jessica **2892** and james **2892** of matthew's 3513 advertised hashes, cursor
650 vs publisher sequence 741 (inside one refresh); jessica `buffered 439 → flushed 316, replay 49, overflow 0`,
james `buffered 434 → flushed 335, replay 67, failed 1, overflow 0`; **gap warnings 0** on all three peers.
So ~73 % of a refresh's pages arrive early on this mesh — reorder, not loss, was the whole storm.
Shape 3 (a real `SnapshotRequest`) is still open: the placeholder stays, now reached only past the window.
Next measure (in flight): a PUBLISHER restart — its sequence allocator restarts from 1 under receivers whose
cursor is at 650+; deltas below the cursor read as `replay` until a snapshot with a new fingerprint re-bases.

## 2026-08-29 three more stations, measured on the same mesh

**Tail loss was a 64-slot channel, not reordering.** With the reorder window in, receivers held a contiguous
0..62 of every 77-page refresh and never saw 63..76: `broadcast_inventory_snapshot` `try_send`s every page into
the 64-slot `P2PCommand` channel that the swarm loop itself drains, and the dual publisher reports Ok when either
plane took the page. Cure: the refresh is serialised into a backlog and PACED — `INVENTORY_PAGES_PER_TICK` = 16
per 1 s tick, a newer refresh supersedes the backlog. `elohim_inventory_pages_published_total{sent|deferred|
superseded|serialize_failed}`. Measured: publisher `sent 624 / deferred 0 / superseded 0` over 8 refreshes;
receiver's last refresh **77 pages, positions 0..76 contiguous**, inventory of matthew **3535 / 3535**.

**A transiently failed page must be held, not dropped.** james's first delta of a refresh failed with
`apply_delta: database is locked`; the `Err` arm dropped it and every later page sat buffered until the next
refresh superseded it. Cure: a failed page is stashed like an early one, and every `Gap` arrival (which carries
the exact cursor as `expected`) flushes from there — the next page to arrive is the retry trigger.

**A publisher restart collapses receivers' view and it never recovers on its own.** After matthew restarted,
its sequence allocator started at 1 while jessica/james held cursor 937: every fresh delta read `replay` (924),
and the snapshot page — byte-identical to the one last applied — kept deduplicating, so the cursor never moved:
receivers' view of matthew **3535 → 46** and stuck (it only clears once the new run's sequence outruns the old
cursor: ~12 min here, DAYS on a fleet peer with days of uptime — i.e. every deploy). Cure:
`peer_blob_inventory::apply_snapshot` treats a snapshot more than `PUBLISHER_RESTART_GAP` (200) below the cursor
as a restart and re-bases the cursor to it (`SnapshotApplyOutcome::Rebased`, counted, flushes held pages); unit
test `a_publisher_restart_rebases_the_cursor`. The proper wire-level fix (a publisher boot epoch in the page
header) is deferred: msgpack positional structs make a trailing field a mixed-version hazard.

Also on the way: the mesh's restart guard read two a2o fixture rows with `sha256-…` anchors as a "zome input
shape mismatch" and refused every restart — the probe now picks a row whose anchor is an action hash (`uhCkk…`).

Live publisher-restart run (19:22:53, receivers' cursor 1092 vs sequences from 1): james back to 3535 at
matthew's second refresh, jessica at its third (the first refresh's deltas arrived before its snapshot re-based
the cursor and read as replay — bounded to one refresh). Recovery ≤ 3 refreshes; before this cut it was
cursor/78 refreshes.
