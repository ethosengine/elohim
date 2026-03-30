# Resilient HTML5 App Delivery — Three-Sprint Design

**Date:** 2026-03-30
**Status:** Approved (Sprint 1 implemented)
**Context:** Evolution of Trust HTML5 app causes storage container OOM under browser load (30+ concurrent asset requests). The immediate fix (doorway projection cache) is one concern among three permanent architectural roles.

**A2O Scenarios:**
| Feature File | Sprint | Scenarios |
|---|---|---|
| `genesis/a2o/features/delivery/web2-absorption.feature` | Sprint 1 | 7 — cache population, hits, coalescing, invalidation, replicas |
| `genesis/a2o/features/delivery/delivery-diagnostics.feature` | Sprint 1+2 | 11 — X-Cache headers, layer observability, controlled degradation |
| `genesis/a2o/features/delivery/client-resilience.feature` | Sprint 2 | 11 — SW registration, offline, capability probe, delivery modes |
| `genesis/a2o/features/federation/peer-advertisement.feature` | Sprint 2 | 16 — gossipsub heartbeat, neighbor table, dynamic state changes |
| `genesis/a2o/features/delivery/peer-mesh.feature` | Sprint 3 | 10 — LAN mesh, multi-peer resolution, fallback chain, QueryDelivery |
| `genesis/a2o/features/elohim/network-health-posture.feature` | Sprint 3+ | 19 — aggregate posture, attestation-gated introspection, elohim reasoning |
| `genesis/a2o/features/shefa/human-resilience.feature` | Future | — resilience profile, mutual aid, stewardship |

## Problem

A browser loading an HTML5 app fires 30-40 concurrent requests for JS, CSS, fonts, and images. All requests funnel through `doorway → storage`, overwhelming storage's HTTP server and causing OOMKilled (exit code 137) in constrained containers. Storage is a P2P node with conductor, DHT, and blob store responsibilities — it should not be a CDN.

Layer 1 (elohim-cache-core ExtractionCache) is implemented and works — files are served from disk, not re-extracted from ZIP. But every request still flows through storage. The problem is traffic pattern, not decompression.

## Architecture: Three Permanent Concerns

```
CONCERN 1 — WEB2 ABSORPTION (Sprint 1: Doorway Projection Cache)
  Role: Protect the P2P network from browser traffic patterns.
        Onboarding flywheel — anyone with a URL gets the protocol experience.

  Browser → Doorway → MongoDB (HIT) → return (storage never touched)
                    → MongoDB (MISS) → Storage → cache in MongoDB → return

  Permanent. Scales horizontally via doorway replicas sharing MongoDB.

CONCERN 2 — CLIENT RESILIENCE (Sprint 2: Service Worker + Capability Negotiation)
  Role: Make every browser/Tauri instance a capable, self-sufficient peer.
        Negotiate delivery based on what the serving peer can actually do.

  Browser → SW Cache (HIT) → return (zero network)
          → SW Cache (MISS) → ask peer: "can you serve extracted files?"
            → YES (peer has cache) → fetch individual files
            → NO  (peer has ZIP only) → fetch ZIP → SW extracts locally

  Permanent. The client's own resilience layer. Universal across browser + Tauri.

CONCERN 3 — PEER MESH (Sprint 3: P2P App Delivery)
  Role: Peers serve peers. The protocol doing what it was designed for.
        Doorway becomes one peer among many, not the mandatory funnel.

  Browser → SW Cache (HIT) → return
          → SW Cache (MISS) → EPR resolve knownLocations
            → LAN peer via mDNS? → fetch (Tauri<>Tauri or Tauri→browser)
            → Remote peer via relay? → fetch
            → Doorway (one of N sources)? → existing path

  Permanent. True decentralization of content delivery.
```

All three coexist permanently. None replaces the other. Doorway is not a band-aid — it is the web2 absorption layer that protects the entire P2P network and serves as the onboarding flywheel into the protocol.

**The invariant across all three:** The ZIP blob is truth. CID-addressed, content-verified. Every cache layer — MongoDB, extraction disk, SW CacheStorage — is a projection that can be rebuilt from the blob. Hash-based invalidation propagates through all layers on re-seed.

## P2P Design Gate

All entities in this design are **Operational (Category C)**. No new DHT entry types. No new storage projections with `dht_anchor_hash`. Pure cache/delivery infrastructure.

### Entity: AppFileCache (doorway MongoDB projection)
- **Classification:** Operational (C)
- **Justification:** Extracted app files cached in doorway's MongoDB. Fully reconstructable from the ZIP blob in elohim-storage.
- **Content Address Strategy:** Content-Derived (CID) — keyed by `{app_id}:{blob_hash}:{file_path}`
- **Source of Truth:** ZIP blob in elohim-storage BlobStore
- **Anti-Pattern Check:** Not creating DHT entry types for cache state.

### Entity: DeliveryCapabilities (extension of NodeCapabilities)
- **Classification:** Operational (C)
- **Justification:** Peer's advertised delivery ability. Ephemeral network state. Extends existing `NodeCapabilities` struct.
- **Source of Truth:** Each peer's local config. Advertised via identify protocol + gossipsub.
- **Anti-Pattern Check:** Not putting ephemeral capability state on DHT.

### Entity: SW AppCache (client-side)
- **Classification:** Operational (C)
- **Justification:** Service Worker's local cache. Entirely client-side. Reconstructable by re-fetching.
- **Source of Truth:** The ZIP blob.
- **Anti-Pattern Check:** No server-side state created.

## Sprint 1: Doorway Projection Cache

### Design

**doorway/doorway-service/src/routes/apps.rs** transforms from dumb proxy to cache-first handler:

```
GET /apps/{app_id}/{file_path}

1. Compute cache key: "apps:{app_id}:{file_path}"
2. MongoDB lookup (app_file_cache collection):
   - Match on app_id + file_path + blob_hash (from content projection)
   - HIT → return cached bytes + content_type. Storage never touched.
   - MISS → continue to step 3.
3. Proxy to storage: GET {STORAGE_URL}/apps/{app_id}/{file_path}
4. On 200 response: cache {bytes, content_type, blob_hash} in MongoDB
5. Return response to browser.
```

### MongoDB Collection Schema

```
// Source of truth: ZIP blob in elohim-storage BlobStore (operational cache, Category C)
// Reconstruction: delete collection, next request re-fetches from storage
app_file_cache {
  _id: ObjectId,
  app_id: String,           // "evolution-of-trust"
  file_path: String,        // "pixi.min.js"
  blob_hash: String,        // "sha256-abc123..." (invalidation key)
  agreement_id: String,     // EPR agreement authorizing this cache (stub: self-negotiated)
  content_type: String,     // "application/javascript"
  data: Binary,             // raw file bytes
  cached_at: DateTime,
  last_accessed: DateTime,
}

Compound index: { app_id: 1, file_path: 1, blob_hash: 1 } (unique)
TTL index: { last_accessed: 1 }, expireAfterSeconds: 86400 (24h)
```

**Note:** `agreement_id` references the EPR agreement that authorizes this doorway to cache and serve this content. For Matthew's self-operated alpha node, this is a self-negotiated, self-accepted agreement. The field exists from day one so the governance path is wired, not bolted on later.

### Why MongoDB (not in-memory ContentCache)

- ContentCache tops at 10,000 entries. Evolution of Trust alone has 30-40 files. A few apps would blow the budget.
- Shared across doorway replicas (horizontal scaling).
- Survives pod restarts (no cold-start thundering herd).
- 16MB document limit is fine — individual app files are typically <1MB.
- Already available in production (ProjectionStore uses it).

### Request Coalescing

When 30 browsers simultaneously request a cold app, DashMap-based in-flight tracker prevents 30 parallel proxies to storage:

```rust
in_flight: DashMap<String, broadcast::Sender<(Vec<u8>, String)>>
// Key: "apps:{app_id}:{file_path}"
// First request proxies to storage, broadcasts result
// Concurrent requests wait for broadcast, then cache
```

Same pattern as ExtractionCache's `begin_extraction`.

### Invalidation

- Content re-seeded with new blob_hash
- Doorway's projection subscriber already watches content updates
- On html5-app content update: delete all docs where app_id matches + old hash
- Next request triggers cache warm-up from storage

### Files Touched

| File | Change |
|------|--------|
| `doorway/doorway-service/src/routes/apps.rs` | Cache-first handler, coalescing, MongoDB read/write |
| `doorway/doorway-service/src/cache/mod.rs` | AppFileCache struct + MongoDB collection init |
| `doorway/doorway-service/src/server/http.rs` | Wire AppFileCache into AppState |
| `doorway/doorway-service/src/projection/subscriber.rs` | Invalidation hook on html5-app content updates |

## Sprint 2: Service Worker + Capability Negotiation

### Part A: Capability Advertisement

Extends existing `NodeCapabilities` (identity.rs) with delivery information:

```rust
pub struct NodeCapabilities {
    // ... existing fields (storage, always_on, max_storage_bytes, etc.) ...

    pub delivery: DeliveryCapabilities,
}

pub struct DeliveryCapabilities {
    /// Can serve individual extracted files from cache
    pub serves_extracted: bool,

    /// Can serve raw compressed blobs (client must extract)
    pub serves_compressed: bool,

    /// Content hashes this peer can serve file-by-file right now.
    /// Type-agnostic — could be HTML5 apps, course packages, doc bundles.
    /// How content became ready (extraction, projection, native) is irrelevant.
    pub ready_content: Vec<String>,  // CIDs/hashes

    /// Cache infrastructure tier
    pub cache_tier: CacheTier,
}

pub enum CacheTier {
    /// Doorway projection cache (MongoDB, survives restarts, shared across replicas)
    Projection,
    /// Storage extraction cache (disk, device-local, budget-constrained)
    Extraction,
    /// No cache — can only serve raw blob from blob store
    BlobOnly,
}
```

**Design rationale for `ready_content`:** The delivery layer says "what can I serve?" EPR metadata says "what IS this content?" Governance says "SHOULD this be served?" Three layers, three concerns. Delivery capabilities are type-agnostic — they speak in content hashes, not application vocabulary. If something goes wrong with executable content, elohim has the full EPR trace for accountability and indemnification.

**Flow:**
- elohim-storage reports `DeliveryCapabilities` based on ExtractionCache state
- Doorway reports `CacheTier::Projection` once MongoDB cache is warm
- Both broadcast via existing `CapacityAnnouncement` gossipsub
- `ready_content` updates when extraction/projection cache adds/evicts content
- Existing identify protocol carries static capabilities; gossipsub carries dynamic `ready_content` changes

### Part B: Service Worker — Universal Client Extraction

Single SW codebase, registered in both Angular browser app and Tauri WebView.

**Intercept pattern for `/apps/` requests:**

```
fetch event for /apps/{app_id}/{file_path}:

1. Check SW CacheStorage for {app_id}:{blob_hash}:{file_path}
   → HIT: return cached Response (zero network)

2. MISS: Check if we have the ZIP blob cached
   → Have ZIP: extract file from cached ZIP, cache individual file, return

3. No ZIP cached: negotiate with peer
   → Query content metadata for blob_hash + knownLocations
   → Pick best peer (capability-aware, prefer serves_extracted)

   a. Peer serves_extracted?
      → Fetch individual file, cache in SW, return

   b. Peer serves_compressed only?
      → Fetch entire ZIP once
      → Extract ALL files into SW CacheStorage
      → Return requested file

4. Invalidation:
   → Content update signal carries new blob_hash
   → SW compares against cached blob_hash
   → Mismatch: evict all files for that app_id
   → Next request triggers fresh fetch
```

**Capability probe (lightweight, before bulk asset fetches):**

```
HEAD /apps/{app_id}/_capability
→ Response headers:
   X-Delivery-Mode: extracted | compressed
   X-Blob-Hash: sha256-abc123...
   X-Cache-Tier: projection | extraction | blob-only
```

Single HEAD request, then SW chooses strategy for the 30+ subsequent asset fetches.

### Files Touched

| File | Change |
|------|--------|
| `elohim/elohim-storage/src/identity.rs` | `DeliveryCapabilities` struct, `CacheTier` enum |
| `elohim/elohim-cache-core/src/extraction/cache.rs` | `ready_content_hashes()` method |
| `steward/node/src/pod/capacity.rs` | Serialize `DeliveryCapabilities` in gossipsub |
| `doorway/doorway-service/src/routes/apps.rs` | `HEAD /_capability` endpoint |
| `elohim/elohim-storage/src/http.rs` | Same capability HEAD endpoint |
| `app/elohim-app/src/sw.ts` (new) | Service Worker with `/apps/` intercept |
| `app/elohim-app/src/main.ts` | SW registration |
| `app/elohim-library/.../connection/` | Capability-aware peer selection |

## Sprint 3: P2P Mesh Delivery

### EPR knownLocations Extension

The EPR Document tier already carries `capabilities: [String]` per known location. Add delivery-specific vocabulary:

- `"serves_extracted"` — can serve individual files
- `"serves_compressed"` — can serve raw archive blob
- `"warm:{blob_hash}"` — has this specific content extracted and ready

No schema change to EPR spec — new vocabulary in existing capabilities array.

### Multi-Peer Resolution (client-side)

```
Content request for blob in html5-app:

1. EPR resolve → get knownLocations for this content

2. Score and sort peers:

   Preference order (best → safest fallback):
   a. LAN peer (mDNS) with warm extraction    → fastest, zero WAN
   b. LAN peer with compressed blob            → fast, client extracts
   c. Doorway with projection cache            → reliable, always-on
   d. Remote peer (relay) with warm extraction → slower, distributed
   e. Remote peer with compressed blob         → slower, client extracts
   f. Any peer with blob store access          → last resort

   Scoring factors:
   - Network proximity (LAN > WAN > relay)
   - Delivery capability (extracted > compressed)
   - Recency (lastSeen)
   - Tier (network_node > home_node > laptop)
   - Warm content match (has THIS blob hash ready)

3. Try best peer:
   → HEAD /_capability (HTTP) or QueryDelivery (libp2p)
   → Confirms capability still true (cache may have evicted)
   → Confirmed: fetch via chosen delivery mode
   → Stale: fall to next peer in ranked list

4. Fallback chain automatic:
   → Each peer failure promotes next candidate
   → SW extraction is always the safety net
```

### Tauri <> Tauri (LAN Mesh)

Two Tauri nodes on the same LAN (e.g., Matthew + Jessica in the same household):
- mDNS discovery already works via steward/node
- Both run elohim-storage with ExtractionCache
- Both advertise `ready_content` via gossipsub
- Jessica's SW sees Matthew's node has content warm on LAN
- Direct HTTP to Matthew's storage — never leaves the house

### libp2p Protocol Extension

```rust
// Extension to existing EprRequest/EprResponse enums
pub enum EprRequest {
    // ... existing variants ...
    QueryDelivery { blob_hash: String },
}

pub enum EprResponse {
    // ... existing variants ...
    DeliveryInfo {
        serves_extracted: bool,
        serves_compressed: bool,
        cache_tier: CacheTier,
        warm: bool,  // this specific blob is ready
    },
}
```

Same protocol framing (`/elohim/epr/1.0.0`), new message variant. Backward compatible — old peers return `Error("unknown variant")`, client falls back.

### Files Touched

| File | Change |
|------|--------|
| `elohim/elohim-storage/src/p2p/epr_protocol.rs` | `QueryDelivery`/`DeliveryInfo` variants |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Handle QueryDelivery in EPR handler |
| `app/elohim-library/.../cache/content-resolver.ts` | Multi-peer scoring + delivery negotiation |
| `app/elohim-app/src/sw.ts` | P2P-aware fetch with peer fallback chain |
| `app/elohim-library/.../connection/connection-strategy.ts` | LAN peer discovery integration |
| `steward/node/src/network/` | mDNS capability exchange |

### Stubs for Future Sprints

- **Browser → Browser** (WebRTC data channel): SW transport adapter stub, implementation Sprint 4+
- **Cross-WAN peer discovery without doorway**: Requires relay infrastructure maturity
- **Shefa bandwidth metering**: Economic coupling exists in EPR agreement, metering later

## Acceptance Criteria

### Sprint 1
1. First load of Evolution of Trust: doorway proxies to storage, caches all files in MongoDB
2. Second load: zero requests reach storage (all MongoDB hits)
3. Storage container stays stable under any browser load pattern
4. Cache invalidates when app is re-seeded with new blob_hash
5. Works across doorway replicas (shared MongoDB)
6. `agreement_id` field present on cache entries (stub value for self-operated node)

### Sprint 2
1. Service Worker registers in both browser and Tauri WebView
2. SW serves cached app files offline (zero network after first load)
3. SW capability probe chooses correct delivery mode per peer
4. When peer is blob-only, SW fetches ZIP and extracts locally
5. `DeliveryCapabilities` broadcast via existing gossipsub
6. `ready_content` updates dynamically as cache state changes

### Sprint 3
1. Tauri node on LAN serves app files to another Tauri node via mDNS
2. Browser SW resolves multiple peers via EPR knownLocations
3. Peer scoring prefers LAN > doorway > remote
4. Fallback chain degrades gracefully (extracted → compressed → raw)
5. `QueryDelivery` libp2p message works, old peers degrade gracefully

## Key Files Reference

| File | Purpose |
|------|---------|
| `doorway/doorway-service/src/routes/apps.rs` | Current proxy → Sprint 1 cache-first handler |
| `doorway/doorway-service/src/cache/` | Existing cache infra (ContentCache, TieredBlobCache, ProjectionStore) |
| `elohim/elohim-storage/src/http.rs:2766-2985` | handle_app_request with Layer 1 ExtractionCache |
| `elohim/elohim-storage/src/identity.rs` | NodeCapabilities → Sprint 2 DeliveryCapabilities |
| `elohim/elohim-cache-core/src/extraction/` | ExtractionCache, DiskBackend, CacheBackend trait |
| `elohim/elohim-storage/src/p2p/epr_protocol.rs` | EPR request-response → Sprint 3 QueryDelivery |
| `steward/node/src/pod/capacity.rs` | CapacityAnnouncement gossipsub |
| `genesis/plans/2026-03-29-html5-app-two-layer-cache-prompt.md` | Original problem + Layer 2 prompt |
| `genesis/plans/2026-03-29-elohim-cache-core-extraction-cache-design.md` | Layer 1 extraction cache spec |
| `genesis/plans/2026-03-29-compute-reporting-enrichment-design.md` | Peer capability advertising |
