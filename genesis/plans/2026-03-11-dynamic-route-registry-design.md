# Dynamic Route Registry — Doorway as Federated P2P Gateway

**Date**: 2026-03-11
**Status**: Approved
**Scope**: doorway-service, elohim-storage, doorway-client crate

## Problem

Doorway has 13 copy-paste proxy files (~150 lines each) that hardcode `/api/v1/*` routes to a single `STORAGE_URL`. Every new elohim-storage endpoint requires a new file, a `mod.rs` entry, and a match arm in `http.rs`. This is why `POST /api/v1/mastery` returns 405 — the route was never added.

More fundamentally, the hardcoded model assumes one backend. Doorway is designed to be a federated web2 gateway for a P2P network where multiple peers register routes and doorways collaborate to project content to the world.

## Architecture

### What Doorway Is

Doorway = node + steward + web2 projection. Every doorway operator is a peer first. Doorway provides:

- **Ingress**: web2 users discover and access P2P content via HTTP/DNS
- **Projection**: caches P2P content in MongoDB, shielding peers from read storms
- **Account recovery**: web2 safety net for hosted humans graduating to full P2P agency
- **Federation**: doorways share projection caches, forming a distributed CDN layer

### Trust Model

Route registration is deliberately lightweight. The doorway operator does not validate route content — the protocol does. Content served through any route lands on protocol-validated, reach-moderated values of the network. The doorway operator trusts the network, not individual routes.

What IS EPR-governed is the stewardship contract: the doorway operator providing projection, DNS, and compute to the peers they serve.

### Peer Registration

Peers register with doorway to get web2 projection. A peer can register with multiple doorways for redundancy, account recovery, and geographic reach. Doorways federate with each other and share projection caches via their MongoDB layer.

## Startup Sequence

```
1. Doorway starts with STORAGE_URL (its own elohim-storage)
2. GET {STORAGE_URL}/manifest → steward's route surface
3. Register into RouteRegistry as steward peer
4. Compile route table
5. Start HTTP server
6. (Background) DiscoveryService connects to conductor
   → discovers peer relationships from DNAs
   → registers additional peer routes as discovered
```

The steward's elohim-storage is the first peer — registered through the same mechanism any peer would use. `STORAGE_URL` is not "the backend to proxy to" but "the steward's peer endpoint."

## HTTP Request Routing

```
Request arrives
    │
    ├─ Built-in routes (doorway's own concerns):
    │   /health, /version, /ready
    │   /hc/* (conductor WebSocket proxy)
    │   /bootstrap, /signal (P2P infrastructure)
    │   /auth/* (account, login, JWT)
    │   /admin/* (steward dashboard)
    │   /doorway/register (peer registration API)
    │   /api/v1/cache/* (projection/resolver read path)
    │
    ├─ Registry lookup:
    │   RouteRegistry.match_request(method, path)
    │       │
    │       ├─ Match → route to target:
    │       │   ├─ StorageProxy → forward to peer's elohim-storage
    │       │   ├─ ZomeCall → call conductor (future)
    │       │   └─ AgentProxy → forward to external agent
    │       │
    │       │   Per-route policy:
    │       │   - auth_required → validate JWT (hosted human write path)
    │       │   - cache_ttl > 0 → check projection first (visitor read path)
    │       │   - rate_limit → enforce
    │       │
    │       └─ No match → 404
    │
    └─ Static assets (Angular app, WASM)
```

Built-in routes are doorway infrastructure. Everything else — content, mastery, governance, blobs, collectives — comes from the registry.

The hosted-human vs visitor split happens through per-route policy:
- `auth_required: true` + valid JWT → hosted human → proxy to peer's storage
- `auth_required: false` or no JWT → visitor → projection cache first, storage fallback
- `cache_ttl > 0` → doorway caches response (protecting peers)

## Peer Registration Flow

```
Peer's elohim-storage → POST /doorway/register
{
    agent_pubkey: "uhCAk...",
    endpoint: "https://peer-storage.local:8090",
    capabilities: ["content", "blobs"],
    signature: "...",
    routes: { ... },       // optional explicit routes
    ttl_secs: 86400
}

Doorway validates:
  ✓ Signature fresh (replay protection)
  ✓ Agent key valid
  ✗ Does NOT validate route content (protocol handles that)

RouteRegistry compiles peer routes into route table.
Peer must heartbeat/re-register before TTL expires.
```

Route conflicts (multiple peers serve same path): most specific path wins. Equal specificity: steward peer is default.

## Multi-Doorway Federation

Peers register with multiple doorways for:
- Multiple points of account recovery
- CDN-like geographic distribution
- Redundancy

Doorways federate with each other:
- Share projection caches via MongoDB layer
- Cache miss on doorway-A can be served by doorway-B's projection before hitting P2P
- Federation infrastructure already exists (`routes/federation.rs`)

## Elohim-Storage: Route Manifest

New endpoint: `GET /manifest`

Returns the route surface this storage instance serves:

```json
{
    "version": 1,
    "routes": [
        { "method": "GET", "path": "/api/v1/mastery/{contentId}", "cache_ttl_secs": 300 },
        { "method": "POST", "path": "/api/v1/mastery", "auth_required": true },
        { "method": "POST", "path": "/api/v1/mastery/engagement", "auth_required": true },
        { "method": "GET", "path": "/api/v1/governance/*", "cache_ttl_secs": 600 },
        { "method": "GET", "path": "/db/content/{id}", "cache_ttl_secs": 3600 },
        ...
    ],
    "blob_proxy": { "enabled": true, "base_path": "/blob" },
    "stream_proxy": { "enabled": true, "base_path": "/stream" }
}
```

This uses the existing `DoorwayRoutes` type from the `doorway-client` crate — the same contract for any peer.

## What Changes

| Component | Action |
|-----------|--------|
| `AppState` | Add `route_registry: Arc<RouteRegistry>` |
| `main.rs` | Instantiate RouteRegistry, self-register steward's storage on boot |
| `http.rs` | Replace 13 hardcoded `/api/v1/*` match arms with single registry lookup |
| elohim-storage | Add `GET /manifest` endpoint returning `DoorwayRoutes` |
| `RouteRegistry` | Add `RouteSource::StewardPeer`, method matching in lookup |
| 13 proxy files | Delete: governance, attestations, contributors, steward, presence, economic_events, exchange, custodians, compute, flow_planning, stewarded_resources, stewardship, account |
| `collectives.rs` | Keep temporarily — cross-domain path rewriting moves to registry path mapping later |
| `DiscoveryService` | Keep as-is — wires into registry when DNA introspection matures |

## What Stays

| Component | Reason |
|-----------|--------|
| Built-in routes in `http.rs` | Doorway infrastructure (health, auth, admin, conductor proxy) |
| `/api/v1/cache/*` | Resolver/projection read path — doorway-native concern |
| Projection/Resolver | Read-side optimization, registry routes opt in via `cache_ttl` |
| Federation routes | Doorway-to-doorway infrastructure |
| `doorway-client` crate | Already defines `DoorwayRoutes`, `AgentRegistration`, `AgentCapability` — used as-is |
