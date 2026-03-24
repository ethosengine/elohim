# Doorway Observability & Resource Accounting

**Date:** 2026-03-24
**Status:** Approved
**Scope:** Per-peer projection health, doorway resource usage metrics, and foundation for EPR hosting contracts.

## The Problem

Doorway now projects content from multiple peers (activation sprint), but the operator has no visibility into:
- Which peer subscribers are healthy vs stuck reconnecting
- How much compute (storage, bandwidth, memory) the doorway consumes to serve the network
- Per-governance-context resource attribution for future EPR hosting contracts

## Key Architectural Principles

### Doorway accounts for its own compute, not content

Content is sharded across the P2P network. Doorway doesn't "host" content — it **projects** it. The accounting tracks what the doorway spends to serve the network: projection storage, CDN caching, bandwidth, DNS registration. Content ownership and stewardship are P2P-layer concerns.

### EPR governance context, not author

The accounting unit is the EPR governance context (qahal dimension), not the individual author. Every projected document carries its EPR triple: lamad (knowledge) + shefa (value) + qahal (governance). Attribution flows through the stewardship affinity graph, not a single author field.

### Pass-through peers

Some stewards may want DNS routing without projection caching — they handle their own traffic, doorway just points the domain. Near-zero resource consumption, near-zero accounting. The system should accommodate this as a first-class pattern alongside full projection.

### Business logic in Rust

Angular renders what doorway-service computes. No aggregation, no business logic, no metric calculation in the frontend. The `/status` endpoint returns fully-formed data. Angular displays it.

### Three consumers

1. **The doorway operator** — sees infrastructure health, resource usage, peer status
2. **Stewards** — see their hosting contract consumption (future, once EPR contracts are formalized)
3. **The operator elohim agent** — reads structured data from `/status` to reason about maintenance, capacity, and health autonomously

## Design

### 1. Peer Projection Health

A `PeerHealthRegistry` (new struct, `DashMap`-based) lives in AppState. The signal subscriber forwarding tasks (built in the activation sprint) update it inline — no new background tasks, just atomic counter increments.

```rust
/// Per-peer projection subscriber health
pub struct PeerProjectionHealth {
    /// Conductor identifier (e.g., "conductor-0")
    pub conductor_id: String,
    /// Conductor app interface URL
    pub conductor_url: String,
    /// Connection state: "connected", "authenticating", "reconnecting", "failed"
    pub state: String,
    /// Signals received since last restart
    pub signals_received: u64,
    /// Last signal timestamp (None if never connected)
    pub last_signal_at: Option<String>,
    /// Reconnect attempt count (0 when healthy)
    pub reconnect_attempts: u32,
}
```

**Data flow:** Subscriber forwarding task receives signal → increments `signals_received` on the corresponding `PeerHealthEntry` in the registry → updates `last_signal_at`. Subscriber reconnect loop updates `state` and `reconnect_attempts`.

### 2. Doorway Resource Usage

Assembled at request time when `/status` is called. No continuous background collection — metrics are either read from existing state or queried cheaply.

```rust
pub struct DoorwayResourceUsage {
    /// MongoDB projection storage in bytes
    pub projection_bytes: u64,
    /// Document count in projection store
    pub projection_documents: u64,
    /// Hot cache entries (in-memory DashMap)
    pub hot_cache_entries: usize,
    /// Requests served since startup, by doc_type
    pub requests_by_type: HashMap<String, u64>,
    /// Total requests served via /api/v1/cache/
    pub total_requests: u64,
    /// Active signal subscriber connections
    pub active_subscribers: usize,
    /// DNS registrations this doorway serves
    pub dns_registrations: usize,
}
```

**Sources:**

| Metric | Source | Cost |
|--------|--------|------|
| `projection_bytes`, `projection_documents` | MongoDB `collStats` command | Cached 30s to avoid per-request overhead |
| `hot_cache_entries` | `ProjectionStore::hot_cache_stats()` | Free (reads DashMap length) |
| `requests_by_type`, `total_requests` | `DashMap<String, AtomicU64>` in AppState | Incremented inline in `routes/api.rs` handler |
| `active_subscribers` | Read from `PeerHealthRegistry` | Free (count connected entries) |
| `dns_registrations` | Count from route registry | Free |

### 3. Status Response Integration

The existing `StatusResponse` in `routes/status.rs` gets two new fields:

```rust
pub struct StatusResponse {
    // ... existing: service, version, bootstrap, cache, conductor,
    //               orchestrator, storage, federation, import ...

    /// Peer projection health — one entry per signal subscriber
    pub peers: Vec<PeerProjectionHealth>,

    /// Doorway resource usage — what this doorway consumes to serve the network
    pub resources: DoorwayResourceUsage,
}
```

The HTML status page (`status.html` Askama template) gets corresponding sections for peer health cards and a resource usage summary.

### 4. Request Counter in Cache API Handler

In `routes/api.rs`, the `handle_api_request` function increments counters after serving a response:

```rust
// After building response:
if let Some(ref counters) = state.request_counters {
    counters.increment(route.doc_type);
}
```

`RequestCounters` wraps `DashMap<String, AtomicU64>` with a simple `increment()` and `snapshot()` API. Lives in a new `metrics` module (or inline in `routes/api.rs` if small enough).

## What Changes

| File | What |
|------|------|
| `src/projection/subscriber.rs` | Report connection state changes to PeerHealthRegistry |
| `src/server/http.rs` | Add `PeerHealthRegistry` and `RequestCounters` to AppState |
| `src/main.rs` | Initialize registry, pass to subscriber forwarding tasks |
| `src/routes/status.rs` | Add `peers` and `resources` to StatusResponse, query MongoDB collStats |
| `src/routes/api.rs` | Increment request counters per doc_type |
| `templates/status.html` | Render peer health cards and resource usage section |

## What Doesn't Change

- Health probes (`/health`, `/ready`) — stay lightweight, no new queries
- Projection engine signal processing — no overhead added to the hot path
- Angular data-loader — still reads from `/api/v1/cache/`, unaware of metrics
- EPR contract formalization — future sprint, this provides the raw data it will need

## Future (Not This Sprint)

- **EPR hosting contracts** — formalize projection hosting as REA economic events with governance context attribution
- **Per-governance-context breakdown** — aggregate projection_bytes by EPR qahal context (requires EPR Head parsing)
- **Pass-through routing mode** — DNS-only peers with near-zero resource footprint
- **Operator elohim agent** — consumes `/status` to autonomously manage capacity, restart stuck subscribers, alert on storage growth
- **Billing/settlement** — doorway resource usage feeds into shefa economic protocol for mutual credit settlement between doorway operators and stewards
