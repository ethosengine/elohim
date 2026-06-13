//! Per-peer health tracking registry.
//!
//! Snapshots are not point-in-time consistent across fields. A concurrent
//! `record_signal()` between reading `signals_received` and `health` may
//! cause the snapshot to show a signal count that hasn't yet been reflected
//! in the health state. Acceptable for operator dashboards; automated
//! decisions should treat snapshots as eventually-consistent.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::ServiceHealth;

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

struct PeerEntry {
    address: String,
    health: RwLock<ServiceHealth>,
    reason: RwLock<String>,
    signals_received: AtomicU64,
    last_signal_at: RwLock<Option<DateTime<Utc>>>,
    reconnect_attempts: AtomicU32,
}

pub struct PeerHealthRegistry {
    peers: DashMap<String, PeerEntry>,
}

impl PeerHealthRegistry {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
        }
    }

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

    pub fn record_signal(&self, peer_id: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            entry.signals_received.fetch_add(1, Ordering::Relaxed);
            *entry.last_signal_at.write() = Some(Utc::now());
        }
    }

    pub fn update_health(&self, peer_id: &str, health: ServiceHealth, reason: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            *entry.health.write() = health;
            *entry.reason.write() = reason.to_string();
            if health == ServiceHealth::Healthy {
                entry.reconnect_attempts.store(0, Ordering::Relaxed);
            }
        }
    }

    pub fn record_reconnect(&self, peer_id: &str) {
        if let Some(entry) = self.peers.get(peer_id) {
            entry.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
            *entry.health.write() = ServiceHealth::Degraded;
            *entry.reason.write() = "reconnecting".to_string();
        }
    }

    pub fn active_count(&self) -> usize {
        self.peers
            .iter()
            .filter(|e| *e.value().health.read() == ServiceHealth::Healthy)
            .count()
    }

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

/// Operational circuit state for an upstream. Cat C node-local — no DHT entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Pure, deterministic per-upstream circuit breaker.
///
/// Advanced ONLY by injected outcomes + a monotonic `tick` (a warm-up pass
/// counter), never wall-clock — so the state machine is unit-testable without
/// time or network. Opens after `fail_threshold` consecutive failures; stays
/// open for `cooldown_ticks`; then admits exactly ONE half-open trial.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    consecutive_failures: u32,
    opened_at_tick: Option<u64>,
    error_streak: u32,
    last_good_tick: Option<u64>,
    fail_threshold: u32,
    cooldown_ticks: u64,
}

impl CircuitBreaker {
    pub fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_tick: None,
            error_streak: 0,
            last_good_tick: None,
            fail_threshold: fail_threshold.max(1),
            cooldown_ticks,
        }
    }

    pub fn record_outcome(&mut self, ok: bool, tick: u64) {
        if ok {
            self.consecutive_failures = 0;
            self.error_streak = 0;
            self.last_good_tick = Some(tick);
            self.state = CircuitState::Closed;
            self.opened_at_tick = None;
        } else {
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            self.error_streak = self.error_streak.saturating_add(1);
            if self.state == CircuitState::HalfOpen
                || self.consecutive_failures >= self.fail_threshold
            {
                self.state = CircuitState::Open;
                self.opened_at_tick = Some(tick);
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }

    /// Returns true if this upstream should be skipped on `tick`.
    /// Side effect: an Open breaker whose cooldown has elapsed transitions to
    /// HalfOpen and admits exactly one trial (returns false once); subsequent
    /// calls return true until an outcome is recorded.
    pub fn should_skip(&mut self, tick: u64) -> bool {
        match self.state {
            CircuitState::Closed => false,
            CircuitState::HalfOpen => true, // trial already admitted; await outcome
            CircuitState::Open => {
                let elapsed = self
                    .opened_at_tick
                    .map(|t| tick.saturating_sub(t))
                    .unwrap_or(u64::MAX);
                if elapsed >= self.cooldown_ticks {
                    self.state = CircuitState::HalfOpen;
                    false // admit the one trial
                } else {
                    true
                }
            }
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn error_streak(&self) -> u32 {
        self.error_streak
    }

    pub fn last_good_tick(&self) -> Option<u64> {
        self.last_good_tick
    }
}

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

    #[test]
    fn circuit_opens_after_k_consecutive_failures() {
        let mut cb = CircuitBreaker::new(3, 5);
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_outcome(false, 1);
        cb.record_outcome(false, 2);
        assert!(!cb.is_open(), "2 failures < K=3 should stay closed");
        cb.record_outcome(false, 3);
        assert!(cb.is_open(), "3rd consecutive failure must open");
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.error_streak(), 3);
    }

    #[test]
    fn circuit_success_resets_streak() {
        let mut cb = CircuitBreaker::new(3, 5);
        cb.record_outcome(false, 1);
        cb.record_outcome(false, 2);
        cb.record_outcome(true, 3);
        assert!(!cb.is_open());
        assert_eq!(cb.error_streak(), 0);
        assert_eq!(cb.last_good_tick(), Some(3));
    }

    #[test]
    fn circuit_should_skip_open_until_cooldown_then_halfopen_one_trial() {
        let mut cb = CircuitBreaker::new(1, 5);
        cb.record_outcome(false, 10); // opens at tick 10
        assert!(cb.is_open());
        assert!(cb.should_skip(11), "within cooldown: skip");
        assert!(cb.should_skip(14), "tick 14, elapsed 4 < 5: skip");
        assert!(
            !cb.should_skip(15),
            "tick 15, elapsed 5 >= cooldown: half-open admits one trial"
        );
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // A second should_skip while HalfOpen does NOT re-admit (one trial outstanding)
        assert!(
            cb.should_skip(16),
            "half-open trial already admitted: skip until outcome recorded"
        );
    }

    #[test]
    fn circuit_halfopen_success_closes_failure_reopens() {
        let mut cb = CircuitBreaker::new(1, 5);
        cb.record_outcome(false, 0); // open at 0
        assert!(!cb.should_skip(5)); // half-open
        cb.record_outcome(true, 6);
        assert_eq!(cb.state(), CircuitState::Closed);
        // reopen path
        let mut cb2 = CircuitBreaker::new(1, 5);
        cb2.record_outcome(false, 0);
        assert!(!cb2.should_skip(5));
        cb2.record_outcome(false, 6);
        assert_eq!(
            cb2.state(),
            CircuitState::Open,
            "half-open failure re-opens"
        );
    }

    #[test]
    fn circuit_state_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&CircuitState::HalfOpen).unwrap(),
            "\"half-open\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitState::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitState::Closed).unwrap(),
            "\"closed\""
        );
    }
}
