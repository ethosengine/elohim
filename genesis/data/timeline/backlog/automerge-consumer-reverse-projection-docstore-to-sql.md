---
id: "backlog-automerge-consumer-reverse-projection-docstore-to-sql"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Automerge sync consumer heals nothing: applied changes land in the sled DocStore ONLY — no reverse-projection back into the SQL `content` table, so a degraded peer's SERVING path never converges (the elohim.host blobHash=null 404 class)"
slug: "automerge-consumer-reverse-projection-docstore-to-sql"
written: "2026-07-01"
author: "overnight shift — p2p-sync feature-completeness (automerge-content-sync-projection-completeness)"
status: "backlog"
priority: "high"
ci_status: blocked
jobs: [elohim]
tags: [automerge, content-sync, dataplane, consumer, docstore, sql-projection, provenance, elohim-host, blobHash, heal, p2p-design-gate]
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/sync/projector.rs
  - genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
---

## The gap (the capstone the producer-side work sets up but does NOT close)

The Automerge content-sync plane is a producer→wire→consumer chain. As of the
2026-07-01 producer-completeness shift the PRODUCER side is done: full-field
projection incl. `blobHash` (`sync/projector.rs::projected_fields`) + gated
idempotent corpus back-fill. But the CONSUMER side is inert for healing:

- `SyncResponse::Changes` (`p2p/mod.rs:6549`) and `AnnounceChange`
  (`p2p/mod.rs:6402`) both call `sync_manager.apply_changes(...)`, which writes
  the merged Automerge doc into the **sled DocStore only**.
- **Nothing projects that converged doc back into the SQL `content` table.** All
  HTTP serving reads (`lookup_slug_blob_hash`, `/db/content`, the SPA mount at
  `http.rs:5601`) read the SQL projection, not the DocStore.

**Consequence — this is exactly the elohim.host failure class:** a peer whose
`content` row lost its `blobHash` (deploy PATCH never landed on a degraded
conductor → `blobHash=null` → `App not found` 404) can now RECEIVE a healthy
peer's full-field doc over the sync plane… into sled… where serving never looks.
The plane converges the CRDT layer but cannot heal the serving path. Closing this
loop is what would let "get the p2p dataplane syncing" actually fix a 404 without
the DHT write path (which is operator-owned; see
[[conductor-websocket-flap-breaks-deploy-write-path]]).

## Why this is a `/brainstorm` ceiling item, not a blind build

Two genuine design decisions gate it — do NOT wire reverse-projection without them:

1. **Provenance / authority (security).** Writing a PEER's synced content into the
   local AUTHORITATIVE SQL table means accepting peer-asserted content. The
   backfill backlog notes the docs are "PROJECTIONS of already-notarized content"
   — but the consumer must *verify* that (dht_anchor_hash / signature / reach
   authorization) before trusting a doc into serving truth. Un-gated, this is a
   content-injection vector. (Sibling posture: `reach_authorization.rs` is
   author-earn + receiver-preauth, not delivery-filter.)
2. **Two-plane authority.** There are TWO peer-content planes: the shard/
   replication plane (`run_replication_cycle` / `ShardRequest::ListContent` /
   gap_queue — byte/record replication that DOES write SQL) and this Automerge
   metadata plane. Which one owns authoritative content-row heal? If the shard
   plane is the authority, the Automerge plane may be metadata/fog-of-war only and
   reverse-projection would be wrong/redundant. The lighting-plan spec must
   designate this before building. (mod.rs doc header calls the sync docs "Content
   metadata … node stubs for fog-of-war" — which argues AGAINST authoritative
   heal here.)

## Shape (to design when picked — p2p-design-gate first)

- A consumer projector: on `apply_changes` for `PROJECTION_NAMESPACE`, resolve the
  doc's `id` + fields → upsert into `content` via a provenance-gated path.
- Reuse `projected_fields`' key set (id/hAppId/title/…/blobHash/serverBlobHash/…)
  as the read-back contract — the producer/consumer field list is one source of
  truth (mirror the `PROJECTION_NAMESPACE` compile-coupling discipline).
- Idempotent + last-writer/CRDT-merge-aware; must not fight the shard plane's writes.

Domain D5 (data plane). Depends on: producer-completeness (LANDED 2026-07-01).
Effort: L. Blocked-by: the two design decisions above → route to `/brainstorm`.
