# Elohim Compute Coordination Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the `elohim-agent` Rust crate into `elohim-node` with admission control, REA compute accounting, and capacity gossip — training-wheels mode where contract shapes are correct but mesh routing is deferred.

**Architecture:** Three composing layers integrated into elohim-node: (1) admission controller with priority queue and budget enforcement, (2) REA commitment/event recording per invocation, (3) gossipsub capacity announcements. The TypeScript sidecar remains as dev fallback. Doorway proxies to whichever is available.

**Tech Stack:** Rust (elohim-node, elohim-agent, axum, libp2p gossipsub, rmp-serde), TypeScript/Angular (NativeBackend, ElohimPresenceService), Vitest, cargo test

**Design doc:** `genesis/plans/2026-03-07-elohim-compute-coordination-design.md`

---

### Task 1: Add elohim-agent dependency to elohim-node

**Files:**
- Modify: `elohim-node/Cargo.toml`

**Context:** elohim-node currently has no dependency on the elohim-agent crate. The crate lives at `../elohim/elohim-agent` relative to elohim-node. elohim-node uses `RUSTFLAGS=""` for builds (not the WASM getrandom flag).

**Step 1: Add the dependency**

In `elohim-node/Cargo.toml`, add to `[dependencies]`:

```toml
elohim-agent = { path = "../elohim/elohim-agent", default-features = false }
constitution = { path = "../elohim/constitution" }
```

Note: Do NOT enable the `typescript` feature — that's for ts-rs type export, not needed at runtime.

**Step 2: Verify it compiles**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors. If there are version conflicts, resolve by aligning workspace deps.

**Step 3: Commit**

```bash
git add elohim-node/Cargo.toml
git commit -m "build(elohim-node): add elohim-agent and constitution dependencies"
```

---

### Task 2: Admission types — AdmissionDecision, DeferReason, MeshHint

**Files:**
- Create: `elohim-node/src/pod/admission.rs`
- Modify: `elohim-node/src/pod/mod.rs` (add `pub mod admission;`)
- Test: inline `#[cfg(test)] mod tests`

**Context:** These types are the contract shapes that must be right now. They'll be used by the admission controller and returned to clients. The `pod/` directory already has `models.rs` with `ComputeCapability`, `Observation`, `Action` types.

**Step 1: Write the failing test**

Create `elohim-node/src/pod/admission.rs`:

```rust
//! Admission control types for elohim compute coordination.
//!
//! These types define the contract between request admission and response.
//! Training-wheels: single-node admission with correct shapes for mesh routing.

use serde::{Deserialize, Serialize};

/// Result of admission evaluation for an incoming compute request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "camelCase")]
pub enum AdmissionDecision {
    /// Request accepted — will be queued and processed.
    Accepted {
        commitment_id: String,
        queue_position: u32,
        estimated_wait_ms: u64,
    },
    /// Request deferred — node is at capacity, try later or elsewhere.
    Deferred {
        reason: DeferReason,
        mesh_hints: Vec<MeshHint>,
        retry_after_ms: u64,
    },
    /// Request declined — cannot serve this request at all.
    Declined {
        reason: String,
    },
}

/// Why a request was deferred (not declined — it could be served later).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DeferReason {
    BudgetExhausted,
    QueueFull,
    CapabilityUnavailable,
    SystemPressure,
}

/// Hint about a neighbor node that might have capacity.
/// Training-wheels: empty vec, populated when gossip neighbor table is built.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshHint {
    pub node_id: String,
    pub budget_remaining: u32,
    pub estimated_wait_ms: u64,
    pub capabilities: Vec<String>,
}

/// Priority levels for queue ordering. Maps to ElohimRequest.priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admission_accepted_serializes() {
        let decision = AdmissionDecision::Accepted {
            commitment_id: "commit-123".into(),
            queue_position: 2,
            estimated_wait_ms: 5000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("accepted"));
        assert!(json.contains("commitmentId"));
    }

    #[test]
    fn test_admission_deferred_with_empty_hints() {
        let decision = AdmissionDecision::Deferred {
            reason: DeferReason::BudgetExhausted,
            mesh_hints: vec![],
            retry_after_ms: 10000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("deferred"));
        assert!(json.contains("meshHints"));
        assert!(json.contains("[]"));
    }

    #[test]
    fn test_admission_deferred_with_mesh_hints() {
        let decision = AdmissionDecision::Deferred {
            reason: DeferReason::QueueFull,
            mesh_hints: vec![MeshHint {
                node_id: "node-456".into(),
                budget_remaining: 50,
                estimated_wait_ms: 2000,
                capabilities: vec!["path-recommendation".into()],
            }],
            retry_after_ms: 5000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("node-456"));
        assert!(json.contains("path-recommendation"));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(RequestPriority::Urgent > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_defer_reason_equality() {
        assert_eq!(DeferReason::BudgetExhausted, DeferReason::BudgetExhausted);
        assert_ne!(DeferReason::QueueFull, DeferReason::SystemPressure);
    }
}
```

**Step 2: Add module declaration**

In `elohim-node/src/pod/mod.rs`, add:
```rust
pub mod admission;
```

**Step 3: Run tests to verify they pass**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test pod::admission --lib 2>&1 | tail -10`
Expected: `test result: ok. 5 passed`

**Step 4: Commit**

```bash
git add elohim-node/src/pod/admission.rs elohim-node/src/pod/mod.rs
git commit -m "feat(elohim-node): add admission control types (AdmissionDecision, MeshHint, DeferReason)"
```

---

### Task 3: REA compute types — ComputeCommitment and ComputeEvent

**Files:**
- Create: `elohim-node/src/pod/compute_rea.rs`
- Modify: `elohim-node/src/pod/mod.rs` (add `pub mod compute_rea;`)
- Test: inline `#[cfg(test)] mod tests`

**Context:** REA (Resource-Event-Agent) orthodoxy: one economic exchange = one event. Commitments represent intent (accepted request), events represent fact (fulfilled request). Cancelled commitments are tracked — they're part of the economic story.

**Step 1: Write types and tests**

Create `elohim-node/src/pod/compute_rea.rs`:

```rust
//! REA compute accounting types.
//!
//! Every inference invocation produces an immutable economic event.
//! Commitments are created on admission (intent); events on fulfillment (fact).
//! Training-wheels: local storage via AuditLog. Production: Holochain DHT.

use serde::{Deserialize, Serialize};

/// Status of a compute commitment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitmentStatus {
    /// Request accepted, waiting in queue.
    Pending,
    /// Request processed, ComputeEvent created.
    Fulfilled,
    /// Request cancelled (e.g., another node served it first).
    Cancelled,
}

/// REA Commitment — intent to serve a compute request.
/// Created when admission accepts a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeCommitment {
    pub id: String,
    pub request_id: String,
    pub node_id: String,
    pub requester_id: String,
    pub capability: String,
    pub estimated_tokens: u32,
    pub status: CommitmentStatus,
    pub created_at: String,
    pub fulfilled_at: Option<String>,
    pub cancelled_at: Option<String>,
}

impl ComputeCommitment {
    /// Create a new pending commitment.
    pub fn new(
        request_id: String,
        node_id: String,
        requester_id: String,
        capability: String,
        estimated_tokens: u32,
    ) -> Self {
        Self {
            id: format!("commit-{}", uuid::Uuid::new_v4()),
            request_id,
            node_id,
            requester_id,
            capability,
            estimated_tokens,
            status: CommitmentStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
            fulfilled_at: None,
            cancelled_at: None,
        }
    }

    /// Mark as fulfilled, returning the event that records what happened.
    pub fn fulfill(
        &mut self,
        tokens_used: u32,
        model: String,
        time_ms: u64,
    ) -> ComputeEvent {
        self.status = CommitmentStatus::Fulfilled;
        self.fulfilled_at = Some(chrono::Utc::now().to_rfc3339());

        ComputeEvent {
            id: format!("event-{}", uuid::Uuid::new_v4()),
            commitment_id: self.id.clone(),
            provider_id: self.node_id.clone(),
            receiver_id: self.requester_id.clone(),
            action: "use".into(),
            resource_type: "inference-tokens".into(),
            tokens_used,
            model,
            time_ms,
            capability: self.capability.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Mark as cancelled (another node served the request).
    pub fn cancel(&mut self) {
        self.status = CommitmentStatus::Cancelled;
        self.cancelled_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// REA Economic Event — immutable record of compute consumed.
/// One per invocation. Never aggregated into synthetic summary events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeEvent {
    pub id: String,
    pub commitment_id: String,
    pub provider_id: String,
    pub receiver_id: String,
    pub action: String,
    pub resource_type: String,
    pub tokens_used: u32,
    pub model: String,
    pub time_ms: u64,
    pub capability: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_lifecycle_fulfilled() {
        let mut commitment = ComputeCommitment::new(
            "req-1".into(),
            "node-A".into(),
            "user-1".into(),
            "path-recommendation".into(),
            1500,
        );
        assert_eq!(commitment.status, CommitmentStatus::Pending);
        assert!(commitment.fulfilled_at.is_none());

        let event = commitment.fulfill(1247, "claude-haiku-4-5".into(), 890);

        assert_eq!(commitment.status, CommitmentStatus::Fulfilled);
        assert!(commitment.fulfilled_at.is_some());
        assert_eq!(event.commitment_id, commitment.id);
        assert_eq!(event.tokens_used, 1247);
        assert_eq!(event.provider_id, "node-A");
        assert_eq!(event.receiver_id, "user-1");
        assert_eq!(event.action, "use");
        assert_eq!(event.resource_type, "inference-tokens");
    }

    #[test]
    fn test_commitment_lifecycle_cancelled() {
        let mut commitment = ComputeCommitment::new(
            "req-2".into(),
            "node-A".into(),
            "user-2".into(),
            "spiral-detection".into(),
            800,
        );
        commitment.cancel();

        assert_eq!(commitment.status, CommitmentStatus::Cancelled);
        assert!(commitment.cancelled_at.is_some());
        assert!(commitment.fulfilled_at.is_none());
    }

    #[test]
    fn test_event_links_commitment() {
        let mut commitment = ComputeCommitment::new(
            "req-3".into(),
            "node-B".into(),
            "user-3".into(),
            "content-safety-review".into(),
            2000,
        );
        let event = commitment.fulfill(1800, "claude-sonnet-4".into(), 1200);

        assert_eq!(event.commitment_id, commitment.id);
        assert_eq!(event.capability, "content-safety-review");
        assert_eq!(event.model, "claude-sonnet-4");
    }

    #[test]
    fn test_commitment_serialization() {
        let commitment = ComputeCommitment::new(
            "req-4".into(),
            "node-C".into(),
            "user-4".into(),
            "path-recommendation".into(),
            1000,
        );
        let json = serde_json::to_string(&commitment).unwrap();
        assert!(json.contains("requestId"));
        assert!(json.contains("nodeId"));
        assert!(json.contains("estimatedTokens"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_event_serialization() {
        let mut commitment = ComputeCommitment::new(
            "req-5".into(),
            "node-D".into(),
            "user-5".into(),
            "spiral-detection".into(),
            500,
        );
        let event = commitment.fulfill(480, "llama-70b".into(), 3000);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("commitmentId"));
        assert!(json.contains("inference-tokens"));
        assert!(json.contains("llama-70b"));
    }
}
```

**Step 2: Add module declaration**

In `elohim-node/src/pod/mod.rs`, add:
```rust
pub mod compute_rea;
```

**Step 3: Run tests**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test pod::compute_rea --lib 2>&1 | tail -10`
Expected: `test result: ok. 5 passed`

**Step 4: Commit**

```bash
git add elohim-node/src/pod/compute_rea.rs elohim-node/src/pod/mod.rs
git commit -m "feat(elohim-node): add REA compute commitment and event types"
```

---

### Task 4: Capacity announcement type for gossipsub

**Files:**
- Create: `elohim-node/src/pod/capacity.rs`
- Modify: `elohim-node/src/pod/mod.rs` (add `pub mod capacity;`)
- Test: inline `#[cfg(test)] mod tests`

**Context:** Nodes broadcast their compute capacity every 30 seconds via gossipsub topic `/elohim/compute/capacity/1.0.0`. The message uses MessagePack serialization (same framing as existing protocols: 4-byte BE length + MessagePack data). Training-wheels: broadcasts own capacity, no consumers yet.

**Step 1: Write types and tests**

Create `elohim-node/src/pod/capacity.rs`:

```rust
//! Capacity announcement types for gossipsub compute discovery.
//!
//! Nodes periodically broadcast their available compute capacity.
//! Training-wheels: broadcast only, no neighbor table consumption yet.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for compute capacity announcements.
pub const CAPACITY_TOPIC: &str = "/elohim/compute/capacity/1.0.0";

/// Broadcast interval in seconds.
pub const CAPACITY_BROADCAST_INTERVAL_SECS: u64 = 30;

/// A node's compute capacity announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityAnnouncement {
    pub node_id: String,
    pub timestamp: u64,
    pub budget_remaining: u32,
    pub active_requests: u32,
    pub queue_depth: u32,
    pub estimated_tokens_per_sec: f32,
    pub capabilities: Vec<String>,
    pub ready: bool,
}

impl CapacityAnnouncement {
    /// Encode to MessagePack bytes (4-byte BE length prefix + msgpack).
    pub fn encode(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        let msgpack = rmp_serde::encode::to_vec(self)?;
        let len = (msgpack.len() as u32).to_be_bytes();
        let mut buf = Vec::with_capacity(4 + msgpack.len());
        buf.extend_from_slice(&len);
        buf.extend_from_slice(&msgpack);
        Ok(buf)
    }

    /// Decode from MessagePack bytes (skip 4-byte BE length prefix).
    pub fn decode(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        if bytes.len() < 4 {
            return Err(rmp_serde::decode::Error::LengthMismatch(4));
        }
        rmp_serde::decode::from_slice(&bytes[4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_announcement() -> CapacityAnnouncement {
        CapacityAnnouncement {
            node_id: "node-123".into(),
            timestamp: 1709827200,
            budget_remaining: 42,
            active_requests: 2,
            queue_depth: 5,
            estimated_tokens_per_sec: 150.0,
            capabilities: vec![
                "path-recommendation".into(),
                "content-safety-review".into(),
            ],
            ready: true,
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = sample_announcement();
        let encoded = original.encode().unwrap();
        let decoded = CapacityAnnouncement::decode(&encoded).unwrap();

        assert_eq!(decoded.node_id, "node-123");
        assert_eq!(decoded.budget_remaining, 42);
        assert_eq!(decoded.active_requests, 2);
        assert_eq!(decoded.capabilities.len(), 2);
        assert!(decoded.ready);
    }

    #[test]
    fn test_encode_has_length_prefix() {
        let announcement = sample_announcement();
        let encoded = announcement.encode().unwrap();

        // First 4 bytes are BE length of the msgpack payload
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(len as usize, encoded.len() - 4);
    }

    #[test]
    fn test_json_serialization() {
        let announcement = sample_announcement();
        let json = serde_json::to_string(&announcement).unwrap();
        assert!(json.contains("nodeId"));
        assert!(json.contains("budgetRemaining"));
        assert!(json.contains("estimatedTokensPerSec"));
    }

    #[test]
    fn test_topic_constant() {
        assert_eq!(CAPACITY_TOPIC, "/elohim/compute/capacity/1.0.0");
    }
}
```

**Step 2: Add module declaration and verify**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test pod::capacity --lib 2>&1 | tail -10`
Expected: `test result: ok. 4 passed`

**Step 3: Commit**

```bash
git add elohim-node/src/pod/capacity.rs elohim-node/src/pod/mod.rs
git commit -m "feat(elohim-node): add CapacityAnnouncement type with MessagePack encoding"
```

---

### Task 5: AdmissionController with priority queue and budget

**Files:**
- Modify: `elohim-node/src/pod/admission.rs` (add controller implementation)
- Test: inline `#[cfg(test)] mod tests` (expand existing)

**Context:** The AdmissionController evaluates incoming requests and decides: accept (enqueue), defer (at capacity), or decline (can't serve). It owns an in-memory priority queue and a budget counter. Uses the types from Task 2.

**Step 1: Write failing tests first**

Add to the `tests` module in `admission.rs`:

```rust
#[test]
fn test_accept_within_budget() {
    let controller = AdmissionController::new(AdmissionConfig {
        budget_limit: 100,
        max_queue_depth: 10,
        max_concurrent: 5,
        capabilities: vec!["path-recommendation".into()],
        node_id: "node-test".into(),
    });
    let decision = controller.evaluate("req-1", "user-1", "path-recommendation", RequestPriority::Normal);
    assert!(matches!(decision, AdmissionDecision::Accepted { .. }));
}

#[test]
fn test_defer_budget_exhausted() {
    let controller = AdmissionController::new(AdmissionConfig {
        budget_limit: 0,
        max_queue_depth: 10,
        max_concurrent: 5,
        capabilities: vec!["path-recommendation".into()],
        node_id: "node-test".into(),
    });
    let decision = controller.evaluate("req-1", "user-1", "path-recommendation", RequestPriority::Normal);
    match decision {
        AdmissionDecision::Deferred { reason, mesh_hints, .. } => {
            assert_eq!(reason, DeferReason::BudgetExhausted);
            assert!(mesh_hints.is_empty()); // Training wheels: no neighbors
        }
        other => panic!("Expected Deferred, got {:?}", other),
    }
}

#[test]
fn test_decline_unknown_capability() {
    let controller = AdmissionController::new(AdmissionConfig {
        budget_limit: 100,
        max_queue_depth: 10,
        max_concurrent: 5,
        capabilities: vec!["path-recommendation".into()],
        node_id: "node-test".into(),
    });
    let decision = controller.evaluate("req-1", "user-1", "unknown-capability", RequestPriority::Normal);
    assert!(matches!(decision, AdmissionDecision::Declined { .. }));
}

#[test]
fn test_defer_queue_full() {
    let controller = AdmissionController::new(AdmissionConfig {
        budget_limit: 100,
        max_queue_depth: 2,
        max_concurrent: 5,
        capabilities: vec!["path-recommendation".into()],
        node_id: "node-test".into(),
    });
    // Fill the queue
    controller.evaluate("req-1", "user-1", "path-recommendation", RequestPriority::Normal);
    controller.evaluate("req-2", "user-2", "path-recommendation", RequestPriority::Normal);
    // Third should be deferred
    let decision = controller.evaluate("req-3", "user-3", "path-recommendation", RequestPriority::Normal);
    match decision {
        AdmissionDecision::Deferred { reason, .. } => {
            assert_eq!(reason, DeferReason::QueueFull);
        }
        other => panic!("Expected Deferred, got {:?}", other),
    }
}

#[test]
fn test_budget_decrements_on_fulfill() {
    let controller = AdmissionController::new(AdmissionConfig {
        budget_limit: 10,
        max_queue_depth: 10,
        max_concurrent: 5,
        capabilities: vec!["path-recommendation".into()],
        node_id: "node-test".into(),
    });
    assert_eq!(controller.budget_remaining(), 10);
    controller.record_usage(3);
    assert_eq!(controller.budget_remaining(), 7);
}
```

**Step 2: Implement AdmissionController**

Add to `admission.rs` (above the tests module):

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// Configuration for the admission controller.
#[derive(Debug, Clone)]
pub struct AdmissionConfig {
    pub budget_limit: u32,
    pub max_queue_depth: u32,
    pub max_concurrent: u32,
    pub capabilities: Vec<String>,
    pub node_id: String,
}

/// Queued request entry.
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub request_id: String,
    pub requester_id: String,
    pub capability: String,
    pub priority: RequestPriority,
    pub commitment_id: String,
    pub enqueued_at: String,
}

/// Admission controller — evaluates, queues, and tracks compute requests.
///
/// Training-wheels: single-node, in-memory queue, static budget.
/// Production: mesh-aware routing, persistent queue, dynamic budget from governance.
pub struct AdmissionController {
    config: AdmissionConfig,
    budget_remaining: AtomicU32,
    queue: Mutex<Vec<QueuedRequest>>,
    active_count: AtomicU32,
}

impl AdmissionController {
    pub fn new(config: AdmissionConfig) -> Self {
        let budget = config.budget_limit;
        Self {
            config,
            budget_remaining: AtomicU32::new(budget),
            queue: Mutex::new(Vec::new()),
            active_count: AtomicU32::new(0),
        }
    }

    /// Evaluate whether to accept, defer, or decline a request.
    pub fn evaluate(
        &self,
        request_id: &str,
        requester_id: &str,
        capability: &str,
        priority: RequestPriority,
    ) -> AdmissionDecision {
        // 1. Capability check — decline if not registered
        if !self.config.capabilities.iter().any(|c| c == capability) {
            return AdmissionDecision::Declined {
                reason: format!("Capability '{}' not available on this node", capability),
            };
        }

        // 2. Budget check — defer if exhausted
        if self.budget_remaining.load(Ordering::Relaxed) == 0 {
            return AdmissionDecision::Deferred {
                reason: DeferReason::BudgetExhausted,
                mesh_hints: vec![], // Training wheels: no neighbors
                retry_after_ms: 30_000,
            };
        }

        // 3. Queue depth check — defer if full
        let mut queue = self.queue.lock().unwrap();
        if queue.len() as u32 >= self.config.max_queue_depth {
            return AdmissionDecision::Deferred {
                reason: DeferReason::QueueFull,
                mesh_hints: vec![],
                retry_after_ms: 10_000,
            };
        }

        // 4. Accept — create commitment and enqueue
        let commitment_id = format!("commit-{}", uuid::Uuid::new_v4());
        let position = queue.len() as u32;

        queue.push(QueuedRequest {
            request_id: request_id.into(),
            requester_id: requester_id.into(),
            capability: capability.into(),
            priority,
            commitment_id: commitment_id.clone(),
            enqueued_at: chrono::Utc::now().to_rfc3339(),
        });

        // Sort by priority (urgent first)
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        AdmissionDecision::Accepted {
            commitment_id,
            queue_position: position,
            estimated_wait_ms: (position as u64 + 1) * 2000, // ~2s per queued request
        }
    }

    /// Record token usage (called after fulfillment).
    pub fn record_usage(&self, tokens: u32) {
        self.budget_remaining.fetch_sub(
            tokens.min(self.budget_remaining.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
    }

    /// Dequeue the highest-priority request.
    pub fn dequeue(&self) -> Option<QueuedRequest> {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0)) // Already sorted by priority
        }
    }

    /// Remove a specific request (e.g., cancelled by mesh race).
    pub fn cancel(&self, request_id: &str) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let before = queue.len();
        queue.retain(|r| r.request_id != request_id);
        queue.len() < before
    }

    /// Current budget remaining.
    pub fn budget_remaining(&self) -> u32 {
        self.budget_remaining.load(Ordering::Relaxed)
    }

    /// Current queue depth.
    pub fn queue_depth(&self) -> u32 {
        self.queue.lock().unwrap().len() as u32
    }

    /// Active request count.
    pub fn active_requests(&self) -> u32 {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Mark a request as actively processing.
    pub fn mark_active(&self) {
        self.active_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Mark a request as complete.
    pub fn mark_complete(&self) {
        self.active_count.fetch_sub(1, Ordering::Relaxed);
    }
}
```

**Step 3: Run tests**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test pod::admission --lib 2>&1 | tail -10`
Expected: `test result: ok. 10 passed` (5 from Task 2 + 5 new)

**Step 4: Commit**

```bash
git add elohim-node/src/pod/admission.rs
git commit -m "feat(elohim-node): implement AdmissionController with priority queue and budget"
```

---

### Task 6: Wire ElohimAgentService into elohim-node startup

**Files:**
- Create: `elohim-node/src/elohim_service.rs` (module for agent service init + HTTP handler)
- Modify: `elohim-node/src/main.rs` (add module, init service, add route)
- Test: inline `#[cfg(test)] mod tests`

**Context:** The `ElohimAgentService` from the `elohim-agent` crate provides the invoke() pipeline. We initialize it at startup with an AnthropicBackend (API key from env), register capabilities, and expose an HTTP endpoint. The doorway currently proxies `/api/v1/elohim/invoke` to `state.args.elohim_agent_url` (the sidecar at :8095). We'll add a new endpoint on elohim-node (e.g., :8091 or on the existing storage HTTP port) that doorway can route to instead.

**Important:** Check how `elohim-agent/src/service.rs` constructs the service and what types it expects. The AnthropicBackend needs an API key from `ANTHROPIC_API_KEY` env var.

**Step 1: Create the service module**

Create `elohim-node/src/elohim_service.rs`:

```rust
//! Elohim Agent Service integration — wires the elohim-agent crate into elohim-node.
//!
//! Initializes LLM backends, capability registry, and constitutional stack.
//! Exposes HTTP handler for doorway to proxy to.

use std::sync::Arc;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use elohim_agent::backend::AnthropicBackend;
use elohim_agent::capability::ElohimCapability;
use elohim_agent::service::{ElohimAgentService, ServiceConfig};
use constitution::stack::StackContext;

use crate::pod::admission::{AdmissionController, AdmissionConfig, AdmissionDecision, RequestPriority};
use crate::pod::compute_rea::ComputeCommitment;

/// Shared state for the elohim agent HTTP handler.
pub struct ElohimNodeState {
    pub agent_service: ElohimAgentService,
    pub admission: AdmissionController,
    pub node_id: String,
}

/// Request body from doorway proxy.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeRequest {
    pub request_id: String,
    pub capability: String,
    pub params: serde_json::Value,
    pub requester_id: Option<String>,
    pub priority: Option<String>,
}

/// Response envelope — wraps either the agent response or an admission decision.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeResponse {
    pub request_id: String,
    #[serde(flatten)]
    pub result: InvokeResult,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum InvokeResult {
    Fulfilled {
        #[serde(flatten)]
        response: serde_json::Value,
    },
    Deferred {
        defer_reason: String,
        mesh_hints: Vec<serde_json::Value>,
        retry_after_ms: u64,
    },
    Declined {
        decline_reason: String,
    },
}

/// Initialize the ElohimAgentService with configured backends.
pub async fn initialize_agent_service(
    node_id: &str,
    api_key: Option<String>,
) -> Result<ElohimAgentService, String> {
    let mut backends: Vec<Arc<dyn elohim_agent::backend::LlmBackend>> = Vec::new();

    // Add Anthropic backend if API key is available
    if let Some(key) = api_key {
        info!("Initializing Anthropic backend for elohim-agent");
        backends.push(Arc::new(AnthropicBackend::claude_haiku(key)));
    }

    // Always add mock backend as fallback
    backends.push(Arc::new(elohim_agent::backend::MockBackend::new()));

    let service = ElohimAgentService::new(backends)
        .with_config(ServiceConfig {
            agent_id: node_id.to_string(),
            default_timeout_ms: 30_000,
            max_concurrent: 10,
            audit_enabled: true,
        });

    // Initialize constitutional stack (Global layer defaults)
    service
        .initialize(StackContext::agent_only(node_id))
        .await
        .map_err(|e| format!("Failed to initialize constitutional stack: {e}"))?;

    // Register supported capabilities
    service.register_capabilities(vec![
        ElohimCapability::PathRecommendation,
        ElohimCapability::ContentSafetyReview,
        ElohimCapability::SpiralDetection,
        ElohimCapability::AttestationRecommendation,
    ]).await;

    info!(node_id, "ElohimAgentService initialized with {} backends",
        if api_key.is_some() { 2 } else { 1 });

    Ok(service)
}

// NOTE: The actual axum handler and route registration will be wired in
// after verifying the service initializes correctly. This is the foundation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_request_deserializes() {
        let json = r#"{
            "requestId": "req-123",
            "capability": "path-recommendation",
            "params": {"pathId": "know-thyself"},
            "requesterId": "user-1",
            "priority": "normal"
        }"#;
        let req: InvokeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_id, "req-123");
        assert_eq!(req.capability, "path-recommendation");
    }

    #[test]
    fn test_invoke_response_fulfilled_serializes() {
        let resp = InvokeResponse {
            request_id: "req-123".into(),
            result: InvokeResult::Fulfilled {
                response: serde_json::json!({"payload": "test"}),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("fulfilled"));
        assert!(json.contains("requestId"));
    }

    #[test]
    fn test_invoke_response_deferred_serializes() {
        let resp = InvokeResponse {
            request_id: "req-456".into(),
            result: InvokeResult::Deferred {
                defer_reason: "budgetExhausted".into(),
                mesh_hints: vec![],
                retry_after_ms: 10000,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("deferred"));
        assert!(json.contains("retryAfterMs"));
    }
}
```

**Step 2: Add module declaration to main.rs**

At the top of `elohim-node/src/main.rs`, add:
```rust
mod elohim_service;
```

**Step 3: Run tests**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test elohim_service --lib 2>&1 | tail -10`
Expected: `test result: ok. 3 passed`

Note: If there are import errors because `ElohimAgentService::new()`, `AnthropicBackend::claude_haiku()`, etc. have different signatures than documented, adjust the code to match the actual API. Read the source files to verify before writing:
- `elohim/elohim-agent/src/service.rs` lines 93-136
- `elohim/elohim-agent/src/backend/anthropic.rs` lines 29-68

**Step 4: Commit**

```bash
git add elohim-node/src/elohim_service.rs elohim-node/src/main.rs
git commit -m "feat(elohim-node): wire ElohimAgentService initialization with admission control"
```

---

### Task 7: HTTP invoke endpoint on elohim-node

**Files:**
- Modify: `elohim-node/src/elohim_service.rs` (add axum handler)
- Modify: `elohim-node/src/main.rs` (add route to existing router)

**Context:** Add `POST /elohim/invoke` to elohim-node's axum router. This endpoint:
1. Deserializes `InvokeRequest`
2. Calls `AdmissionController.evaluate()`
3. If accepted: calls `ElohimAgentService.invoke()`, records REA event
4. If deferred/declined: returns the admission decision

The doorway will need to be updated (Task 9) to route here instead of/in addition to the sidecar.

**Step 1: Implement the handler**

Add to `elohim_service.rs`:

```rust
/// Axum handler for POST /elohim/invoke
pub async fn handle_invoke(
    State(state): State<Arc<ElohimNodeState>>,
    Json(request): Json<InvokeRequest>,
) -> (StatusCode, Json<InvokeResponse>) {
    let requester = request.requester_id.as_deref().unwrap_or("anonymous");
    let priority = match request.priority.as_deref() {
        Some("urgent") => RequestPriority::Urgent,
        Some("high") => RequestPriority::High,
        Some("low") => RequestPriority::Low,
        _ => RequestPriority::Normal,
    };

    // 1. Admission check
    let decision = state.admission.evaluate(
        &request.request_id,
        requester,
        &request.capability,
        priority,
    );

    match decision {
        AdmissionDecision::Accepted { commitment_id, .. } => {
            // 2. Create REA commitment
            let mut commitment = ComputeCommitment::new(
                request.request_id.clone(),
                state.node_id.clone(),
                requester.to_string(),
                request.capability.clone(),
                1500, // estimated tokens — TODO: per-capability estimation
            );

            state.admission.mark_active();

            // 3. Invoke the agent service
            let agent_request = elohim_agent::request::ElohimRequest::new(
                request.capability.clone(),
                request.params.clone(),
            )
            .with_requester(requester);

            let result = state.agent_service.invoke(agent_request).await;
            state.admission.mark_complete();

            match result {
                Ok(response) => {
                    // 4. Fulfill commitment, create REA event
                    let tokens = response.cost.as_ref()
                        .map(|c| c.tokens_processed)
                        .unwrap_or(0);
                    let time_ms = response.cost.as_ref()
                        .map(|c| c.time_ms)
                        .unwrap_or(0);
                    let model = "anthropic".to_string(); // TODO: from backend

                    let _event = commitment.fulfill(tokens, model, time_ms);
                    state.admission.record_usage(tokens.min(1)); // At least 1 budget unit

                    let response_json = serde_json::to_value(&response)
                        .unwrap_or_default();

                    (StatusCode::OK, Json(InvokeResponse {
                        request_id: request.request_id,
                        result: InvokeResult::Fulfilled { response: response_json },
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "ElohimAgentService invoke failed");
                    commitment.cancel();

                    (StatusCode::INTERNAL_SERVER_ERROR, Json(InvokeResponse {
                        request_id: request.request_id,
                        result: InvokeResult::Declined {
                            decline_reason: "Internal processing error".into(),
                        },
                    }))
                }
            }
        }
        AdmissionDecision::Deferred { reason, mesh_hints, retry_after_ms } => {
            let hints: Vec<serde_json::Value> = mesh_hints
                .iter()
                .map(|h| serde_json::to_value(h).unwrap_or_default())
                .collect();

            (StatusCode::OK, Json(InvokeResponse {
                request_id: request.request_id,
                result: InvokeResult::Deferred {
                    defer_reason: format!("{:?}", reason),
                    mesh_hints: hints,
                    retry_after_ms,
                },
            }))
        }
        AdmissionDecision::Declined { reason } => {
            (StatusCode::OK, Json(InvokeResponse {
                request_id: request.request_id,
                result: InvokeResult::Declined {
                    decline_reason: reason,
                },
            }))
        }
    }
}

/// Health endpoint for the elohim agent.
pub async fn handle_health(
    State(state): State<Arc<ElohimNodeState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "budgetRemaining": state.admission.budget_remaining(),
        "activeRequests": state.admission.active_requests(),
        "queueDepth": state.admission.queue_depth(),
    }))
}
```

**Step 2: Wire into main.rs router**

In `main.rs`, after initializing the storage HTTP server but before the dashboard, add:

```rust
// Initialize elohim agent service
let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
let agent_service = elohim_service::initialize_agent_service(&node_id, api_key)
    .await
    .unwrap_or_else(|e| {
        warn!("ElohimAgentService init failed: {e}, running without inference");
        // Fallback: mock-only service
        // ... handle gracefully
    });

let admission = AdmissionController::new(AdmissionConfig {
    budget_limit: std::env::var("ELOHIM_BUDGET_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100),
    max_queue_depth: 50,
    max_concurrent: 10,
    capabilities: vec![
        "path-recommendation".into(),
        "content-safety-review".into(),
        "spiral-detection".into(),
        "attestation-recommendation".into(),
    ],
    node_id: node_id.clone(),
});

let elohim_state = Arc::new(ElohimNodeState {
    agent_service,
    admission,
    node_id: node_id.clone(),
});

// Add elohim routes to existing router
let elohim_router = axum::Router::new()
    .route("/elohim/invoke", axum::routing::post(elohim_service::handle_invoke))
    .route("/elohim/health", axum::routing::get(elohim_service::handle_health))
    .with_state(elohim_state);
```

**Step 3: Run tests and verify build**

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors

Run: `cd /projects/elohim/elohim-node && RUSTFLAGS="" cargo test --lib 2>&1 | tail -5`
Expected: All tests pass

**Step 4: Commit**

```bash
git add elohim-node/src/elohim_service.rs elohim-node/src/main.rs
git commit -m "feat(elohim-node): add HTTP invoke endpoint with admission control and REA accounting"
```

---

### Task 8: Gossipsub capacity broadcast

**Files:**
- Modify: `elohim-node/src/pod/capacity.rs` (add broadcast function)
- Modify: P2P swarm setup (add topic subscription)

**Context:** Subscribe to the `/elohim/compute/capacity/1.0.0` gossipsub topic and broadcast the node's capacity every 30 seconds. Training-wheels: broadcast only, no neighbor table consumption.

**Important:** Before implementing, read the actual P2P swarm setup in `elohim-node/src/p2p/` to understand how gossipsub topics are currently registered and how messages are published. Adapt the pattern — don't invent a new one.

**Step 1: Add broadcast function to capacity.rs**

```rust
/// Build a capacity announcement from current node state.
pub fn build_announcement(
    node_id: &str,
    budget_remaining: u32,
    active_requests: u32,
    queue_depth: u32,
    capabilities: &[String],
    ready: bool,
) -> CapacityAnnouncement {
    CapacityAnnouncement {
        node_id: node_id.into(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        budget_remaining,
        active_requests,
        queue_depth,
        estimated_tokens_per_sec: 0.0, // TODO: measure actual throughput
        capabilities: capabilities.to_vec(),
        ready,
    }
}
```

**Step 2: Wire into P2P swarm**

Read `elohim-node/src/p2p/` to find:
1. Where gossipsub topics are subscribed
2. How `swarm.behaviour_mut().gossipsub.publish()` is called
3. Where periodic tasks are spawned

Add the capacity topic subscription alongside existing topics. Spawn a 30-second interval task that:
1. Builds a `CapacityAnnouncement` from `AdmissionController` state
2. Encodes via `.encode()` (MessagePack with length prefix)
3. Publishes to the capacity topic

**Step 3: Test the announcement builder**

Add to capacity.rs tests:
```rust
#[test]
fn test_build_announcement() {
    let ann = build_announcement(
        "node-test",
        42,
        2,
        5,
        &["path-recommendation".into()],
        true,
    );
    assert_eq!(ann.node_id, "node-test");
    assert_eq!(ann.budget_remaining, 42);
    assert!(ann.timestamp > 0);
}
```

**Step 4: Commit**

```bash
git add elohim-node/src/pod/capacity.rs elohim-node/src/p2p/
git commit -m "feat(elohim-node): broadcast compute capacity via gossipsub"
```

---

### Task 9: Update doorway to route to elohim-node

**Files:**
- Modify: `doorway/src/routes/elohim_agent.rs` (add elohim-node URL fallback)
- Modify: `doorway/src/config.rs` (add elohim-node URL config)

**Context:** Doorway currently proxies `/api/v1/elohim/*` to `state.args.elohim_agent_url` (the TypeScript sidecar at :8095). We need it to also try the elohim-node endpoint. Strategy: try elohim-node first, fall back to sidecar. This lets both coexist during the transition.

**Step 1: Add config for elohim-node URL**

In `doorway/src/config.rs` (the Args struct), add:

```rust
/// URL of the elohim-node compute endpoint (e.g., http://localhost:8091)
#[arg(long, env = "ELOHIM_NODE_URL", default_value = "")]
pub elohim_node_url: String,
```

**Step 2: Update forward_to_agent to try node first**

In `elohim_agent.rs`, modify `handle_elohim_agent_request` to try elohim-node first:

```rust
// Try elohim-node first (if configured), fall back to sidecar
let agent_url = if !state.args.elohim_node_url.is_empty() {
    // Probe node health first
    let node_health = format!("{}/elohim/health", state.args.elohim_node_url.trim_end_matches('/'));
    match reqwest::get(&node_health).await {
        Ok(resp) if resp.status().is_success() => state.args.elohim_node_url.clone(),
        _ => state.args.elohim_agent_url.clone(), // Fall back to sidecar
    }
} else {
    state.args.elohim_agent_url.clone()
};
```

**Step 3: Test both paths**

Add tests verifying:
- When `elohim_node_url` is empty, uses sidecar URL
- URL construction is correct for both endpoints

**Step 4: Commit**

```bash
git add doorway/src/routes/elohim_agent.rs doorway/src/config.rs
git commit -m "feat(doorway): route elohim requests to node with sidecar fallback"
```

---

### Task 10: Update Angular NativeBackend for deferred responses

**Files:**
- Modify: `elohim-app/src/app/elohim/services/backends/native-backend.ts`
- Modify: `elohim-app/src/app/elohim/models/elohim-agent.model.ts` (add deferred status handling)
- Test: create `elohim-app/src/app/elohim/services/backends/native-backend.spec.ts`

**Context:** The `NativeBackend` currently only handles `fulfilled` and error responses. With admission control, the backend also needs to handle `deferred` responses — where the node says "I've accepted your request but I'm busy, try again in X ms" or "I'm at capacity, here are neighbors."

**Step 1: Write failing tests**

Create `native-backend.spec.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { firstValueFrom } from 'rxjs';
import { NativeBackend } from './native-backend';

describe('NativeBackend', () => {
  let backend: NativeBackend;

  beforeEach(() => {
    backend = new NativeBackend(() => 'mock-jwt');
  });

  it('should handle deferred response', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        requestId: 'req-1',
        status: 'deferred',
        deferReason: 'BudgetExhausted',
        meshHints: [],
        retryAfterMs: 10000,
      }),
    } as Response);

    const response = await firstValueFrom(
      backend.invoke(
        { requestId: 'req-1', capability: 'path-recommendation', params: {}, priority: 'normal' } as any,
        { id: 'elohim-1' } as any,
      ),
    );

    expect(response.status).toBe('deferred');
  });

  it('should include auth header when token available', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({ requestId: 'req-1', status: 'fulfilled' }),
    } as Response);

    await firstValueFrom(
      backend.invoke(
        { requestId: 'req-1', capability: 'test', params: {}, priority: 'normal' } as any,
        { id: 'elohim-1' } as any,
      ),
    );

    const [, options] = fetchSpy.mock.calls[0];
    expect((options as any).headers.Authorization).toBe('Bearer mock-jwt');
  });
});
```

**Step 2: Run tests to verify they fail, then make them pass**

The `deferred` status should be surfaced through the response. Update `callDoorway()` in `native-backend.ts` to pass through `status: 'deferred'` responses rather than treating them as errors.

**Step 3: Commit**

```bash
git add elohim-app/src/app/elohim/services/backends/native-backend.ts \
       elohim-app/src/app/elohim/services/backends/native-backend.spec.ts
git commit -m "feat(elohim-app): handle deferred responses in NativeBackend"
```

---

### Task 11: A2O scenarios for compute coordination

**Files:**
- Create: `genesis/a2o/features/elohim/compute-coordination.feature`
- Modify: `genesis/a2o/steps/ui/elohim-presence.steps.ts` (add new step definitions)

**Context:** BDD scenarios verifying the compute coordination flow end-to-end. These are aspirational — they describe the target behavior. Steps may return 'pending' in HTTP mode.

**Step 1: Write the feature file**

```gherkin
@e2e @elohim @compute
Feature: Elohim Compute Coordination

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @browser-only
  Scenario: Authenticated learner receives insight within budget
    Given human "Timothy" is logged in on doorway "alpha" with device
    And the elohim node has inference budget remaining
    When the learner completes the assessment
    Then an elohim insight section appears below the results
    And the insight shows tokens processed
    And the insight shows processing time in milliseconds

  @browser-only @wip
  Scenario: Request deferred when budget exhausted
    Given human "Timothy" is logged in on doorway "alpha" with device
    And the elohim node has exhausted its inference budget
    When the learner completes the assessment
    Then the insight shows a capacity unavailable message

  @browser-only @wip
  Scenario: Compute cost recorded as economic event
    Given human "Timothy" is logged in on doorway "alpha" with device
    When the learner receives an elohim insight
    Then a compute economic event is recorded
    And the event captures the tokens used and model
```

**Step 2: Add step skeletons**

Add step definitions that return 'pending' for unimplemented steps:

```typescript
Given('the elohim node has inference budget remaining', async function (this: E2EWorld) {
  // Training wheels: assume budget is available in dev mode
  return 'pending';
});

Given('the elohim node has exhausted its inference budget', async function (this: E2EWorld) {
  return 'pending';
});

Then('the insight shows a capacity unavailable message', async function (this: E2EWorld) {
  return 'pending';
});

Then('a compute economic event is recorded', async function (this: E2EWorld) {
  return 'pending';
});

Then('the event captures the tokens used and model', async function (this: E2EWorld) {
  return 'pending';
});
```

**Step 3: Commit**

```bash
git add genesis/a2o/features/elohim/compute-coordination.feature \
       genesis/a2o/steps/ui/elohim-presence.steps.ts
git commit -m "feat(a2o): add compute coordination acceptance scenarios"
```

---

## Task Summary

| Task | Component | Layer | Description |
|------|-----------|-------|-------------|
| 1 | elohim-node | Foundation | Add elohim-agent dependency |
| 2 | elohim-node | Admission | AdmissionDecision, DeferReason, MeshHint types |
| 3 | elohim-node | REA | ComputeCommitment and ComputeEvent types |
| 4 | elohim-node | Gossip | CapacityAnnouncement with MessagePack encoding |
| 5 | elohim-node | Admission | AdmissionController with priority queue and budget |
| 6 | elohim-node | Backend | Wire ElohimAgentService initialization |
| 7 | elohim-node | Backend | HTTP invoke endpoint with full lifecycle |
| 8 | elohim-node | Gossip | Broadcast capacity via gossipsub |
| 9 | doorway | Routing | Route to elohim-node with sidecar fallback |
| 10 | elohim-app | Frontend | Handle deferred responses in NativeBackend |
| 11 | a2o | Testing | BDD scenarios for compute coordination |

**Dependencies:** Tasks 2-4 are independent (types only). Task 5 depends on 2. Task 6 depends on 1. Task 7 depends on 5+6. Task 8 depends on 4. Task 9 depends on 7. Task 10 depends on 7. Task 11 is independent.

**Parallelizable:** Tasks 2, 3, 4 can run in parallel. Tasks 8, 9, 10, 11 can run in parallel after their dependencies complete.
