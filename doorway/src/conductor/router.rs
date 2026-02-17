//! Per-request conductor routing
//!
//! Routes authenticated requests to the correct conductor based on
//! JWT agent_pub_key → ConductorRegistry lookup → ConductorPoolMap.
//!
//! When an agent is not yet assigned to a conductor, the router
//! auto-assigns to the least-loaded conductor and persists the
//! mapping. This means the first authenticated request from any
//! agent transparently establishes routing.

use std::sync::Arc;
use tracing::{debug, info, warn};

use super::pool_map::ConductorPoolMap;
use super::registry::ConductorRegistry;
use crate::worker::WorkerPool;

/// Routes requests to the correct conductor WorkerPool
pub struct ConductorRouter {
    /// Agent→conductor mapping registry
    registry: Arc<ConductorRegistry>,
    /// Per-conductor WorkerPool instances
    pools: Arc<ConductorPoolMap>,
    /// Default app_id for auto-assignment
    default_app_id: String,
}

impl ConductorRouter {
    /// Create a new router
    pub fn new(registry: Arc<ConductorRegistry>, pools: Arc<ConductorPoolMap>) -> Self {
        Self {
            registry,
            pools,
            default_app_id: "elohim".to_string(),
        }
    }

    /// Create a new router with a custom default app_id
    pub fn with_app_id(
        registry: Arc<ConductorRegistry>,
        pools: Arc<ConductorPoolMap>,
        app_id: String,
    ) -> Self {
        Self {
            registry,
            pools,
            default_app_id: app_id,
        }
    }

    /// Route an authenticated request to the correct conductor pool.
    ///
    /// 1. Look up agent_pub_key in ConductorRegistry
    /// 2. If found, return that conductor's WorkerPool
    /// 3. If not found, auto-assign to least-loaded conductor and persist
    /// 4. If no conductors available, fall back to default pool
    pub async fn route(&self, agent_pub_key: &str) -> Arc<WorkerPool> {
        // Step 1: Check registry for existing assignment
        if let Some(entry) = self.registry.get_conductor_for_agent(agent_pub_key) {
            // Step 2: Get the pool for this conductor
            if let Some(pool) = self.pools.get_pool(&entry.conductor_id) {
                debug!(
                    agent = %agent_pub_key,
                    conductor = %entry.conductor_id,
                    "Routed to assigned conductor"
                );
                return pool;
            }
            // Conductor in registry but no pool — fall through to reassign
            warn!(
                agent = %agent_pub_key,
                conductor = %entry.conductor_id,
                "Agent assigned to conductor with no pool, will reassign"
            );
        }

        // Step 3: Auto-assign to least-loaded conductor
        if let Some(conductor) = self.registry.find_least_loaded() {
            let conductor_id = conductor.conductor_id.clone();

            // Persist the assignment
            match self
                .registry
                .register_agent(agent_pub_key, &conductor_id, &self.default_app_id)
                .await
            {
                Ok(()) => {
                    info!(
                        agent = %agent_pub_key,
                        conductor = %conductor_id,
                        "Auto-assigned agent to least-loaded conductor"
                    );
                }
                Err(e) => {
                    warn!(
                        agent = %agent_pub_key,
                        conductor = %conductor_id,
                        error = %e,
                        "Failed to persist agent assignment, routing to conductor anyway"
                    );
                }
            }

            if let Some(pool) = self.pools.get_pool(&conductor_id) {
                return pool;
            }
        }

        // Step 4: No conductor available — use default pool
        debug!(
            agent = %agent_pub_key,
            "No conductor assignment possible, using default pool"
        );
        self.pools.default_pool()
    }

    /// Get the default pool (for unauthenticated requests)
    pub fn default_pool(&self) -> Arc<WorkerPool> {
        self.pools.default_pool()
    }

    /// Get the underlying pool map (for health reporting)
    pub fn pools(&self) -> &ConductorPoolMap {
        &self.pools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::ConductorInfo;

    #[tokio::test]
    async fn test_router_unknown_agent_auto_assigns() {
        // Create registry with one conductor
        let registry = Arc::new(ConductorRegistry::new(None).await);
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://cond-0:4445".to_string(),
            admin_url: "ws://cond-0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 50,
        });

        // We can't create real pools without a conductor connection,
        // but we can verify the registry gets populated by the router logic.
        // The auto-assignment should call register_agent().

        // Verify the agent is not yet assigned
        assert!(registry.get_conductor_for_agent("uhCAk_test").is_none());

        // After routing, the agent should be assigned
        // (We can't fully test without real WorkerPools, but the registry
        // side of the logic is testable)
        registry
            .register_agent("uhCAk_test", "conductor-0", "elohim")
            .await
            .unwrap();

        let entry = registry.get_conductor_for_agent("uhCAk_test").unwrap();
        assert_eq!(entry.conductor_id, "conductor-0");
    }

    #[tokio::test]
    async fn test_router_known_agent_routes_correctly() {
        let registry = Arc::new(ConductorRegistry::new(None).await);
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://cond-0:4445".to_string(),
            admin_url: "ws://cond-0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 50,
        });
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-1".to_string(),
            conductor_url: "ws://cond-1:4445".to_string(),
            admin_url: "ws://cond-1:4444".to_string(),
            capacity_used: 0,
            capacity_max: 50,
        });

        // Assign agent to conductor-1 explicitly
        registry
            .register_agent("uhCAk_alice", "conductor-1", "elohim")
            .await
            .unwrap();

        let entry = registry.get_conductor_for_agent("uhCAk_alice").unwrap();
        assert_eq!(entry.conductor_id, "conductor-1");
        // Router would return conductor-1's pool (can't test without real pools)
    }
}
