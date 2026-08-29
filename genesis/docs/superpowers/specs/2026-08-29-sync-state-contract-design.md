---
title: "The sync-state contract — one vocabulary for 'where am I in this stream and am I caught up'"
id: sync-state-contract
tier: spec
status: Draft
created: 2026-08-29
maintainers: Matthew Dowell + Claude Fable 5
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: every sync stream in elohim-storage declares its SyncStreamState and the /p2p/status rollup reads them, OR superseded-by-implementation
domain: dataplane sync planes (inventory gossip · Automerge doc sync · acquisition pull) x observability-per-decision
habits: [dataplane-convergence]
topic: [sync, epoch, cursor, caught-up, inventory, automerge, acquisition, observability, local-first]
cites:
  - genesis/data/timeline/backlog/inventory-refresh-pages-dropped-as-gaps.md
  - genesis/data/timeline/backlog/pull-leg-drains-before-iroh-book-warms.md
  - genesis/data/timeline/backlog/alpha-pull-leg-fetch-error-storm.md
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
---

# The sync-state contract

## Why this exists

On 2026-08-29 the household mesh measured four convergence defects in one afternoon, and every one of them was
the same defect wearing a different costume: a sync stream whose *position* was implicit, whose *epoch* did not
exist, and whose *caught-up* answer was a guess.

| Stream | What was implicit | What it cost |
|---|---|---|
| Inventory refresh (gossip pages) | The publisher's sequence restarted at 1 on reboot; receivers kept the old cursor | Receivers' view of a restarted peer collapsed 3535 → 46 and stayed there for `old_cursor / 78` refreshes — days on a long-lived fleet peer, i.e. every deploy |
| Inventory refresh | "In order" was the only position contract; no reorder tolerance | 184 / 2046 hashes visible (~9 %), 3k gap warnings/hour, snapshot request a placeholder |
| Automerge doc → row | The reverse heal fires on doc changes only; a row landing after its doc is never re-read | 134 half rows for a whole 915 s recovery while the doc held the hash |
| Acquisition pull | `local_has` = "row exists" stood in for "record complete" | Half records served by a half survivor counted as done |

The local-first sync engines the industry converged on (Electric, Zero, PowerSync, LiveStore) have one thing this
plane lacked: an **explicit sync-state contract** — every replica can say, for every stream, *which epoch of the
publisher it follows, what position it has acknowledged, and whether that is the whole of what the publisher has
declared*. They can afford to say it because one server owns each stream. Here no node does — the DHT notarises,
peers gossip, and the doorway projects — so the contract has to be **per (receiver, publisher, stream)** and has
to survive a publisher restart without anyone to ask. That is the whole design.

## The contract

Every sync stream declares a `SyncStreamState` (`elohim-storage/src/p2p/sync_state.rs`):

```
stream:    which stream (inventory | docs | pull), keyed by the PUBLISHER's transport peer id
epoch:     the publisher's boot epoch — changes when the publisher restarts; NEVER reused
position:  the last position applied in order, within the epoch (monotone per epoch)
declared:  the publisher's declared end of the epoch so far (its latest position), if known
caught_up: position == declared  (unknown declared → NOT caught up — honest absence, C4)
```

Rules (each maps to a measured defect above):

1. **Epoch before position.** A position from a newer epoch supersedes any position from an older one; a
   position from an older epoch is a replay. A receiver never has to *detect* a restart — the epoch says so.
2. **Position is per-epoch monotone; arrival order is not a contract.** Out-of-order arrivals are held (bounded)
   and applied when contiguous; only a hole that persists past the window is a loss.
3. **Caught-up is a comparison, not a timer.** `position == declared`. A stream whose `declared` the receiver
   has never learned is *not caught up* and reports `null`, never `true`.
4. **Every state is a signal.** Each stream exports its state per publisher (`/p2p/status` rollup + a gauge
   family), so "are we converged" is read, never inferred from log silence.

## How each stream maps

| Stream | epoch | position | declared | caught-up signal |
|---|---|---|---|---|
| Inventory pages | high 32 bits of `sequence` = publisher boot epoch (station 1, below) | low 32 bits = page counter | last page counter of the latest refresh (`page_count` from the snapshot page's refresh) | `peer_inventory_cursor.last_sequence` vs the refresh's last sequence |
| Automerge docs | n/a — CRDT; heads are content-addressed | set of local heads per doc | remote heads per doc (learned in the sync round) | `local_heads ⊇ remote_heads` per doc; rollup = docs not caught up = 0 |
| Acquisition pull | reconcile generation (`reconcile_initialized` + count) | items satisfied (bytes present) | desired set size | `pull.caughtUp` = pending == 0 AND initialized (already C4-honest: null before the first reconcile) |
| Row ↔ doc projection | n/a | fields present in the doc | fields present in the row | the back-fill's `filled=` report; steady state = `projected 0` at cold start |

## Station 1 — epoch-carrying inventory sequences (landed with this spec)

The inventory publisher's `SequenceAllocator` now starts at `epoch_base(boot_epoch)` = `boot_epoch << 32`
instead of 0, where `boot_epoch` = seconds since 2026-01-01 at process start (31 bits of headroom = 68 years).
Nothing changes on the wire: `sequence` is still a `u64`, still strictly monotone within a run, and after a
restart it is strictly GREATER than anything the old run published — so an old receiver (fleet build) simply
sees a large forward jump and applies the new epoch's snapshot; a new receiver decodes `(epoch, counter)` for
its logs and metrics. The `PUBLISHER_RESTART_GAP` heuristic in `apply_snapshot` stays as the fallback for
publishers that have not yet upgraded. Mixed-version safety is the reason this rides inside the existing field
rather than as a new page-header field: the inventory pages are msgpack positional structs, and a trailing
field is a hazard for every receiver one build behind.

## Stations that follow

2. `SyncStreamState` for the inventory stream exported per publisher on `/p2p/status` (`inventory.streams[]`),
   with `caught_up` computed against the latest refresh's last sequence.
3. Docs: `sync.docsBehind` (count of docs whose local heads do not contain the remote heads) beside
   `syncDocuments` (the count reporting bug is a separate row).
4. Pull: `pull.caughtUp` already exists; add `pull.epoch` (reconcile generation) so a stale rollup is visible.
5. A `SnapshotRequest` that is answered (the placeholder becomes a unicast inventory pull), reached only past the
   reorder window.

## What this deliberately is not

Not a server. No node owns a stream; the contract is symmetric and per-peer. Not a new entity: every piece of
state here is Ephemeral (C) — receiver-side cursors reconstructable from the next refresh — so no DHT type, no
migration beyond what `peer_inventory_cursor` already holds, no HTTP route of its own (the rollup is the
existing `/p2p/status` projection). Not a vocabulary for reach or heads: reach (audience), content head
(version) and replication (custody) stay three planes; this contract lives entirely inside the replication plane.
