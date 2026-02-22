//! Per-conductor WorkerPool management
//!
//! Maps conductor IDs to their dedicated WorkerPool instances.
//! Each conductor in the pool gets its own set of workers, enabling
//! per-request routing based on agent→conductor mappings.
//!
//! The default pool handles unauthenticated requests and acts as
//! the fallback when no conductor-specific pool is available.

use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::worker::{PoolMetrics, WorkerPool};

/// Maps conductor IDs to per-conductor WorkerPool instances with agent tracking
pub struct ConductorPoolMap {
    /// conductor_id → WorkerPool
    pools: DashMap<String, Arc<WorkerPool>>,
    /// conductor_id → current agent count
    agent_counts: DashMap<String, Arc<AtomicUsize>>,
    /// Fallback pool for unauthenticated requests or unknown conductors
    default_pool: Arc<WorkerPool>,
    /// Maximum agents per conductor before overflow routing
    max_agents_per_conductor: usize,
}

/// Summary of a single conductor pool's health and load
#[derive(Debug, Clone)]
pub struct ConductorPoolStatus {
    /// Conductor identifier
    pub conductor_id: String,
    /// Whether the pool has at least one connected worker
    pub healthy: bool,
    /// Number of agents currently assigned to this conductor
    pub agent_count: usize,
    /// Maximum agents allowed on this conductor
    pub capacity_max: usize,
    /// Worker pool metrics (utilization, queue depth, error rate)
    pub pool_metrics: PoolMetrics,
}

/// Default maximum agents per conductor
const DEFAULT_MAX_AGENTS_PER_CONDUCTOR: usize = 50;

impl ConductorPoolMap {
    /// Create a new pool map with a default fallback pool
    pub fn new(default_pool: Arc<WorkerPool>) -> Self {
        Self {
            pools: DashMap::new(),
            agent_counts: DashMap::new(),
            default_pool,
            max_agents_per_conductor: DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
        }
    }

    /// Create a new pool map with a custom agent capacity limit
    pub fn with_capacity(default_pool: Arc<WorkerPool>, max_agents_per_conductor: usize) -> Self {
        Self {
            pools: DashMap::new(),
            agent_counts: DashMap::new(),
            default_pool,
            max_agents_per_conductor,
        }
    }

    /// Add a pool for a specific conductor
    pub fn add_pool(&self, conductor_id: &str, pool: Arc<WorkerPool>) {
        self.pools.insert(conductor_id.to_string(), pool);
        self.agent_counts
            .entry(conductor_id.to_string())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)));
    }

    /// Get the pool for a specific conductor
    pub fn get_pool(&self, conductor_id: &str) -> Option<Arc<WorkerPool>> {
        self.pools.get(conductor_id).map(|p| p.value().clone())
    }

    /// Get the default pool (for unauthenticated requests)
    pub fn default_pool(&self) -> Arc<WorkerPool> {
        Arc::clone(&self.default_pool)
    }

    /// Increment agent count for a conductor. Returns the new count.
    pub fn increment_agents(&self, conductor_id: &str) -> usize {
        let counter = self
            .agent_counts
            .entry(conductor_id.to_string())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)));
        counter.value().fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement agent count for a conductor. Returns the new count.
    pub fn decrement_agents(&self, conductor_id: &str) -> usize {
        if let Some(counter) = self.agent_counts.get(conductor_id) {
            let prev = counter.value().fetch_sub(1, Ordering::Relaxed);
            prev.saturating_sub(1)
        } else {
            0
        }
    }

    /// Get current agent count for a conductor
    pub fn agent_count(&self, conductor_id: &str) -> usize {
        self.agent_counts
            .get(conductor_id)
            .map(|c| c.value().load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Set the agent count for a conductor (e.g., after loading from registry)
    pub fn set_agent_count(&self, conductor_id: &str, count: usize) {
        let counter = self
            .agent_counts
            .entry(conductor_id.to_string())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)));
        counter.value().store(count, Ordering::Relaxed);
    }

    /// Check if a conductor has capacity for more agents
    pub fn has_capacity(&self, conductor_id: &str) -> bool {
        self.agent_count(conductor_id) < self.max_agents_per_conductor
    }

    /// Get the healthiest pool: the healthy conductor with the lowest agent count
    /// and available capacity. Returns None if no healthy conductor has capacity.
    pub fn get_healthiest_pool(&self) -> Option<(String, Arc<WorkerPool>)> {
        self.pools
            .iter()
            .filter(|entry| entry.value().is_healthy())
            .filter(|entry| self.has_capacity(entry.key()))
            .min_by_key(|entry| self.agent_count(entry.key()))
            .map(|entry| (entry.key().clone(), entry.value().clone()))
    }

    /// Count how many conductor pools have at least one connected worker
    pub fn healthy_count(&self) -> usize {
        self.pools
            .iter()
            .filter(|entry| entry.value().is_healthy())
            .count()
    }

    /// Total number of conductor pools (excluding default)
    pub fn total_count(&self) -> usize {
        self.pools.len()
    }

    /// Get the maximum agents per conductor limit
    pub fn max_agents_per_conductor(&self) -> usize {
        self.max_agents_per_conductor
    }

    /// Get status for all conductor pools
    pub fn all_pool_statuses(&self) -> Vec<ConductorPoolStatus> {
        self.pools
            .iter()
            .map(|entry| {
                let conductor_id = entry.key().clone();
                let pool = entry.value();
                ConductorPoolStatus {
                    conductor_id: conductor_id.clone(),
                    healthy: pool.is_healthy(),
                    agent_count: self.agent_count(&conductor_id),
                    capacity_max: self.max_agents_per_conductor,
                    pool_metrics: pool.metrics(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::PoolConfig;

    // Note: WorkerPool::new requires a running conductor, so we test
    // the pool map logic separately from actual conductor connections.

    #[test]
    fn test_pool_map_counts() {
        let _config = PoolConfig {
            worker_count: 2,
            conductor_url: "ws://test:4445".to_string(),
            request_timeout_ms: 5000,
            max_queue_size: 100,
        };
        // Pool creation requires async + running conductor
        // Integration tests should verify full flow
    }

    #[test]
    fn test_agent_count_tracking() {
        // We can't create real WorkerPools, but we can test the
        // agent counting logic directly on the DashMap.
        let counts: DashMap<String, Arc<AtomicUsize>> = DashMap::new();
        counts.insert("cond-0".to_string(), Arc::new(AtomicUsize::new(0)));
        counts.insert("cond-1".to_string(), Arc::new(AtomicUsize::new(0)));

        // Simulate incrementing
        let c0 = counts.get("cond-0").unwrap();
        c0.value().fetch_add(1, Ordering::Relaxed);
        c0.value().fetch_add(1, Ordering::Relaxed);
        assert_eq!(c0.value().load(Ordering::Relaxed), 2);

        let c1 = counts.get("cond-1").unwrap();
        c1.value().fetch_add(1, Ordering::Relaxed);
        assert_eq!(c1.value().load(Ordering::Relaxed), 1);

        // cond-1 has fewer agents
        let least_loaded = counts
            .iter()
            .min_by_key(|e| e.value().load(Ordering::Relaxed))
            .map(|e| e.key().clone());
        assert_eq!(least_loaded, Some("cond-1".to_string()));
    }

    #[test]
    fn test_capacity_check() {
        let counts: DashMap<String, Arc<AtomicUsize>> = DashMap::new();
        let max = 50usize;

        counts.insert("cond-0".to_string(), Arc::new(AtomicUsize::new(49)));
        counts.insert("cond-1".to_string(), Arc::new(AtomicUsize::new(50)));

        let cond0_has_cap = counts
            .get("cond-0")
            .map(|c| c.value().load(Ordering::Relaxed) < max)
            .unwrap_or(false);
        assert!(cond0_has_cap);

        let cond1_has_cap = counts
            .get("cond-1")
            .map(|c| c.value().load(Ordering::Relaxed) < max)
            .unwrap_or(false);
        assert!(!cond1_has_cap);
    }
}
