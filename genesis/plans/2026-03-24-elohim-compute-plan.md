# elohim-compute Shared Crate + Doorway Integration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a shared `elohim-compute` crate defining uniform health/resource/request reporting types and traits, then wire it into doorway as the proving ground.

**Architecture:** Trait-first IoC — `HealthReporter` and `ResourceReporter` traits define the fleet contract. Concrete `RequestCounters` and `PeerHealthRegistry` provide thread-safe concurrent internals with clean snapshot types (MongoDB-ready). Doorway implements the traits and adds a `compute: ComputeReport` field to its existing `StatusResponse`.

**Tech Stack:** Rust, serde, dashmap, chrono, parking_lot. No async runtime in the shared crate.

**Design doc:** `genesis/plans/2026-03-24-elohim-compute-shared-crate-design.md`

**RUSTFLAGS:** All doorway builds MUST use `RUSTFLAGS=""` to override the Holochain WASM env.

---

### Task 1: Create elohim-compute crate scaffold

**Files:**
- Create: `elohim/elohim-compute/Cargo.toml`
- Create: `elohim/elohim-compute/src/lib.rs`
- Modify: `elohim/Cargo.toml` (add workspace member)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "elohim-compute"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Elohim Protocol"]
description = "Shared compute reporting types and traits for the Elohim fleet"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dashmap = "6.1"
chrono = { version = "0.4", features = ["serde"] }
parking_lot = "0.12"
```

**Step 2: Create src/lib.rs (empty scaffold)**

```rust
//! Shared compute reporting for the Elohim fleet.
//!
//! Defines uniform types and traits for health, resource usage,
//! request throughput, and peer health — consumed by every service's
//! `/status` endpoint and the operator elohim agent.

pub mod health;
pub mod resources;
pub mod counters;
pub mod peers;
pub mod report;

pub use health::{HealthReporter, ServiceHealth};
pub use resources::{ResourceReporter, ResourceSnapshot};
pub use counters::{RequestCounters, RequestCounterSnapshot};
pub use peers::{PeerHealthRegistry, PeerHealthSnapshot};
pub use report::ComputeReport;
```

**Step 3: Add to elohim workspace**

In `elohim/Cargo.toml`, add `"elohim-compute"` to the `[workspace] members` array:

```toml
members = [
    "constitution",
    "elohim-agent/elohim-agent-service",
    "eae",
    "elohim-compute",
]
```

**Step 4: Create empty module files**

Create these files with placeholder `// TODO` comments so the crate compiles:

- `elohim/elohim-compute/src/health.rs`
- `elohim/elohim-compute/src/resources.rs`
- `elohim/elohim-compute/src/counters.rs`
- `elohim/elohim-compute/src/peers.rs`
- `elohim/elohim-compute/src/report.rs`

Each file: empty (we'll fill them in subsequent tasks with TDD).

**Step 5: Verify crate compiles**

Run: `cd elohim/elohim-compute && cargo build`
Expected: Clean build (empty modules, no errors)

**Step 6: Commit**

```
feat(compute): scaffold elohim-compute shared crate

Empty crate structure with module declarations. Types and traits
added in subsequent commits via TDD.
```

---

### Task 2: Implement ServiceHealth enum and HealthReporter trait

**Files:**
- Modify: `elohim/elohim-compute/src/health.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_health_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceHealth::Offline).unwrap(),
            "\"offline\""
        );
    }

    #[test]
    fn test_service_health_deserializes() {
        let h: ServiceHealth = serde_json::from_str("\"healthy\"").unwrap();
        assert_eq!(h, ServiceHealth::Healthy);
        let d: ServiceHealth = serde_json::from_str("\"degraded\"").unwrap();
        assert_eq!(d, ServiceHealth::Degraded);
    }

    #[test]
    fn test_service_health_default_is_offline() {
        assert_eq!(ServiceHealth::default(), ServiceHealth::Offline);
    }

    #[test]
    fn test_service_health_display() {
        assert_eq!(format!("{}", ServiceHealth::Healthy), "healthy");
        assert_eq!(format!("{}", ServiceHealth::Degraded), "degraded");
        assert_eq!(format!("{}", ServiceHealth::Offline), "offline");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd elohim/elohim-compute && cargo test health`
Expected: FAIL — types don't exist yet

**Step 3: Write the implementation**

```rust
//! Service health state machine and reporting trait.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Three-state health vocabulary for any service in the fleet.
///
/// K8s-familiar. Used for both service-level and peer-level health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceHealth {
    /// All subsystems nominal
    Healthy,
    /// Partially functional, operator attention needed
    Degraded,
    /// Not serving, intervention required
    Offline,
}

impl Default for ServiceHealth {
    fn default() -> Self {
        Self::Offline
    }
}

impl fmt::Display for ServiceHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Offline => write!(f, "offline"),
        }
    }
}

/// Trait for services to report their health.
///
/// Implement this on your service's state struct (or a wrapper).
/// The `ComputeReport::build()` method reads from this trait.
pub trait HealthReporter: Send + Sync {
    /// Service identifier (e.g., "doorway", "steward", "storage")
    fn service_id(&self) -> &str;

    /// Current health state
    fn health(&self) -> ServiceHealth;

    /// Human-readable reason for current state
    /// (e.g., "4/4 conductors connected, storage reachable")
    fn health_reason(&self) -> String;

    /// When the service started (for uptime calculation)
    fn started_at(&self) -> DateTime<Utc>;
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-compute && cargo test health`
Expected: All 4 tests PASS

**Step 5: Commit**

```
feat(compute): add ServiceHealth enum and HealthReporter trait

Three-state health vocabulary (Healthy/Degraded/Offline) with
serde lowercase serialization and Display impl. HealthReporter
trait defines the IoC contract for fleet-wide health reporting.
```

---

### Task 3: Implement RequestCounters

**Files:**
- Modify: `elohim/elohim-compute/src/counters.rs`

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
        assert_eq!(*snap.by_category.get("Content").unwrap(), 2);
        assert_eq!(*snap.by_category.get("LearningPath").unwrap(), 1);
    }

    #[test]
    fn test_empty_snapshot() {
        let counters = RequestCounters::new();
        let snap = counters.snapshot();
        assert_eq!(snap.total, 0);
        assert!(snap.by_category.is_empty());
    }

    #[test]
    fn test_snapshot_serializes_camel_case() {
        let counters = RequestCounters::new();
        counters.increment("Content");
        let snap = counters.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"byCategory\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn test_concurrent_increments() {
        use std::sync::Arc;
        use std::thread;

        let counters = Arc::new(RequestCounters::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&counters);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.increment("Content");
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let snap = counters.snapshot();
        assert_eq!(snap.total, 1000);
        assert_eq!(*snap.by_category.get("Content").unwrap(), 1000);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd elohim/elohim-compute && cargo test counters`
Expected: FAIL — types don't exist yet

**Step 3: Write the implementation**

```rust
//! Thread-safe request throughput counters.
//!
//! Tracks requests by category (e.g., doc_type, endpoint, protocol).
//! Updated inline in hot paths via atomic increments.
//! `snapshot()` produces a clean, serializable, MongoDB-ready shape.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of request counters — the persistable/serializable shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCounterSnapshot {
    /// Total requests since startup
    pub total: u64,
    /// Requests broken down by category
    pub by_category: HashMap<String, u64>,
}

/// Thread-safe request counters by category.
///
/// Hot path: `increment()` is one atomic add + one DashMap upsert.
/// Cold path: `snapshot()` reads all atomics (no locking).
pub struct RequestCounters {
    total: AtomicU64,
    by_category: DashMap<String, AtomicU64>,
}

impl RequestCounters {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            by_category: DashMap::new(),
        }
    }

    /// Increment counter for a category. Called inline in request handlers.
    pub fn increment(&self, category: &str) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.by_category
            .entry(category.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot all counters for serialization.
    pub fn snapshot(&self) -> RequestCounterSnapshot {
        let by_category: HashMap<String, u64> = self
            .by_category
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();

        RequestCounterSnapshot {
            total: self.total.load(Ordering::Relaxed),
            by_category,
        }
    }
}

impl Default for RequestCounters {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-compute && cargo test counters`
Expected: All 4 tests PASS

**Step 5: Commit**

```
feat(compute): add RequestCounters with atomic increments

DashMap<String, AtomicU64> pattern for zero-lock hot-path counting.
Snapshot produces clean HashMap<String, u64> for JSON/MongoDB.
```

---

### Task 4: Implement PeerHealthRegistry

**Files:**
- Modify: `elohim/elohim-compute/src/peers.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceHealth;

    #[test]
    fn test_register_and_snapshot() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].peer_id, "conductor-0");
        assert_eq!(snapshot[0].address, "ws://host:8445");
        assert_eq!(snapshot[0].health, ServiceHealth::Offline);
        assert_eq!(snapshot[0].signals_received, 0);
        assert!(snapshot[0].last_signal_at.is_none());
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
    fn test_update_health() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.update_health("conductor-0", ServiceHealth::Healthy, "connected");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].health, ServiceHealth::Healthy);
        assert_eq!(snapshot[0].reason, "connected");
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
        assert_eq!(snapshot[0].health, ServiceHealth::Degraded);
    }

    #[test]
    fn test_healthy_resets_reconnect_attempts() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.record_reconnect("conductor-0");
        registry.record_reconnect("conductor-0");
        registry.update_health("conductor-0", ServiceHealth::Healthy, "reconnected");
        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].reconnect_attempts, 0);
        assert_eq!(snapshot[0].health, ServiceHealth::Healthy);
    }

    #[test]
    fn test_active_count() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.register("conductor-1", "ws://host2:8445");
        registry.update_health("conductor-0", ServiceHealth::Healthy, "connected");
        assert_eq!(registry.active_count(), 1);
        registry.update_health("conductor-1", ServiceHealth::Healthy, "connected");
        assert_eq!(registry.active_count(), 2);
    }

    #[test]
    fn test_unknown_peer_id_is_noop() {
        let registry = PeerHealthRegistry::new();
        registry.record_signal("nonexistent");
        registry.update_health("nonexistent", ServiceHealth::Healthy, "test");
        registry.record_reconnect("nonexistent");
        assert_eq!(registry.snapshot().len(), 0);
    }

    #[test]
    fn test_snapshot_serializes_camel_case() {
        let registry = PeerHealthRegistry::new();
        registry.register("conductor-0", "ws://host:8445");
        registry.record_signal("conductor-0");
        let snapshot = registry.snapshot();
        let json = serde_json::to_string(&snapshot[0]).unwrap();
        assert!(json.contains("\"peerId\""));
        assert!(json.contains("\"signalsReceived\""));
        assert!(json.contains("\"lastSignalAt\""));
        assert!(json.contains("\"reconnectAttempts\""));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd elohim/elohim-compute && cargo test peers`
Expected: FAIL — types don't exist yet

**Step 3: Write the implementation**

```rust
//! Per-peer health tracking registry.
//!
//! Thread-safe registry for monitoring subscriber/connection health.
//! Updated inline by forwarding tasks, read by status endpoint.
//! `snapshot()` produces `Vec<PeerHealthSnapshot>` — MongoDB-ready.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::ServiceHealth;

/// Snapshot of a single peer's health — the persistable/serializable shape.
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

/// Internal mutable entry per peer — not exposed.
struct PeerEntry {
    address: String,
    health: RwLock<ServiceHealth>,
    reason: RwLock<String>,
    signals_received: AtomicU64,
    last_signal_at: RwLock<Option<DateTime<Utc>>>,
    reconnect_attempts: AtomicU32,
}

/// Registry tracking health of all peers/subscribers.
///
/// Thread-safe. Updated by forwarding tasks, read by status route.
pub struct PeerHealthRegistry {
    peers: DashMap<String, PeerEntry>,
}

impl PeerHealthRegistry {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
        }
    }

    /// Register a new peer. Initial state is Offline ("registered, not yet connected").
    pub fn register(&self, peer_id: &str, address: &str) {
        self.peers.insert(
            peer_id.to_string(),
            PeerEntry {
                address: address.to_string(),
                health: RwLock::new(ServiceHealth::Offline),
                reason: RwLock::new("registered".to_string()),
                signals_received: AtomicU64::new(0),
                last_signal_at: RwLock::new(None),
                reconnect_attempts: AtomicU32::new(0),
            },
        );
    }

    /// Record a signal received from a peer.
    pub fn record_signal(&self, peer_id: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            entry.signals_received.fetch_add(1, Ordering::Relaxed);
            *entry.last_signal_at.write() = Some(Utc::now());
        }
    }

    /// Update peer health state with a reason.
    /// Setting Healthy resets reconnect_attempts to 0.
    pub fn update_health(&self, peer_id: &str, health: ServiceHealth, reason: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            *entry.health.write() = health;
            *entry.reason.write() = reason.to_string();
            if health == ServiceHealth::Healthy {
                entry.reconnect_attempts.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Record a reconnection attempt. Sets health to Degraded.
    pub fn record_reconnect(&self, peer_id: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            entry.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
            *entry.health.write() = ServiceHealth::Degraded;
            *entry.reason.write() = "reconnecting".to_string();
        }
    }

    /// Count of peers in Healthy state.
    pub fn active_count(&self) -> usize {
        self.peers
            .iter()
            .filter(|e| *e.value().health.read() == ServiceHealth::Healthy)
            .count()
    }

    /// Snapshot all peer health for serialization.
    pub fn snapshot(&self) -> Vec<PeerHealthSnapshot> {
        self.peers
            .iter()
            .map(|entry| PeerHealthSnapshot {
                peer_id: entry.key().clone(),
                address: entry.address.clone(),
                health: *entry.health.read(),
                reason: entry.reason.read().clone(),
                signals_received: entry.signals_received.load(Ordering::Relaxed),
                last_signal_at: *entry.last_signal_at.read(),
                reconnect_attempts: entry.reconnect_attempts.load(Ordering::Relaxed),
            })
            .collect()
    }
}

impl Default for PeerHealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-compute && cargo test peers`
Expected: All 8 tests PASS

**Step 5: Commit**

```
feat(compute): add PeerHealthRegistry with typed health states

DashMap-based peer registry with atomic signal counters and
parking_lot RwLock for health state. Snapshots produce clean
PeerHealthSnapshot with DateTime<Utc> timestamps.
```

---

### Task 5: Implement ResourceSnapshot and ResourceReporter trait

**Files:**
- Modify: `elohim/elohim-compute/src/resources.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::RequestCounterSnapshot;
    use std::collections::HashMap;

    #[test]
    fn test_resource_snapshot_serializes_camel_case() {
        let snap = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 42,
                by_category: HashMap::new(),
            },
            active_connections: 3,
            managed_storage_bytes: 1024 * 1024,
            managed_document_count: 100,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"activeConnections\""));
        assert!(json.contains("\"managedStorageBytes\""));
        assert!(json.contains("\"managedDocumentCount\""));
    }

    #[test]
    fn test_resource_snapshot_roundtrip() {
        let snap = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 10,
                by_category: HashMap::from([("Content".to_string(), 10)]),
            },
            active_connections: 2,
            managed_storage_bytes: 500,
            managed_document_count: 5,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: ResourceSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.requests.total, 10);
        assert_eq!(deserialized.active_connections, 2);
        assert_eq!(deserialized.managed_storage_bytes, 500);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd elohim/elohim-compute && cargo test resources`
Expected: FAIL — types don't exist yet

**Step 3: Write the implementation**

```rust
//! Resource usage snapshot and reporting trait.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::RequestCounterSnapshot;

/// Point-in-time resource usage snapshot — the persistable shape.
///
/// Generic enough for any service. Domain-specific metrics go in
/// the `extensions` field on `ComputeReport`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    /// When this snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// Request throughput since startup
    pub requests: RequestCounterSnapshot,
    /// Active peer/subscriber connections
    pub active_connections: usize,
    /// Storage bytes this service manages (projection cache, blob store, etc.)
    pub managed_storage_bytes: u64,
    /// Document/entry count in managed storage
    pub managed_document_count: u64,
}

/// Trait for services to report their resource usage.
pub trait ResourceReporter: Send + Sync {
    /// Point-in-time resource snapshot
    fn resource_snapshot(&self) -> ResourceSnapshot;

    /// Service-specific extension data (projection bytes, P2P peers, etc.)
    fn extension_snapshot(&self) -> serde_json::Value;
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-compute && cargo test resources`
Expected: All 2 tests PASS

**Step 5: Commit**

```
feat(compute): add ResourceSnapshot and ResourceReporter trait

Generic resource usage shape with request counters, connection
count, and managed storage metrics. ResourceReporter trait
provides the IoC contract.
```

---

### Task 6: Implement ComputeReport envelope

**Files:**
- Modify: `elohim/elohim-compute/src/report.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HealthReporter, PeerHealthSnapshot, RequestCounterSnapshot, ResourceSnapshot,
        ServiceHealth,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    struct MockReporter;

    impl HealthReporter for MockReporter {
        fn service_id(&self) -> &str {
            "test-service"
        }
        fn health(&self) -> ServiceHealth {
            ServiceHealth::Healthy
        }
        fn health_reason(&self) -> String {
            "all systems go".to_string()
        }
        fn started_at(&self) -> chrono::DateTime<Utc> {
            Utc::now() - chrono::Duration::seconds(120)
        }
    }

    #[test]
    fn test_build_from_reporter() {
        let reporter = MockReporter;
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 42,
                by_category: HashMap::from([("Content".to_string(), 42)]),
            },
            active_connections: 3,
            managed_storage_bytes: 1024,
            managed_document_count: 10,
        };
        let peers = vec![PeerHealthSnapshot {
            peer_id: "conductor-0".to_string(),
            address: "ws://host:8445".to_string(),
            health: ServiceHealth::Healthy,
            reason: "connected".to_string(),
            signals_received: 100,
            last_signal_at: Some(Utc::now()),
            reconnect_attempts: 0,
        }];
        let extensions = serde_json::json!({ "hotCacheEntries": 500 });

        let report = ComputeReport::build(&reporter, resources, peers, extensions);

        assert_eq!(report.service_id, "test-service");
        assert_eq!(report.health, ServiceHealth::Healthy);
        assert_eq!(report.health_reason, "all systems go");
        assert!(report.uptime_seconds >= 119); // at least ~120s minus clock skew
        assert_eq!(report.resources.requests.total, 42);
        assert_eq!(report.peers.len(), 1);
        assert_eq!(report.extensions["hotCacheEntries"], 500);
    }

    #[test]
    fn test_report_serializes_camel_case() {
        let reporter = MockReporter;
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 0,
                by_category: HashMap::new(),
            },
            active_connections: 0,
            managed_storage_bytes: 0,
            managed_document_count: 0,
        };

        let report = ComputeReport::build(
            &reporter,
            resources,
            vec![],
            serde_json::Value::Null,
        );

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"serviceId\""));
        assert!(json.contains("\"healthReason\""));
        assert!(json.contains("\"startedAt\""));
        assert!(json.contains("\"uptimeSeconds\""));
    }

    #[test]
    fn test_report_roundtrip() {
        let reporter = MockReporter;
        let resources = ResourceSnapshot {
            timestamp: Utc::now(),
            requests: RequestCounterSnapshot {
                total: 5,
                by_category: HashMap::new(),
            },
            active_connections: 1,
            managed_storage_bytes: 2048,
            managed_document_count: 3,
        };
        let mut report = ComputeReport::build(&reporter, resources, vec![], serde_json::json!({}));
        report.version = "0.1.0".to_string();

        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ComputeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.service_id, "test-service");
        assert_eq!(deserialized.version, "0.1.0");
        assert_eq!(deserialized.health, ServiceHealth::Healthy);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd elohim/elohim-compute && cargo test report`
Expected: FAIL — types don't exist yet

**Step 3: Write the implementation**

```rust
//! ComputeReport — the top-level envelope every service returns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{HealthReporter, PeerHealthSnapshot, ResourceSnapshot, ServiceHealth};

/// Uniform compute report returned by every service's `/status` endpoint.
///
/// The operator elohim agent deserializes this one type to reason about
/// the entire fleet. Service-specific data goes in `extensions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeReport {
    /// Service identifier (e.g., "doorway", "steward", "storage")
    pub service_id: String,
    /// Service version (caller fills from env!("CARGO_PKG_VERSION"))
    pub version: String,
    /// Current health state
    pub health: ServiceHealth,
    /// Human-readable reason for current state
    pub health_reason: String,
    /// When the service started
    pub started_at: DateTime<Utc>,
    /// Seconds since service started
    pub uptime_seconds: u64,
    /// Point-in-time resource usage
    pub resources: ResourceSnapshot,
    /// Per-peer/subscriber health
    pub peers: Vec<PeerHealthSnapshot>,
    /// Service-specific extensions (typed on producer, opaque to fleet consumer)
    pub extensions: serde_json::Value,
}

impl ComputeReport {
    /// Build a report from a HealthReporter and pre-assembled data.
    ///
    /// The `version` field is left empty — caller should set it from
    /// `env!("CARGO_PKG_VERSION")` which can only be read from the
    /// consuming crate, not from a library.
    pub fn build(
        reporter: &dyn HealthReporter,
        resources: ResourceSnapshot,
        peers: Vec<PeerHealthSnapshot>,
        extensions: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        let started = reporter.started_at();
        let uptime = (now - started).num_seconds().max(0) as u64;

        Self {
            service_id: reporter.service_id().to_string(),
            version: String::new(),
            health: reporter.health(),
            health_reason: reporter.health_reason(),
            started_at: started,
            uptime_seconds: uptime,
            resources,
            peers,
            extensions,
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-compute && cargo test report`
Expected: All 3 tests PASS

**Step 5: Run full crate test suite**

Run: `cd elohim/elohim-compute && cargo test`
Expected: All 21 tests PASS (4 health + 4 counters + 8 peers + 2 resources + 3 report)

**Step 6: Commit**

```
feat(compute): add ComputeReport envelope with builder

Top-level struct every service returns from /status. Composed
from HealthReporter trait, ResourceSnapshot, peer health, and
service-specific extensions. Full crate test suite: 21 tests.
```

---

### Task 7: Add elohim-compute dependency to doorway

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`

**Step 1: Add dependency**

Add to `[dependencies]` section (after the `doorway-client` line at line 107):

```toml
# Shared compute reporting types
elohim-compute = { path = "../../elohim/elohim-compute" }
```

**Step 2: Verify doorway builds with the new dependency**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Clean check (no compile errors)

**Step 3: Commit**

```
chore(doorway): add elohim-compute dependency
```

---

### Task 8: Add PeerHealthRegistry and RequestCounters to AppState

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs:43-110` (AppState struct)
- Modify: `doorway/doorway-service/src/server/http.rs:112-180` (new() constructor)
- Modify: `doorway/doorway-service/src/server/http.rs:187-255` (with_services() constructor)
- Modify: `doorway/doorway-service/src/server/http.rs:264-344` (with_pool() constructor)
- Modify: `doorway/doorway-service/src/server/http.rs:350-427` (with_projection() constructor)

**Step 1: Add fields to AppState**

In `AppState` struct, after the `journal_inference_available` field (line 109), add:

```rust
    /// Per-peer projection subscriber health (shared crate)
    pub peer_health: Arc<elohim_compute::PeerHealthRegistry>,
    /// Request throughput counters for /api/v1/cache/ (shared crate)
    pub request_counters: Arc<elohim_compute::RequestCounters>,
    /// Service boot time (for uptime in ComputeReport)
    pub started_at: chrono::DateTime<chrono::Utc>,
```

**Step 2: Add initialization to all four constructors**

In each of `new()`, `with_services()`, `with_pool()`, and `with_projection()`, add these three lines to the `Self { ... }` block:

```rust
            peer_health: Arc::new(elohim_compute::PeerHealthRegistry::new()),
            request_counters: Arc::new(elohim_compute::RequestCounters::new()),
            started_at: chrono::Utc::now(),
```

There are four constructors to update:
- `new()` — around line 147-180
- `with_services()` — around line 221-254
- `with_pool()` — around line 310-343
- `with_projection()` — around line 393-426

**Step 3: Verify build**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Clean check

**Step 4: Run existing tests to verify no regressions**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins`
Expected: All existing tests pass (test suite should be 340+ tests)

**Step 5: Commit**

```
feat(doorway): add PeerHealthRegistry and RequestCounters to AppState

Initializes shared compute reporting structs in all four AppState
constructors. No behavioral changes — wiring comes next.
```

---

### Task 9: Wire PeerHealthRegistry into subscriber forwarding

**Files:**
- Modify: `doorway/doorway-service/src/main.rs:505-547` (subscriber forwarding loop)

**Step 1: Add registry wiring to the subscriber loop**

In `main.rs`, the subscriber loop starts at line 505. Before the `for` loop (after line 504), clone the registry:

```rust
            let peer_health = Arc::clone(&state.peer_health);
```

Inside the `for` loop, after `spawn_subscriber` (line 515) and before the forwarding task, register the peer:

```rust
                peer_health.register(&conductor_id, conductor_app_url);
```

Replace the forwarding task (lines 521-538) with:

```rust
                let peer_health_clone = Arc::clone(&peer_health);
                let conductor_id_clone = conductor_id.clone();
                tokio::spawn(async move {
                    peer_health_clone.update_health(
                        &conductor_id_clone,
                        elohim_compute::ServiceHealth::Healthy,
                        "connected",
                    );
                    loop {
                        match sub_rx.recv().await {
                            Ok(signal) => {
                                peer_health_clone.record_signal(&conductor_id_clone);
                                if fwd_tx.send(signal).is_err() {
                                    break; // engine dropped
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                peer_health_clone.update_health(
                                    &conductor_id_clone,
                                    elohim_compute::ServiceHealth::Offline,
                                    "channel closed",
                                );
                                break;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                peer_health_clone.update_health(
                                    &conductor_id_clone,
                                    elohim_compute::ServiceHealth::Degraded,
                                    &format!("lagged {n} signals"),
                                );
                                warn!(
                                    conductor = %conductor_id_clone,
                                    lagged = n,
                                    "Signal forwarder lagged"
                                );
                            }
                        }
                    }
                });
```

Note: `conductor_id` is already defined at line 520 as `format!("conductor-{i}")`. The `state` variable is `Arc<AppState>` at this point (created around line 416).

**Step 2: Verify build**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release`
Expected: Clean build

**Step 3: Commit**

```
feat(doorway): wire PeerHealthRegistry into subscriber forwarding

Registers each conductor peer on spawn, updates health state on
connect/signal/lag/close. Uses typed ServiceHealth enum instead
of string-based state tracking.
```

---

### Task 10: Increment request counters in cache API handler

**Files:**
- Modify: `doorway/doorway-service/src/routes/api.rs:199-209`

**Step 1: Add counter increment after route parsing**

In `handle_api_request()`, after `CacheRoute::parse(path)` succeeds and before the `parse_requester_identity` call (between lines 208 and 210), add:

```rust
    // Track request for resource accounting
    state.request_counters.increment(route.doc_type);
```

**Step 2: Verify build**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release`
Expected: Clean build

**Step 3: Run existing tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins`
Expected: All tests pass

**Step 4: Commit**

```
feat(doorway): increment request counters in cache API handler

One-line increment per request, keyed by doc_type. Zero allocation
on hot path (atomic add into existing DashMap entry).
```

---

### Task 11: Add ComputeReport to StatusResponse

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs`

**Step 1: Add DoorwayHealthReporter struct**

After the `Diagnostics` struct (around line 164), add:

```rust
/// Doorway's implementation of the shared HealthReporter trait.
struct DoorwayHealthReporter<'a> {
    state: &'a AppState,
    conductor_connected: bool,
    storage_reachable: bool,
}

impl elohim_compute::HealthReporter for DoorwayHealthReporter<'_> {
    fn service_id(&self) -> &str {
        "doorway"
    }

    fn health(&self) -> elohim_compute::ServiceHealth {
        if self.conductor_connected && self.storage_reachable {
            elohim_compute::ServiceHealth::Healthy
        } else if self.state.args.dev_mode {
            elohim_compute::ServiceHealth::Degraded
        } else if !self.conductor_connected && !self.storage_reachable {
            elohim_compute::ServiceHealth::Offline
        } else {
            elohim_compute::ServiceHealth::Degraded
        }
    }

    fn health_reason(&self) -> String {
        let conductor = if self.conductor_connected { "connected" } else { "disconnected" };
        let storage = if self.storage_reachable { "reachable" } else { "unreachable" };
        let subscribers = self.state.peer_health.active_count();
        format!(
            "conductor {}, storage {}, {} active subscriber(s)",
            conductor, storage, subscribers
        )
    }

    fn started_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.state.started_at
    }
}
```

**Step 2: Add compute field to StatusResponse**

In the `StatusResponse` struct (line 167), after the `diagnostics` field (line 196), add:

```rust
    /// Uniform compute report (shared fleet contract)
    pub compute: elohim_compute::ComputeReport,
```

**Step 3: Add fetch_projection_stats helper**

Before `build_status_data()` (around line 258), add:

```rust
use std::sync::OnceLock;
use tokio::sync::Mutex as TokioMutex;

/// Cached MongoDB projection stats (refreshed every 30s)
static PROJECTION_STATS_CACHE: OnceLock<TokioMutex<(std::time::Instant, u64, u64)>> =
    OnceLock::new();

async fn fetch_projection_stats(state: &Arc<AppState>) -> (u64, u64) {
    let cache = PROJECTION_STATS_CACHE.get_or_init(|| {
        TokioMutex::new((
            std::time::Instant::now() - std::time::Duration::from_secs(60),
            0,
            0,
        ))
    });

    let mut cached = cache.lock().await;
    if cached.0.elapsed().as_secs() < 30 {
        return (cached.1, cached.2);
    }

    let result = if let Some(ref mongo) = state.mongo {
        let db = mongo.inner().database(mongo.db_name());
        match db
            .run_command(bson::doc! { "collStats": "projected_entries" })
            .await
        {
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

Note: Check that `mongo.inner()` and `mongo.db_name()` are the correct accessor methods on the MongoClient wrapper. If the wrapper uses different method names, adjust accordingly.

**Step 4: Build compute report in build_status_data()**

In `build_status_data()`, after the `diagnostics` block and before the final `StatusResponse { ... }` construction (around line 486), add:

```rust
    // Build uniform compute report
    let (projection_bytes, projection_documents) = fetch_projection_stats(&state).await;
    let hot_cache_entries = state
        .projection
        .as_ref()
        .map(|p| p.hot_cache_stats().total_entries)
        .unwrap_or(0);

    let reporter = DoorwayHealthReporter {
        state: &state,
        conductor_connected,
        storage_reachable: storage.reachable,
    };
    let resources = elohim_compute::ResourceSnapshot {
        timestamp: chrono::Utc::now(),
        requests: state.request_counters.snapshot(),
        active_connections: state.peer_health.active_count(),
        managed_storage_bytes: projection_bytes,
        managed_document_count: projection_documents,
    };
    let peers = state.peer_health.snapshot();
    let extensions = serde_json::json!({
        "hotCacheEntries": hot_cache_entries,
        "cacheHitRate": cache_stats.hit_rate(),
    });
    let mut compute = elohim_compute::ComputeReport::build(&reporter, resources, peers, extensions);
    compute.version = env!("CARGO_PKG_VERSION").to_string();
```

Note: `cache_stats` is already computed earlier in the function (around line 351). `conductor_connected` is also already available (around line 266).

**Step 5: Add compute to the StatusResponse construction**

In the final `StatusResponse { ... }` block (around line 487-502), add after `diagnostics`:

```rust
        compute,
```

**Step 6: Update the test**

In the existing `test_status_serialization` test (line 901), add the `compute` field to the test `StatusResponse` construction:

```rust
            compute: elohim_compute::ComputeReport {
                service_id: "doorway".to_string(),
                version: "0.1.0".to_string(),
                health: elohim_compute::ServiceHealth::Healthy,
                health_reason: "all systems go".to_string(),
                started_at: chrono::Utc::now(),
                uptime_seconds: 60,
                resources: elohim_compute::ResourceSnapshot {
                    timestamp: chrono::Utc::now(),
                    requests: elohim_compute::RequestCounterSnapshot {
                        total: 0,
                        by_category: std::collections::HashMap::new(),
                    },
                    active_connections: 0,
                    managed_storage_bytes: 0,
                    managed_document_count: 0,
                },
                peers: vec![],
                extensions: serde_json::json!({}),
            },
```

And add assertions:

```rust
        assert!(json.contains("\"compute\""));
        assert!(json.contains("\"serviceId\""));
        assert!(json.contains("\"uptimeSeconds\""));
```

**Step 7: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release && RUSTFLAGS="" cargo test --lib --bins`
Expected: Clean build, all tests pass

**Step 8: Commit**

```
feat(doorway): add ComputeReport to /status endpoint

DoorwayHealthReporter implements shared HealthReporter trait.
StatusResponse gains compute field with peer health, request
counters, projection stats, and hot cache extensions. MongoDB
collStats cached 30s for managed storage metrics.
```

---

### Task 12: Update status.html template

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs:204-252` (StatusPageTemplate)
- Modify: `doorway/doorway-service/templates/status.html`

**Step 1: Add template view types**

In `routes/status.rs`, after the existing `PeerView` struct (around line 218), add:

```rust
/// A projection subscriber peer card for the status page
pub struct ProjectionPeerView {
    pub conductor_id: String,
    pub dot_color: String,
    pub health: String,
    pub reason: String,
    pub signals_received: String,
    pub last_signal: String,
    pub reconnect_attempts: u32,
}

/// Resource usage summary for the status page
pub struct ResourceUsageView {
    pub projection_storage: String,
    pub projection_documents: String,
    pub hot_cache_entries: String,
    pub total_requests: String,
    pub active_subscribers: String,
}
```

**Step 2: Add fields to StatusPageTemplate**

In `StatusPageTemplate` (line 234), after `attestation_log` (line 251), add:

```rust
    pub projection_peers: Vec<ProjectionPeerView>,
    pub resource_usage: ResourceUsageView,
```

**Step 3: Populate in status_page() handler**

In the `status_page()` function, after the existing `peers` construction and before the `StatusPageTemplate` construction (around line 641), add:

```rust
    // Build projection peer views from compute report
    let projection_peers: Vec<ProjectionPeerView> = data
        .compute
        .peers
        .iter()
        .map(|p| {
            let dot_color = match p.health {
                elohim_compute::ServiceHealth::Healthy => "green",
                elohim_compute::ServiceHealth::Degraded => "yellow",
                elohim_compute::ServiceHealth::Offline => "red",
            };
            ProjectionPeerView {
                conductor_id: p.peer_id.clone(),
                dot_color: dot_color.to_string(),
                health: p.health.to_string(),
                reason: p.reason.clone(),
                signals_received: format_number(p.signals_received),
                last_signal: p
                    .last_signal_at
                    .map(|t| {
                        let secs = (chrono::Utc::now() - t).num_seconds().max(0);
                        if secs < 60 {
                            format!("{}s ago", secs)
                        } else if secs < 3600 {
                            format!("{}m ago", secs / 60)
                        } else {
                            format!("{}h ago", secs / 3600)
                        }
                    })
                    .unwrap_or_else(|| "never".to_string()),
                reconnect_attempts: p.reconnect_attempts,
            }
        })
        .collect();

    let resource_usage = ResourceUsageView {
        projection_storage: format_bytes(data.compute.resources.managed_storage_bytes),
        projection_documents: format_number(data.compute.resources.managed_document_count),
        hot_cache_entries: format_number(
            data.compute.extensions["hotCacheEntries"]
                .as_u64()
                .unwrap_or(0),
        ),
        total_requests: format_number(data.compute.resources.requests.total),
        active_subscribers: format_number(data.compute.resources.active_connections as u64),
    };
```

And add `projection_peers` and `resource_usage` to the `StatusPageTemplate { ... }` construction.

**Step 4: Add helper formatting functions**

Before the `status_page()` function, add:

```rust
/// Format bytes as human-readable (KB/MB/GB)
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a number with comma separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
```

**Step 5: Add HTML sections to status.html template**

In `templates/status.html`, after the Components section (line 155) and before the Federation Peers section (line 157), add:

```html
    <!-- Projection Subscribers -->
    {% if !projection_peers.is_empty() %}
    <div class="section">
      <h2>Projection Subscribers</h2>
      <div class="peer-grid">
        {% for peer in projection_peers %}
        <div class="peer-card">
          <div class="peer-header">
            <span class="dot dot-{{ peer.dot_color }}"></span>
            <span class="peer-id">{{ peer.conductor_id }}</span>
          </div>
          <div class="peer-details">
            <span>{{ peer.health }}: {{ peer.reason }}</span>
            <span>Signals: {{ peer.signals_received }}</span>
            <span>Last: {{ peer.last_signal }}</span>
            {% if peer.reconnect_attempts > 0 %}
            <span>Reconnects: {{ peer.reconnect_attempts }}</span>
            {% endif %}
          </div>
        </div>
        {% endfor %}
      </div>
    </div>
    {% endif %}

    <!-- Resource Usage -->
    <div class="section">
      <h2>Resource Usage</h2>
      <div class="stats-row">
        <div class="stat-card">
          <div class="label">Projection Storage</div>
          <div class="value">{{ resource_usage.projection_storage }}</div>
        </div>
        <div class="stat-card">
          <div class="label">Documents</div>
          <div class="value">{{ resource_usage.projection_documents }}</div>
        </div>
        <div class="stat-card">
          <div class="label">Hot Cache</div>
          <div class="value">{{ resource_usage.hot_cache_entries }}</div>
        </div>
        <div class="stat-card">
          <div class="label">Requests</div>
          <div class="value">{{ resource_usage.total_requests }}</div>
        </div>
        <div class="stat-card">
          <div class="label">Subscribers</div>
          <div class="value">{{ resource_usage.active_subscribers }}</div>
        </div>
      </div>
    </div>
```

**Step 6: Build (Askama validates templates at compile time)**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build --release`
Expected: Clean build

**Step 7: Commit**

```
feat(doorway): render projection peers and resource usage on /status page

New Projection Subscribers card section with health dots, signal
counts, and last-signal timestamps. New Resource Usage stats row
with projection storage, documents, hot cache, requests, and
active subscriber counts.
```

---

### Task 13: Final verification

**Step 1: Run full test suite**

Run: `cd elohim/elohim-compute && cargo test`
Expected: All 21 crate tests pass

**Step 2: Run doorway tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins`
Expected: All tests pass (340+ existing + any new)

**Step 3: Run clippy on both crates**

Run: `cd elohim/elohim-compute && cargo clippy -- -D warnings`
Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings`
Expected: Clean

**Step 4: Run fmt on both crates**

Run: `cd elohim/elohim-compute && cargo fmt --check`
Run: `cd doorway/doorway-service && cargo fmt --check`
Expected: Clean

**Step 5: Verify /status JSON shape**

Review `StatusResponse` struct and confirm:
- `compute: ComputeReport` present with all fields
- `compute.peers: Vec<PeerHealthSnapshot>` present
- `compute.resources: ResourceSnapshot` present
- All fields are `camelCase` via serde rename
- All existing fields unchanged (backward compat)

**Step 6: Commit any final fixes, then push**

Run: `git push`
Expected: Pre-push hook passes all gates
