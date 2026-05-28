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

// ---------------------------------------------------------------------------
// Publisher
// ---------------------------------------------------------------------------

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use holochain_client::AdminWebsocket;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::p2p::P2PCommand;

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
                .encode(cid.agent_pubkey().get_raw_39())
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
///
/// Shutdown via `broadcast::Receiver<()>` matches the codebase convention
/// (see `p2p::P2PWorker::run` in `p2p/mod.rs`).
pub fn spawn_agent_info_publisher(
    admin_ws: Arc<AdminWebsocket>,
    p2p_tx: Sender<P2PCommand>,
    interval: Duration,
    mut shutdown: broadcast::Receiver<()>,
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

        info!(
            target: "elohim_storage::agent_info",
            interval_secs = interval.as_secs(),
            "agent_info publisher entering heartbeat loop"
        );

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!(target: "elohim_storage::agent_info", "publisher shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = publish_once(&admin_ws, &p2p_tx).await {
                        warn!(
                            target: "elohim_storage::agent_info",
                            error = %e,
                            "publish tick failed; will retry next interval"
                        );
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

// ---------------------------------------------------------------------------
// Subscriber worker (pull-based)
// ---------------------------------------------------------------------------

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

/// Spawn the subscriber worker task. Owns the receiver end of the bounded
/// mpsc, drains it on a tick, batches into a single `add_agent_info` RPC per
/// drain, and rate-limits to `cfg.max_rate_per_sec` envelopes/sec.
///
/// The gossip-edge handler that try_sends into the mpsc lives in `p2p/mod.rs`
/// — see the `CONDUCTOR_AGENT_INFO_TOPIC` arm in the swarm event loop.
///
/// Shutdown via `broadcast::Receiver<()>` matches the codebase convention
/// (see `p2p::P2PWorker::run` in `p2p/mod.rs`).
pub fn spawn_agent_info_subscriber_worker(
    admin_ws: Arc<AdminWebsocket>,
    mut rx: Receiver<ConductorAgentInfo>,
    cfg: SubscriberConfig,
    mut shutdown: broadcast::Receiver<()>,
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
                _ = shutdown.recv() => {
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
                        Ok(()) => debug!(
                            target: "elohim_storage::agent_info",
                            batch_size = n,
                            "add_agent_info batch ingested"
                        ),
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
        // ~150-byte JSON payload + ~30 bytes envelope ≈ 180 bytes for this fixture.
        // Bound of 512 covers real kitsune2 agent_info (~400-600 bytes typical) with
        // some slack for unusual cases — but tight enough to catch runaway field
        // additions in future revisions.
        assert!(bytes.len() < 512, "payload should fit in 512B; got {} bytes", bytes.len());
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

    #[test]
    fn verify_structural_rejects_negative_timestamp() {
        let mut bad = sample();
        bad.published_at = -1;
        assert!(bad.verify_structural().is_err());
    }

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
        assert!(!json_contains_any_own_key(
            r#"{"agent":"uhCAk_alice_pk"}"#,
            &own
        ));
    }

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
        older.published_at -= 1_000_000;
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
        let mut lim = RateLimiter::new(10);
        let start = tokio::time::Instant::now();
        lim.acquire(10).await;
        lim.acquire(10).await;
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 900, "burst should take ≥900ms; got {}ms", elapsed);
        assert!(elapsed < 1500, "burst shouldn't take >1.5s; got {}ms", elapsed);
    }

    #[test]
    fn subscriber_config_defaults_are_sane() {
        let d = SubscriberConfig::default();
        assert_eq!(d.queue_capacity, 256);
        assert_eq!(d.max_rate_per_sec, 20);
        assert_eq!(d.max_batch, 32);
        assert_eq!(d.batch_interval, Duration::from_millis(200));
    }
}
