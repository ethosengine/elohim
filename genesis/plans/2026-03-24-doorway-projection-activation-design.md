# Doorway Projection Activation — Web2 Surface for P2P Content

**Date:** 2026-03-24
**Status:** Approved
**Scope:** Activate the dormant projection infrastructure so doorway serves commons/public content from all peers, not just the operator's storage.

## The Problem

Doorway currently proxies `/db/*` requests to a single `STORAGE_URL` — the operator's own elohim-storage. Content stewarded by other peers returns 404 even though it exists on their nodes. A visitor at `alpha.elohim.host/lamad` should see all commons content across the network, not just Matthew's.

## Architecture: Doorway as the Peer's Porch

Doorway is not anonymous infrastructure — it's attested for by the operator's peer identity, analogous to a domain registrar's record. Matthew operates this doorway and vouches for its integrity, but the doorway has its own agent key on the P2P network. Public visitors interact with the doorway's identity, not Matthew's personal trust context.

**elohim-storage (SQLite)** = the peer. Matthew's node. Stores his stewarded content. Participates in P2P. Builds EPR Heads. This is the protocol participant.

**doorway (MongoDB)** = the web2 bridge. Separate process on the same device. Projects content from ALL peers the operator serves into its own cache. Handles anonymous web traffic, auth, federation, account recovery. This is the community service layer.

Doorway does NOT write into the peer's SQLite — that's the peer's data. Doorway projects into MongoDB, its own data store.

```
5 peer conductors (Matthew, Jessica, Pete, Terrance, Frank)
  → signal subscribers in doorway (one per conductor)
  → MongoDB projection cache (doorway's own data store)
  → /api/v1/cache/ serves aggregated commons content

Matthew's elohim-storage (SQLite)
  → his stewarded content only
  → /db/ routes serve HIS peer data
  → P2P participation, EPR Heads for his content
```

## P2P Relationship

Peers behind the doorway handle their own discovery and routing natively — that's a P2P capability. Doorway's projection cache is complementary and protective: it shields peers from heavy web2 read load. Content availability and peer discovery remain P2P concerns.

Doorway subscribes to peer performance signals to inform routing decisions when direct peer reads are needed (authenticated content, reach-gated requests). The routing intelligence lives in the P2P layer, not in doorway.

The doorway's own peer identity sees commons/public EPR Heads on the network — those flow freely without trust context. Navigation between peers happens through the content graph's EPR relationships, not through the operator's personal trust. The operator attests that this doorway is legitimate infrastructure, but the doorway's view of the network is limited to its own (minimal) trust context.

## What Exists (Dormant)

The entire projection infrastructure is built but not activated:

| Component | File | Status |
|-----------|------|--------|
| Projection engine (type-agnostic signals) | `projection/engine.rs` | Built, not spawned |
| Projection store (MongoDB + hot cache) | `projection/store.rs` | Built, memory-only mode |
| Signal subscriber (conductor WebSocket) | `projection/subscriber.rs` | Built, not spawned |
| `/api/v1/cache/` HTTP handler | `routes/api.rs` | Built, responds to requests |
| DoorwayResolver (Projection → peer fallback) | `cache/resolution.rs` | Built |
| Angular `ProjectionAPIService` | `projection-api.service.ts` | Built, uses `/api/v1/cache/` |
| Angular `DoorwayCacheService` | `doorway-cache.service.ts` | Built |
| MongoDB deployment | `manifests/infra/alpha-mongodb.yaml` | Deployed |
| DNA `Cacheable` impl with reach | `content_store_integrity/src/lib.rs` | Built, emits signals |

## Activation Design

### 1. Spawn Signal Subscribers

In `main.rs`, for each peer storage URL (`STORAGE_URL` + `STORAGE_URLS`), resolve the conductor admin URL (same pod, known port) and spawn a signal subscriber. Each subscriber:

- Connects to the conductor's admin WebSocket
- Authenticates via `AppAuthenticationToken`
- Listens for `DoorwaySignal` / `CacheSignal` on the app interface
- Writes received signals to the shared `ProjectionStore`

All subscribers write to the **same** MongoDB projection — content from all peers aggregates naturally.

If a conductor is unreachable, log and retry with backoff. Don't block startup.

### 2. Initialize ProjectionStore with MongoDB

Change `AppState` initialization in `main.rs` from memory-only to MongoDB-backed. The MongoDB connection string comes from environment (`MONGODB_URL`). Graceful degradation: if MongoDB is unreachable, fall back to memory-only with a warning.

### 3. Read Path

**Public/commons content (web2 visitors):**
```
GET /api/v1/cache/Content/{id}
  → Hot cache (DashMap, in-memory)
  → MongoDB projection
  → Peer fallback (route to a peer's storage, cache result for next time)
```

**Auth-gated content (authenticated users):**
```
GET /api/v1/cache/Content/{id} with auth header
  → Projection (if commons/public, serve from cache)
  → Peer storage with auth context forwarded (reach-gated content)
  → Do NOT cache auth-gated responses in projection
```

**Direct peer access (writes, steward queries):**
```
POST /db/content, GET /db/stats, etc.
  → Direct to operator's STORAGE_URL (no change)
```

### 4. Angular Read Switch

`DataLoaderService` switches from `StorageClientService` (→ `/db/`) to `ProjectionAPIService` (→ `/api/v1/cache/`) for content reads. The `ProjectionAPIService` already exists with `getContent()`, `getContents()`, `getPath()`, `getPaths()`.

Writes stay on `/db/` path (seeding, mutations go direct to the operator's storage).

### 5. Signal Data Shape

The DNA's `Content` entry implements `Cacheable` and carries `reach`, `content_type`, `title`, `description`, `tags`, `blob_cid`. The `CacheSignal::upsert(&content)` serializes the full struct. The projection response includes reach levels — Angular's reach badges work from this data.

Stewardship allocations are not in the content signal (they're a separate entry type). For stewardship display, the trust tab's `StewardshipAllocationService` calls the operator's storage directly. This is correct — stewardship is per-peer data.

## What Changes

| File | What |
|------|------|
| `doorway-service/src/main.rs` | Spawn subscribers, init ProjectionStore with MongoDB |
| `doorway-service/src/services/discovery.rs` | Already fixed (Text WS response + multi-DNA parsing) |
| `elohim-app/src/app/elohim/services/data-loader.service.ts` | Read content from projection API |
| `doorway-service/src/routes/api.rs` | May need minor adjustments for Content shape alignment |

## What Doesn't Change

- `/db/` routes (still direct to operator's storage for writes)
- EPR Head building (storage does this for P2P)
- Reach badges (ContentView shape preserved in projection)
- Trust tab stewardship (loads from operator's storage)
- P2P content resolution (EPR protocol handles cross-peer discovery independently)

## Future (Not This Sprint)

- CDN federation between doorways via shared MongoDB
- Peer performance signal routing (doorway subscribes to P2P health signals)
- EPR Head projection (doorway extends operator's EPR Heads to web2 visitors)
- Doorway-to-doorway content replication via DNS routing
