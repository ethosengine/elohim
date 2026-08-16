---
id: "backlog-chunked-blob-over-16mb-not-durable-mesh-repro"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Blobs over the 16MB chunk threshold are not durably persisted — chunked writes leave no disk artifacts under STORAGE_DIR and become unreadable within ~40min (local-mesh repro; likely the alpha dangling-blobHash 404 class)"
slug: "chunked-blob-over-16mb-not-durable-mesh-repro"
written: "2026-08-16"
author: "claude (local-mesh-saga-delivery shift, live RCA)"
status: "open"
priority: "high"
tags: [blob-store, chunking, durability, dataplane, local-mesh, ssr, resiliency-saga]
cites:
  - elohim/elohim-storage/src/blob_store.rs
  - genesis/data/timeline/backlog/doorway-boot-self-heal-family-mesh-repro.md
---

# >16MB chunked blobs: written, served, then gone (no restart, no delete)

Evidence trail from the 2026-08-16 local mesh (matthew peer,
`STORAGE_DIR=/tmp/elohim-local-mesh/matthew`, release binary from dev HEAD):

1. `PUT /blob/sha256-889c…` (18,372,047 bytes — the elohim-host-landing
   browser bundle) at 04:33Z → log `"Stored blob with manifest …
   shards: 18, encoding: chunked"` (blob_store.rs `MAX_INLINE_SIZE` = 16MB,
   chunk threshold comment "matches Holochain entry limit").
2. 04:33–04:35Z: `GET /blob/sha256-889c…` → 200, log
   `"Serving reassembled blob … shards: 18"`. An 18MB control blob
   (`sha256-8c7eec…`, random bytes, PUT direct at 04:32Z) also 200.
3. 05:13Z onward: `GET /blob/sha256-889c…` → 404 via
   `"blob heal: racing peers for LOCAL MISS"`. Storage did NOT restart in
   the window. The control blob 8c7eec STILL serves 200 at 05:37Z.
4. Disk truth under `$STORAGE_DIR/blobs`: whole-blob prefix dirs only
   (65MB, all <16MB objects). NO `*.d` chunk dirs, NO `*.chunks` index
   files, NO 889c or 8c7eec artifacts — for EITHER chunked blob, including
   the one still serving. `sync.sled` is 6.6MB (too small to hold 18MB).
   Wherever chunked blobs live, it is not the durable STORAGE_DIR tree.
5. No production caller of `BlobStore::delete` exists (grep; only a
   doc-store test). The SQL `shard_manifests`/`shard_locations` plane holds
   no row for the landing (those tables carry the RS/custody manifests —
   `encoding: none` rows — a separate plane from blob_store's internal
   chunk manifest).
6. Same loss at first staging attempt (hash 798c…, 04:11Z upload,
   absent by 04:18Z sweep: `shard_manifest_backfill … missing_blob: 2,
   restamp_bytes_missing: 1`) — reproducible, both times only the >16MB
   member of a 4-blob staging batch (2.7/3.5/14.4MB siblings all persisted
   under `blobs/blobs/<prefix>` and survive).

## Suspected mechanism (unproven — the RCA handoff)

Chunked writes take a different persistence path than inline writes
(`blob_store.rs:346` builds a `<blob>.d/` chunk dir — but no such dir ever
appears under STORAGE_DIR), so either the chunk path writes to a different
root (cwd-relative? an unset base?), or chunks live in an in-memory/cache
tier with eviction. The differential survival of the two 18MB blobs (the
read-hammered one died; the untouched control lived) suggests an eviction
or overwrite triggered by an interacting sweep (the landing is the only
blob with content-row + custody + redistribute interest).

## Strongest current hypothesis (added same session)

The 18 shards likely DO persist — as ordinary content-addressed blobs in
`blobs/blobs/<prefix>` (the store's count grew 27→47 across the staging
runs). What goes missing is the MANIFEST that names them: reassembly needs
the chunk/shard manifest, and the landing's manifest row is absent from
`shard_manifests` while the 04:18Z sweep reported exactly
`divergent_manifests: 1, manifests_restamped: 0, restamp_bytes_missing: 1`.
The staging re-run rotated `blobHash` (798c→889c — zip re-runs are not
content-stable), making the stored manifest divergent from the content row;
the restamp path could not rebuild it ("bytes missing") and the old one was
not kept. Manifest-loss, not byte-loss — which also explains the control
blob surviving (no content row, nothing to diverge against). If confirmed,
the fix is manifest-plane: restamp must never orphan a still-reassemblable
blob, and a divergent manifest with intact shards should be re-derivable
from the shards themselves.

## Decisive follow-up (same session, survival experiment)

Re-staged with a STABLE hash (no rotation): the blob survived 30 minutes of
sweeps (10×3-min probes, all 200) — the rotation-triggered loss is real. But
the sharper fact: **even the SERVING chunked blob has zero on-disk artifacts
under `$STORAGE_DIR/blobs`** (no prefix dir, no `*.d`, no `*.chunks`, and
nothing new written to the tree during the window). Chunked (>16MB) blobs
are being served from process memory in the current build — the
`chunk_dir` persistence path (blob_store.rs:346) is either not invoked or
writes to another root. Consequences: every restart loses every >16MB blob
silently, and the earlier 40-minute losses were most plausibly in-memory
eviction/replacement (the read-hammered entry died; the untouched control
lived — consistent with an eviction or keyed-overwrite, not a sweep
delete: the shard_manifest_backfill restamp arm was read and only prunes
`shard_locations` metadata, never bytes). Fix site: the chunked write path
must persist chunk artifacts under STORAGE_DIR and reassembly must read
them back — restart-durability is the acceptance test.

## Third decisive fact — /apps extraction cannot see chunked blobs at all

With the chunked blob SERVING via `GET /blob/{hash}` (memory reassembly,
200 in ~1s), `GET /apps/elohim-host-landing/index.html` returns
`{"error": "App ZIP blob not found: sha256-889c…"}` in 0.13s — every time,
cold and warm, converged fleet. The /apps ZIP extractor resolves the bundle
through a lookup that only finds on-disk whole blobs; memory-chunked
(>16MB) blobs are invisible to it. Consequence: a >16MB browser bundle can
NEVER serve as an app through /apps on this build — the SSR shell fetch
(doorway `projected_shell_url` → `/apps/{slug}/index.html`) fails, the CSR
fallback needs the same surface, and the root mount stays 404/503. On the
local mesh this structurally pins saga ch03/ch04/ch05/ch09/ch10. The fix
must make the /apps extraction path read through the same blob API that
/blob serves (reassembly-aware), in addition to the durability fix above.

RCA pointer for the successor: `get_blob_or_heal` (http.rs:2830) calls
`self.blob_store.get(hash)` — nominally the SAME method the 200-serving
/blob route uses. The /apps-miss-while-/blob-hits split therefore implicates
either (a) two BlobStore INSTANCES with different roots on one process
(cf. the June-dated 101MB store at `$XDG_DATA_HOME/elohim-storage` — the
default-root class the STORAGE_DIR arg comment warns about), or (b) a
hash-form divergence upstream of the call (`parse_content_address`
normalization vs the row's stored form). One instrumented run with the
store root logged at both call sites decides it.

- The elohim-host-landing BROWSER bundle is ~18MB — over the threshold —
  on alpha too. The documented 2026-06-09 regression class ("dangling
  blobHash rows that 404 the projected EPR apps") has this signature.
- ch04/ch05/ch09/ch10 of the resiliency saga all sit behind a durable,
  reassemblable landing bundle; on the mesh this bug re-reds them at will.
- The grandma-photos epic promise (bytes survive) is violated silently:
  upload 200, serve 200, then quiet loss with no counter and no signal —
  the exact anti-pattern the observation layer exists to catch.

## Verification hooks

- `PUT` an 17MB+ blob, confirm `<hash>` chunk artifacts exist under
  `$STORAGE_DIR` and survive a process restart; `GET` remains 200 after
  every background sweep class has run at least once (custody-backfill,
  shard_manifest_backfill, salvage recheck, inventory broadcast).
- Saga ch04 root-serve stays green across two measure runs 1h apart on the
  mesh without re-staging.

## Status 2026-08-16 — manifest-durability fix landed (Q2, minutes-quiesce W1.1)

Three of this record's suspicions are now resolved against the source, and
two of them were wrong in a way worth preserving:

1. **Bytes were never lost.** Chunked shards ARE persisted, as ordinary
   content-addressed blobs under `blobs/blobs/<prefix>` — one per 1MB shard,
   each under its OWN hash. What never exists on disk is an artifact named
   for the COMPOSITE hash: `put_blob_bytes` (`http.rs`) stores each shard and
   never stores the whole blob under its own name. So "no `889c…` prefix dir"
   is expected, not evidence of memory-only storage. `blob_store.rs`'s internal
   `.d/`/`.chunks` chunker is dead code on this path — it is never invoked by
   the HTTP ingest route, which is why no such artifact ever appeared.
2. **The /apps-vs-/blob split was neither (a) two BlobStore roots nor (b) a
   hash-form divergence.** `get_blob_or_heal` read the COMPOSITE hash from
   `blob_store` — a guaranteed local miss for any `encoding != "none"` blob,
   because the composite is never stored. `/blob` hit only because it consulted
   the in-process manifest map first and reassembled; `/apps` never consulted a
   manifest at all. 404 by construction, no instrumented run needed.
3. **The ~40-minute loss is NOT in-process eviction.** The manifest map is
   insert-only: no `remove`/`clear`/`retain` call exists anywhere in the crate,
   and exactly one `HttpServer` is constructed per process (`main.rs`). An
   entry cannot be evicted or overwritten out of it in-flight. The remaining
   candidates are (i) a storage process restart not observed at the time, or
   (ii) the probe reaching a DIFFERENT storage process than the PUT did — the
   local mesh runs one storage per peer and doorway B carries its own
   single-target storage URL, so an A-minted composite is structurally
   unservable from B. (ii) is the same cross-process class this fix addresses
   locally and W1.2 (manifest propagation) addresses across peers. Honest
   status: not decided between (i) and (ii) from this record's evidence.

**Fix (working tree, elohim-storage):** the durable `shard_manifests`
projection — already written at ingest, never read on the serving path — is
now the fallback. `resolve_manifest()` checks the in-process map, then
`db::shard_manifests::get_manifest_by_blob_hash()`, hydrating the map on a
hit; `GET /blob/{hash}`, `GET /manifest/{hash}` and `get_blob_or_heal` all
route through it, and `get_blob_or_heal` reassembles from local shards before
racing peers. Restart-fatal and `/apps`-404-by-construction are both closed;
a shard-level local miss now 404s naming the missing shard index/hash instead
of reporting the composite as simply absent. Regression tests:
`tests/chunked_blob_manifest_durability.rs`.

Still open from this record: cross-PEER serving of a composite (a peer holding
replicated shards has no manifest row until W1.2 propagates one), and the
blobHash-rotation restamp path that orphaned the first staging attempt.
