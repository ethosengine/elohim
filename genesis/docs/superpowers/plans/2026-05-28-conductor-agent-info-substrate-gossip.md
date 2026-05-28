# Conductor Agent-Info Substrate Gossip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement step-zero substrate gossip that propagates Holochain `AgentInfoSigned` blobs across the libp2p mesh so every conductor's peer cache survives the Phase 1 doorway-A / doorway-B signal partition, closing the seeder's cross-mesh sync gap.

**Architecture:** One new module (`p2p/conductor_agent_info_gossip.rs`) with three internal tokio tasks (publisher heartbeat, subscriber edge in the existing swarm loop, subscriber worker with bounded mpsc + rate limiter). Wire payload is a thin envelope around a kitsune2 agent_info JSON string round-tripped via `admin_ws.agent_info(None)` → DualGossipPublisher → `admin_ws.add_agent_info(Vec<String>)`. Conductor handles signature verification + dedup on ingest. Behind feature flag `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` (default false).

**Tech Stack:** Rust (elohim-storage), `holochain_client = 0.9.0-dev.12`, `rmp-serde` (MessagePack), `tokio::sync::mpsc` (bounded channel), the existing `DualGossipPublisher` from `p2p_iroh/dual_publish/`, Gherkin/Cucumber for a2o scenarios.

**Spec:** `genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md` (commit `1ed33f548`).

## Source of Truth Declaration (per P2P Design Gate)

This plan introduces **no new schema, no new DHT entry type, no new diesel table, no new HTTP route**. The only new data shape is `ConductorAgentInfo` — an operational (Category C) in-flight gossip envelope that wraps an opaque kitsune2 v2 `AgentInfoSigned` JSON string. The envelope is decoded, verified, injected into the conductor's existing peer cache via admin RPC, and dropped.

| Data shape | Category | Source of Truth |
|---|---|---|
| `ConductorAgentInfo` (gossip wire payload) | C — operational | Publishing peer's local conductor, via `admin_ws.agent_info(None)`. Substrate gossip is replication only; the envelope is never stored. |
| Receiver-side peer cache | (existing) | The embedded Holochain conductor's own internal store, written via `admin_ws.add_agent_info(Vec<String>)`. Owned and managed by Holochain. Not under this plan's authority. |

All three forbidden alternatives from the gate were explicitly ruled out in the spec and inherited here: (a) NO new DHT entry type for "peer manifest" — `AgentInfoSigned` already serves that role; (b) NO diesel projection table — the conductor's peer cache IS the persistent store; (c) NO slug addressing — topic_id is content-derived BLAKE3 over topic name. See spec section "P2P Design Gate Output" for the full classification.

---

## File Structure

**New files (3)**

| Path | Purpose | Approx LOC |
|---|---|---|
| `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs` | Wire payload, publisher, subscriber worker, rate limiter, topic constant, unit tests | 280 |
| `elohim/elohim-storage/tests/iroh_gossip_dual_publish_conductor_agent_info.rs` | Cross-stack dual-publish byte-parity test mirroring `iroh_gossip_dual_publish_identity_binding.rs` | 90 |
| `genesis/a2o/features/federation/cross-mesh-discovery.feature` | Two Gherkin scenarios (seeded cross-mesh, signal-A failure) | 35 |

**Modified files (4)**

| Path | Change | Approx LOC delta |
|---|---|---|
| `elohim/elohim-storage/src/p2p/topics.rs` | Re-export `TOPIC_CONDUCTOR_AGENT_INFO` constant | +5 |
| `elohim/elohim-storage/src/p2p/mod.rs` | Add `P2PCommand::PublishConductorAgentInfo` variant, outbound dual-publish arm, inbound subscribe arm that try_sends to bounded mpsc | +60 |
| `elohim/elohim-storage/src/main.rs` | Read feature-flag env var, create bounded mpsc, spawn publisher + subscriber worker, plumb mpsc sender into P2PNode config | +40 |
| `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` | Add row #13 + bump publisher count | +3 |

**Out of scope (do NOT touch in this plan):** `doorway/**`, `elohim/holochain/dna/**`, Holochain conductor config YAML, any diesel migration, any HTTP route.

## PR Breakdown

- **PR1** (Tasks 1-7): The full Rust module + wiring + unit tests + cross-stack parity test + catalog row. Single PR — pieces aren't independently mergeable (the module is dead code without the main.rs spawn, the spawn won't compile without the mpsc-sender plumbing). Pre-push gate runs `elohim-storage` quality gate (~100s based on prior gate timing).
- **PR2** (Task 8): A2O scenario file. Independent of PR1 — pre-push runs the existing `genesis/` validation gates (~30s; nothing new is being validated, just the standard genesis pre-push check on a new .feature file). Can land before, during, or after PR1.
- **Flag flip** (Task 9): Operator-side coordination note, not a PR. Operator enables the env var on matthew + adam first via `kubectl set env`, watches metrics, then enables cluster-wide once clean. After one stable week with the flag forced-on everywhere, a tiny follow-on PR removes the flag from the code.

---

## Task 1: Wire Payload + Topic Constant + Round-Trip Tests

**Files:**
- Create: `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (one line: add `pub mod conductor_agent_info_gossip;` next to the existing `pub mod identity_binding_gossip;`)

- [ ] **Step 1.1: Create the module file with payload struct + topic constant + structural verify + round-trip codec**

```rust
//! Conductor agent-info substrate gossip — propagates Holochain
//! `AgentInfoSigned` JSON strings across the libp2p mesh so every embedded
//! conductor's peer cache survives the Phase 1 doorway-A / doorway-B signal
//! partition, even when only one signal server is reachable from any given
//! pod's perspective.
//!
//! ## Design classification (per p2p-design-gate)
//!
//! Category C — operational. In-flight gossip envelope, never stored.
//! Receivers decode, verify, inject into the conductor's existing peer cache
//! via admin RPC, then drop. Lost messages are reconstructed by the next 60s
//! heartbeat. No persistence beyond the conductor's own internal store.
//!
//! Source of truth: the publishing peer's embedded conductor's
//! `admin_ws.agent_info(None)` admin RPC. The substrate gossip is purely a
//! transport mechanism; the conductor remains authoritative for signature
//! verification + dedup on `admin_ws.add_agent_info`.
//!
//! ## Spec
//!
//! `genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md`

use serde::{Deserialize, Serialize};

/// Wire payload for the `elohim/conductor/agent-info/v1` gossipsub topic.
///
/// Carries an opaque kitsune2 v2 agent_info JSON string. Receiver passes
/// `agent_info_json` directly to `admin_ws.add_agent_info(vec![json])`
/// without inspecting its internals — the conductor itself does signature
/// verification + dedup. Edge handler does only cheap structural checks
/// (`verify_structural`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConductorAgentInfo {
    /// Opaque kitsune2 v2 agent_info JSON string. Publisher reads via
    /// `admin_ws.agent_info(None)`, receiver passes to
    /// `admin_ws.add_agent_info`. The substrate never cracks open the JSON.
    pub agent_info_json: String,
    /// Microsecond unix timestamp at publish. Subscriber uses this for
    /// last-seen dedup (drop messages older than the most-recent seen for the
    /// same peer key) and operators use it for observability (how stale is
    /// any given entry in the cache).
    pub published_at: i64,
}

impl ConductorAgentInfo {
    /// MessagePack encode (named fields — forward-compat for future fields).
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// MessagePack decode from gossipsub-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Cheap structural check. Called on the gossip edge before try_sending
    /// into the bounded mpsc — drops obviously-malformed payloads before they
    /// reach the worker queue. Full validation happens in the conductor on
    /// `add_agent_info`.
    pub fn verify_structural(&self) -> Result<(), &'static str> {
        if self.agent_info_json.is_empty() {
            return Err("agent_info_json is empty");
        }
        if self.published_at <= 0 {
            return Err("published_at must be a positive microsecond timestamp");
        }
        Ok(())
    }
}

/// Gossipsub topic name. Use this constant at all publish/subscribe sites to
/// prevent compile-time typo drift.
pub const CONDUCTOR_AGENT_INFO_TOPIC: &str = "elohim/conductor/agent-info/v1";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConductorAgentInfo {
        ConductorAgentInfo {
            agent_info_json: r#"{"agent":"uhCAk_sample_pubkey","space":"uhC0k_sample_space","urls":["wss://signal.elohim.host/uhCAk_sample_pubkey"],"expires_at":1234567890}"#.to_string(),
            published_at: 1_700_000_000_000_000,
        }
    }

    #[test]
    fn roundtrip_preserves_payload() {
        let original = sample();
        let bytes = original.to_bytes().expect("to_bytes");
        let decoded = ConductorAgentInfo::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(original, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        let bytes = sample().to_bytes().expect("to_bytes");
        // ~150-byte JSON payload + ~30 bytes envelope ≈ 180 bytes
        // Allow generous headroom for real kitsune2 agent_info (which is ~400-600 bytes).
        assert!(bytes.len() < 1024, "payload should fit in 1KB; got {} bytes", bytes.len());
    }

    #[test]
    fn verify_structural_passes_valid_payload() {
        assert_eq!(sample().verify_structural(), Ok(()));
    }

    #[test]
    fn verify_structural_rejects_empty_json() {
        let mut bad = sample();
        bad.agent_info_json = String::new();
        assert!(bad.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_zero_timestamp() {
        let mut bad = sample();
        bad.published_at = 0;
        assert!(bad.verify_structural().is_err());
    }
}
```

- [ ] **Step 1.2: Register the module in `p2p/mod.rs`**

Add this line right after the existing `pub mod identity_binding_gossip;` (around line 56 — find with `grep -n 'pub mod identity_binding_gossip' src/p2p/mod.rs`):

```rust
pub mod conductor_agent_info_gossip;
```

- [ ] **Step 1.3: Run unit tests — verify they pass**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo test --lib p2p::conductor_agent_info_gossip
```

Expected: 5 tests pass (`roundtrip_preserves_payload`, `wire_bytes_are_small`, `verify_structural_passes_valid_payload`, `verify_structural_rejects_empty_json`, `verify_structural_rejects_zero_timestamp`).

- [ ] **Step 1.4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs \
        elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): conductor agent-info gossip wire payload + topic constant (step zero PR1.1)"
```

---

## Task 2: Publisher (publish_once + spawn + own-key filter)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs` (append publisher impl + tests)

- [ ] **Step 2.1: Add `now_micros()` helper + `json_contains_any_own_key` filter helper to the module**

Append before the `#[cfg(test)]` block:

```rust
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Microsecond unix timestamp. Used by both publisher (sets `published_at`)
/// and subscriber (last-seen dedup).
pub fn now_micros() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

/// Substring check: does `json` mention any of `own_keys`?
///
/// The kitsune2 v2 agent_info JSON envelope embeds the agent pubkey as a
/// base64 (or base58) string field — implementation-time field naming is
/// confirmed by inspecting one real agent_info() return from a dev conductor.
/// We use substring match because:
///   - the own_keys set is small (~4 entries: one per DNA the pod's conductor
///     is a member of),
///   - any pubkey occurrence in the JSON envelope is sufficient evidence that
///     this entry is OUR own (foreign agent_infos contain foreign pubkeys),
///   - substring is cheaper than `serde_json::Value` parse per-entry.
///
/// If kitsune2 v2 field naming proves ambiguous (e.g. a pubkey-shaped string
/// appears in another field), fall back to structured parse. The fallback is
/// still O(N) where N is small.
pub fn json_contains_any_own_key(json: &str, own_keys: &HashSet<String>) -> bool {
    own_keys.iter().any(|key| json.contains(key))
}
```

- [ ] **Step 2.2: Add the publisher's `publish_once` + `spawn_agent_info_publisher` functions**

Append after the helpers:

```rust
use crate::p2p::P2PCommand;
use holochain_client::AdminWebsocket;
use tokio::sync::mpsc::Sender;

/// Publish all of this conductor's own agent_infos to the topic, once.
///
/// Reads `admin_ws.agent_info(None)` to get the full peer cache, filters down
/// to OWN entries (the conductor's own agents per `list_cell_ids()`), wraps
/// each in a `ConductorAgentInfo` envelope, and sends one
/// `P2PCommand::PublishConductorAgentInfo` per envelope to the P2P swarm task.
///
/// Filtering is load-bearing: `agent_info()` returns the FULL peer cache
/// (including entries learned from gossip), so without the filter every pod
/// would re-publish everyone's info — amplification, confusion, and a load
/// curve that grows with cluster size squared.
pub async fn publish_once(
    admin_ws: &AdminWebsocket,
    p2p_tx: &Sender<P2PCommand>,
) -> Result<usize, PublishError> {
    let all_json = admin_ws
        .agent_info(None)
        .await
        .map_err(|e| PublishError::AdminRpc(format!("agent_info: {e}")))?;

    let own_cell_ids = admin_ws
        .list_cell_ids()
        .await
        .map_err(|e| PublishError::AdminRpc(format!("list_cell_ids: {e}")))?;

    let own_keys: HashSet<String> = own_cell_ids
        .iter()
        .map(|cid| {
            // base64-encode the agent_pub_key bytes for substring match against
            // the JSON. kitsune2 v2 serializes pubkeys as base64 strings.
            use base64::Engine;
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(cid.agent_pub_key().get_raw_39())
        })
        .collect();

    let now = now_micros();
    let mut published = 0usize;

    for json in all_json {
        if !json_contains_any_own_key(&json, &own_keys) {
            continue;
        }
        let envelope = ConductorAgentInfo {
            agent_info_json: json,
            published_at: now,
        };
        if let Err(e) = p2p_tx
            .send(P2PCommand::PublishConductorAgentInfo(envelope))
            .await
        {
            return Err(PublishError::ChannelClosed(format!(
                "P2P command channel closed: {e}"
            )));
        }
        published += 1;
    }

    debug!(target: "elohim_storage::agent_info", published, "publish_once tick complete");
    Ok(published)
}

/// Spawned after `happ_manager::wait_for_ready` returns. Fires `publish_once`
/// immediately on spawn (seed-time race fix), then loops on the heartbeat tick.
pub fn spawn_agent_info_publisher(
    admin_ws: Arc<AdminWebsocket>,
    p2p_tx: Sender<P2PCommand>,
    interval: Duration,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Seed-time race fix: publish immediately, don't wait for first tick.
        if let Err(e) = publish_once(&admin_ws, &p2p_tx).await {
            warn!(target: "elohim_storage::agent_info", error = %e, "publish_now (cold-start) failed");
        }

        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // First .tick() resolves immediately; consume it so the next one waits.
        tick.tick().await;

        info!(target: "elohim_storage::agent_info", interval_secs = interval.as_secs(), "agent_info publisher entering heartbeat loop");

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!(target: "elohim_storage::agent_info", "publisher shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = publish_once(&admin_ws, &p2p_tx).await {
                        warn!(target: "elohim_storage::agent_info", error = %e, "publish tick failed; will retry next interval");
                    }
                }
            }
        }
    })
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("admin RPC failed: {0}")]
    AdminRpc(String),
    #[error("channel closed: {0}")]
    ChannelClosed(String),
}
```

- [ ] **Step 2.3: Add unit test for own-key filter**

Append inside the `#[cfg(test)]` mod tests block:

```rust
    #[test]
    fn own_key_filter_keeps_self_entries() {
        let own: HashSet<String> = ["uhCAk_alice_pk".to_string()].into_iter().collect();
        let json_with_own = r#"{"agent":"uhCAk_alice_pk","urls":["wss://x"]}"#;
        let json_without = r#"{"agent":"uhCAk_bob_pk","urls":["wss://y"]}"#;
        assert!(json_contains_any_own_key(json_with_own, &own));
        assert!(!json_contains_any_own_key(json_without, &own));
    }

    #[test]
    fn own_key_filter_empty_own_keys_rejects_all() {
        let own: HashSet<String> = HashSet::new();
        assert!(!json_contains_any_own_key(r#"{"agent":"uhCAk_alice_pk"}"#, &own));
    }
```

- [ ] **Step 2.4: Verify the module references compile — `P2PCommand::PublishConductorAgentInfo` doesn't exist yet, so this should fail**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo check --lib 2>&1 | grep -E 'error|PublishConductorAgentInfo' | head -5
```

Expected: error referencing `PublishConductorAgentInfo` not found in `P2PCommand`. This is the expected failing state — Task 4 adds the variant.

- [ ] **Step 2.5: Run only the new own-key filter unit tests in isolation (they don't depend on P2PCommand)**

The own-key filter tests are pure-function tests with no P2PCommand dependency. They can pass even while the rest of the module has a compile error in publisher functions. Use a unit-only filter:

If the module-wide compile fails, defer running these tests until Task 4 completes and the module compiles, then run them together in Task 4's verify step. Do not commit yet — the module is in a half-built state.

- [ ] **Step 2.6: Add the `base64` and `thiserror` dependencies (only if not present in Cargo.toml)**

Check first:

```bash
grep -E '^base64\s*=|^thiserror\s*=' /projects/elohim/elohim/elohim-storage/Cargo.toml | head -5
```

Both are almost certainly already present (storage uses them widely). If `base64` is missing, add to `[dependencies]`:

```toml
base64 = "0.22"
```

If `thiserror` is missing, add:

```toml
thiserror = "1"
```

If both are present, skip this step.

(Do not commit yet — Task 4 will commit the full publisher + variant together once everything compiles.)

---

## Task 3: Subscriber Worker (verify_and_dedupe + worker loop + rate limiter)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs` (append subscriber worker + tests)

- [ ] **Step 3.1: Add `SubscriberConfig`, the `RateLimiter`, and `verify_and_dedupe`**

Append before the `#[cfg(test)]` block:

```rust
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;

/// Per-pod tuning for the subscriber worker. Defaults sized for typical-steward
/// device archetype; chromebook-class lowers `max_rate_per_sec` and
/// `max_batch`, performance-class raises them. Operator-overridable via env vars.
#[derive(Debug, Clone)]
pub struct SubscriberConfig {
    /// Worker tick interval. Default 200ms.
    pub batch_interval: Duration,
    /// Bounded inbound queue capacity. Default 256 — sized to absorb a burst
    /// from a 14-peer cluster's first publish-now wave (~56 envelopes) with
    /// headroom for restart storms.
    pub queue_capacity: usize,
    /// Max admin RPC calls per second. Default 20/sec on typical-steward;
    /// chromebook-class should use 10, performance-class can do 200.
    pub max_rate_per_sec: u32,
    /// Max envelopes per `add_agent_info` batch. Default 32 — large enough to
    /// amortize RPC overhead, small enough not to overwhelm the conductor's
    /// own add-agent-info processing on chromebook-class.
    pub max_batch: usize,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            batch_interval: Duration::from_millis(200),
            queue_capacity: 256,
            max_rate_per_sec: 20,
            max_batch: 32,
        }
    }
}

impl SubscriberConfig {
    /// Read from environment variables, falling back to defaults.
    /// Operator overrides per pod via `kubectl set env`.
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            batch_interval: std::env::var("AGENT_INFO_BATCH_INTERVAL_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_millis)
                .unwrap_or(d.batch_interval),
            queue_capacity: std::env::var("AGENT_INFO_QUEUE_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(d.queue_capacity),
            max_rate_per_sec: std::env::var("AGENT_INFO_MAX_RATE_PER_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(d.max_rate_per_sec),
            max_batch: std::env::var("AGENT_INFO_MAX_BATCH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(d.max_batch),
        }
    }
}

/// Simple token-bucket rate limiter. Refills `max_per_sec` tokens per second
/// (smoothly: refills proportional to elapsed time). `acquire(n)` waits until
/// at least `n` tokens are available, then consumes them.
pub struct RateLimiter {
    max_per_sec: u32,
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_per_sec: u32) -> Self {
        Self {
            max_per_sec,
            tokens: max_per_sec as f64,
            last_refill: Instant::now(),
        }
    }

    pub async fn acquire(&mut self, n: u32) {
        loop {
            self.refill();
            if self.tokens >= n as f64 {
                self.tokens -= n as f64;
                return;
            }
            // Need (n - tokens) more tokens. Compute wait time + sleep.
            let needed = (n as f64) - self.tokens;
            let secs = needed / (self.max_per_sec as f64);
            tokio::time::sleep(Duration::from_secs_f64(secs.max(0.001))).await;
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let refill = elapsed * (self.max_per_sec as f64);
        self.tokens = (self.tokens + refill).min(self.max_per_sec as f64);
        self.last_refill = now;
    }
}

/// Verify a received envelope and check against the last-seen map.
/// Returns `Some(agent_info_json)` to inject, or `None` to drop.
///
/// Last-seen key is the full JSON string (which uniquely identifies the
/// pubkey + binding + expiration tuple — kitsune2 v2 JSONs differ on any of
/// those, so string equality is a sufficient idempotency key). For more
/// precise dedup by pubkey alone, switch the map key once kitsune2 v2 field
/// naming is confirmed at implementation time.
pub fn verify_and_dedupe(
    msg: ConductorAgentInfo,
    last_seen: &mut HashMap<String, i64>,
) -> Option<String> {
    if msg.verify_structural().is_err() {
        return None;
    }
    if let Some(&prev) = last_seen.get(&msg.agent_info_json) {
        if prev >= msg.published_at {
            return None;
        }
    }
    last_seen.insert(msg.agent_info_json.clone(), msg.published_at);
    Some(msg.agent_info_json)
}
```

- [ ] **Step 3.2: Add the worker spawn function**

Append after `verify_and_dedupe`:

```rust
/// Spawn the subscriber worker task. Owns the receiver end of the bounded
/// mpsc, drains it on a tick, batches into a single `add_agent_info` RPC per
/// drain, and rate-limits to `cfg.max_rate_per_sec` envelopes/sec.
///
/// The gossip-edge handler that try_sends into the mpsc lives in `p2p/mod.rs`
/// — see the `CONDUCTOR_AGENT_INFO_TOPIC` arm in the swarm event loop.
pub fn spawn_agent_info_subscriber_worker(
    admin_ws: Arc<AdminWebsocket>,
    mut rx: Receiver<ConductorAgentInfo>,
    cfg: SubscriberConfig,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(cfg.batch_interval);
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let mut limiter = RateLimiter::new(cfg.max_rate_per_sec);

        info!(
            target: "elohim_storage::agent_info",
            queue_capacity = cfg.queue_capacity,
            max_rate_per_sec = cfg.max_rate_per_sec,
            max_batch = cfg.max_batch,
            "agent_info subscriber worker entering loop"
        );

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!(target: "elohim_storage::agent_info", "subscriber worker shutting down");
                    break;
                }
                _ = tick.tick() => {
                    let mut batch = Vec::with_capacity(cfg.max_batch);
                    while batch.len() < cfg.max_batch {
                        match rx.try_recv() {
                            Ok(msg) => {
                                if let Some(json) = verify_and_dedupe(msg, &mut last_seen) {
                                    batch.push(json);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    if batch.is_empty() {
                        continue;
                    }
                    let n = batch.len();
                    limiter.acquire(n as u32).await;
                    match admin_ws.add_agent_info(batch).await {
                        Ok(()) => debug!(target: "elohim_storage::agent_info", batch_size = n, "add_agent_info batch ingested"),
                        Err(e) => warn!(
                            target: "elohim_storage::agent_info",
                            batch_size = n,
                            error = %e,
                            "add_agent_info batch failed; heartbeat will retry"
                        ),
                    }
                }
            }
        }
    })
}
```

- [ ] **Step 3.3: Add unit tests for verify_and_dedupe + rate limiter**

Append inside the `#[cfg(test)]` mod tests block:

```rust
    #[test]
    fn verify_and_dedupe_accepts_first_seen() {
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let msg = sample();
        let json = msg.agent_info_json.clone();
        assert_eq!(verify_and_dedupe(msg, &mut last_seen), Some(json));
    }

    #[test]
    fn verify_and_dedupe_rejects_older_published_at() {
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let first = sample();
        let _ = verify_and_dedupe(first.clone(), &mut last_seen);
        let mut older = first;
        older.published_at -= 1_000_000; // 1 second older
        assert_eq!(verify_and_dedupe(older, &mut last_seen), None);
    }

    #[test]
    fn verify_and_dedupe_accepts_newer_published_at() {
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let first = sample();
        let _ = verify_and_dedupe(first.clone(), &mut last_seen);
        let mut newer = first;
        newer.published_at += 1_000_000;
        assert!(verify_and_dedupe(newer, &mut last_seen).is_some());
    }

    #[test]
    fn verify_and_dedupe_drops_structurally_invalid() {
        let mut last_seen: HashMap<String, i64> = HashMap::new();
        let mut bad = sample();
        bad.agent_info_json = String::new();
        assert_eq!(verify_and_dedupe(bad, &mut last_seen), None);
    }

    #[tokio::test]
    async fn rate_limiter_throttles_burst() {
        let mut lim = RateLimiter::new(10); // 10 per sec
        let start = tokio::time::Instant::now();
        // Drain initial bucket
        lim.acquire(10).await;
        // Next acquire(10) must wait approximately 1s
        lim.acquire(10).await;
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 900, "burst should take ≥900ms; got {}ms", elapsed);
        assert!(elapsed < 1500, "burst shouldn't take >1.5s; got {}ms", elapsed);
    }

    #[test]
    fn subscriber_config_from_env_falls_back_to_defaults() {
        // Don't set any env vars; defaults should be returned.
        // Use safe-mutation env vars (rare names) for the override portion of this test
        let cfg = SubscriberConfig::from_env();
        // Defaults match the Default impl
        let defaults = SubscriberConfig::default();
        assert_eq!(cfg.queue_capacity, defaults.queue_capacity);
        assert_eq!(cfg.max_rate_per_sec, defaults.max_rate_per_sec);
        assert_eq!(cfg.max_batch, defaults.max_batch);
    }
```

(Don't commit yet — module still references `P2PCommand::PublishConductorAgentInfo`. Task 4 finishes the compile.)

---

## Task 4: P2PCommand Wiring (outbound + inbound)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add P2PCommand variant, outbound dual-publish arm, inbound subscribe arm with try_send into mpsc)

- [ ] **Step 4.1: Add the P2PCommand variant**

Find the existing `pub enum P2PCommand` block in `p2p/mod.rs` (around L712):

```bash
grep -n 'pub enum P2PCommand' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs
```

Find the existing `PublishIdentityBinding` variant (~L758):

```bash
grep -n 'PublishIdentityBinding\|PublishRecoveryRevocation' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs | head -5
```

After `PublishRecoveryRevocation(crate::p2p::recovery_revocation::RecoveryRevocationMessage),`, add:

```rust
    /// Step-zero substrate gossip — broadcast a Holochain conductor's agent_info JSON
    /// to peer pods so their embedded conductors learn about us regardless of
    /// signal_url. Producer: AgentInfoPublisher (`p2p::conductor_agent_info_gossip::publish_once`).
    /// See `genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md`.
    PublishConductorAgentInfo(crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo),
```

- [ ] **Step 4.2: Add the fire-and-forget tracking entry**

Find the section listing fire-and-forget commands (~L997, contains `PublishIdentityBinding(_) => {}`):

```bash
grep -n 'PublishIdentityBinding(_) => {}' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs
```

After the `PublishRecoveryRevocation(_) => {}` line, add:

```rust
                    P2PCommand::PublishConductorAgentInfo(_) => {} // fire-and-forget
```

- [ ] **Step 4.3: Add the outbound dual-publish arm**

Find the existing `PublishIdentityBinding` outbound arm (~L2515):

```bash
grep -n 'P2PCommand::PublishIdentityBinding(payload)' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs
```

After the closing brace of the `P2PCommand::PublishRecoveryRevocation` arm (look around the `PublishIdentityBinding` arm for the matching brace structure ~L2545 area), add:

```rust
            P2PCommand::PublishConductorAgentInfo(payload) => match payload.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = self.gossip_publisher.publish(
                        crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC,
                        bytes,
                    ) {
                        warn!(
                            target: "elohim_storage::agent_info",
                            error = %e,
                            "PublishConductorAgentInfo dual-publish failed"
                        );
                    } else {
                        debug!(
                            target: "elohim_storage::agent_info",
                            published_at = payload.published_at,
                            "conductor agent_info published to substrate"
                        );
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::agent_info",
                    error = %e,
                    "PublishConductorAgentInfo to_bytes failed"
                ),
            },
```

- [ ] **Step 4.4: Add the inbound subscribe arm in the gossipsub event handler**

Find the existing inbound topic match block (where it handles `IDENTITY_BINDING_TOPIC`):

```bash
grep -n 'IDENTITY_BINDING_TOPIC' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs | head -5
```

Locate the inbound handler arm (around the `} else if message.topic.as_str() == IDENTITY_BINDING_TOPIC` block). Add a new `else if` arm immediately after the IDENTITY_BINDING_TOPIC arm closes (preserve indentation to match neighbors):

```rust
                        } else if message.topic.as_str()
                            == crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC
                        {
                            // Step-zero: edge handler. Lightweight — decode, structural verify,
                            // try_send into bounded mpsc. The worker (spawned in main.rs) drains
                            // the mpsc, rate-limits, batches, and calls admin_ws.add_agent_info.
                            // Channel-full drops here are safe: the next 60s publisher heartbeat
                            // re-delivers, so at most one heartbeat window of latency on stabilization.
                            match crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo::from_bytes(
                                &message.data,
                            ) {
                                Ok(payload) => {
                                    if let Err(reason) = payload.verify_structural() {
                                        debug!(
                                            target: "elohim_storage::agent_info",
                                            from = %propagation_source,
                                            reason = %reason,
                                            "ConductorAgentInfo failed structural verify — dropped"
                                        );
                                    } else if let Some(tx) = &self.agent_info_inbound_tx {
                                        if tx.try_send(payload).is_err() {
                                            debug!(
                                                target: "elohim_storage::agent_info",
                                                from = %propagation_source,
                                                "agent_info inbound queue full — dropped (heartbeat will re-deliver)"
                                            );
                                        }
                                    }
                                    // else: feature flag is off, no sender wired — silently ignore
                                }
                                Err(e) => debug!(
                                    target: "elohim_storage::agent_info",
                                    from = %propagation_source,
                                    error = %e,
                                    "ConductorAgentInfo decode failed — dropped"
                                ),
                            }
                        }
```

- [ ] **Step 4.5: Add the `agent_info_inbound_tx` field to the P2PNode struct (or wherever the gossipsub event handler accesses state)**

Find the struct that owns the gossipsub event handler context — search for where `peer_identity_bindings_writer` or similar is held (the IdentityBinding handler accesses it through `self`):

```bash
grep -n 'peer_identity_bindings_writer\|pub struct P2PNode\|pub struct P2PWorker' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs | head -10
```

Add to that struct (preserve neighbors' style):

```rust
    /// Step-zero: bounded mpsc sender for inbound ConductorAgentInfo envelopes
    /// from the substrate gossip topic. `None` when the feature flag is off
    /// (`ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP != "true"`). When `Some`, the
    /// subscriber worker (spawned in main.rs) drains the receiver end.
    pub agent_info_inbound_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo>>,
```

Add `agent_info_inbound_tx: None,` to all constructors of that struct (find with `grep -n 'fn new' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs` and add the field to each).

- [ ] **Step 4.6: Run cargo check — should now compile**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo check --lib 2>&1 | tail -20
```

Expected: clean compile (zero errors). If any structs missing `agent_info_inbound_tx` field, add the field defaulted to `None`.

- [ ] **Step 4.7: Run all unit tests in the module — confirm all pass**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo test --lib p2p::conductor_agent_info_gossip 2>&1 | tail -15
```

Expected: 11 tests pass (5 from Task 1, 2 from Task 2, 4 from Task 3, 1 from Task 3's rate-limiter tokio test — count adjusts based on exact additions).

- [ ] **Step 4.8: Run clippy on the new module**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo clippy --lib -p elohim-storage -- -D warnings 2>&1 | tail -10
```

Expected: clean (zero warnings).

- [ ] **Step 4.9: Commit the module + wiring as one logical unit**

```bash
git add elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/Cargo.toml
git commit -m "feat(storage): conductor agent-info substrate gossip module + P2PCommand wiring (step zero core)"
```

---

## Task 5: main.rs Wiring (mpsc + spawns + feature flag)

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`

- [ ] **Step 5.1: Locate the spawn region (right after `wait_for_ready` returns)**

```bash
grep -n 'wait_for_ready\|admin_ws = manager' /projects/elohim/elohim/elohim-storage/src/main.rs | head -5
```

Around L495. Find the block ending at "Embedded conductor ready, hApp installed" (around L499).

- [ ] **Step 5.2: Add the feature-flag check + spawn logic**

After the `info!("Embedded conductor ready, hApp installed");` line and BEFORE the `Some(manager)` (around L499-500), add:

```rust
        // Step-zero substrate gossip — wire conductor agent_info propagation
        // across the libp2p mesh so each pod's conductor peer cache survives
        // the Phase 1 doorway-A / doorway-B signal partition. Behind a
        // feature flag so initial rollout can enable matthew + adam first,
        // verify metrics, then expand cluster-wide.
        let enable_agent_info_gossip = std::env::var("ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        if enable_agent_info_gossip {
            use elohim_storage::p2p::conductor_agent_info_gossip::{
                spawn_agent_info_publisher,
                spawn_agent_info_subscriber_worker,
                ConductorAgentInfo,
                SubscriberConfig,
            };
            let cfg = SubscriberConfig::from_env();
            let (ai_tx, ai_rx) = tokio::sync::mpsc::channel::<ConductorAgentInfo>(cfg.queue_capacity);
            agent_info_inbound_tx = Some(ai_tx);
            agent_info_subscriber_worker = Some(spawn_agent_info_subscriber_worker(
                std::sync::Arc::new(admin_ws.clone()),
                ai_rx,
                cfg,
                shutdown_token.clone(),
            ));
            // Publisher is spawned later, after p2p_tx is created (the swarm's
            // command sender). We hold a placeholder here that's filled in
            // below when the swarm spins up.
            agent_info_publisher_admin_ws = Some(std::sync::Arc::new(admin_ws.clone()));
            info!(
                target: "elohim_storage::agent_info",
                "step-zero substrate agent_info gossip ENABLED (feature flag on)"
            );
        } else {
            info!(
                target: "elohim_storage::agent_info",
                "step-zero substrate agent_info gossip disabled (feature flag off)"
            );
        }
```

- [ ] **Step 5.3: Declare the holder variables above the conductor-ready block**

Above the `if let Some(admin_url) = ...` or wherever the conductor block starts, declare:

```rust
    // Step-zero agent_info gossip — populated inside the conductor-ready block
    // when the feature flag is on. Read below when wiring p2p_tx + the swarm.
    let mut agent_info_inbound_tx: Option<tokio::sync::mpsc::Sender<elohim_storage::p2p::conductor_agent_info_gossip::ConductorAgentInfo>> = None;
    let mut agent_info_subscriber_worker: Option<tokio::task::JoinHandle<()>> = None;
    let mut agent_info_publisher_admin_ws: Option<std::sync::Arc<holochain_client::AdminWebsocket>> = None;
```

- [ ] **Step 5.4: Plumb `agent_info_inbound_tx` into the P2PNode constructor**

Find where the P2PNode (or P2PWorker, whatever the struct is named per Task 4 Step 4.5) is constructed in main.rs:

```bash
grep -n 'P2PNode::new\|P2PWorker::new\|p2p_node\.' /projects/elohim/elohim/elohim-storage/src/main.rs | head -5
```

At the construction site, set the new field after construction:

```rust
    let mut p2p_node = /* existing construction */;
    p2p_node.agent_info_inbound_tx = agent_info_inbound_tx.clone();
```

(If the struct construction uses field-init syntax, add `agent_info_inbound_tx: agent_info_inbound_tx.clone(),` to the init.)

- [ ] **Step 5.5: Spawn the publisher after the swarm + p2p_tx exist**

Find where `p2p_tx` (the `Sender<P2PCommand>` to the swarm) is created in main.rs:

```bash
grep -n 'P2PCommand\|p2p_tx\|tokio::sync::mpsc::channel' /projects/elohim/elohim/elohim-storage/src/main.rs | head -10
```

After `p2p_tx` exists, add:

```rust
    // Step-zero: spawn agent_info publisher now that p2p_tx is live.
    let _agent_info_publisher_task = if let Some(admin_ws_arc) = agent_info_publisher_admin_ws {
        Some(elohim_storage::p2p::conductor_agent_info_gossip::spawn_agent_info_publisher(
            admin_ws_arc,
            p2p_tx.clone(),
            std::time::Duration::from_secs(60),
            shutdown_token.clone(),
        ))
    } else {
        None
    };
```

- [ ] **Step 5.6: Verify main.rs compiles**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo check --bin elohim-storage 2>&1 | tail -15
```

Expected: clean compile. If errors reference `shutdown_token`, find its actual name in main.rs (might be `shutdown_tx.subscribe()` pattern) and adapt — the spec assumes a CancellationToken; the codebase uses `broadcast::channel::<()>` (see grep earlier). Adapt by wrapping the broadcast receiver in a CancellationToken-like select arm, or use `shutdown_tx.subscribe()` directly inside the spawn function (replace `CancellationToken` arg with `broadcast::Receiver<()>`).

- [ ] **Step 5.7: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): wire agent-info publisher + subscriber worker in main (feature-flagged off by default)"
```

---

## Task 6: topics.rs Re-Export + dual_publish CATALOG row

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/topics.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md`

- [ ] **Step 6.1: Add the re-export to topics.rs**

Find the existing identity binding re-export:

```bash
grep -n 'TOPIC_IDENTITY_BINDING' /projects/elohim/elohim/elohim-storage/src/p2p/topics.rs
```

After `pub const TOPIC_IDENTITY_BINDING: &str = ...;` add:

```rust

/// Step-zero conductor agent-info gossip topic — re-export alias for the
/// canonical constant in `p2p::conductor_agent_info_gossip`. Use this from
/// the unified topics surface; the source-of-truth constant stays in the
/// gossip module.
pub const TOPIC_CONDUCTOR_AGENT_INFO: &str =
    crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC;
```

- [ ] **Step 6.2: Update the CATALOG.md publisher table**

Open `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` and find the publisher table (search for `## Publisher Sites`).

Change the heading line:

```
## Publisher Sites (12 total)
```

to:

```
## Publisher Sites (13 total)
```

After the row for `| 6 | /elohim/feedback-signal/...`, append:

```
| 13 | `elohim/conductor/agent-info/v1` | `P2PCommand::PublishConductorAgentInfo` arm — `src/p2p/mod.rs` (publish at the new arm added in Task 4 Step 4.3) | `ConductorAgentInfo` (`src/p2p/conductor_agent_info_gossip.rs`) | `to_vec_named` |
```

In the "Producer Call Chains" table, append:

```
| 13 | `spawn_agent_info_publisher` heartbeat tick / publish-now on cold start | `PublishConductorAgentInfo` | `src/p2p/conductor_agent_info_gossip.rs` (publish_once) |
```

- [ ] **Step 6.3: Verify compile**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo check --lib 2>&1 | tail -5
```

Expected: clean.

- [ ] **Step 6.4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/topics.rs \
        elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md
git commit -m "docs(storage): catalog conductor agent-info dual-publish + topics re-export"
```

---

## Task 7: Cross-Stack Dual-Publish Parity Test

**Files:**
- Create: `elohim/elohim-storage/tests/iroh_gossip_dual_publish_conductor_agent_info.rs`

- [ ] **Step 7.1: Write the parity test mirroring `iroh_gossip_dual_publish_identity_binding.rs`**

```rust
//! Step zero: ConductorAgentInfo dual-publish byte-parity test.
//!
//! Verifies that publishing a `ConductorAgentInfo` through a
//! `DualGossipPublisher` delivers byte-identical payloads to both the libp2p
//! mock and the iroh mock. The wire format is named-field MessagePack
//! (`rmp_serde::to_vec_named`), matching the inventory + recovery payload
//! convention (see CATALOG row #1, #2, #4).
//!
//! Gated on `p2p-iroh` (mirrors the identity-binding test convention).

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::conductor_agent_info_gossip::{
    ConductorAgentInfo, CONDUCTOR_AGENT_INFO_TOPIC,
};
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

#[derive(Clone, Default)]
struct CaptureMock {
    calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl CaptureMock {
    fn new() -> Self {
        Self::default()
    }
    fn calls(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl GossipPublisher for CaptureMock {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((topic.to_string(), payload));
        Ok(())
    }
}

#[test]
fn conductor_agent_info_dual_publish_byte_parity() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    let payload = ConductorAgentInfo {
        agent_info_json: r#"{"agent":"uhCAk_dual_publish_test_pubkey","space":"uhC0k_dual_publish_test_space","urls":["wss://signal.elohim.host/uhCAk_dual_publish_test_pubkey"],"expires_at":1234567890}"#.to_string(),
        published_at: 1_700_000_000_000_000,
    };
    let bytes = payload.to_bytes().expect("to_bytes should succeed");

    publisher
        .publish(CONDUCTOR_AGENT_INFO_TOPIC, bytes.clone())
        .expect("DualGossipPublisher should succeed");

    let lp_calls = libp2p_sub.calls();
    let iroh_calls = iroh_sub.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p sub must receive one payload");
    assert_eq!(iroh_calls.len(), 1, "iroh sub must receive one payload");

    assert_eq!(lp_calls[0].0, CONDUCTOR_AGENT_INFO_TOPIC);
    assert_eq!(iroh_calls[0].0, CONDUCTOR_AGENT_INFO_TOPIC);

    // Byte parity — both transports must receive the SAME bytes.
    assert_eq!(lp_calls[0].1, iroh_calls[0].1, "byte parity violated");
    assert_eq!(lp_calls[0].1, bytes, "libp2p sub received different bytes than published");

    // Decode and verify round-trip preserves payload.
    let decoded =
        ConductorAgentInfo::from_bytes(&lp_calls[0].1).expect("from_bytes should succeed");
    assert_eq!(decoded, payload);
}
```

- [ ] **Step 7.2: Run the test under the p2p-iroh feature**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
  cargo test --features p2p,p2p-iroh --test iroh_gossip_dual_publish_conductor_agent_info -- --test-threads=1
```

Expected: 1 test passes (`conductor_agent_info_dual_publish_byte_parity`).

- [ ] **Step 7.3: Commit**

```bash
git add elohim/elohim-storage/tests/iroh_gossip_dual_publish_conductor_agent_info.rs
git commit -m "test(storage): conductor agent-info dual-publish byte-parity (libp2p ≡ iroh)"
```

---

## Task 8: A2O Scenarios (Cross-Mesh Discovery Feature)

**Files:**
- Create: `genesis/a2o/features/federation/cross-mesh-discovery.feature`

- [ ] **Step 8.1: Check the federation features directory exists; create if not**

```bash
ls /projects/elohim/genesis/a2o/features/federation/ 2>/dev/null || mkdir -p /projects/elohim/genesis/a2o/features/federation/
```

- [ ] **Step 8.2: Write the feature file**

```gherkin
@step-zero @cross-mesh @phase-1-federation
Feature: Cross-mesh DHT discovery survives the doorway-A / doorway-B partition

  The federation-wiring-audit Phase 1 split the alpha cluster's signaling
  mesh: matthew/jessica/james register at signal.doorway-alpha.elohim.host;
  the 11 remote personas (adam, pete, terrance, frank, gertrude, susan, caleb,
  daniel, emma, eve, nancy) register at signal.elohim.host. The substrate
  step-zero gossip (`elohim/conductor/agent-info/v1`) propagates each pod's
  Holochain AgentInfoSigned over the libp2p mesh (which is already full-mesh
  cross-doorway via P2P_BOOTSTRAP_NODES), so every conductor's peer cache
  learns about every other peer regardless of which signal server they
  registered with.

  These scenarios run against the alpha cluster's a2o pipeline once the
  ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP flag is on cluster-wide.

  Background:
    Given the alpha cluster has 14 humans deployed with per-human primary doorway routing
    And matthew, jessica, james are registered with signal.doorway-alpha.elohim.host
    And adam plus 10 others are registered with signal.elohim.host
    And the ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP feature flag is on for every pod

  @seeder @substrate-replication
  Scenario: Seeded content lands on one peer and reaches the cross-mesh half
    Given the seeder writes Matthew's AccountPackage to elohim-matthew-alpha (hash-mod primary)
    When the seeder completes the Matthew package import
    Then within 30 seconds adam's conductor can DHT-resolve Matthew's ContributorPresence
    And within 30 seconds pete's conductor can DHT-resolve Matthew's identity binding

  @signal-failure @resilience
  Scenario: Signal-server-A goes down mid-session and the cross-mesh half stays reachable
    Given the cluster is in steady state with all conductor peer caches warm
    When signal.doorway-alpha.elohim.host becomes unreachable
    Then existing inter-peer DHT operations continue to complete for at least 5 minutes
    And adam can still DHT-resolve content authored by matthew via cached peer info
```

- [ ] **Step 8.3: Verify the file parses (a2o pre-push gate or local check)**

If the a2o project has a local lint command:

```bash
cd /projects/elohim/genesis/a2o
pnpm exec cucumber-js --dry-run features/federation/cross-mesh-discovery.feature 2>&1 | tail -10
```

If no local command, the pre-push genesis gate runs it. Expected: dry-run reports the two scenarios with `UNDEFINED` step bindings (no step defs yet — these are scenario-first; the bindings will be implemented later under separate work).

- [ ] **Step 8.4: Commit**

```bash
git add genesis/a2o/features/federation/cross-mesh-discovery.feature
git commit -m "test(a2o): cross-mesh DHT discovery scenarios for step-zero substrate gossip"
```

---

## Task 9: Pre-Push + PR Open

- [ ] **Step 9.1: Stage and push PR1 (Tasks 1-7 — all storage changes)**

```bash
cd /projects/elohim
git push origin HEAD:refs/heads/claude/conductor-agent-info-substrate-gossip-pr1 2>&1 | tail -20
```

Expected: pre-push gate runs `elohim-storage` quality gate (~100-200s based on prior gate observation). On success, new remote branch.

- [ ] **Step 9.2: Open PR1 with the spec link as the description**

```bash
gh pr create \
  --base dev \
  --head claude/conductor-agent-info-substrate-gossip-pr1 \
  --title "feat(storage): conductor agent-info substrate gossip — step zero (PR1: module + wiring + parity test)" \
  --body "$(cat <<'EOF'
## Summary

- Step-zero substrate gossip propagating Holochain conductor `AgentInfoSigned` across the libp2p mesh.
- Closes the doorway-A / doorway-B partition risk introduced by Phase 1 per-human primary doorway routing (commit 91f300663).
- Behind feature flag `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` (default false).

## Spec

`genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md` (committed as 1ed33f548).

## What changed

- New module `elohim-storage/src/p2p/conductor_agent_info_gossip.rs` (wire payload, publisher, subscriber worker with pull-based bounded mpsc + rate limiter, unit tests).
- `P2PCommand::PublishConductorAgentInfo` variant + outbound dual-publish arm + inbound subscribe arm wiring in `p2p/mod.rs`.
- `main.rs` spawns publisher + subscriber worker behind the feature flag.
- Topics re-export + dual-publish catalog row #13.
- Cross-stack byte-parity test `iroh_gossip_dual_publish_conductor_agent_info.rs`.

## Test plan

- [x] Unit tests (11 in-module): round-trip, structural verify, own-key filter, verify_and_dedupe, rate limiter
- [x] Cross-stack dual-publish parity: byte-identical payload on libp2p mock and iroh mock
- [ ] Operator: enable flag on matthew + adam first via `kubectl set env`, watch `add_agent_info`-call rate + queue-full count metrics for 24h
- [ ] Operator: roll cluster-wide once 24h soak is clean
- [ ] PR2 (separate): a2o scenarios proving cross-mesh DHT resolution

## Out of scope (deferred to Phase 12+)

- Cold-start with signal_url down (requires substrate-level WebRTC signaling)
- Cross-cluster reach (separate audit sequenced item)
- Multi-URL agent_info (upstream HC schema change)
- Session/recovery doorway-agnostic refactor (`local_sessions.doorway_url` single-pin)
- Iroh subscriber-side wiring (deferred behind Plan 4 Task 8)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 9.3: Stage and push PR2 (Task 8 — a2o feature file)**

```bash
cd /projects/elohim
git push origin HEAD:refs/heads/claude/cross-mesh-discovery-a2o-pr2 2>&1 | tail -15
```

Expected: pre-push gate runs `genesis` validation (~30-60s). On success, new remote branch.

- [ ] **Step 9.4: Open PR2**

```bash
gh pr create \
  --base dev \
  --head claude/cross-mesh-discovery-a2o-pr2 \
  --title "test(a2o): cross-mesh DHT discovery scenarios for step-zero substrate gossip" \
  --body "$(cat <<'EOF'
## Summary

Two Gherkin scenarios under `genesis/a2o/features/federation/cross-mesh-discovery.feature` proving the substrate-level cross-mesh discovery introduced in [PR1](#PR1_URL_HERE).

## Test plan

- [ ] Step definitions implemented in follow-on work (scenarios are first; bindings second)
- [ ] Runs against alpha cluster a2o pipeline once `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` is cluster-wide

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 9.5: Coordination note — operator enables flag, watches metrics, flips cluster-wide**

This is NOT a PR step — it's an operator handoff. The PR1 description's test plan section names the operator actions. After PR1 merges:

1. Operator sets `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP=true` on matthew + adam via `kubectl set env statefulset/elohim-matthew-alpha ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP=true -n elohim-alpha` (and same for adam).
2. Watch for 24h: `add_agent_info`-call rate, inbound-queue-full count, dropped-message count, conductor peer-cache size vs control pod.
3. If clean: roll cluster-wide by setting the flag on every pod's env.
4. After one stable week with the flag forced-on everywhere: open a tiny follow-on PR removing the flag check (the code stays, just becomes unconditional).

---

## Self-Review

**1. Spec coverage**

| Spec section | Plan task | Status |
|---|---|---|
| Wire payload (`ConductorAgentInfo` struct) | Task 1 | covered |
| `CONDUCTOR_AGENT_INFO_TOPIC` constant | Task 1 + Task 6 | covered |
| Publisher (`publish_once` + `spawn_agent_info_publisher`) | Task 2 | covered |
| Publisher "skip non-self entries" filter | Task 2 (`json_contains_any_own_key`) | covered |
| Publish-now on cold start (seed-time race fix) | Task 2 (`spawn_agent_info_publisher` fires `publish_once` before tick loop) | covered |
| Subscriber edge handler (lightweight, try_send) | Task 4 (the `CONDUCTOR_AGENT_INFO_TOPIC` inbound arm in mod.rs) | covered |
| Subscriber worker (bounded mpsc + rate limiter + batched admin RPC) | Task 3 | covered |
| `SubscriberConfig` env-var tuning per device archetype | Task 3 (`SubscriberConfig::from_env`) | covered |
| `verify_and_dedupe` semantics | Task 3 | covered |
| P2PCommand dual-publish wiring | Task 4 | covered |
| main.rs spawns + feature flag | Task 5 | covered |
| topics.rs re-export | Task 6 | covered |
| dual_publish CATALOG row #13 | Task 6 | covered |
| Unit tests (5+ named) | Tasks 1, 2, 3 (11 total tests) | covered |
| Cross-stack parity test | Task 7 | covered |
| A2O scenarios | Task 8 | covered |
| Rollout (flag default off, matthew+adam first, cluster-wide, drop flag after week) | Task 9 Step 9.5 (operator coordination) | covered |
| 24-hour soak metrics | Task 9 Step 9.5 (operator handoff) | covered |
| Error-handling table from spec section 4 | Tasks 3+4 (try_recv drops on closed channel; warn-and-retry on admin RPC failure) | covered |

No gaps.

**2. Placeholder scan**

Searched plan body — no instances of "TBD", "TODO", "implement later", "similar to Task N" (each task is self-contained with full code), "add appropriate error handling" (error paths are explicit in each step).

**3. Type consistency**

- `ConductorAgentInfo` fields: `agent_info_json: String`, `published_at: i64` — consistent across Tasks 1, 2, 3, 4, 7.
- `CONDUCTOR_AGENT_INFO_TOPIC: &str` — used identically in Tasks 1, 4, 7.
- `P2PCommand::PublishConductorAgentInfo(ConductorAgentInfo)` — variant name + payload type consistent in Tasks 2, 4.
- `SubscriberConfig` fields + defaults — consistent in Tasks 3, 5.
- `spawn_agent_info_publisher` / `spawn_agent_info_subscriber_worker` — names consistent across Tasks 2, 3, 5.
- `agent_info_inbound_tx: Option<Sender<ConductorAgentInfo>>` — consistent between Tasks 4 (P2PNode field) and 5 (main.rs holder).

**4. Implementation-time ambiguities (caveats for the executor, not gaps in the plan)**

- **`shutdown_token` vs `shutdown_tx`**: The plan uses `CancellationToken` semantics; the existing codebase uses `tokio::sync::broadcast::channel::<()>`. Task 5 Step 5.6 flags this — adapt either by wrapping in CancellationToken or accepting `broadcast::Receiver<()>` in spawn signatures. The publisher/subscriber worker bodies' `tokio::select!` arm currently uses `shutdown.cancelled()` — change to `shutdown.recv()` if using broadcast.
- **kitsune2 v2 JSON field name for agent_pub_key**: substring match works regardless of field name. Implementation-time confirmation by inspecting a real `agent_info()` return value documents which field carries the pubkey for the comment. The plan is correct as-is; the comment in Task 2 Step 2.1 (`json_contains_any_own_key`) honestly notes this.
- **P2PNode vs P2PWorker struct name** in Task 4 Step 4.5: grep for `peer_identity_bindings_writer` to find the actual struct; the comment instructs the executor to add the field there.

These ambiguities are surface-level (naming/location) — not architectural. Each task step that touches them tells the executor exactly what to grep for + how to adapt.

---

## Execution Handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-28-conductor-agent-info-substrate-gossip.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

---

## Sibling Federation Gaps (out of scope for this plan; named for the EPR-delivery handoff)

Step-zero substrate gossip closes **cross-cluster DHT propagation** — necessary for any cross-doorway delivery story to work. It does NOT close the following concurrent gaps that the doorway-A / doorway-B EPR-delivery story also needs once Wave 3 bundle-split (`ce945bdfd`) lands on dev. Track these as sibling backlog items, not as additions to this plan's tasks.

| Gap | What it is | File / evidence | Fix shape (one option) |
|---|---|---|---|
| **F1 — Remote-receive projection-commitment visibility** | `project-epr` commitments only project on the LOCAL author. Holochain's `post_commit` fires only on local commits (per `dna/CLAUDE.md` gospel); remote peers receive the DHT entry via gossip but never get a `ProjectionSignal::ReaCommitmentCommitted`, so `rea_commitments` table stays empty on every doorway that didn't author. Result: remote doorway's `EprRouter` has no row → `/lamad` → 404 even after this plan's substrate-gossip lands. | DNA `content_store/src/lib.rs:10613` (post_commit iterates `committed_actions` — local only); `elohim-storage/src/rea_projection.rs:336-361`; `elohim-storage/src/services/rea_commitment_service.rs:62,106,177,214` (only emit-sites for `StorageEvent::ProjectionRegistered`) | Seeder dual-POST (POST each projection to BOTH doorways; content-addressed `id` collapses duplicates via 409) **OR** DHT-poll projector that `get_links`-walks `project-epr` anchors scoped to `doorwayId=self` every N seconds and upserts newly-seen commitments + emits SSE. |
| **F2 — Jenkins seeds ONE doorway** | `Seed Projections` stage seeds only `RESOLVED_DOORWAY_HOST` (one doorway per branch family — `doorway-alpha.elohim.host` on dev/feat/claude; `doorway.elohim.host` on main). Default spec set declares projections for BOTH doorways but all land in one storage's `rea_commitments`. | `genesis/Jenkinsfile:355` (`def doorwayHost = env.RESOLVED_DOORWAY_HOST`); `genesis/seeder/src/seed-projections.ts:206-214` (default set is bi-doorway) | Loop the seeder invocation across both doorway hosts in the same stage. ~5-line Jenkinsfile change. Required if F1's chosen fix is dual-POST; redundant-but-harmless if F1's chosen fix is the DHT-poll projector. |
| **F4 — Bundle blob reachability cross-doorway** | Even with EPR projections live on both doorways, the lamad bundle's blob chunks served by doorway-B's storage must already be in doorway-B's pantry. Per `project_inventory_exchange_not_byte_replication`, inventory gossip is in place but byte replication is partial. | self-healing-p2p-dataplane spec (Plans 1+2+3) | Out of this plan's scope but adjacent — flag for tracking. |

EPR delivery cross-doorway requires the union of: **this plan** (substrate-gossip propagates entries cross-cluster) + **F1** (remote storage actually projects them) + **F2** (both doorways are seeded as authors, if F1=dual-POST) + **F4** (the blobs reach the remote doorway). Substrate-gossip unblocks the others; it does not close them.

The HyperCard endpoint (`GET /api/v1/epr/{id}`) and any read-through atom resolution work as a side effect of this plan closing (no additional federation work needed — the existing conductor `get(entry_hash)` path resolves once DHT propagation is live).
