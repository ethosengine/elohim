//! Host routing and selection for multi-operator support
//!
//! Implements host discovery and load balancing via NATS.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db::schemas::{HostDoc, HostStatus};
use crate::nats::{HcWsRequest, NatsClient};

/// How long before a host is considered stale without heartbeat
const HOST_STALE_THRESHOLD: Duration = Duration::from_secs(60);

/// Router for selecting hosts to handle requests
#[derive(Clone)]
pub struct HostRouter {
    /// Cached list of available hosts
    hosts: Arc<RwLock<HashMap<String, CachedHost>>>,
    /// Last time the host list was refreshed (for future NATS-based refresh)
    #[allow(dead_code)]
    last_refresh: Arc<RwLock<Option<Instant>>>,
    /// NATS client for messaging (for future NATS-based routing)
    #[allow(dead_code)]
    nats: Option<NatsClient>,
}

/// Cached host information
#[derive(Debug, Clone)]
struct CachedHost {
    /// Host document from database
    host: HostDoc,
    /// When this entry was last updated
    last_seen: Instant,
}

impl HostRouter {
    /// Create a new host router
    pub fn new(nats: Option<NatsClient>) -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            last_refresh: Arc::new(RwLock::new(None)),
            nats,
        }
    }

    /// Select a host for the given request
    pub async fn select_host(&self, request: &HcWsRequest) -> Option<HostDoc> {
        let hosts = self.hosts.read().await;

        // If specific host requested, try to use it
        if let Some(ref target) = request.target_host {
            if let Some(cached) = hosts.get(target) {
                if cached.host.is_available() && !self.is_stale(cached) {
                    debug!("Selected requested host: {}", target);
                    return Some(cached.host.clone());
                }
            }
            warn!("Requested host {} not available", target);
        }

        // Filter available hosts
        let available: Vec<_> = hosts
            .values()
            .filter(|c| c.host.is_available() && !self.is_stale(c))
            .collect();

        if available.is_empty() {
            warn!("No available hosts");
            return None;
        }

        // Try region preference
        if let Some(ref region) = request.preferred_region {
            let regional: Vec<_> = available
                .iter()
                .filter(|c| c.host.region.as_ref() == Some(region))
                .copied()
                .collect();

            if !regional.is_empty() {
                let host = self.select_least_loaded(&regional);
                debug!("Selected regional host {} in {}", host.node_id, region);
                return Some(host);
            }
        }

        // Fall back to least loaded
        let host = self.select_least_loaded(&available);
        debug!("Selected host {} (least loaded)", host.node_id);
        Some(host)
    }

    /// Select the host with fewest connections
    fn select_least_loaded(&self, hosts: &[&CachedHost]) -> HostDoc {
        hosts
            .iter()
            .min_by_key(|c| c.host.active_connections)
            .map(|c| c.host.clone())
            .expect("hosts should not be empty")
    }

    /// Check if a cached host entry is stale
    fn is_stale(&self, cached: &CachedHost) -> bool {
        cached.last_seen.elapsed() > HOST_STALE_THRESHOLD
    }

    /// Register or update a host
    pub async fn register_host(&self, host: HostDoc) {
        let mut hosts = self.hosts.write().await;
        let node_id = host.node_id.clone();

        hosts.insert(
            node_id.clone(),
            CachedHost {
                host,
                last_seen: Instant::now(),
            },
        );

        info!("Registered host: {}", node_id);
    }

    /// Remove a host from the router
    pub async fn deregister_host(&self, node_id: &str) {
        let mut hosts = self.hosts.write().await;
        if hosts.remove(node_id).is_some() {
            info!("Deregistered host: {}", node_id);
        }
    }

    /// Update host connection count
    pub async fn update_connections(&self, node_id: &str, active: i32) {
        let mut hosts = self.hosts.write().await;
        if let Some(cached) = hosts.get_mut(node_id) {
            cached.host.active_connections = active;
            cached.last_seen = Instant::now();
        }
    }

    /// Update host status
    pub async fn update_status(&self, node_id: &str, status: HostStatus) {
        let mut hosts = self.hosts.write().await;
        if let Some(cached) = hosts.get_mut(node_id) {
            cached.host.status = status;
            cached.last_seen = Instant::now();
        }
    }

    /// Refresh from heartbeat
    pub async fn handle_heartbeat(&self, node_id: &str, active_connections: i32) {
        let mut hosts = self.hosts.write().await;
        if let Some(cached) = hosts.get_mut(node_id) {
            cached.host.active_connections = active_connections;
            cached.host.status = HostStatus::Online;
            cached.last_seen = Instant::now();
            debug!(
                "Heartbeat from {}: {} connections",
                node_id, active_connections
            );
        }
    }

    /// Get list of all registered hosts
    pub async fn list_hosts(&self) -> Vec<HostDoc> {
        let hosts = self.hosts.read().await;
        hosts.values().map(|c| c.host.clone()).collect()
    }

    /// Get count of available hosts
    pub async fn available_count(&self) -> usize {
        let hosts = self.hosts.read().await;
        hosts
            .values()
            .filter(|c| c.host.is_available() && !self.is_stale(c))
            .count()
    }

    /// Clean up stale hosts
    pub async fn cleanup_stale(&self) {
        let mut hosts = self.hosts.write().await;
        let stale: Vec<_> = hosts
            .iter()
            .filter(|(_, c)| self.is_stale(c))
            .map(|(k, _)| k.clone())
            .collect();

        for node_id in stale {
            info!("Removing stale host: {}", node_id);
            hosts.remove(&node_id);
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                self.cleanup_stale().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_host(node_id: &str, connections: i32) -> HostDoc {
        let mut host = HostDoc::new(
            node_id.to_string(),
            node_id.to_string(),
            format!("{}.example.com", node_id),
            "ws://localhost:4444".to_string(),
            4445,
            65535,
        );
        host.active_connections = connections;
        host.max_connections = 100;
        host
    }

    #[tokio::test]
    async fn test_register_and_select() {
        let router = HostRouter::new(None);

        router.register_host(test_host("host-1", 10)).await;
        router.register_host(test_host("host-2", 20)).await;

        let request = HcWsRequest::new(vec![]);
        let selected = router.select_host(&request).await.unwrap();

        // Should select host-1 (fewer connections)
        assert_eq!(selected.node_id, "host-1");
    }

    #[tokio::test]
    async fn test_sticky_session() {
        let router = HostRouter::new(None);

        router.register_host(test_host("host-1", 10)).await;
        router.register_host(test_host("host-2", 5)).await;

        // Request specific host even though host-2 has fewer connections
        let request = HcWsRequest::new(vec![]).with_target_host("host-1".to_string());
        let selected = router.select_host(&request).await.unwrap();

        assert_eq!(selected.node_id, "host-1");
    }

    #[tokio::test]
    async fn test_region_preference() {
        let router = HostRouter::new(None);

        let mut host1 = test_host("host-1", 10);
        host1.region = Some("us-west".to_string());

        let mut host2 = test_host("host-2", 5);
        host2.region = Some("us-east".to_string());

        router.register_host(host1).await;
        router.register_host(host2).await;

        // Request us-west region
        let request = HcWsRequest::new(vec![]).with_region("us-west".to_string());
        let selected = router.select_host(&request).await.unwrap();

        assert_eq!(selected.node_id, "host-1");
    }

    #[tokio::test]
    async fn test_no_available_hosts() {
        let router = HostRouter::new(None);

        let request = HcWsRequest::new(vec![]);
        let selected = router.select_host(&request).await;

        assert!(selected.is_none());
    }
}
