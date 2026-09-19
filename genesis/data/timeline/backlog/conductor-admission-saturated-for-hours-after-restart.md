---
id: "backlog-conductor-admission-saturated-for-hours-after-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "After a storage restart the conductor-admission gate stays full for hours, so nobody can author a head — the app pipeline cannot deliver behind an edge roll"
slug: "conductor-admission-saturated-for-hours-after-restart"
written: "2026-09-19"
author: "pipeline-shakeout shift pickup (2026-09-19)"
status: "backlog"
priority: "high"
tags: [dataplane, conductor-admission, restart, app-delivery, alpha-fleet]
jobs: [elohim, elohim-edge]
cites:
  - elohim/elohim-storage/src/conductor_admission.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - scripts/ci/stage-spa-blob.sh
  - genesis/data/timeline/backlog/storage-sqlite-locked-surfaces-as-500-despite-busy-timeout.md
---

# The admission gate does not drain after a restart

## What was observed

`elohim_conductor_admission_in_flight`, namespace `elohim-alpha`, capacity 5, 30-minute samples 2026-09-19:

- matthew: `5 5 5 5 4 4 4 5 5` from 03:00Z to 07:00Z (storage restarted by edge/dev 1462 at about 03:00Z), then
  `0 0 1 1 1 1 0 1` until its next restart at 11:04Z (edge/dev 1467), then `5 5 5 5` through 12:50Z and counting.
- susan: `5 4 5 4 5 5 5 5 5` from 07:00Z to 11:00Z after the 06:24Z restart, then 0.
- adam, james, jessica, eve: 3–5 for the two hours following each restart.

Loki, `elohim-matthew-alpha-0`, 12:05:48Z–12:50:38Z, repeating:
`conductor admission: shed: no conductor permit for infrastructure within 5000ms (class=interactive, capacity=5, in_flight=5) — nothing was dispatched`
— from the heartbeat, discovery enumeration, peer-status fan-out, household replayer, identity fill, shard
registration, projection reconcile, and at 12:40:46Z from `elohim_storage::provide` for `head_ref: "elohim-host-landing"`.
Alongside it, `apply_snapshot: database is locked` (the already-captured SQLite gap).

The runbook's "restart churn ≈ 20 min" does not describe this. The gate is full for four hours.

## What it breaks

elohim/dev 1712 ran directly behind the 1467 roll. For more than forty minutes on both public doorways every
`blobHash` / `serverBlobHash` PATCH came back `503 {"status":"catching-up","cause":"upstream","circuit":"closed"}`
(the doorway honouring storage's own shed, `storage_proxy.rs:816`), every 3–4.5 MB blob forward to storage timed
out, and the stage concluded `NO doorway could author elohim-host-landing — no live conductor bridge in the
fabric`. Reads were healthy throughout (`/db/p2p/conductor-diagnostics` 200 in under 150 ms, 53 agents), which is
why edge validation — all reads — passed minutes earlier on the same fleet.

It is also the likeliest reason `elohim.host` has served the 09-13 landing row while `doorway-alpha` serves the
09-14 one: heads authored behind a roll do not land.

## Two things the evidence separates

1. The heartbeat — named in the module's own doc as the example of Background work — is shedding as
   `class=interactive`. Classification at the call sites has drifted from the contract.
2. Class is only a wait bound; both classes draw from one FIFO pool with no capacity held back for a person-facing
   write. A dozen cold background loops can hold all five permits indefinitely.

The five are slow calls, not lost ones. At 12:55Z, ten-minute window: matthew releases 5.4 permits a minute with
a mean hold of **55.8 seconds** per conductor call; susan, drained, releases 14 a minute at **4 milliseconds**.
Five permits at 56 s each is exactly 5.4 a minute — the gate is full because matthew's conductor takes a minute
to answer anything, for hours after the restart. The gate is doing its job; the scarce thing is the conductor.

## Missing nodes

- chain / between "storage restarted" → "a head can be authored through this peer" / missing node "the admission
  gate has spare capacity": assertion — `in_flight < capacity` sustained for one sweep; probe — the gauge above.
  State: **unprobed**; neither the edge roll gate nor the app pipeline reads it.
- chain / between "edge deploy finished" → "app pipeline stages heads" / missing node "the fleet accepts writes":
  the orchestrator sequences app directly behind edge, and the app stage's 360 s per-doorway budget is two orders
  of magnitude shorter than the observed saturation. State: **not built**.

## Needs a design decision

Reserved interactive capacity versus a background admission ceiling versus pacing the cold-start loops is a
capacity-contract change in `conductor_admission.rs` — route to a brainstorm, not to iteration.

## 2026-09-19 14:35Z — it is not draining, and the scarce thing has a name

Three and a half hours after the 11:04Z restart matthew and adam are still 5/5. Mean permit hold over 30 minutes,
by zome: matthew — mishpat 60 001 ms, infrastructure 60 001 ms, imagodei 59 254 ms, content_store 55 946 ms; adam —
60 002 / 59 270 / 58 010 / 51 706 ms. Sixty seconds is the call timeout: on these two peers **no zome call is being
answered**, each one holds its permit until it times out. Susan, same binaries: imagodei 0.5 ms, content_store 0.2 ms.

Conductor CPU, 10-minute rate: `elohim-adam-alpha-conductor-0` **4.01 cores**, `elohim-matthew-alpha-conductor-0`
**1.92**, the other five 0.98–1.48. Working set 2.2 GB on both against 0.8–1.7 GB elsewhere. The two peers behind the
two public doorways are the two whose conductors are pegged.

And it is not only a restart effect: across the 12 hours BEFORE the first roll of the night (2026-09-18 14:00Z →
09-19 02:00Z, no restarts) matthew's gate sat at 3–4 of 5 while adam's sat at 0–1. The restart takes a chronically
near-full gate to full. The earlier "drains" in this document each coincide with another restart of that peer's
conductor or storage, not with the passage of time.

Consequence for delivery: the app pipeline cannot author a head through either public doorway while this holds, so
the app + genesis run was deliberately NOT dispatched. Readiness to dispatch it:
`max by (pod)(max_over_time(elohim_conductor_admission_in_flight{pod=~"elohim-(matthew|adam)-alpha-0"}[10m])) < 5`
and mean hold on those pods back under one second.

## 2026-09-19 evening — the operator's k8s read, joined to the code

**From the cluster (operator, cgroup `cpu.stat` sampled 60 s apart, conductor logs, storage `/metrics`):**
all seven conductors are pinned at their CPU limit and 100% throttled — including susan, which answers in under a
millisecond — so the CPU ceiling is real but is **not the discriminator**. Nodes have headroom (ethosengine 19% of
24 cores, shem 48%); the limits bind, not the nodes. Disk is clean (iowait 0.04%, no pressure). What separates the
two stalled peers: they are the only doorway-fronted ones (doorway-alpha → matthew, doorway-alpha-b → adam); adam
alone shows DB connection-pool exhaustion (172 `sqlx::pool::acquire` warnings, 16–22 s to obtain a connection,
worst statement 894 s); and both run a statement susan never runs —
`SELECT hash, blob FROM Entry WHERE hash IN (… ×500) UNION ALL SELECT … FROM PrivateEntry WHERE author = ? AND hash IN (… ×500)`
— matthew 273× (avg 2.0 s), adam 224× (avg 5.0 s, max 12 s), susan 0×. Every zome on the two averages 50–60 s;
susan served 6.4× more content_store calls and stayed at 1.1 s. Not call volume.

**From the code:** the 500 is not a batch size anyone chose for this workload — it is the chunk constant of
`get_entries_by_hashes` in the conductor fork (`holochain-conductor/crates/holochain_data/src/dht/inner/entry.rs`,
`CHUNK_SIZE = 500`), and that function has exactly two read-path callers
(`holochain_state/src/dht_store/reads.rs`): `source_chain_records` (:2194 — a zome reading its agent's source chain
with entries) and `valid_cap_grants` (:2312 — the capability check). A 500-hash chunk appears only when there are at
least 500 candidates, so matthew and adam hold **something that has grown past 500 that susan's has not**: a long
source chain, a large set of capability grants, or both. Two first-party producers are known:

- `elohim-storage/src/hc_client.rs:451-469` calls `authorize_signing_credentials` for three cells on EVERY connect, and
  `closed_chain_fence.rs:271` records that this "COMMITS a CapGrant". Nothing revokes an earlier grant. Connects in
  12 h (Loki): adam 21, susan 11, eve 9, matthew 7 — so today's mint rate does not separate the peers; the age of the
  chain would. matthew and adam are the genesis pair, which is never re-keyed; the other five have been reinstalled.
- six zome sites query the chain with `include_entries(true)` (content_store, imagodei ×2, node-registry) — cost
  linear in chain length, on the peers that author every deployed head.

**Not determined:** which caller dominates, and the actual counts. One read on matthew's conductor database settles
it — the number of CapGrant entries on the storage agent's chain, and the chain length, against susan's — and that
read is the operator's.

**What follows if it is grants:** the cure is first-party and small — reuse one signing credential across
reconnects (persist it beside the agent key) instead of authorizing a new one per connect, and revoke superseded
grants — plus a one-time prune on the genesis pair. If it is chain length: the chain-reading zome sites need a
bounded query, and stories 1.1 / 1.2 of the serving-edge campaign (fewer failing declares, no re-minted contests)
already slow the growth. Raising CPU limits helps every peer and is free, but by the susan comparison it is not the fix.

