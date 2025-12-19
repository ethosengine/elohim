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

**Current Routes**:
- `/admin` - Admin interface (worker pool protection)
- `/app/{port}` - App interface (direct proxy)

**Future Routes (REST Cache Layer)**:
- `/api/v1/{dna}/{zome}/{fn}` - Cached GET requests
- Content-addressed responses are infinitely cacheable

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
7. ✅ Generic route pattern for any DNA/zome

**Route**:
```
GET /api/v1/{dna_hash}/{zome}/{fn}?{args}
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

**Next**: Connect to conductor for actual zome calls

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
         ┌─────────────────────────┐
         │        DOORWAY          │
         │                         │
         │  ┌───────────────────┐  │
         │  │   Axum Router     │  │
         │  │                   │  │
         │  │ /bootstrap/* ────────────┐
         │  │ /signal/*    ────────────┼──► In-Memory State
         │  │ /admin       ────────────┼──► Worker Pool (4 conns)
         │  │ /app/:port   ────────────┼──► Direct Proxy
         │  │ /api/v1/*    ────────────┼──► Cache Layer
         │  │                   │  │   │
         │  └───────────────────┘  │   │
         │                         │   │
         │  ┌───────────────────┐  │   │
         │  │  Bootstrap Store  │◄─────┘
         │  │  - agents map     │  │
         │  │  - space index    │  │
         │  └───────────────────┘  │
         │                         │
         │  ┌───────────────────┐  │
         │  │  Signal Relay     │◄─────┐
         │  │  - connections    │  │   │
         │  │  - rate limiter   │  │   │
         │  └───────────────────┘  │   │
         │                         │   │
         │  ┌───────────────────┐  │   │
         │  │  Response Cache   │◄─────┤
         │  │  - LRU cache      │  │   │
         │  └───────────────────┘  │   │
         │                         │   │
         └─────────┬───────────────┘   │
                   │                   │
                   ▼                   │
         ┌─────────────────┐           │
         │   Holochain     │           │
         │   Conductor     │◄──────────┘
         │                 │
         │  - Admin WS     │
         │  - App WS       │
         └─────────────────┘
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

## Dependencies to Add

```toml
[dependencies]
# MessagePack (for bootstrap)
rmp-serde = "1.1"

# Ed25519 (for bootstrap + signal auth)
ed25519-dalek = "2.1"

# Rate limiting
governor = "0.6"

# Concurrent data structures
dashmap = "5.5"
```

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
