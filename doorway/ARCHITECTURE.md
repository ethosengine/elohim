# Doorway: Consolidated Web2 Gateway for P2P Networks

## Vision

Doorway is the "porch" of the P2P network - the web2 interface that makes decentralized services accessible to the traditional internet. Just as a physical porch has an address (for discovery), a door (for entry), and a mailbox (for messages), Doorway consolidates three separate Holochain infrastructure services into one:

```
┌─────────────────────────────────────────────────────────────┐
│                        DOORWAY                               │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Bootstrap  │  │   Signal    │  │     Gateway         │  │
│  │  (Address)  │  │   (Door)    │  │  (Access & Cache)   │  │
│  │             │  │             │  │                     │  │
│  │ "Who's in   │  │ "Connect    │  │ "Get the data"      │  │
│  │  the space?"│  │  to peers"  │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                                                             │
│                    ↓ All served by ↓                        │
│              doorway.elohim.host                            │
└─────────────────────────────────────────────────────────────┘
```

## Current State (Separate Services)

Today, Holochain requires three separate services:

| Service | Purpose | Current Solution |
|---------|---------|------------------|
| Bootstrap | Agent discovery | CloudFlare Worker (`bootstrap.holo.host`) |
| Signal | WebRTC signaling | SBD server (`sbd.holo.host`) |
| Gateway | Conductor access | Doorway (our solution) |

This means running three separate deployments with separate configurations, scaling concerns, and failure modes.

## Consolidated Routes

One domain. All services. Simple for operators.

```
elohim.host
├── /bootstrap
│   ├── POST /put           → Register agent in space
│   ├── POST /random        → Get random agents in space
│   └── POST /now           → Get server timestamp
│
├── /signal                 → WebRTC signaling (SBD protocol)
│   └── WS  /{pubkey}       → WebSocket relay with Ed25519 auth
│
├── /admin                  → Conductor admin (worker pool)
├── /app/{port}             → App interface (direct proxy)
│
├── /api/v1/{dna}/{zome}/{fn}  → REST cache layer (generic)
│   └── GET with query args     → Cached zome call (if cacheable)
│
└── /status                → Service status & cache metrics
```

**Operator config is just one URL: `elohim.host`**

---

## Component Analysis

### 1. Bootstrap Service

**Source**: `holochain/bootstrap` (CloudFlare Worker)

**Purpose**: Agent discovery - allows agents to find each other in a DHT space.

**Protocol**:
- **MessagePack** serialization
- **Ed25519** signatures for agent authentication
- **KV storage** for agent info (space + agent as key)

**Operations**:

```
POST /put
  Body: MessagePack { agent, space, urls, signed_at_ms, expires_after_ms, signature }
  Response: null (success) or error

POST /random
  Body: MessagePack { space: [u8; 36], limit: number }
  Response: MessagePack array of signed agent infos

POST /now
  Body: empty or MessagePack null
  Response: MessagePack timestamp (milliseconds)
```

**Data Model**:
- **Space**: 36-byte identifier (DNA hash + context)
- **Agent**: 39-byte Ed25519 public key
- **Key**: `{net}:{space}:{agent}` (e.g., `tx5:abc123...:def456...`)
- **Value**: Full signed agent info blob
- **TTL**: Configurable expiry (default varies)

**Storage Requirements**:
- Fast key-value store (in-memory for speed, persistent for durability)
- List operation per space (for random selection)
- TTL/expiry support

**Implementation in Rust**:
```rust
// Bootstrap routes
async fn bootstrap_put(body: Bytes) -> Result<Response, Error>
async fn bootstrap_random(body: Bytes) -> Result<Response, Error>
async fn bootstrap_now() -> Result<Response, Error>

// Storage (in-memory with optional persistence)
struct BootstrapStore {
    agents: DashMap<SpaceAgentKey, SignedAgentInfo>,
    space_index: DashMap<Space, Vec<Agent>>,
}
```

---

### 2. Signal Server (SBD)

**Source**: `holochain/sbd`

**Purpose**: WebRTC signaling relay - allows peers to exchange connection offers/answers.

**Protocol** (SBD Spec):
- **WebSocket** transport
- **Ed25519** authentication (challenge/response)
- **32-byte header** messages (pubkey = forward, zeros = command)

**Commands**:
| Command | Direction | Data | Purpose |
|---------|-----------|------|---------|
| `areq` | S→C | 32-byte nonce | Auth challenge |
| `ares` | C→S | 64-byte signature | Auth response |
| `srdy` | S→C | none | Ready to relay |
| `lbrt` | S→C | i32be | Rate limit (byte-nanos) |
| `lidl` | S→C | i32be | Idle timeout (ms) |
| `keep` | C→S | none | Keepalive |

**Forward Messages**:
- Header: 32-byte pubkey of recipient
- Body: Arbitrary data to relay
- On receive: Header replaced with sender's pubkey

**Connection Flow**:
```
Client                    Server
  |                          |
  |---- WS /{pubkey} ------->|  Connect with claimed pubkey
  |<-------- lbrt -----------|  Rate limit
  |<-------- lidl -----------|  Idle timeout
  |<-------- areq -----------|  Auth challenge (nonce)
  |--------- ares ---------->|  Auth response (signature)
  |<-------- srdy -----------|  Ready
  |                          |
  |<======= forward ========>|  Relay messages
```

**Configuration**:
- `limit_clients`: Max connections (default 32768)
- `limit_ip_kbps`: Rate limit per IP (default 1000 kbps)
- `limit_idle_millis`: Idle timeout (default 10000 ms)
- `trusted_ip_header`: For reverse proxy (e.g., X-Forwarded-For)

**Implementation in Rust**:
```rust
// Signal routes
async fn signal_authenticate(body: Bytes) -> Result<Response, Error>
async fn signal_websocket(
    Path(pubkey): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse

// Connection state
struct SignalConnection {
    pubkey: [u8; 32],
    ws: WebSocket,
    authenticated: bool,
}

// Relay state
struct SignalRelay {
    connections: DashMap<[u8; 32], SignalConnection>,
    ip_rate: IpRateLimiter,
}
```

---

### 3. Gateway (Current Doorway)

**Purpose**: HTTP/WebSocket gateway to Holochain conductor.

**Design Principle: Type-Agnostic**

Doorway is a thin gateway that does NOT define content types or routes.
The Holochain DNA defines what types exist. Doorway simply:
1. Accepts any `{type}/{id}` path
2. Passes the type string through to projection/conductor
3. Returns whatever the DNA returns

This keeps doorway generic and reusable across different DNAs.

**Current Routes**:
- `/admin` - Admin interface (worker pool protection)
- `/app/{port}` - App interface (direct proxy)
- `/api/v1/cache/{type}/{id}` - Generic cache layer (type-agnostic)
- `/api/v1/cache/{type}` - Collection queries

---

## Implementation Plan

### Phase 1: Bootstrap Integration

**Complexity**: Low-Medium
**Dependencies**: None

1. Add MessagePack parsing (rmp-serde crate)
2. Add Ed25519 signature verification (ed25519-dalek crate)
3. Create in-memory store with space indexing
4. Implement PUT/RANDOM/NOW endpoints
5. Add TTL cleanup task

**Routes**:
```
POST /bootstrap/put
POST /bootstrap/random
POST /bootstrap/now
```

### Phase 2: Signal Integration

**Complexity**: Medium-High
**Dependencies**: Phase 1 (shared crypto)

1. Add WebSocket handling (already have tokio-tungstenite)
2. Implement SBD protocol state machine
3. Add IP rate limiting
4. Implement message relay with pubkey routing
5. Add connection state management

**Routes**:
```
PUT  /signal/authenticate
WS   /signal/{pubkey}
```

### Phase 3: REST Cache Layer ✅

**Status**: Implemented
**Complexity**: Medium

Features:
1. ✅ In-memory LRU cache with DashMap storage
2. ✅ TTL-based expiry (configurable per function)
3. ✅ ETag generation (SHA256-based)
4. ✅ HTTP cache headers (Cache-Control, ETag, Vary, X-Cache)
5. ✅ Pattern-based cache invalidation (per DNA, per function)
6. ✅ Cache stats in /status endpoint
7. ✅ Generic route pattern for any content type

**Route**:
```
GET /api/v1/cache/{type}/{id}     → Single document
GET /api/v1/cache/{type}          → Collection query
```

**Cache Rule Discovery Protocol**:

DNAs can optionally implement `__doorway_cache_rules` to declare caching:
```rust
fn __doorway_cache_rules(_: ()) -> ExternResult<Vec<CacheRule>> {
    Ok(vec![CacheRule {
        fn_name: "get_content".into(),
        cacheable: true,
        ttl_secs: 3600,
        public: false,                      // Requires auth by default
        reach_field: Some("reach".into()),  // Check this field...
        reach_value: Some("commons".into()), // ...for public access
        invalidated_by: vec!["create_content".into()],
    }])
}
```

**Defaults** (if no rules implemented):
- `get_*` and `list_*` functions → cacheable, 5 min TTL, auth required
- Other functions → not cacheable via REST API

### Phase 4: Tiered Content Resolution ✅

**Status**: Implemented
**Complexity**: Medium

Integration with `holochain-cache-core` for unified content resolution.

**DoorwayResolver** provides tiered source routing:
```
Request → Projection Cache (MongoDB, fast)
             ↓ miss
          Conductor (DHT, authoritative)
             ↓ miss
          External (CDN/URLs, last resort)
```

Features:
1. ✅ Automatic fallback chain (Projection → Conductor)
2. ✅ Source availability tracking
3. ✅ Resolution statistics and metrics
4. ✅ **Type-agnostic resolution** - doorway passes through any type string
5. ✅ Integration with WorkerPool for conductor calls

**Type-Agnostic Design**:

Doorway does NOT define content types - the Holochain DNA does. The resolver
accepts any type string and passes it through to projection/conductor:

```rust
// AppState includes resolver
pub struct AppState {
    // ...existing fields...
    pub resolver: Arc<DoorwayResolver>,
    pub write_buffer: Option<Arc<DoorwayWriteBuffer>>,
}

// Generic resolution - type comes from request path
// Doorway doesn't know or care what types exist
let result = state.resolver.resolve(doc_type, id).await;

// Works for any type the DNA defines:
resolver.resolve("Content", "manifesto").await;
resolver.resolve("LearningPath", "governance-intro").await;
resolver.resolve("CustomDnaType", "custom-id").await;
```

### Phase 5: Batched Conductor Writes ✅

**Status**: Implemented
**Complexity**: Medium

**DoorwayWriteBuffer** protects conductor from heavy write loads:

```
Write Request → Priority Queue (High/Normal/Bulk)
                    ↓ batched
                Conductor (batched zome calls)
```

Features:
1. ✅ Priority queues: High (identity/auth) → Normal → Bulk (seeding)
2. ✅ Deduplication (last write wins within batch window)
3. ✅ Retry logic with exponential backoff
4. ✅ Backpressure signaling (0-100%)
5. ✅ Auto-flush background task
6. ✅ Presets: `for_seeding()`, `for_recovery()`, `for_interactive()`

**Use Cases**:
- Bulk content imports (seeding)
- Recovery sync from family network
- Incremental projection updates

### Phase 6: Type-Agnostic Projection Signals ✅

**Status**: Implemented
**Complexity**: Medium

DNA signals use a **generic format** - doorway never parses signal content.

**Signal Format**:
```json
{
  "doc_type": "Content",
  "action": "commit",
  "id": "manifesto",
  "data": { ... },              // Opaque - doorway never parses this
  "action_hash": "uhCkk...",
  "entry_hash": "uhCEk...",
  "author": "uhCAk...",

  // Explicit metadata (DNA controls all of this)
  "search_tokens": ["governance", "manifesto", "protocol"],
  "invalidates": ["LearningPath:governance-intro"],
  "ttl_secs": 3600
}
```

**Key Principles**:

1. **Doorway is type-agnostic**: Accepts any `doc_type` string, stores `data` as opaque JSON
2. **DNA computes metadata**: Search tokens, cache invalidation patterns, TTL
3. **No backwards compatibility**: Clean break from type-specific enum

**Signal Processing**:
```rust
// Generic handler - works for any type
pub async fn process_signal(&self, signal: ProjectionSignal) {
    match signal.action.as_str() {
        "commit" | "update" => {
            // Store opaque data with DNA-provided metadata
            let doc = ProjectedDocument::new(
                &signal.doc_type,  // Any type string
                &signal.id,
                &signal.action_hash,
                &signal.author,
                signal.data,       // Opaque - doorway doesn't parse
            )
            .with_entry_hash(signal.entry_hash.as_deref())
            .with_search_tokens(signal.search_tokens);

            self.store.set(doc).await?;
        }
        "delete" => {
            self.store.invalidate(&format!("{}:{}", signal.doc_type, signal.id)).await?;
        }
    }

    // Apply cache invalidations from DNA
    for pattern in signal.invalidates {
        self.store.invalidate(&pattern).await;
    }
}
```

**DNA Responsibility**:

DNA computes all domain logic and tells doorway what to do:
```rust
fn post_commit_signal(content: Content) {
    emit_signal(ProjectionSignal {
        doc_type: "Content".to_string(),
        id: content.id,
        action: "commit".to_string(),
        data: serialize(content),

        // DNA computes these - doorway just applies them
        search_tokens: tokenize(&content.title, &content.description, &content.tags),
        invalidates: vec![],  // Content doesn't invalidate anything
        ttl_secs: None,
    });
}
```

**Benefits**:
- Doorway is fully reusable across different DNAs
- New content types require no doorway changes
- Clear separation: DNA = domain logic, Doorway = infrastructure

---

## Architecture Diagram

```
                    Internet
                        │
                        ▼
              ┌─────────────────┐
              │   Load Balancer │
              │   (nginx/k8s)   │
              └────────┬────────┘
                       │
                       ▼
         ┌─────────────────────────────────────────────────┐
         │                    DOORWAY                       │
         │                                                  │
         │  ┌───────────────────┐  ┌─────────────────────┐ │
         │  │   Hyper Router    │  │  holochain-cache-   │ │
         │  │                   │  │       core          │ │
         │  │ /bootstrap/*  ───────►  ┌───────────────┐  │ │
         │  │ /signal/*     ───────►  │ DoorwayResolver│  │ │
         │  │ /admin        ───────►  │ (Projection →  │  │ │
         │  │ /app/:port    ───────►  │  Conductor)    │  │ │
         │  │ /api/v1/cache ───────►  └───────────────┘  │ │
         │  │                   │  │  ┌───────────────┐  │ │
         │  └───────────────────┘  │  │WriteBuffer    │  │ │
         │                         │  │ (Priority     │  │ │
         │  ┌───────────────────┐  │  │  Batching)    │  │ │
         │  │  Bootstrap Store  │  │  └───────────────┘  │ │
         │  │  - agents map     │  └─────────────────────┘ │
         │  │  - space index    │                          │
         │  └───────────────────┘                          │
         │                                                  │
         │  ┌───────────────────┐  ┌─────────────────────┐ │
         │  │  Signal Relay     │  │  ProjectionStore    │ │
         │  │  - connections    │  │  (MongoDB cache)    │ │
         │  │  - rate limiter   │  └─────────────────────┘ │
         │  └───────────────────┘                          │
         │                                                  │
         │  ┌───────────────────┐  ┌─────────────────────┐ │
         │  │  WorkerPool       │  │  TieredBlobCache    │ │
         │  │  (4 connections)  │  │  (media streaming)  │ │
         │  └─────────┬─────────┘  └─────────────────────┘ │
         │            │                                     │
         └────────────┼─────────────────────────────────────┘
                      │
                      ▼
         ┌─────────────────┐
         │   Holochain     │
         │   Conductor     │
         │                 │
         │  - Admin WS     │
         │  - App WS       │
         └─────────────────┘
```

**Data Flow (Content Resolution)**:
```
GET /api/v1/cache/{type}/{id}
        │
        ▼
  DoorwayResolver.resolve(type, id)    ← Type-agnostic!
        │
        ├── 1. Check ProjectionStore (MongoDB)
        │       └── HIT → Return immediately
        │
        └── 2. Fallback to WorkerPool → Conductor
                └── HIT → Cache in Projection, Return

Examples:
  /api/v1/cache/Content/manifesto      → resolve("Content", "manifesto")
  /api/v1/cache/LearningPath/intro     → resolve("LearningPath", "intro")
  /api/v1/cache/CustomType/custom-id   → resolve("CustomType", "custom-id")
```

**Data Flow (Batched Writes)**:
```
Bulk Import Request
        │
        ▼
  DoorwayWriteBuffer.queue_content_create(...)
        │
        ├── Priority: High (identity) → Flush immediately
        ├── Priority: Normal (content) → Batch moderately
        └── Priority: Bulk (seeding) → Batch aggressively
                │
                ▼
          Batched zome calls to Conductor
```

---

## Benefits of Consolidation

1. **Single Deployment**: One container, one config, one scaling concern
2. **Shared Resources**: Connection pools, rate limiters, crypto
3. **Unified Monitoring**: One set of metrics, logs, traces
4. **Simplified Networking**: One hostname, one TLS cert
5. **Reduced Latency**: No inter-service hops
6. **Operational Simplicity**: One thing to deploy and manage

---

## Configuration (Proposed)

```yaml
doorway:
  listen: "0.0.0.0:8080"
  conductor_url: "ws://localhost:4444"

  # Worker pool for admin interface
  worker_pool:
    size: 4
    queue_size: 100

  # Bootstrap service
  bootstrap:
    enabled: true
    ttl_seconds: 3600
    max_agents_per_space: 10000

  # Signal relay (SBD)
  signal:
    enabled: true
    max_clients: 32768
    rate_limit_kbps: 1000
    idle_timeout_ms: 10000

  # REST cache layer
  cache:
    enabled: true
    max_entries: 10000
    ttl_seconds: 300
```

---

## Key Dependencies

```toml
[dependencies]
# Shared caching primitives (O(log n) operations, WASM-compatible)
holochain-cache-core = { path = "../holochain-cache-core" }

# MessagePack (for bootstrap/Holochain protocol)
rmp-serde = "1.3"

# Ed25519 (for bootstrap + signal auth)
ed25519-dalek = "2.1"

# Concurrent data structures
dashmap = "6.1"

# MongoDB for projection cache
mongodb = "3.1"

# Async runtime
tokio = { version = "1.43", features = ["full"] }
```

**holochain-cache-core** provides:
- `ContentResolver` - Type-agnostic tiered source routing (Projection → Conductor → External)
- `WriteBuffer` - Priority-based batching (High → Normal → Bulk)
- `BlobCache` - O(log n) LRU eviction for media

Note: holochain-cache-core is type-agnostic. It doesn't know what content types
exist - it just routes requests to the appropriate source tier.

---

## Migration Path

Since `elohim.host` already points to Doorway, both bootstrap and signal are ready now:

1. ✅ Bootstrap is enabled by default in Doorway
2. ✅ Signal is enabled by default in Doorway
3. Point nodes to:
   - Bootstrap: `https://elohim.host/bootstrap`
   - Signal: `wss://elohim.host/signal`
4. Phase out standalone `holostrap.elohim.host`

No protocol changes needed - same MessagePack/SBD protocols. Same domain, everything.

---

## Two-Path Architecture: Web2 vs P2P

Elohim Protocol has two distinct access paths, each with its own caching layer:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         GOVERNANCE LAYER                             │
│                                                                      │
│                    DNA (content_store + infrastructure)              │
│                    ┌─────────────────────────────────┐              │
│                    │  Reach enforcement lives HERE   │              │
│                    │  - Author sets reach            │              │
│                    │  - Qahal can govern reach       │              │
│                    │  - Cryptographically enforced   │              │
│                    └───────────────┬─────────────────┘              │
│                                    │                                 │
│                    ┌───────────────┴───────────────┐                │
│                    ▼                               ▼                │
│              WEB2 PATH                        P2P PATH              │
│              (Doorway)                        (Agent)               │
│                                                                      │
│         DNA says "reach=X"              DNA says "reach=X"          │
│         Doorway enforces                Agent enforces              │
└─────────────────────────────────────────────────────────────────────┘
```

### Web2 Path (Doorway)

For visitors accessing via browser without a Holochain conductor:

```
Browser → Doorway → [ProjectionStore] → DHT / elohim-storage
                          │
                    MongoDB cache
                    (content + blob endpoints)
```

- **ProjectionStore**: Unified metadata cache (content docs + blob endpoints)
- **TieredBlobCache**: Byte cache only (no metadata duplication)
- **Governed**: Doorway enforces reach from ProjectionStore

### P2P Path (Agent)

For users running elohim-app with their own conductor:

```
Agent App → [holochain-cache-core] → Conductor → DHT
                    │
Agent App → [elohim-storage] → Local + P2P fetch from peers
```

- **holochain-cache-core**: Agent-side cache for conductor calls
- **elohim-storage**: Blob storage + P2P replication based on relationship
- **Governed**: DNA enforces reach before serving to other agents

### Key Principles

| Principle | Description |
|-----------|-------------|
| **DNA is Source of Truth** | Reach decisions live in the DNA, not caches |
| **Doorway is a Thin Proxy** | Governed gateway, not storage layer |
| **elohim-storage is Relationship-Based** | "I store my data AND help replicate yours based on our relationship" |
| **Caches are Enforcement Points** | They enforce DNA decisions, don't make them |

### elohim-storage: Embodied Relationship Resilience

Unlike traditional storage, elohim-storage participates in P2P solidarity:

```rust
elohim-storage:
  "I store MY data"
  "I help YOU replicate/backup/deliver YOURS"
  "...based on our relationship in the DNA"
```

This enables:
- **Recovery**: Family network can restore your data
- **Resilience**: Content survives individual node failures
- **Trust**: Replication follows consent-based relationships

### Doorway Cache Simplification

Doorway uses a unified metadata model:

```
Signal Flow:
  CacheSignal (content_store DNA)
       │
       ▼
  ProjectionStore ← Single source of truth
  { id, title, reach,       for all content metadata
    blobHash, blobEndpoints }
       │
       ▼
  TieredBlobCache (bytes only)
  { hash → bytes }
```

- **One metadata store** (ProjectionStore) instead of two
- **Blob endpoints** stored with content, not separately
- **Reach** stored once, enforced consistently
