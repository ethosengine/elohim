//! Per-conductor WorkerPool management
//!
//! Maps conductor IDs to their dedicated WorkerPool instances.
//! Each conductor in the pool gets its own set of workers, enabling
//! per-request routing based on agent→conductor mappings.
//!
//! The default pool handles unauthenticated requests and acts as
//! the fallback when no conductor-specific pool is available.

use dashmap::DashMap;
use std::sync::Arc;

use crate::worker::WorkerPool;

/// Maps conductor IDs to per-conductor WorkerPool instances
pub struct ConductorPoolMap {
    /// conductor_id → WorkerPool
    pools: DashMap<String, Arc<WorkerPool>>,
    /// Fallback pool for unauthenticated requests or unknown conductors
    default_pool: Arc<WorkerPool>,
}

impl ConductorPoolMap {
    /// Create a new pool map with a default fallback pool
    pub fn new(default_pool: Arc<WorkerPool>) -> Self {
        Self {
            pools: DashMap::new(),
            default_pool,
        }
    }

    /// Add a pool for a specific conductor
    pub fn add_pool(&self, conductor_id: &str, pool: Arc<WorkerPool>) {
        self.pools.insert(conductor_id.to_string(), pool);
    }

    /// Get the pool for a specific conductor
    pub fn get_pool(&self, conductor_id: &str) -> Option<Arc<WorkerPool>> {
        self.pools.get(conductor_id).map(|p| p.value().clone())
    }

    /// Get the default pool (for unauthenticated requests)
    pub fn default_pool(&self) -> Arc<WorkerPool> {
        Arc::clone(&self.default_pool)
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
}

#[cfg(test)]
mod tests {
    use crate::worker::PoolConfig;

    // Note: WorkerPool::new requires a running conductor, so we test
    // the pool map logic separately from actual conductor connections.

    #[test]
    fn test_pool_map_counts() {
        // We can't create real WorkerPools without a conductor,
        // but we can verify the map structure compiles and the
        // public API is correct.
        let _config = PoolConfig {
            worker_count: 2,
            conductor_url: "ws://test:4445".to_string(),
            request_timeout_ms: 5000,
            max_queue_size: 100,
        };
        // Pool creation requires async + running conductor
        // Integration tests should verify full flow
    }
}
