---
id: "backlog-pull-leg-drains-before-iroh-book-warms"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A warm-restarted peer drains its whole pull queue in the first seconds after boot, before its iroh peer book has learned any peer — so the transport selector sees single-plane peers and libp2p carries every pull with no decision to make"
slug: "pull-leg-drains-before-iroh-book-warms"
written: "2026-08-29"
author: "M4 transport self-awareness cut"
status: "wip"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, iroh, peer-book, boot-ordering, transport-selection, ratchet-lane-P]
---

## Measured (dual household mesh, 2026-08-29, warm recovery jessica<-matthew, cut m4-select-v2)

- jessica booted ~02:53:00Z; `iroh peer learned from transport manifest` at 02:53:08Z (james) and 02:53:10Z
  (matthew, relayed); the acquisition drain (5 s cadence, budget 50) had already dispatched all 11 pulls —
  `elohim_acquisition_dispatch_total{transport="libp2p"} 11`, **zero** `elohim_transport_route_total` rows:
  the planner saw no dual peer, so no Bulk decision existed to make.
- The same run's Small races DID sample iroh (jessica->matthew 15 ms vs libp2p 816 ms degraded), and the
  write-time push routed every shard `transport=iroh` — selection works once the book is warm; the pull
  leg simply runs before it is.

## Cure shapes

1. Persist the iroh peer book across restarts (a warm cache the announcer refreshes), or
2. gate the FIRST acquisition drain on `doorway_bootstrap` / the first manifest round (bounded wait, e.g.
   ≤10 s, never indefinite), or
3. seed the book from the doorway's `/p2p/manifests` BEFORE the drain loop starts (the T0' bootstrap already
   exists but reports "watching for an empty peer book" after the drain).

Fleet relevance: a long-running peer's book is warm and its pull leg runs continuously, so the fleet
reading of `route_total{reason}` is where the selector's Bulk behaviour actually shows.

## 2026-08-29 cure shape 2 landed (local evidence; mesh measurement pending)

`acquisition::first_drain` (pure, 5 tests) gates the FIRST dispatch tick: hold while the iroh book is empty and
`< FIRST_DRAIN_HOLD` (10 s) has elapsed; release on `book_warm` (the moment the book learns a peer), `no_iroh_leg`
(nothing to wait for) or `expired` (bounded, never indefinite). Counted in
`elohim_acquisition_first_drain_total{outcome}` (pre-touched). Read beside `route_total`: a boot released `expired`
routed with single-plane peers, so a flat route series there is the book, not the selector. To measure: warm
recovery on the dual mesh — expect `route_total{op_class="bulk"}` rows to appear on the recovering peer's first drain.

## 2026-08-29 MEASURED: the hold is honest but too short on this mesh — next cut is shape 3

Warm recovery jessica (dual, survivors' books warm): `P2P node started` 11:22:36.49 → `first acquisition drain
released reason=expired since_boot_ms=10004` → first `iroh peer learned from transport manifest` at **+28 s**
(11:23:04.81); manifest rounds then land every 30 s (:04/:07/:34/:37). All three peers on the simultaneous
`storage-restart` read the same `held 2 → expired 1`. So `FIRST_DRAIN_HOLD` = 10 s covers the row's +8..10 s
measurement but not this mesh's ~30 s announcer cadence; a hold sized to the cadence would tax every boot by 30 s.
The pull leg nevertheless ROUTED this run — the reconcile keeps re-enqueueing across a 3.4k-row recovery, so the
selector saw dual peers from the second minute on: 34 bulk decisions, `best_rtt` iroh 27 : libp2p 3 (EWMA 18 ms
vs 367 ms), dispatch iroh 225 / libp2p 73. The one-shot small-queue shape (11 pulls, gone before the book warms)
is what shape 3 cures: seed the book from the doorway's `/p2p/manifests` BEFORE the first drain (today the T0'
bootstrap waits its 30 s grace and then only watches for an EMPTY book). Keep the 10 s hold as the bounded floor;
make the doorway seed the release, not the deadline.

### Sharpening (2026-08-29, sibling review): the hold is in the wrong UNIT, not the wrong size

The hold is wall-clock; the book fills from a 30 s manifest cadence. So `FIRST_DRAIN_HOLD` is not a tunable:
under one round it always expires first, over one round it is "wait for a round" plus a 30 s tax on every boot.
Do not spend a cut widening it. Shape 3 removes the dependency (doorway-seeded book gates the release, the 10 s
floor stays as the bound). Pinned on the constant's doc comment in `acquisition.rs` so the cheap fix is refused at
the site where it would be typed.


## 2026-08-29 cure shape 3 landed (local evidence; mesh measurement below when it lands)

`p2p_iroh::doorway_bootstrap::spawn_doorway_bootstrap` now issues ONE **boot seed** — a bounded GET of the doorway's
`/p2p/manifests` at T0, verify-then-upsert as before — whenever the book is empty at spawn, BEFORE the 30 s grace it
used to sleep through. The watch leg (empty-book fallback after the grace) is unchanged. So the first-drain release
is an event (`book_warm`, within the first 5 s tick) instead of the `expired` deadline; `FIRST_DRAIN_HOLD` stays the
floor for a node with no doorway or an unreachable one. Counted in `elohim_iroh_doorway_bootstrap_reads_total{phase,
result}` (pre-touched; `boot`/`watch` × `seeded`/`none_accepted`/`empty`) — read beside `first_drain_total`: a boot
that reads `{boot,seeded} 1` should release `book_warm`, and a fleet whose `boot` column only ever reads `empty` has
doorways with no board (the pull leg is back on the floor, visibly). Two wiremock tests pin the shape: an empty book
is seeded well inside a 60 s grace with exactly one GET; a warm book issues none. To measure: warm recovery on the
dual mesh — expect `first_drain_total{outcome="book_warm"} 1`, `held ≤ 1`, and `iroh peer learned from the doorway
bulletin board` before the first `... from transport manifest` line.

## 2026-08-29 MEASURED: shape 3 turns the release into an event — `book_warm` at 1–343 ms on every boot

Cold simultaneous `just mesh start` (dual, three peers, shape-3 binary): matthew and jessica read the board at T0
(`{boot,seeded}` 1 and 2 peers) and released `book_warm` at 1 ms / 3 ms; the first gossip manifest landed +26 s later.
james — first up, nobody had announced yet — read `{boot,empty}` and fell to the floor (`held 2 → expired 1`, watch leg
seeded it at +30 s). That cold-start ordering got a bounded retry the same day: `BOOT_SEED_RETRY` 2 s ×
`BOOT_SEED_ATTEMPTS` 5 (0/2/4/6/8 s, inside the 10 s hold, only while the book is empty; wiremock test
`an_empty_board_at_boot_is_re_read_inside_the_hold`). Simultaneous warm `storage-restart matthew jessica james`
on that binary — the shape that read `held 2 → expired 1` on all three this morning — now reads on ALL three:
`{boot,seeded}` 1 (accepted 2, own 1) → `first acquisition drain released reason=book_warm` at **139 / 343 / 6 ms**,
`held 0`, `expired 0`; gossip's first manifest still +28 s out. Warm recovery jessica<-matthew on the same binary:
boot seed → `book_warm` at 1 ms; pull-leg dispatch iroh 274 / libp2p 29 (this morning, with the first drain
single-plane: 225 / 73 — libp2p's share fell 24.5 % → 9.6 %), route `best_rtt` iroh 27 : libp2p 3, `prior_iroh` 2.
Fleet read: `first_drain_total{outcome="book_warm"}` should be 1 per boot on every dual peer with a doorway; a
`{boot,empty}` column that stays non-zero names a doorway whose board is not being posted to.

## 2026-08-29 recovery on the shape-3 binary: NOT-RECOVERED 915 s, P1 plateau 134 — half records, not the boot seed

`just mesh recovery warm jessica` (labels cut=shape3-2026-08-29 seed=doorway-boot): P0/P2/P3/P4 green, P1 stuck at
140→134 `<id>=null` rows. Warm recovery WIPES the content db (script: "wipe DocStore + content db + blobs"), so
jessica re-acquired the whole corpus from the two survivors. The 134 rows were re-inserted by the acquisition store
at 14:43:53–14:44:12 (`created_at == updated_at`, SQLite default stamp = `bulk_create_content`) with `blob_hash`
NULL and `blob_cid` set — the served record was HALF, because the best-RTT survivor (james, iroh) held only a half
row for them: james had restarted 2 min after the seed set matthew's `blob_hash` and projected those rows from
matthew's sync docs before its own pull reached them. This morning both survivors were converged → 439 s PASS.
Neither boot seed nor first-drain timing is in the chain (these ids never appear in any acquisition line before
14:43); the pull leg would have done the same at 10 s.

Why the half rows never healed: `reverse_project_content_doc` is EDGE-triggered on doc changes. jessica's doc for
the row landed at 14:42:49 (row absent → heal no-ops "the replication plane's job"); the half row landed at 14:43:53;
no further doc change → never re-read. The doc DID hold the hash (every reverse heal on jessica reads
`from None → bafkrei…`; heads present for the half rows). Cure landed: `sync::projector::heal_half_rows_from_docs`
— level-triggered, keyset-paged sweep (200 half rows per sync round, wraps) that asks each half row's doc; a doc
without a blobHash leaves the legitimate half state alone (matthew itself has 1369 declared-but-unuploaded rows).
Counted in `elohim_sync_half_row_heal_total{outcome}` (pre-touched). Test pins heal / leave-alone / cursor wrap.

Also found on the way, filed separately: `inventory-refresh-pages-dropped-as-gaps` (a dual-mode peer's view of a
neighbour's blob inventory sits at ~9 % — 77-page refresh, out-of-order pages dropped as gaps, snapshot request is
a placeholder).

## 2026-08-29 second recovery on the heal-sweep binary: 96 P1-bad — two more stations in the same chain

Run 2 (labels cut=shape3-halfrow-heal): the sweep healed 54 during the run but 96 half rows read `no_doc_hash`:
jessica held matthew's CURRENT doc (changes applied 15:35) and it had no `blobHash`. **matthew's own docs were stale**
against its rows: a warm restart of matthew ran the cold-start back-fill and re-projected **1270** docs
(`scanned 3442 projected 1270`), after which jessica's stuck set fell 262 → 27 within three sweep ticks. Whatever
wrote those rows' `blob_hash` did not ride the `ContentUpdated` producer path, so the doc only caught up at the next
cold start — chain / between row-write→doc / missing node: *every blob_hash writer re-projects the doc* (open;
candidates: the seed's blob-attach, blob-heal "persisted and recorded"). The back-fill's `doc_matches` is the
level-triggered version of that node and it already exists — it just only runs at boot.

Station 2: matthew's sweep reported the identical `healed 72 / no_doc_hash 128` every tick. `update_content`'s
amber rule kept the EXISTING blob_hash on a green (anchored) row even when it was NULL — a NULL is not a notarized
hash; guarding it is empty-wins. Cure landed: the guard applies only when the green hash is present
(`amber_write_fills_a_null_blob_hash_on_a_green_row`), and `reverse_project_content_doc` now returns healed iff the
row carries the converged hash after the write, so the counter cannot re-count an unchanged row.
