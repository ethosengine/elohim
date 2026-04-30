# Doorway — Federated P2P Gateway

Doorway is the "porch" of the P2P network — the web2 interface that makes decentralized services accessible to the traditional internet. Every doorway operator is a peer first. Doorway provides ingress, projection caching, account recovery, and federation.

## Submodules

| Module | Purpose |
|--------|---------|
| `doorway-service/` | Rust gateway: bootstrap, signal, conductor proxy, route registry, projection cache |
| `doorway-app/` | Angular operator dashboard: node health, federation, graduation pipeline, user management |

## CRITICAL: No Per-Domain Proxy Files

**NEVER create per-domain proxy files in doorway-service.** This is the most important rule for this crate.

Doorway uses a **dynamic RouteRegistry**. When elohim-storage boots, it self-registers via `GET /manifest`, and all its routes become available through the registry. Adding a new endpoint to elohim-storage automatically makes it routable through doorway — no doorway code changes needed.

| Scenario | What To Do |
|----------|-----------|
| New endpoint in elohim-storage | Add it to `build_manifest()` in storage's `http.rs`. Done. |
| New peer registers routes | They POST to `/doorway/register`. Registry compiles their routes. |
| Need custom gateway logic | Add a match arm in `http.rs` ABOVE the registry fallback. Rare. |
| "I need a governance proxy" | **NO.** The registry handles this. `governance.rs` was deleted for a reason. |
| Route returns 404 | Check that storage's `build_manifest()` declares the route. |

We deleted 13 identical ~150-line proxy files (governance, attestations, contributors, steward, presence, economic_events, exchange, custodians, compute, flow_planning, stewarded_resources, stewardship, account). They must never come back.

A dedicated match arm in `http.rs` is only needed when the route requires **doorway-specific logic** that can't be expressed as a simple storage proxy: custom auth gating, path rewriting across domains, non-storage targets (agent sidecar), or WebSocket upgrades.

## CRITICAL: No Blob Fan-Out — Doorway is Single-Target Dispatch

**Doorway forwards each request to a SINGLE storage target.** It does NOT iterate `STORAGE_URLS` looking for which peer holds a particular blob. If a request lands on a peer that doesn't have the bytes, that is a substrate replication problem to fix in elohim-storage's P2P layer — never a doorway dispatcher fix.

| Scenario | What To Do |
|----------|-----------|
| `GET /blob/<hash>` returns 404 because that peer doesn't have it | Fix it in elohim-storage P2P layer (commons-reach blobs must replicate to all eligible peers). NOT in doorway. |
| "We need doorway to try peer A, then peer B, then peer C" | **NO.** That's reintroducing P2P-aware logic into the web2 projection. The bytes must be on the routed peer; if they aren't, that's a substrate bug. |
| Singular `STORAGE_URL` points at the wrong peer for the content | Sharper symptom of the same substrate gap. Replication should make the choice irrelevant. |

Two reasons this rule exists:

1. **Three-layer truth model** (DHT / libp2p / doorway-projection). Doorway is the web2 projection of the network, not a P2P participant. Peer-aware blob routing in doorway would re-introduce the P2P logic that elohim-storage already owns, creating two competing sources of "where does this blob live" truth.

2. **The substrate is responsible for byte mobility.** The self-healing P2P dataplane campaign (`genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`) is what makes commons-reach blobs reachable from any peer. Inventory exchange between peers is not the same thing as byte replication — the latter requires Plan 1 (distribute at ingest) + Plan 2 (verifier) + Plan 3 (reconstruction).

What doorway DOES do for blobs:
- Forward `GET`/`HEAD` to its configured storage target (singular `STORAGE_URL` or a registry-routed peer).
- Cache the response in the tiered blob cache so subsequent requests are served locally.
- Never know or care which physical peer holds which blob.

### Known TODO — blob-tier cache write-on-fetch

The tiered blob cache (`TieredBlobCache`) is allocated and its cleanup loop runs, but its blob tier is never written on the storage-proxy path. `routes::forward_to_storage` returns the upstream response directly to the client without populating the cache. As a result every `/blob/<hash>` request round-trips to elohim-storage. Fixing this is independent of any P2P substrate work and is what makes doorway actually behave like a projection cache. Symptom in `/admin/cache/stats`: `appFileCache.cachedFiles: 0` and `Tiered cache cleanup completed expired_blobs=0` indefinitely.

## Architecture

### Three Consolidated Services

```
doorway.elohim.host
  /bootstrap     — Agent discovery (MessagePack + Ed25519)
  /signal        — WebRTC signaling relay (SBD protocol)
  /admin         — Steward operator dashboard API
  /api/v1/cache  — Projection cache (type-agnostic content resolution)
  /auth          — JWT auth for hosted humans
  /* (registry)  — Dynamic routes from peer manifest registration
```

### Request Routing

```
Request → http.rs match block
  ├─ Built-in routes (health, auth, admin, conductor, bootstrap, signal, cache)
  ├─ Special routes with custom logic (collectives, elohim-agent, identity)
  ├─ Registry lookup: state.route_registry.match_request(method, path)
  │   └─ StorageProxy → routes::forward_to_storage(req, endpoint, path)
  └─ 404
```

### Steward Boot Sequence

```
1. Doorway starts with STORAGE_URL (its own elohim-storage)
2. GET {STORAGE_URL}/manifest → steward's route surface (DoorwayRoutes)
3. RouteRegistry compiles manifest routes as StorageProxy targets
4. HTTP server starts — all manifest routes are live
5. (Background) DiscoveryService connects to conductor for DNA route discovery
```

### Route Sources and Targets

```
Sources:                              Targets:
  StewardPeer  (boot self-register)     StorageProxy  (forward to peer storage)
  Dna          (DNA introspection)      ZomeCall      (conductor call, future)
  ExternalAgent (POST /doorway/register) AgentProxy   (external HTTP endpoint)
  Builtin      (health, auth, admin)    BlobProxy     (blob serving)
                                        StreamProxy   (media streaming)
```

### Trust Model

Route registration is lightweight. Doorway does NOT validate route content — the protocol does. Content served through any route lands on protocol-validated, reach-moderated values of the network. The doorway operator trusts the network, not individual routes. What IS EPR-governed is the stewardship contract: providing projection, DNS, and compute.

## Two Scaling Axes

Doorway has two fundamentally different scaling concerns:

**Axis 1 — Projection (visitors reading content):** Classic web2. Served from MongoDB cache. Scales horizontally with replicas and doorway-federated CDN. Unbounded growth. Does not self-resolve.

**Axis 2 — Identity Hosting (humans transitioning to P2P):** Requires conductor cells. Scales via conductor pool. Bounded by hosted user count. Self-resolves via graduation flywheel — users leave for their own devices.

## Federation

Doorways are **projections of the DHT**, not authorities. Unlike traditional fediverse instances:
- Users can authenticate through ANY doorway (agent key = identity)
- All doorways project the same DHT (content appears everywhere via gossip)
- Content is addressed by hash (CDN-like caching is trivial)
- DNA validation rules run on every node (invalid content rejected by DHT)

Peers register with multiple doorways for redundancy, account recovery, and geographic distribution. Doorways share projection caches via MongoDB.

## Reach Enforcement

Doorway gates HTTP requests based on content reach levels:

| Reach | Anonymous | Authenticated |
|-------|-----------|---------------|
| commons | allowed | allowed |
| regional-private | denied | allowed |
| local | denied | requires relationship |
| private | denied | requires beneficiary match |

## Design Vocabulary

The protocol's storage/distribution language — `quilt`, `pantry`, `stock`, `draw`, `shard`, `RS(N,K)` — is defined in `genesis/graphos/vocabulary.md`. Wire-level identifiers (HTTP `/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names; the new vocabulary applies to design discussion, signal/event names, and any new identifier we invent. The legacy `/store/{hash}` and `/api/blob/{hash}` paths were retired in the 2026-04-30 vocabulary cleanup; the canonical path is `/blob/{hash}` (registry-routed via storage's manifest).

## Reference Documentation

Detailed design docs live in `doorway-service/`:
- `ARCHITECTURE.md` — Component-level details: bootstrap, signal, gateway, cache, resolver
- `FEDERATION.md` — Cross-doorway patterns, DID discovery, P2P bootstrap role
- `SCALING.md` — Two-axis scaling model, graduation flywheel, K8s modeling
- `REACH.md` — Reach enforcement rules, caching, DNA integration
- `RECOVERY-PROTOCOL.md` — Social recovery, shard distribution, agency restoration
- `RECOVERY-SPRINT-PLAN.md` — Recovery protocol implementation phases
- `genesis/graphos/vocabulary.md` — Storage and distribution vocabulary register
