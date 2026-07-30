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
use tracing::warn;

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

/// Default maximum agents per conductor (used when the env lever is unset or unparseable).
pub const DEFAULT_MAX_AGENTS_PER_CONDUCTOR: usize = 50;

/// Env lever overriding [`DEFAULT_MAX_AGENTS_PER_CONDUCTOR`] at startup.
///
/// # Why this lever exists
///
/// Kitsune2 budgets gossip **per space**, not per agent: a space runs one
/// outbound initiate round at a time, and a single local agent still below its
/// target storage arc holds the whole space in the slow initiate path. Every
/// cell's storage arc resets to `Empty` on conductor start, so arc-convergence
/// cost scales with the hosted-agent count while the convergence budget does
/// not. Past roughly 30 hosted agents on one conductor, arc convergence stalls
/// and the storage projection never reports `caught_up` (measured on the alpha
/// apex doorway: 1 of ~35 agents converged in 14.5h). Capping provisioning is
/// the operator's ceiling for that; see
/// `genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md`.
///
/// # Semantics
///
/// - unset → [`DEFAULT_MAX_AGENTS_PER_CONDUCTOR`]
/// - unparseable (non-numeric, negative) → [`DEFAULT_MAX_AGENTS_PER_CONDUCTOR`], logged at `warn!`
/// - `0` → honored as a deliberate provisioning freeze (see [`parse_max_agents_per_conductor`])
///
/// Read ONCE at startup (`main.rs`) and threaded into both capacity gates —
/// never read on a request path.
pub const MAX_AGENTS_PER_CONDUCTOR_ENV: &str = "DOORWAY_MAX_AGENTS_PER_CONDUCTOR";

/// Pure parse of the [`MAX_AGENTS_PER_CONDUCTOR_ENV`] value.
///
/// Split from the env read so it is testable without mutating process env
/// (`set_var` in one test leaks into every other test in the binary).
///
/// A value of `0` is honored, not clamped: it is the operator's explicit
/// "provision no new hosted agents on any conductor" freeze. Existing hosted
/// agents are unaffected — `AgentProvisioner::provision_agent` short-circuits
/// on `find_existing_app` BEFORE the capacity check, so a cap at or below the
/// current population stops growth without evicting or locking out anyone.
pub fn parse_max_agents_per_conductor(raw: Option<&str>) -> usize {
    match raw.map(str::trim) {
        None | Some("") => DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
        Some(v) => match v.parse::<usize>() {
            Ok(n) => n,
            Err(e) => {
                warn!(
                    env = MAX_AGENTS_PER_CONDUCTOR_ENV,
                    value = %v,
                    error = %e,
                    default = DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
                    "Unparseable agent cap; falling back to default"
                );
                DEFAULT_MAX_AGENTS_PER_CONDUCTOR
            }
        },
    }
}

/// Read the per-conductor agent cap from the environment.
///
/// Call this ONCE at startup and thread the result; do not call it per request.
pub fn max_agents_per_conductor_from_env() -> usize {
    parse_max_agents_per_conductor(std::env::var(MAX_AGENTS_PER_CONDUCTOR_ENV).ok().as_deref())
}

impl ConductorPoolMap {
    /// Create a new pool map at the compiled-in default capacity.
    ///
    /// This ctor does NOT read the environment. Startup code that wants the
    /// `DOORWAY_MAX_AGENTS_PER_CONDUCTOR` lever must use
    /// [`ConductorPoolMap::with_capacity`] with
    /// [`max_agents_per_conductor_from_env`].
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

    /// Check if a conductor has capacity for more agents.
    ///
    /// # Behavior at the cap
    ///
    /// This gate governs **request routing**, not provisioning. When it returns
    /// `false`, `ConductorRouter::route` logs a `warn!` and overflows the request
    /// to the healthiest conductor with remaining capacity; if none has capacity
    /// it keeps using the agent's assigned conductor (degrade, never drop). No
    /// request is ever refused or panicked on by this gate.
    ///
    /// The **provisioning** gate is separate and lives in
    /// `AgentProvisioner::provision_agent`, which compares
    /// `ConductorInfo::capacity_used` against `ConductorInfo::capacity_max`
    /// (seeded from the same env lever in `main.rs`). At the cap it returns a
    /// clean `Err("Conductor {id} at capacity ({used}/{max})")` — never a panic —
    /// which surfaces as `503 SERVICE_UNAVAILABLE` on the registration path
    /// (`auth_routes`) and `500` with the same message on
    /// `POST /admin/hosted-users`. Already-hosted agents are unaffected:
    /// `provision_agent` returns early from `find_existing_app` before the
    /// capacity check, so lowering the cap to the current population stops
    /// growth without evicting or locking out an existing hosted human.
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
            auth_token: None,
            token_minter: None,
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

    #[test]
    fn max_agents_unset_uses_default() {
        assert_eq!(
            parse_max_agents_per_conductor(None),
            DEFAULT_MAX_AGENTS_PER_CONDUCTOR
        );
        assert_eq!(
            parse_max_agents_per_conductor(Some("")),
            DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
            "empty string is treated as unset"
        );
        assert_eq!(
            parse_max_agents_per_conductor(Some("   ")),
            DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
            "whitespace-only is treated as unset"
        );
    }

    #[test]
    fn max_agents_valid_override_is_honored() {
        assert_eq!(parse_max_agents_per_conductor(Some("32")), 32);
        assert_eq!(
            parse_max_agents_per_conductor(Some(" 32 ")),
            32,
            "surrounding whitespace (k8s env YAML) is tolerated"
        );
        assert_eq!(parse_max_agents_per_conductor(Some("200")), 200);
        assert_eq!(
            parse_max_agents_per_conductor(Some("0")),
            0,
            "0 is a deliberate provisioning freeze, not a parse failure"
        );
    }

    #[test]
    fn max_agents_invalid_falls_back_to_default() {
        for bad in ["abc", "-1", "32.5", "1_000", "50 agents"] {
            assert_eq!(
                parse_max_agents_per_conductor(Some(bad)),
                DEFAULT_MAX_AGENTS_PER_CONDUCTOR,
                "{bad:?} should fall back to the default"
            );
        }
    }

    #[test]
    fn max_agents_from_env_defaults_when_unset() {
        // The lever is not set in the test environment; the env reader must
        // agree with the pure parser's unset case.
        if std::env::var(MAX_AGENTS_PER_CONDUCTOR_ENV).is_err() {
            assert_eq!(
                max_agents_per_conductor_from_env(),
                DEFAULT_MAX_AGENTS_PER_CONDUCTOR
            );
        }
    }
}
