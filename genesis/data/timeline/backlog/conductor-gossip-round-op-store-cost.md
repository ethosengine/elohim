---
id: "backlog-conductor-gossip-round-op-store-cost"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every kitsune2 gossip round costs the conductor a full index walk, 512 uncached slice-hash reads and a stuck bookmark that re-sends the last op: the per-round op-store cost the fork pays on every space"
slug: "conductor-gossip-round-op-store-cost"
written: "2026-10-12"
author: "claude-fable-5-1 (sprint 2026-10-11-idle-cpu-residual-sprint)"
status: "open"
priority: "medium"
severity: medium
nodes: [alpha, household]
tags: [conductor-fork, kitsune2, gossip, sqlite, cpu, performance, perf-cpu, perf-io, perf-scale, idle-is-free, dataplane-convergence, trustful-self, friction-verify, plane-notary, unit-peer, phase-steady, lane-background, custody-everyone]
cites:
  - genesis/docs/superpowers/sprints/2026-10-11-idle-cpu-residual-sprint.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
  - genesis/data/timeline/backlog/household-lamad-gossip-wedge-large-dht.md
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
---

> Placement note: sibling of `household-lamad-gossip-wedge-large-dht` (the same full-arc rounds, at the size
> where they stop completing) and a candidate row for `arch-scale-risk-backlog` (query 1 grows with op count,
> query 4 with peer count). Kept standalone as one bounded code reading of one seam; fold if the cluster wants it.

# What a gossip round costs the op store

**How it was seen.** A 60 s sqlx statement-log window on the three household conductors at rest (2026-10-12
02:41Z, scheduler latch on, `genesis/a2o/reports/recovery/idle-residual-20261011/stmt-summary-bef264f46.md`):
840–937 statements a second per conductor, the same four shapes in the same proportions on 5, 10 and 100 cells,
all four issued by kitsune2 gossip rounds. The household fires a round every ~1 s per space on purpose (the
mesh's dev acceleration, `app/elohim-app/scripts/hc-mesh.sh:2638-2650`); the fleet runs the upstream default of
120–300 s (`genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml:136-139` sets only
`roundTimeoutMs` / `maxConcurrentAcceptedRounds`). So alpha pays each of these ~100× less often — but pays them
per space per peer per round, and three of the four scale with database size or hosted-agent count, which is
the fleet's shape, not the household's. Read from the fork at `perf/scheduler-latch` HEAD (Explore, read-only).

**1. The "new ops since" scan — 72–79% of DB time, ~90 ms a call.** `op_ids_since_time_batch_sql`
(`crates/holochain_data/src/dht/inner/sync_queries.rs:173-237`) ← `HolochainOpStore::retrieve_op_ids_bounded`
(`crates/holochain_p2p/src/op_store.rs:288-335`) ← `K2Gossip::retrieve_new_op_ids`
(`kitsune2_gossip-0.5.0/src/respond.rs:315-343`), once per side per round (`respond/initiate.rs:108`,
`respond/accept.rs:59`). With a full arc the predicate `storage_center_loc >= 0 AND <= u32::MAX` walks the whole
`elohim_ChainOp_loc_sync_idx` (`sync_queries.rs:901`) — `when_integrated` is that index's LAST column — then sorts,
however recent `new_since` is: the cost is proportional to the DHT's op count, not to what is new. Two fork
defects ride on it: (a) the page loop stops only when a page adds no new ids, and the filter is `>= cursor`, so the
second page re-reads the cursor row and every call with data costs two statements (`op_store.rs:325`); (b) when
the limit is not hit the fork returns the last op's `when_integrated` where the kitsune2 contract
(`kitsune2_api-0.5.0/src/op_store.rs:134-135`) requires the call's start time, so the peer's bookmark never
advances past the last op and every round re-sends at least that op id. Cure: an index led by
`(locally_validated, when_integrated)` covering loc, op_type, action_hash, basis_hash, serialized_size (or skip
the loc predicate for a full arc); stop the loop when a page is short; return a `now` captured before the query
when the limit is not hit (with slack — the DHT diff catches stragglers). Invariant: same rows, same order; the
bookmark never moves past an op integrated after the read.

**2. Slice hashes — 88–92% of statements, 33 µs each, 512 a snapshot.** `retrieve_slice_hashes`
(`op_store.rs:413-430` → `inner/slice_hash.rs:93-108`) returns a Vec per SECTOR, and
`HashPartition::disc_top_hash` (`kitsune2_dht-0.5.0-dev.6/src/hash.rs:266-286`) asks for each covered sector in
turn: `SECTOR_SIZE = 1 << 23` (`constant.rs:11`) → 512 sectors → 512 statements per minimal snapshot
(`dht.rs:168-187`), one snapshot per side per round; nothing is cached. elohim-storage's health path adds 512 per
space per call through `dump_network_metrics(None, true)` (`elohim/elohim-storage/src/hc_client.rs:1629`). Cure:
an in-memory `(arc_start, arc_end) → Vec` map in `HolochainOpStore`, invalidated in `store_slice_hash` and by the
`DELETE FROM SliceHash` wipe (`holochain_state/src/dht_store.rs:1073`). Invariant: a read always sees the latest
stored hash per slice.

**3. `check_op_hashes_present` — 4–8% of DB time at 12–15 ms.** `sync_queries.rs:254-301` via `op_store.rs:257-286`
from `CoreFetch::request_ops` (`kitsune2_core-0.5.0/src/factories/core_fetch.rs:172`), once per round in which the
peer sent op ids — nearly every round because of 1(b). 12–15 ms is too slow for primary-key `IN` lookups; either
the lists are long (a missing bookmark falls back to now − 30 min) or the plan is bad: `EXPLAIN QUERY PLAN` owed.
Fixing 1(b) removes most calls. Invariant: exclude only held limbo ops and `locally_validated = 1` ops;
cache-only copies are still re-delivered.

**4. `peer_meta` — 57 reads and 7 writes a second here.** Single-key get/put
(`holochain_data/src/peer_meta_store/inner.rs:18,40`); `select_responsive_and_least_recently_gossiped`
(`kitsune2_gossip-0.5.0/src/initiate.rs:255-290`) costs two reads per candidate agent per initiation (~30
candidates per space implied), plus bookmark, timestamp and round counters (`gossip.rs:234,278`,
`respond.rs:129,290,300`). Scales with agents in the peer store — adam's shape. Cure: a write-through in-memory
cache in `HolochainPeerMetaStore` (`holochain_p2p/src/peer_meta_store.rs:72-121`) honouring expiry and
`get_all_by_key`.

**What this is not.** Not the publish sweep (the statement looks like it; it is not), not the scheduler (cured on
`perf/scheduler-latch`), not sys-validation. Not a household-only artifact either: the cadence is, the per-round
cost is not.

**Done-when.** On the household at production cadence (`gossip-cadence.sh prod`) and on alpha, a statement window
shows query 1 below 10 ms with a plan that does not walk the loc index, one statement per call, and the peer's
bookmark advancing between rounds; slice-hash reads per snapshot at 0 after the first; `idle-is-free` re-read.
