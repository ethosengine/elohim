---
id: "backlog-self-heal-custody-reconcile-kick-storm"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "T23 custody reconcile has no unsatisfiable-kick state — the same 5 commitments are re-kicked every pass forever, and on the large-corpus peer the O(corpus) pass rebuild pins ~2 cores and 1.3 GB"
slug: "self-heal-custody-reconcile-kick-storm-after-death-witness"
written: "2026-09-07"
author: "claude (rust-architect — live READ-ONLY diagnosis on the sprint-0908 household mesh, 22:58–23:10Z)"
status: "open"
priority: "high"
relatedNodeIds: []
tags: [self-heal-exhaustion, custody-reconcile, T23, T21, blob-fetch, backoff, convergence, dataplane-convergence, runtime-death-witnessed, local-mesh, household-nodes, declare]
cites:
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/reconcile/custody_sweep.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/p2p/behaviour.rs
  - elohim/elohim-storage/src/blob_store.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/pprof_endpoint/mod.rs
---

# The custody reconcile kick storm — measured, mechanism, and the missing convergence condition

> Diagnosed READ-ONLY on the running household mesh (`/tmp/elohim-local-mesh`),
> binary = family/sprint debug slot at the batch-2 tree (`6bef71b84`), all three
> storage peers restarted together at **22:58:17–22:58:20Z**. Nothing was
> restarted, killed, or written to by this diagnosis.

## Symptom, with numbers

matthew (`:8090`, pid 110622, started 22:58:17Z) vs its two siblings on the
**same binary, same age, same mesh**:

| peer | RSS | CPU | T21 fetch failures since 22:58 |
|---|---|---|---|
| matthew `:8090` | **1 345 MB** | **~150–195 %** (977 s CPU in 11.4 min) | ~290 |
| jessica `:8091` | 331 MB | ~9 % | **979** |
| james `:8092` | 330 MB | ~9 % | **967** |

- `/health` and `/metrics` on `:8090` answer intermittently between **2.6 ms and
  4.0 s** (10 s+ observed earlier). 167× hyper `Connection error: connection
  closed before message completed` — clients giving up on a starved server.
- `T23: custody reconcile pass completed` fires **~7–8×/min** with
  `snapshot_size: 3589→3592, commitments_examined: 50→66, kicks_fired: 5–8,
  fallback_kicks: 5, placement_gaps: 0, connected_peers: 2` — the same numbers,
  pass after pass, 432 passes logged. Passes **overlap**: 23:07:49.647Z and
  23:07:50.188Z are 0.54 s apart.
- `/p2p/status` on matthew: `connectedPeers: 2`, `reconcilePassesTotal: 94`,
  **`kicksFiredTotal: 502`** since the 22:58 restart — 502 kicks, and the
  local pantry still misses the same hashes.
- **Correction to the reported symptom:** the dominant T21 error is
  **`Timeout` (678), not `ConnectionClosed` (56)** — 92 % timeouts. Split across
  both peer ids proportionally (`12D3KooWFPKj…` 401 T / 33 CC;
  `12D3KooWPnhU…` 277 T / 23 CC).

## 1. What is burning the CPU

**Evidence (confirmed).** `top -H` + `/proc/110622/task/*/stat`:

- The two hot threads are named **`http-server`** — the runtime built at
  `elohim/elohim-storage/src/main.rs:270-272` with `.worker_threads(2)` and
  `.thread_name("http-server")`.
- They are **pure userspace**: `utime=168.9 s / stime=1.1 s` (≈ 99 % user, near-zero
  syscalls) — a compute loop, **not** an I/O or retry storm.
- Their **TIDs cycle**: `110668/110865` at 23:05:36Z (418 s each) were gone by
  23:07:28Z, replaced by `117839/124058`, then `126016`. Tokio applies
  `thread_name` to the **blocking pool** as well as workers, and blocking threads
  die after their idle keep-alive. So these are *blocking-pool* threads running
  long CPU-bound tasks back-to-back, two at a time, continuously — not the two
  wedged workers.
- Each new thread runs at **82–95 % of its own lifetime** in CPU.

**Ruled out.**
- *Blob-fetch retry storm*: matthew fires ~26 T21 failures/min; **jessica fires 89/min
  and sits at 9 % CPU**. The retry volume cannot be the burner.
- *Gossip republish*: 4 publishes/min steady (`T22: published bounded inventory
  refresh` = 3–5/min); the 33 and 80 `gossipsub publish failed` bursts are at
  22:55 and 22:58 — the two restart moments, "no peers subscribed yet".
- *Feedback projector sweep / notify wake (T6/T7a)*: `SweepScheduler`
  (`services/feedback_projector.rs:228-240`) and `admit_notified_signal`
  (`:1714-1729`) have **no `spawn_blocking` site** anywhere in
  `feedback_projector.rs` or `epr_atom_service.rs` (27 `spawn_blocking` sites in
  the crate, none in the projector, reconcile, blob_store, or p2p paths), and
  neither emits a single log line since the 22:58 restart.
- *Conductor-metrics poller*: `/db/p2p/conductor-diagnostics` answers normally
  (15 agents, all `storageArc: [0, 4294967295]`, no tombstones).

**The remaining candidate, strongly indicated but NOT proven:** the T23 pass
itself. It is the only per-pass work that is **O(corpus)** and it is the only
thing that distinguishes matthew (3 589 blobs) from jessica/james (small):

- `BlobStoreSnapshot::from_store` (`reconcile/custody.rs:63-71`) calls
  `blob_store.list_hashes()` on **every pass**, which is a full two-level
  `read_dir` walk plus a **`path.metadata()` stat per file**
  (`blob_store.rs:632-672`) — ~3 590 stats + 3 590 `String` allocations per pass,
  ~8 passes/min, with passes overlapping.
- The commitment scan has **no `.limit()`** — it loads *all* `custody-blob` rows
  (`reconcile/custody.rs:173-176`) and iterates them (`:188`).
- The 1 345 MB RSS on an 11-minute-old process, against 330 MB siblings, is the
  same corpus-proportional signature.

**The decisive probe already exists and is cheap:** `GET
/debug/pprof/profile?seconds=N` (`pprof_endpoint/mod.rs:1-27`) — Go-pprof wire
contract, env-gated, currently returns **404** because the gate is unset. Turn
it on for the next mesh start and this question becomes a 15-second answer
instead of an inference.

## 2. Why every outbound blob fetch fails

**It is not the custody gate refusing.** The `/elohim/blob/1.0.0` inbound
handler (`p2p/mod.rs:6100-6205`) has **no reach or custody gate at all**. Every
branch — malformed address, found, not-found, manifest, DB error — builds a
typed `BlobFetchResponse` and calls `send_response(channel, response)`
(`:6190-6198`). There is **no silent drop**, so a refusal cannot masquerade as
`ConnectionClosed` on this protocol.

- `blob_reach::blob_serve_verdict` is wired **only** into the HTTP
  `GET /blob/{hash}` route (`http.rs:3792-3850`).
- `private_reach::private_serve_verdict` is wired **only** into the shard
  replication plane (`/elohim/shard/*`), and there it answers a typed
  `ShardResponse::Error("reach-withheld: …")` — never a drop.
- The 363 `reach-withheld` lines in matthew's log are `site: list_content`,
  `reason: ward-unresolved` — the **inventory listing** plane, not blob serve.

**What is actually happening.** On every pass `fallback_kicks == kicks_fired`
(5 == 5). Per `reconcile/custody.rs:291-304`, that means the gossip
`peer_blob_inventory` names **zero** holders for those blob hashes, so the sweep
falls back to racing whichever peers are currently connected — blind. Neither
connected peer holds the bytes. The request-response timeout is **30 s**
(`behaviour.rs:377-384` with `config.request_timeout`, default at
`p2p/mod.rs:686`), max response 16 MiB. The requests expire.

Why they expire rather than returning a fast `NotFound`: **the storm is mutual
and matthew is its amplifier.** jessica (979) and james (967) fire *more*
timeouts than matthew, and matthew — with a 2-worker runtime whose blocking pool
is saturated — is the peer that cannot answer their inbound blob requests within
30 s. Each peer's unsatisfiable kicks keep the others' request queues full.

## 3. The missing convergence condition

`kicks_fired: 5` is **not** a configured cap. `FALLBACK_CANDIDATE_CAP = 8`
(`custody.rs:91`) caps *candidate peers raced per kick*, not kicks per pass. The
5 is simply the live count of own-provider `custody-blob` commitments whose hash
is absent from the local pantry that pass (`custody.rs:242-244`:
`if !local_store.has(&blob_hash) { … }`).

**Satisfaction has exactly one definition: the bytes appeared in the local
pantry.** There is:

- **no backoff** — `FetchOutcome::Miss` and `NoCandidates` only `debug!` and
  return (`custody_sweep.rs:256-269`); nothing is written anywhere;
- **no failure counter** — no DB table, no in-memory map keyed on `(cid)` or
  `(cid, peer)`;
- **no terminal "unsatisfiable" state** — the *only* terminal case is
  `FetchOutcome::InvalidAddress` (a structurally malformed content address,
  `custody_sweep.rs` ~`:276-283`), which is about address hygiene, not
  reachability;
- **no record of the refusal/absence in the snapshot** — the snapshot only
  records what is present, so an absent-and-unfetchable hash is
  indistinguishable from an absent-and-about-to-arrive one.

> **The missing condition, stated once:** the sweep has a *satisfied* state and a
> *malformed* state, but **no exhausted state**. A CID that no reachable peer
> holds is re-kicked every pass, forever, at full fan-out — 502 kicks in 11
> minutes on matthew for the same 5 commitments — and the substrate never says
> "this custody obligation cannot be met right now."

`placement_gaps: 0` on every pass is the tell: the honest signal for "I am the
provider and nobody has these bytes" exists and is **not** being emitted, because
the blind fallback kick fires first and consumes the outcome.

## 4. Did jessica's SIGKILL + ark restart cause this? No

- Failures split across **both** peer ids in proportion (401/277 Timeout,
  33/23 ConnectionClosed) — james, which was never killed, fails identically.
- jessica **and** james each log ~970 T21 failures of their own since 22:58.
- All three storage processes were restarted **after** the death-witness drills
  (22:58:17–20Z) and the storm survived that restart intact.
- `connected_peers: 2` on every pass and `/p2p/status connectedPeers: 2` —
  the libp2p mesh is healthy; there is no stale-address problem.

The death-witness drills are the *occasion* (they left custody commitments whose
bytes never landed), not the *cause*. The cause is that the sweep has no way to
stop asking.

## The smallest cure (design only — no code in this atom)

Three changes, in dependency order. The first alone converts the storm into a
signal; the second alone removes the CPU floor.

1. **Give the kick an exhausted state (the actual convergence fix).**
   Carry a per-`(blob_hash)` attempt record — in-memory `DashMap` is sufficient;
   it need not survive restart — holding `{ attempts, last_attempt_at,
   next_eligible_at, last_outcome }`. On `Miss` / `NoCandidates` / `Timeout`,
   apply exponential backoff (e.g. 30 s → 5 min cap) and increment. After N
   consecutive non-hit outcomes **with zero gossip-inventory candidates**, mark
   the hash `unsatisfiable` and stop kicking it; re-arm on either (a) a gossip
   inventory update that names a holder, or (b) a new peer connecting. Any
   `FetchOutcome::Hit` clears the record.

2. **Emit the placement gap instead of the blind fallback kick.**
   When `peer_blob_inventory` names zero holders, the honest move is
   `placement_gaps += 1` (the signal already exists and already has a cooldown at
   `custody.rs:328-340`), *not* a blind race against whoever happens to be
   connected. Blind fallback should be reserved for the case where inventory is
   merely *stale*, not *empty*.

3. **Stop rebuilding the whole pantry index every pass.**
   `BlobStoreSnapshot::from_store` should be cached with an invalidation hook on
   blob write/delete, or at minimum rebuilt on its own slow cadence (e.g. 60 s)
   decoupled from the sweep tick — and the sweep should hold a single-flight
   guard so passes cannot overlap (observed 0.54 s apart). Independently, bound
   the commitment scan (`custody.rs:173-176`) with a `.limit()` + rotating
   offset so pass cost is O(batch), not O(corpus).

Also (one-line, unblocks all future instances of this question): **default the
pprof gate on for local-mesh runs** so `/debug/pprof/profile` is available
without a restart when a peer goes hot.

## Which habit this serves

- **`dataplane-convergence`** — primary. A custody obligation that can never be
  met must reach a declared terminal state; today it burns fan-out forever and
  the convergence measure cannot distinguish "healing" from "hopeless."
- **`runtime-death-witnessed`** — secondary. The death-witness drills are what
  seeded the unfetchable commitments; a drill that leaves a permanent kick storm
  behind is a drill whose blast radius is not yet contained.

## Done when

1. A custody-blob commitment whose bytes no reachable peer holds stops being
   kicked within ≤ 5 passes, and the pass logs a non-zero `placement_gaps`
   instead of a `fallback_kicks`.
2. `kicksFiredTotal` on `/p2p/status` goes **flat** on a mesh with an
   unsatisfiable commitment, instead of climbing ~45/min.
3. On the large-corpus peer (≥ 3 500 blobs), steady-state storage CPU is within
   2× of its small-corpus siblings, and `/health` p99 is < 250 ms.
4. A re-arm test: after the exhausted state is reached, connecting a peer that
   *does* hold the bytes resumes the kick and the blob lands.
5. A regression scenario carrying the parameter discovered here — 3 589 blobs,
   ~8 passes/min, 502 kicks / 11 min / 5 commitments — so the O(corpus) pass cost
   is measured, not remembered.

## Operational note (2026-09-07)

matthew's `:8090` still serves; it is latency-degraded, not wedged. Control-plane
traffic (POST a compute task, poll status) works with generous timeouts. Any leg
that needs **matthew to serve blob bytes over libp2p within 30 s** should be
expected to time out. A mesh restart clears the CPU but **not** the loop — the
storm survived the 22:58 restart and re-established within ~7 minutes.
