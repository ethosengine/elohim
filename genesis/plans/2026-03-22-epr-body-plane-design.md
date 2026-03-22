# EPR Body Plane — P2P Content Delivery Between Peers

**Date:** 2026-03-22
**Status:** Approved
**Scope:** Wire content body fetch via shard protocol after EPR Head resolution, persist to local SQLite, add single-content EPR publication, remove operator workaround

## Problem

The EPR header plane (sprint 1) resolves metadata — title, description, content type, tags, stewards, reach — but never delivers the actual content body. When a learner navigates to content stewarded by another peer, they see a skeleton card with no content. The operator workaround (`79d98cc7`) papers over this by giving Matthew all 3,525 content nodes, but the real fix is completing the body plane so peers fetch content bytes from each other.

## Architecture: DHT as Index, Shard Protocol as Delivery

The protocol spec (Part II) defines three tiers:

- **Tier 1: EPR Head (~500B)** — gossipped on DHT. Rich contextual memory: title, description, content type, stewards, reach, tags. Never carries content body. The DHT knows *what exists and where*.
- **Tier 2: EPR Document (~5-50KB)** — cached by interested peers. Full pillar context. Future work.
- **Tier 3: Content Bytes (any size)** — delivered via `/elohim/shard/1.0.0`. The shard protocol delivers *the actual thing*.

The DHT is the index, not the database. Actual content bytes always flow peer-to-peer via the shard protocol. Once a peer fetches content, it persists to local SQLite — native streaming performance from then on. Any peer who cached the content can serve it to the next reader. This is how peers recreate YouTube: the replication IS the reading.

## Design

### Resolution Flow (end-to-end)

When `GET /db/content/{id}` returns 404:

```
1. EPR Resolve     -> P2PHandle::resolve_epr(id)
                      -> Kademlia lookup or request-response to peer
                      -> Returns EPR Head (metadata + description only)

2. Shard Fetch     -> P2PHandle::fetch_shard(blob_cid)
                      -> ShardRequest::Get { hash: blob_cid }
                      -> Returns raw content bytes
                      -> Verify: sha256(bytes) == blob_cid

3. Local Persist   -> Write content record + body to SQLite
                      -> Store blob in BlobStore
                      -> Tag metadata: { "resolved_via": "p2p" }
                      -> Future GETs are local (native performance)

4. Return          -> Full ContentView from SQLite (same as locally-seeded)
```

The frontend never knows the content came from P2P. It made one HTTP GET and got back a full content record.

### P2PHandle API Extensions

Two new methods on `P2PHandle`:

```rust
/// Fetch content bytes via shard protocol from a connected peer.
pub async fn fetch_shard(&self, blob_cid: &str) -> Option<Vec<u8>>

/// Full resolution: EPR Head -> shard fetch -> returns (EprHead, content_bytes).
/// Combines both steps so the HTTP handler doesn't orchestrate P2P.
pub async fn resolve_and_fetch(&self, id: &str) -> Option<(EprHead, Vec<u8>)>
```

`resolve_and_fetch` is the single call the HTTP handler makes. Internally:
1. `resolve_epr(id)` -> EPR Head with `blob_cid`
2. `FetchShard { hash: blob_cid, reply }` command to swarm event loop
3. Swarm sends `ShardRequest::Get` to the peer that served the EPR Head
4. Verify `sha256(bytes) == blob_cid`
5. Return `(EprHead, bytes)` or `None`

**Decoupling for future multi-peer resolution:** The resolution logic is cleanly separated from the public API. Today it picks one peer. Tomorrow the resolver can rank peers by latency, try multiple in parallel, or load-balance across shard holders. The `resolve_and_fetch` contract stays stable.

### Content Persistence After P2P Fetch

When `resolve_and_fetch` returns `(EprHead, content_bytes)`:

1. Store blob via `BlobStore::store(&content_bytes)` -> hash + CID
2. Write content record to SQLite via `services.content.create()` using EPR Head fields + content_body decoded from blob bytes
3. Tag `metadata` with `{ "resolved_via": "p2p" }` for diagnostics
4. Return `ContentView` from the newly-created record

From this point on, the content is local. No more P2P needed for this content ID.

### Single Content Create — EPR Publication

`POST /db/content` (single) currently skips EPR Head publication. Add the same publish pattern used by bulk: after `services.content.create()` succeeds, `tokio::spawn` with a cloned `P2PHandle` to publish the EPR Head.

### Removing the Operator Workaround

Commit `79d98cc7` bypasses `filterBySteward()` for Matthew so he gets all content. Once the body plane works:

1. Remove the `if humanId != "human-matthew-manager"` bypass in `genesis/Jenkinsfile`
2. Matthew gets only his stewarded content via seeding (~1,362 nodes)
3. When Matthew navigates to cross-steward content -> 404 -> EPR resolve -> shard fetch -> persist -> renders natively
4. The a2o scenario becomes exercisable end-to-end

The workaround removal is the verification: if Matthew can navigate a path that crosses stewardship boundaries without "Content Not Found," the sprint is done.

## Files Changed

| Action | File | What |
|--------|------|------|
| Modify | `elohim/elohim-storage/src/p2p/mod.rs` | `FetchShard` command, `resolve_and_fetch`, resolver abstraction |
| Modify | `elohim/elohim-storage/src/http.rs` | Content GET calls `resolve_and_fetch`, persists to SQLite. Single-create publishes EPR Head |
| Modify | `genesis/Jenkinsfile` | Remove operator stewardship bypass |
| Modify | `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` | Update scenario to verify full content body |

No new files. No frontend changes. No new DB migrations.

## Risks

- **Shard fetch timeout on first request**: The first cross-peer fetch adds latency (EPR resolve + shard GET + SQLite write). Mitigate with 5s timeout on each step. After first fetch, content is local.
- **Blob CID mismatch**: If seeder content changes between peers, CID verification fails. Mitigate: CIDs are deterministic from content — same content always produces same CID.
- **Circular resolution**: Peer A asks Peer B, Peer B asks Peer A. Mitigate: only resolve from peers, never re-request content you were asked to serve.

## Verification

1. `cargo clippy -- -D warnings` clean
2. EPR + shard tests pass
3. Two-peer smoke test: seed content to peer 1 only, request from peer 2, verify full content body returned
4. Full stack: `pnpm run hc:start:seed` with stewardship filtering active for all peers. Navigate cross-steward path. Content renders.
