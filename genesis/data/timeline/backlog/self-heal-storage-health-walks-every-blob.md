---
id: "backlog-self-heal-storage-health-walks-every-blob"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storage /health walks every blob file, so its latency grows with the store and restart arms time out"
slug: "self-heal-storage-health-walks-every-blob"
written: "2026-10-28"
author: "claude (push-20 serving-receipt lane, session 3f9afd7a, steward human:matthew)"
status: "wip"
priority: "high"
ci_status: none
jobs: [elohim-edge]
tags: [self-heal, storage, health, liveness, probe, performance, household, restart]
---

# Storage /health cost grows with the number of blobs it holds

## What happened

On 2026-10-28 the push-20 household receipt lane
(`just test mesh features/dataplane/epr-app-deliverability.feature`, run
`20261028T201527Z-a392b6ab`) passed 5 of 7 stations. Stations 3 and 4 failed only at
`hc-mesh.sh storage-restart`. The arm printed `matthew :8090 did not come back within the wait` and the
same for jessica. Station 3 failed at `both warm doorways keep serving that rendered build while all
storage peers are down`. Station 4 failed at `peer "jessica" comes back`, with the 240 s step timeout.
The restarted processes were in fact running and serving. They answered `/health` too slowly for the
arm's probes.

Measured on the household with the same binary on all three peers (debug `mesh-bin` build at
a392b6abb), with host load average about 52 on 24 cores and `/proc/pressure/cpu` "some" about 55%:

| peer | blobs (`/health` `blobs.total`) | `/health` time to first byte |
|---|---|---|
| matthew :8090 | 3715 | 2.8–5.0 s |
| jessica :8091 | 3714 | 3.0–4.6 s |
| james :8092 | 211 | 0.14–0.40 s |

A 404 on the same listener answers in about 1 ms, so the cost is inside the handler and not in
accept or queueing. The doorways' `/health` answers in about 2 ms.

## The path

- `elohim/elohim-storage/src/http.rs:3586` `handle_health` calls `self.blob_store.stats().await?` at
  `http.rs:3642` on every request.
- `elohim/elohim-storage/src/blob_store.rs:613` `BlobStore::stats` does `read_dir` on `blobs/` and on
  every shard directory. For each entry it does a `fs::metadata`, and for 7-byte files a `fs::read` to
  detect the `CHUNKED` marker, after which it reads the chunk index. Every call is tokio async fs, which
  means one `spawn_blocking` hop per syscall. The walk is O(blobs) IO plus O(blobs) blocking-pool
  round trips on the liveness endpoint. Under CPU pressure each hop waits for a blocking-pool thread,
  so latency scales with blob count times scheduler delay.

## Why it matters

- **Restart arms.** `app/elohim-app/scripts/hc-mesh.sh:3811` waits for the restarted set with
  `curl -m 2 /health`, and `hc-mesh.sh:3830` then gives each target 6 tries of `curl -m 5`. A peer that
  holds a few thousand blobs on a loaded host never answers inside either bound during its boot burst,
  so every chaos station that restarts storage reports a live peer as dead. Each deliverability run
  adds fixture bundles to matthew and jessica, so the store grows run over run and the margin shrinks.
- **Fleet probes.** `genesis/orchestrator/manifests/edgenode/alpha.yaml` (lines 300–315) points both
  the readiness and liveness probes at `/health` with `timeoutSeconds: 15`. Alpha peers hold far more
  blobs than the household. When the walk crosses 15 s under node pressure, kubelet restarts a healthy
  peer, and the restart's own boot burst makes the next probe slower. That is a self-inflicted
  crashloop with no exhaustion report.
- **Every scraper pays it.** The quiesce gate, the doorways' pool health, `just mesh status`/`wait`
  and the runtime-harvest poller all read `/health`. Each read costs a full directory walk.

## Fix direction

`/health` must not do O(n) IO. Options, in order of preference:

1. Keep `total_blobs`, `total_bytes` and `chunked_blobs` as counters maintained incrementally on blob
   put, delete and chunk-index write. Seed them once at boot from a background walk, and have `/health`
   report `stats: warming` until the seed completes. `/health` reads atomics only.
2. Otherwise, cache the `stats()` result with a TTL and refresh it from a background task, never on the
   request path, and report the cache age in the body.

Also move the walk itself (wherever it remains, for example a reconcile or audit) to one
`spawn_blocking` closure using `std::fs`, instead of one async hop per syscall.

## Done when

`/health` time to first byte is independent of blob count: under 50 ms on a peer holding 10k blobs on
a loaded household. A regression test pins that the handler performs no filesystem walk (for example,
a store with N blobs where `/health` issues no `read_dir`). The deliverability lane's storage-restart
stations pass on a household whose matthew and jessica hold more than 3000 blobs at host load average
above 40.

## Resolution (2026-10-28, branch `fix/storage-health-stats`)

Option 1 landed, seeded synchronously instead of in the background. `BlobStore` now keeps a running
tally of `total_blobs`, `total_bytes` and `chunked_blobs`. `BlobStore::new` seeds it with one walk on a
blocking thread using plain `std::fs`, before the store is handed to anything. After that, `store`,
`store_dag_cbor`, the chunked store and `delete` move the tally by exactly what they create or remove.
`stats()` is now a synchronous read of three atomics. `/health` and the shutdown log call it, and
neither walks the tree any more.

- **Why a synchronous seed and no `warming` state.** No write can race the seed walk, so the counts are
  exact from the first `/health`. The payload keeps its field names and types and gains no freshness
  field, which leaves the a2o consumer's before/after equality
  (`runtime-upgrade-propagation.steps.ts` `readBlobTotal`) meaningful. The walk runs once at boot, on
  one blocking thread, with no per-syscall async hop. Alpha's readiness probe already waits 120 s
  before its first check.
- **Exactness under concurrency.** Each blob name maps to one of 256 stripe locks, so a store and a
  delete of the same name never interleave. Concurrent stores of the same bytes create and count the
  blob once. A failed write removes its partial file, so neither the tree nor the tally keeps it.
- **Tests.** `blob_store.rs` covers four cases: the tally equals a fresh walk after every inline,
  DAG-CBOR, chunked, marker-lookalike, repeat, double-delete and absent-delete operation; `stats()`
  ignores entries planted and removed behind the store's back, which proves it does not walk; the seed
  counts what the old walk counted, including zero-byte placeholders; and concurrent same-blob stores
  count once. `http.rs` `health_reports_the_blob_tally_without_walking_the_tree` deletes the blob tree
  after three writes and shows `/health` still answers `blobs.total = 3` and the exact `bytes`.
- **Residual.** A change made to the blob tree by anything other than this process's `BlobStore`, such
  as an operator's `rm` or another process, is not seen until the next start. No production writer does
  this today. Every write in elohim-storage goes through these methods, and the eprfs tier uses its own
  root.

Still open for `done`: the deliverability lane's storage-restart stations passing on a household
whose matthew and jessica hold more than 3000 blobs, with a `/health` latency measurement.
