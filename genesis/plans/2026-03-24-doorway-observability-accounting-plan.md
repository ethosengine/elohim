# Doorway Observability & Resource Accounting — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add per-peer projection health monitoring and doorway resource usage metrics to the `/status` endpoint.

**Architecture:** A `PeerHealthRegistry` (DashMap-based) in AppState tracks per-subscriber connection state and signal counts. A `RequestCounters` struct tracks API request throughput by doc_type. Both are updated inline with no background tasks. The `/status` route assembles `DoorwayResourceUsage` at request time from these counters plus a cached MongoDB `collStats` call. Angular does zero aggregation — it renders JSON from `/status`.

**Tech Stack:** Rust (doorway-service), DashMap, MongoDB collStats, Askama templates

**Design doc:** `genesis/plans/2026-03-24-doorway-observability-accounting-design.md`

---

### Task 1: Create PeerHealthRegistry

**Files:**
- Create: `doorway/doorway-service/src/projection/peer_health.rs`
- Modify: `doorway/doorway-service/src/projection/mod.rs`

**Step 1: Write the tests**

```rust
// In peer_health.rs at bottom
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].conductor_id, "conductor-0");
        assert_eq!(snapshot[0].state, "registered");
        assert_eq!(snapshot[0].signals_received, 0);
    }

    #[test]
    fn test_record_signal() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.record_signal("conductor-0");
        registry.record_signal("conductor-0");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].signals_received, 2);
        assert!(snapshot[0].last_signal_at.is_some());
    }

    #[test]
    fn test_update_state() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.update_state("conductor-0", "connected");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].state, "connected");
        assert_eq!(snapshot[0].reconnect_attempts, 0);
    }

    #[test]
    fn test_record_reconnect() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.record_reconnect("conductor-0");
        registry.record_reconnect("conductor-0");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].reconnect_attempts, 2);
        assert_eq!(snapshot[0].state, "reconnecting");
    }

    #[test]
    fn test_active_count() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.register("conductor-1", "ws://host2:8445");
        registry.update_state("conductor-0", "connected");
        assert_eq!(registry.active_count(), 1);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib peer_health`
Expected: FAIL — module doesn't exist

**Step 3: Write the implementation**

```rust
//! Per-peer projection subscriber health tracking.
//!
//! Updated inline by subscriber forwarding tasks — no background threads.
//! Read by /status route to build operator/agent view.

use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Snapshot of a single peer's projection health (returned by /status)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerProjectionHealth {
    pub conductor_id: String,
    pub conductor_url: String,
    pub state: String,
    pub signals_received: u64,
    pub last_signal_at: Option<String>,
    pub reconnect_attempts: u32,
}

/// Internal mutable entry per peer
struct PeerEntry {
    conductor_url: String,
    state: parking_lot::RwLock<String>,
    signals_received: AtomicU64,
    last_signal_at: parking_lot::RwLock<Option<Instant>>,
    reconnect_attempts: AtomicU32,
}

/// Registry tracking health of all peer signal subscribers.
///
/// Thread-safe. Updated by forwarding tasks, read by status route.
pub struct PeerHealthRegistry {
    peers: DashMap<String, PeerEntry>,
    boot_time: Instant,
}

impl PeerHealthRegistry {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            boot_time: Instant::now(),
        }
    }

    /// Register a new peer subscriber
    pub fn register(&self, conductor_id: &str, conductor_url: &str) {
        self.peers.insert(
            conductor_id.to_string(),
            PeerEntry {
                conductor_url: conductor_url.to_string(),
                state: parking_lot::RwLock::new("registered".to_string()),
                signals_received: AtomicU64::new(0),
                last_signal_at: parking_lot::RwLock::new(None),
                reconnect_attempts: AtomicU32::new(0),
            },
        );
    }

    /// Record a signal received from a peer
    pub fn record_signal(&self, conductor_id: &str) {
        if let Some(entry) = self.peers.get(conductor_id) {
            entry.signals_received.fetch_add(1, Ordering::Relaxed);
            *entry.last_signal_at.write() = Some(Instant::now());
        }
    }

    /// Update connection state (e.g., "connected", "authenticating", "failed")
    pub fn update_state(&self, conductor_id: &str, state: &str) {
        if let Some(entry) = self.peers.get(conductor_id) {
            *entry.state.write() = state.to_string();
            if state == "connected" {
                entry.reconnect_attempts.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Record a reconnection attempt
    pub fn record_reconnect(&self, conductor_id: &str) {
        if let Some(entry) = self.peers.get(conductor_id) {
            entry.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
            *entry.state.write() = "reconnecting".to_string();
        }
    }

    /// Count of peers in "connected" state
    pub fn active_count(&self) -> usize {
        self.peers
            .iter()
            .filter(|e| *e.value().state.read() == "connected")
            .count()
    }

    /// Snapshot all peer health for serialization
    pub fn snapshot(&self) -> Vec<PeerProjectionHealth> {
        self.peers
            .iter()
            .map(|entry| {
                let last_signal = entry.last_signal_at.read().map(|t| {
                    let secs_ago = t.elapsed().as_secs();
                    format!("{}s ago", secs_ago)
                });

                PeerProjectionHealth {
                    conductor_id: entry.key().clone(),
                    conductor_url: entry.conductor_url.clone(),
                    state: entry.state.read().clone(),
                    signals_received: entry.signals_received.load(Ordering::Relaxed),
                    last_signal_at: last_signal,
                    reconnect_attempts: entry.reconnect_attempts.load(Ordering::Relaxed),
                }
            })
            .collect()
    }
}
```

**Step 4: Add module to projection/mod.rs**

Add `pub mod peer_health;` and re-export:
```rust
pub mod peer_health;
pub use peer_health::{PeerHealthRegistry, PeerProjectionHealth};
```

**Step 5: Run tests to verify they pass**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib peer_health`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/projection/peer_health.rs doorway/doorway-service/src/projection/mod.rs
git commit -m "feat(doorway): add PeerHealthRegistry for per-subscriber health tracking"
```

---

### Task 2: Create RequestCounters

**Files:**
- Create: `doorway/doorway-service/src/projection/request_counters.rs`
- Modify: `doorway/doorway-service/src/projection/mod.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_and_snapshot() {
        let counters = RequestCounters::new();
        counters.increment("Content");
        counters.increment("Content");
        counters.increment("LearningPath");
        let snap = counters.snapshot();
        assert_eq!(snap.total, 3);
        assert_eq!(*snap.by_type.get("Content").unwrap(), 2);
        assert_eq!(*snap.by_type.get("LearningPath").unwrap(), 1);
    }

    #[test]
    fn test_empty_snapshot() {
        let counters = RequestCounters::new();
        let snap = counters.snapshot();
        assert_eq!(snap.total, 0);
        assert!(snap.by_type.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib request_counters`
Expected: FAIL — module doesn't exist

**Step 3: Write the implementation**

```rust
//! Request throughput counters for the /api/v1/cache/ endpoint.
//!
//! Tracks requests by doc_type. Updated inline in the cache API handler.
//! Read by /status route for resource accounting.

use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of request counters (returned by /status)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCounterSnapshot {
    pub total: u64,
    pub by_type: HashMap<String, u64>,
}

/// Thread-safe request counters by doc_type.
pub struct RequestCounters {
    total: AtomicU64,
    by_type: DashMap<String, AtomicU64>,
}

impl RequestCounters {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            by_type: DashMap::new(),
        }
    }

    /// Increment counter for a doc_type
    pub fn increment(&self, doc_type: &str) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.by_type
            .entry(doc_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot all counters for serialization
    pub fn snapshot(&self) -> RequestCounterSnapshot {
        let by_type: HashMap<String, u64> = self
            .by_type
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();

        RequestCounterSnapshot {
            total: self.total.load(Ordering::Relaxed),
            by_type,
        }
    }
}
```

**Step 4: Add module to projection/mod.rs**

Add `pub mod request_counters;` and re-export:
```rust
pub mod request_counters;
pub use request_counters::{RequestCounters, RequestCounterSnapshot};
```

**Step 5: Run tests to verify they pass**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib request_counters`
Expected: All 2 tests PASS

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/projection/request_counters.rs doorway/doorway-service/src/projection/mod.rs
git commit -m "feat(doorway): add RequestCounters for cache API throughput tracking"
```

---

### Task 3: Wire PeerHealthRegistry into AppState and subscriber forwarding

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs:43-110` (AppState struct)
- Modify: `doorway/doorway-service/src/main.rs:500-547` (subscriber forwarding loop)

**Step 1: Add fields to AppState**

In `server/http.rs`, add to the `AppState` struct (after `route_registry` field, before `journal_inference_available`):

```rust
    /// Per-peer projection subscriber health
    pub peer_health: Arc<crate::projection::PeerHealthRegistry>,
    /// Request throughput counters for /api/v1/cache/
    pub request_counters: Arc<crate::projection::RequestCounters>,
```

**Step 2: Initialize in all AppState constructors**

In each constructor (`new()`, `with_services()`, `with_pool()`, `with_projection()`), add to the `Self { ... }` block:

```rust
    peer_health: Arc::new(crate::projection::PeerHealthRegistry::new()),
    request_counters: Arc::new(crate::projection::RequestCounters::new()),
```

**Step 3: Wire into subscriber forwarding in main.rs**

In `main.rs`, the subscriber loop (around line 505-547). After `spawn_subscriber`, register the peer. In the forwarding task, record signals:

```rust
// Before the for loop, clone the registry
let peer_health = Arc::clone(&state.peer_health);

for (i, conductor_app_url) in conductor_urls.iter().enumerate() {
    let conductor_id = format!("conductor-{i}");

    // Register peer in health registry
    peer_health.register(&conductor_id, conductor_app_url);

    // ... existing subscriber_config + spawn_subscriber ...

    // Forward this subscriber's signals to the shared engine channel
    let mut sub_rx = subscriber.subscribe();
    let fwd_tx = all_signals_tx.clone();
    let conductor_id_clone = conductor_id.clone();
    let peer_health_clone = Arc::clone(&peer_health);
    tokio::spawn(async move {
        peer_health_clone.update_state(&conductor_id_clone, "connected");
        loop {
            match sub_rx.recv().await {
                Ok(signal) => {
                    peer_health_clone.record_signal(&conductor_id_clone);
                    if fwd_tx.send(signal).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    peer_health_clone.update_state(&conductor_id_clone, "disconnected");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        conductor = %conductor_id_clone,
                        lagged = n,
                        "Signal forwarder lagged"
                    );
                }
            }
        }
    });

    // ... existing info! log ...
}
```

Note: `state` is `Arc<AppState>` at this point (line 416+), so `state.peer_health` works.

**Step 4: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release && RUSTFLAGS="" cargo test --lib --bins`
Expected: Clean build, 346+ tests pass (344 existing + 7 new from Tasks 1-2)

**Step 5: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): wire PeerHealthRegistry and RequestCounters into AppState"
```

---

### Task 4: Increment request counters in cache API handler

**Files:**
- Modify: `doorway/doorway-service/src/routes/api.rs:191-296`

**Step 1: Add counter increment after route parsing**

In `handle_api_request()`, after the `CacheRoute::parse(path)` check succeeds (around line 208), add:

```rust
    // Track request for resource accounting
    state.request_counters.increment(route.doc_type);
```

This is a single line. It goes right after the route is parsed, before any response logic.

**Step 2: Build and verify**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release`
Expected: Clean build

**Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/api.rs
git commit -m "feat(doorway): increment request counters in cache API handler"
```

---

### Task 5: Add peers and resources to StatusResponse

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs`

**Step 1: Add new structs**

Add `DoorwayResourceUsage` struct near the other stats structs (after `Diagnostics`, around line 164):

```rust
/// Doorway resource usage — what this doorway consumes to serve the network.
/// Content ownership is a P2P concern. This tracks doorway's own compute:
/// projection, caching, bandwidth, DNS.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayResourceUsage {
    /// MongoDB projection storage in bytes
    pub projection_bytes: u64,
    /// Document count in projection store
    pub projection_documents: u64,
    /// Hot cache entries (in-memory)
    pub hot_cache_entries: usize,
    /// Requests served since startup
    pub requests: crate::projection::RequestCounterSnapshot,
    /// Active signal subscriber connections
    pub active_subscribers: usize,
    /// Registered storage peers (route registry)
    pub registered_peers: usize,
}
```

**Step 2: Add fields to StatusResponse**

In `StatusResponse` (line 168), add after `diagnostics`:

```rust
    /// Per-peer projection subscriber health
    pub peers: Vec<crate::projection::PeerProjectionHealth>,
    /// Doorway resource usage (compute this doorway provides to the network)
    pub resources: DoorwayResourceUsage,
```

**Step 3: Build peers and resources in build_status_data()**

At the end of `build_status_data()`, before the final `StatusResponse { ... }` construction (around line 487), add:

```rust
    // Peer projection health from subscriber registry
    let peers = state.peer_health.snapshot();

    // Doorway resource usage
    let (projection_bytes, projection_documents) =
        fetch_projection_stats(state).await;
    let hot_cache_entries = state
        .projection
        .as_ref()
        .map(|p| p.hot_cache_stats().total_entries)
        .unwrap_or(0);
    let requests = state.request_counters.snapshot();
    let active_subscribers = state.peer_health.active_count();
    let registered_peers = state.route_registry.peer_count();

    let resources = DoorwayResourceUsage {
        projection_bytes,
        projection_documents,
        hot_cache_entries,
        requests,
        active_subscribers,
        registered_peers,
    };
```

And add `peers` and `resources` to the `StatusResponse { ... }` construction.

**Step 4: Add fetch_projection_stats helper**

Add a helper function that queries MongoDB `collStats` (cached 30s):

```rust
use std::sync::OnceLock;
use tokio::sync::Mutex;

/// Cached MongoDB projection stats (refreshed every 30s)
static PROJECTION_STATS_CACHE: OnceLock<Mutex<(std::time::Instant, u64, u64)>> = OnceLock::new();

async fn fetch_projection_stats(state: &Arc<AppState>) -> (u64, u64) {
    let cache = PROJECTION_STATS_CACHE.get_or_init(|| {
        Mutex::new((std::time::Instant::now() - std::time::Duration::from_secs(60), 0, 0))
    });

    let mut cached = cache.lock().await;
    if cached.0.elapsed().as_secs() < 30 {
        return (cached.1, cached.2);
    }

    // Query MongoDB collStats
    let result = if let Some(ref mongo) = state.mongo {
        let db = mongo.inner().database(mongo.db_name());
        match db.run_command(bson::doc! { "collStats": "projected_entries" }).await {
            Ok(doc) => {
                let bytes = doc.get_i64("size").unwrap_or(0) as u64;
                let count = doc.get_i64("count").unwrap_or(0) as u64;
                (bytes, count)
            }
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    };

    *cached = (std::time::Instant::now(), result.0, result.1);
    result
}
```

Note: This requires adding `use bson;` to the imports if not already present. Check existing imports in status.rs — `bson` may already be in scope via other uses.

**Step 5: Check route_registry has peer_count()**

The `RouteRegistry` may not have a `peer_count()` method. Check `services/route_registry.rs`. If it doesn't exist, count the registered steward peers:

```rust
// If peer_count() doesn't exist, use 0 for now:
let registered_peers = 0; // TODO: add RouteRegistry::peer_count()
```

**Step 6: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release && RUSTFLAGS="" cargo test --lib --bins`
Expected: Clean build, all tests pass

**Step 7: Commit**

```bash
git add doorway/doorway-service/src/routes/status.rs
git commit -m "feat(doorway): add peer health and resource usage to /status endpoint"
```

---

### Task 6: Update status.html template

**Files:**
- Modify: `doorway/doorway-service/templates/status.html`

**Step 1: Find the template**

Read `doorway/doorway-service/templates/status.html` and find where federation peers are rendered. The new peer health section goes after the existing components section and before or alongside the federation section.

**Step 2: Add StatusPageTemplate fields**

In `routes/status.rs`, add to `StatusPageTemplate` (around line 234):

```rust
    pub projection_peers: Vec<ProjectionPeerView>,
    pub resource_usage: ResourceUsageView,
```

Create the view types:

```rust
pub struct ProjectionPeerView {
    pub conductor_id: String,
    pub dot_color: String,
    pub state: String,
    pub signals_received: String,
    pub last_signal: String,
    pub reconnect_attempts: u32,
}

pub struct ResourceUsageView {
    pub projection_storage: String,  // e.g., "450 MB"
    pub projection_documents: String, // e.g., "12,340"
    pub hot_cache_entries: String,
    pub total_requests: String,
    pub active_subscribers: String,
}
```

**Step 3: Populate in the HTML page handler**

In the HTML handler function (around line 535+), map the status data to template views. Format bytes as human-readable (KB/MB/GB), format numbers with comma separators.

**Step 4: Add HTML sections to template**

Add a "Projection Peers" card section and a "Resource Usage" card section to the template, following the existing card pattern used for federation peers and components.

**Step 5: Build and verify**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release`
Expected: Clean build (Askama validates templates at compile time)

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/routes/status.rs doorway/doorway-service/templates/status.html
git commit -m "feat(doorway): render peer health and resource usage on /status HTML page"
```

---

### Task 7: Final verification

**Step 1: Run full test suite**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins`
Expected: 350+ tests pass (344 existing + new)

**Step 2: Run clippy**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings`
Expected: Clean

**Step 3: Run fmt**

Run: `cd doorway/doorway-service && cargo fmt --check`
Expected: Clean

**Step 4: Verify /status JSON shape**

Review `StatusResponse` struct and confirm:
- `peers: Vec<PeerProjectionHealth>` present
- `resources: DoorwayResourceUsage` present
- All fields are `camelCase` via serde rename

**Step 5: Push**

Run: `git push`
Expected: Pre-push hook passes all gates
