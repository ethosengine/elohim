---
id: "backlog-blob-put-large-body-shard-verify-drop"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "PUT /blob of a 69MB bundle dies with an empty reply — 'Shard hash mismatch during blob storage' drops the connection instead of answering, and the seed path declared a head nobody could materialize"
slug: "blob-put-large-body-shard-verify-drop"
written: "2026-08-22"
author: "orchestrator (wave-4 landing verification)"
status: "open"
priority: "high"
severity: high
tags: [dataplane, blob-store, shard-manifest, honest-shed, seed-path, bounded-code-fix]
---

# Large-blob PUT: shard-verify failure closes the connection with no response

## Observed (2026-08-22 ~05:32, local mesh, binary at 2a2d27e69)

The wave-4 prologue staged a 69MB (71,763,974 B) `elohim-host-landing` server
bundle. The doorway's `/admin/seed/blob` forward to matthew's storage failed
("error sending request"), and a direct retry at rest reproduced it
deterministically:

- `curl -X PUT --data-binary @landing-server.zip http://localhost:8090/blob/sha256-a7531572…`
  → all 71,763,974 bytes sent → **"Empty reply from server"** (no status line at
  all) in ~18s.
- matthew's log at that moment:
  `WARN Shard hash mismatch during blob storage expected=sha256-c320c27b… actual=sha256-c378932f… index=3`
- The full-blob sha256 of the sent bytes matches the address (the doorway
  verified it at cache time), so the BYTES are right — the shard-level
  expectation is what's wrong.

Earlier waves staged 18MB and 35MB bundles through the same path fine; 69MB
crosses into the sharded regime differently (MAX_INLINE_SIZE = 16MB,
CHUNK_SIZE = 1MB in blob_store.rs) — but note the 35MB bundle was ALSO above
the inline cap and passed, so this is not simply "sharding is broken."

## Two defects here

1. **Protocol honesty:** a shard-verify failure during `handle_put_blob` must
   answer (409/422 with the mismatch detail), never close the connection with
   no response. An empty reply is indistinguishable from a crash and burned
   the seeder's 30s timeout as "connection error". Same honest-shed doctrine
   as ADMISSION_SHED_MARKER.
2. **Where did the wrong expected shard hash come from?** `expected=c320c27b…`
   at index 3 was compared against freshly-chunked incoming bytes. Suspect a
   PRE-EXISTING shard manifest for this hash (the wave-4 prologue's
   seed-commitments/drill legs referenced `sha256-a7531572…` in custody rows
   BEFORE the bytes landed; shard-manifest producers are now shape-aware,
   c04c2b423) whose expected shard hashes disagree with the PUT path's
   chunking. If so, any blob whose manifest is minted before its bytes arrive
   can never be PUT — a manifest-before-bytes deadlock on the seed path.

## Repro assets

- The exact zip: `/tmp/elohim-local-mesh/landing-server.zip` (survives until
  container restart; re-derivable via the prologue's stage-landing-server-A
  zip step — note the zip is timestamp-non-deterministic, every run mints a
  new hash, which is also why the declared head moves every prologue).
- Mitigation applied at the seam (commit pending with this row):
  `scripts/ci/stage-spa-blob.sh` now FAILS the stage when the doorway answers
  `forwarded_to_storage:false` instead of stamping ✓ and declaring a head
  against unavailable bytes. The mesh was healed by re-stamping
  `serverBlobHash` to the last staged bundle (`sha256-f44b0b30…`).

## CI-deploy exposure

The fleet stages bundles through the same `stage-spa-blob.sh` → doorway →
storage forward. A prod bundle crossing the same trigger would previously
have deployed green while every conductor-side materialization failed —
the honest-failure change above turns that into a red Stage instead. The
storage-side fix (answer honestly + resolve the expected-shard-hash source)
is what actually closes the class.

## Also noticed

The local server-bundle zip DOUBLED between waves (35MB → 69MB) with no
corresponding source change — suspect stale chunk accumulation in the dist
dir being re-zipped (Angular emits new hashed chunk files beside old ones).
Worth a `git clean`-style prune in the zip step; unbounded growth here is
also what pushed the blob across the failing size class.

## Done when

A >64MB PUT with correct bytes stores and serves (or answers a real 4xx/5xx
naming the shard mismatch), the expected-shard-hash source is identified and
reconciled with the PUT path's chunking, and the prologue stages a fresh
bundle end-to-end with `forwarded_to_storage:true`.
