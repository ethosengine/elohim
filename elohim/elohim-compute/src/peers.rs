//! Per-peer health tracking registry.

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
