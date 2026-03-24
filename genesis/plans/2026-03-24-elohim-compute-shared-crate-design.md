# elohim-compute: Shared Compute Reporting Crate

**Date:** 2026-03-24
**Status:** Approved
**Scope:** Shared crate for uniform service health, resource usage, request throughput, and peer health reporting across the Elohim fleet.

## The Problem

Every service reinvents its own metrics/health/stats patterns independently:
- Doorway has `CacheStats`, `BootstrapStats`, `ConductorStats`, `FederationHealthStats` — all bespoke structs
- Steward/node has `NodeMetrics` with K8s-style `NodeConditions` — excellent model but isolated
- elohim-storage has a bare `{ status: "ok", blobs, bytes }` health endpoint
- No shared traits, no shared types, no shared crate
- Zero request counters anywhere, no uniform health contract

The orchestrator and future operator elohim agent can't reason uniformly about the fleet.

## User Story

> As a doorway operator (and eventually as the operator elohim agent), I need every service in my cluster to report health, resource usage, and request throughput in a uniform shape, so I can monitor the fleet without service-specific parsing, and so improvements to metrics in one service automatically benefit all services.

## Key Architectural Principles

### Trait-first IoC contract

Services implement `HealthReporter` and `ResourceReporter` traits. The shared crate defines the contract; services fill it. Compile-time safety, loose coupling.

### Snapshot types are the persistable shape

All public types use `DateTime<Utc>` (not `Instant`), `HashMap` (not `DashMap`), plain integers (not atomics). The concurrent internals (`RequestCounters`, `PeerHealthRegistry`) are private; their `snapshot()` methods produce the clean, serializable, MongoDB-ready shape.

### Three-state health vocabulary

```
Healthy — all subsystems nominal
Degraded — partially functional, operator attention needed
Offline — not serving, intervention required
```

K8s-familiar. Reused for both service-level and peer-level health.

### Minimal dependencies

`serde`, `serde_json`, `dashmap`, `chrono`, `parking_lot`. No `sysinfo`, no `tokio`, no async runtime. Pure data types + thread-safe counters.

### Extensions for service-specific data

Each service's domain metrics (doorway: projection stats, hot cache; steward: P2P peers, cluster status) go in `extensions: serde_json::Value`. Typed on the producing side, opaque to fleet-level consumers.

## Crate Structure

```
elohim/elohim-compute/
├── Cargo.toml
└── src/
    ├── lib.rs          # Re-exports
    ├── health.rs       # ServiceHealth enum + HealthReporter trait
    ├── resources.rs    # ResourceSnapshot + ResourceReporter trait
    ├── counters.rs     # RequestCounters (DashMap<String, AtomicU64>)
    ├── peers.rs        # PeerHealthSnapshot + PeerHealthRegistry
    └── report.rs       # ComputeReport (top-level envelope)
```

**Location:** `elohim/elohim-compute/` — added to `elohim/Cargo.toml` workspace members. Doorway references via `path = "../../elohim/elohim-compute"`.

## Core Types

### ServiceHealth — state machine

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    Offline,
}
```

### HealthReporter trait

```rust
pub trait HealthReporter: Send + Sync {
    fn service_id(&self) -> &str;
    fn health(&self) -> ServiceHealth;
    fn health_reason(&self) -> String;
    fn started_at(&self) -> DateTime<Utc>;
}
```

### ResourceReporter trait

```rust
pub trait ResourceReporter: Send + Sync {
    fn resource_snapshot(&self) -> ResourceSnapshot;
    fn extension_snapshot(&self) -> serde_json::Value;
}
```

### ResourceSnapshot

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub requests: RequestCounterSnapshot,
    pub active_connections: usize,
    pub managed_storage_bytes: u64,
    pub managed_document_count: u64,
}
```

### RequestCounterSnapshot

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCounterSnapshot {
    pub total: u64,
    pub by_category: HashMap<String, u64>,
}
```

### PeerHealthSnapshot

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerHealthSnapshot {
    pub peer_id: String,
    pub address: String,
    pub health: ServiceHealth,
    pub reason: String,
    pub signals_received: u64,
    pub last_signal_at: Option<DateTime<Utc>>,
    pub reconnect_attempts: u32,
}
```

### ComputeReport — the envelope

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeReport {
    pub service_id: String,
    pub version: String,
    pub health: ServiceHealth,
    pub health_reason: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub resources: ResourceSnapshot,
    pub peers: Vec<PeerHealthSnapshot>,
    pub extensions: serde_json::Value,
}
```

## Concrete Implementations

### RequestCounters

Thread-safe request counter. `DashMap<String, AtomicU64>` internally, `RequestCounterSnapshot` externally.

```rust
impl RequestCounters {
    pub fn new() -> Self;
    pub fn increment(&self, category: &str);  // Hot path: one atomic add
    pub fn snapshot(&self) -> RequestCounterSnapshot;
}
```

### PeerHealthRegistry

Thread-safe peer state tracking. `DashMap<String, PeerEntry>` internally, `Vec<PeerHealthSnapshot>` externally.

```rust
impl PeerHealthRegistry {
    pub fn new() -> Self;
    pub fn register(&self, peer_id: &str, address: &str);
    pub fn record_signal(&self, peer_id: &str);
    pub fn update_health(&self, peer_id: &str, health: ServiceHealth, reason: &str);
    pub fn record_reconnect(&self, peer_id: &str);
    pub fn active_count(&self) -> usize;
    pub fn snapshot(&self) -> Vec<PeerHealthSnapshot>;
}
```

Uses `parking_lot::RwLock` for string fields, atomics for counters. `DateTime<Utc>` internally so snapshots need no conversion.

### ComputeReport::build()

Convenience builder that reads from a `HealthReporter`:

```rust
impl ComputeReport {
    pub fn build(
        reporter: &dyn HealthReporter,
        resources: ResourceSnapshot,
        peers: Vec<PeerHealthSnapshot>,
        extensions: serde_json::Value,
    ) -> Self;
}
```

Caller fills `version` from `env!("CARGO_PKG_VERSION")` after construction (can't read that from a library).

## Doorway Integration

### AppState additions

```rust
pub peer_health: Arc<PeerHealthRegistry>,
pub request_counters: Arc<RequestCounters>,
pub started_at: DateTime<Utc>,
```

### DoorwayHealthReporter

Doorway-local struct implementing `HealthReporter`. Derives health from conductor connectivity + storage reachability + projection subscriber state.

### Subscriber forwarding wiring (main.rs)

- `peer_health.register(conductor_id, url)` before spawning each subscriber
- `peer_health.update_health(id, Healthy, "connected")` when forwarding starts
- `peer_health.record_signal(id)` on each signal
- `peer_health.update_health(id, Degraded, "lagged N signals")` on lag
- `peer_health.update_health(id, Offline, "channel closed")` on close

### Request counter wiring (routes/api.rs)

One line after `CacheRoute::parse(path)`: `state.request_counters.increment(route.doc_type)`

### StatusResponse extension (routes/status.rs)

New `compute: ComputeReport` field added to existing `StatusResponse`. All existing fields preserved for backward compat. `build_status_data()` assembles the report via `ComputeReport::build()`.

MongoDB `collStats` query (cached 30s) provides `managed_storage_bytes` and `managed_document_count`.

### HTML status page (templates/status.html)

Two new card sections: Projection Peers (health dot + signal count + last signal) and Resource Usage (managed storage, requests, connections). Rendered from `ComputeReport` data.

## What Changes

| File | What |
|------|------|
| `elohim/elohim-compute/` (new) | Shared crate: types, traits, counters, registry |
| `elohim/Cargo.toml` | Add `elohim-compute` to workspace members |
| `doorway/doorway-service/Cargo.toml` | Add `elohim-compute` dependency |
| `doorway/doorway-service/src/server/http.rs` | Add `peer_health`, `request_counters`, `started_at` to AppState |
| `doorway/doorway-service/src/main.rs` | Wire PeerHealthRegistry into subscriber forwarding |
| `doorway/doorway-service/src/routes/api.rs` | Increment request counters |
| `doorway/doorway-service/src/routes/status.rs` | Add `compute` field, `DoorwayHealthReporter`, `fetch_projection_stats` |
| `doorway/doorway-service/templates/status.html` | Render peer health cards and resource usage section |

## What Doesn't Change

- Health probes (`/health`, `/ready`) — stay lightweight
- Signal processing hot path — `record_signal()` is one atomic increment
- Existing `StatusResponse` fields — all preserved
- Angular data-loader — unaware of compute metrics
- steward/node — adopts the shared crate in a follow-up sprint

## Future (Not This Sprint)

- **Steward adoption** — steward/node imports `elohim-compute`, maps `NodeMetrics` to `ComputeReport`
- **elohim-storage adoption** — storage implements `HealthReporter`, replaces bare health endpoint
- **MongoDB persistence** — snapshot types are designed for direct MongoDB insertion (uptime history, request rate trends)
- **Operator elohim agent** — reads `ComputeReport` from all services, reasons about fleet health
- **`sysinfo` feature flag** — optional `SystemResourceCollector` for services that want CPU/memory/disk
- **EPR hosting contracts** — `ComputeReport` feeds into shefa economic protocol for mutual credit settlement
