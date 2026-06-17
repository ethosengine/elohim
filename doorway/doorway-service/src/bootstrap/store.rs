//! Bootstrap storage
//!
//! In-memory storage for agent info with TTL support.

use super::types::{Agent, SignedAgentInfo, Space};
use super::MAX_HOLD_MS;
use dashmap::DashMap;
use rand::seq::SliceRandom;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Key for storing agent info: network:space:agent
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentKey {
    /// Network type (tx2 or tx5)
    pub network: String,
    /// Space (DNA hash)
    pub space: Space,
    /// Agent pubkey
    pub agent: Agent,
}

impl AgentKey {
    pub fn new(network: &str, space: &Space, agent: &Agent) -> Self {
        Self {
            network: network.to_string(),
            space: *space,
            agent: *agent,
        }
    }

    /// Create a display string for logging
    pub fn display(&self) -> String {
        format!(
            "{}:{}:{}",
            self.network,
            hex::encode(&self.space[..8]),
            hex::encode(&self.agent[..8])
        )
    }
}

/// Stored agent entry with expiry
#[derive(Debug, Clone)]
struct AgentEntry {
    /// The raw MessagePack bytes (returned as-is to clients)
    raw_bytes: Vec<u8>,
    /// When this entry expires (absolute time)
    expires_at: Instant,
}

/// Index of agents in a space
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpaceKey {
    network: String,
    space: Space,
}

/// Bootstrap store with concurrent access
pub struct BootstrapStore {
    /// Agent info storage: key -> entry
    agents: DashMap<AgentKey, AgentEntry>,
    /// Index of agents per space: (network, space) -> list of agents
    space_index: DashMap<SpaceKey, Vec<Agent>>,
    /// Kitsune2-protocol store (HC 0.6 conductors) — lives inside the legacy
    /// store so both protocols share the one `state.bootstrap` handle and the
    /// one cleanup task. Behind a trait object so a mongo-backed impl can share
    /// one bootstrap table across doorway replicas/edges (kills genesis-pair
    /// islanding); env-selected in [`BootstrapStore::new`].
    k2: Box<dyn super::k2::K2Store>,
}

impl BootstrapStore {
    /// Create a new bootstrap store.
    ///
    /// Safe-by-default: the k2 backend is the in-memory [`super::k2::MemK2Store`]
    /// UNLESS BOTH a `mongo` client is provided AND `BOOTSTRAP_MONGODB_DB` is set
    /// — only then does it select the shared mongo-backed [`super::k2::MongoK2Store`]
    /// that kills genesis-pair islanding. With the env unset, behavior is
    /// byte-identical to the pre-mongo store.
    pub fn new(mongo: Option<&crate::db::mongo::MongoClient>) -> Self {
        let k2: Box<dyn super::k2::K2Store> =
            match (mongo, std::env::var("BOOTSTRAP_MONGODB_DB").ok()) {
                (Some(m), Some(db)) => {
                    info!("k2 bootstrap store: shared mongo-backed DB '{}'", db);
                    Box::new(super::k2_mongo::MongoK2Store::new(m, &db))
                }
                _ => Box::new(super::k2::MemK2Store::new()),
            };
        Self {
            agents: DashMap::new(),
            space_index: DashMap::new(),
            k2,
        }
    }

    /// The kitsune2-protocol store (PUT /bootstrap/{space}/{agent}).
    pub fn k2(&self) -> &dyn super::k2::K2Store {
        self.k2.as_ref()
    }

    /// Store agent info, returning the key string
    pub fn put(&self, network: &str, info: &SignedAgentInfo, raw_bytes: Vec<u8>) -> String {
        let key = AgentKey::new(network, &info.agent_info.space, &info.agent);
        let display_key = key.display();

        // Calculate expiry: min of agent's requested expiry and MAX_HOLD
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let agent_expires_at_ms = info.expires_at_ms();
        let max_hold_expires_at_ms = now_ms + MAX_HOLD_MS;
        let expires_at_ms = agent_expires_at_ms.min(max_hold_expires_at_ms);

        // Convert to Instant for local expiry
        let ttl_ms = expires_at_ms.saturating_sub(now_ms);
        let expires_at = Instant::now() + Duration::from_millis(ttl_ms);

        let entry = AgentEntry {
            raw_bytes,
            expires_at,
        };

        // Store the agent
        let is_new = !self.agents.contains_key(&key);
        self.agents.insert(key.clone(), entry);

        // Update space index if new agent
        if is_new {
            let space_key = SpaceKey {
                network: network.to_string(),
                space: info.agent_info.space,
            };
            self.space_index
                .entry(space_key)
                .or_default()
                .push(info.agent);
        }

        display_key
    }

    /// Get random agents from a space
    pub fn random(&self, network: &str, space: &Space, limit: usize) -> Vec<Vec<u8>> {
        let space_key = SpaceKey {
            network: network.to_string(),
            space: *space,
        };

        let now = Instant::now();

        // Get agents in this space
        let agents = match self.space_index.get(&space_key) {
            Some(list) => list.clone(),
            None => return Vec::new(),
        };

        // Shuffle and take up to limit
        let mut rng = rand::thread_rng();
        let mut shuffled: Vec<_> = agents.iter().collect();
        shuffled.shuffle(&mut rng);

        let mut result = Vec::new();
        for agent in shuffled.into_iter().take(limit) {
            let key = AgentKey::new(network, space, agent);
            if let Some(entry) = self.agents.get(&key) {
                // Skip expired entries
                if entry.expires_at > now {
                    result.push(entry.raw_bytes.clone());
                }
            }
        }

        result
    }

    /// Cleanup expired entries (async — the k2 leg may be a mongo-backed store).
    pub async fn cleanup(&self) -> usize {
        let now = Instant::now();
        let mut removed = 0;

        // Find expired agents
        let expired: Vec<AgentKey> = self
            .agents
            .iter()
            .filter(|entry| entry.expires_at <= now)
            .map(|entry| entry.key().clone())
            .collect();

        // Remove expired entries
        for key in expired {
            if self.agents.remove(&key).is_some() {
                removed += 1;

                // Update space index
                let space_key = SpaceKey {
                    network: key.network.clone(),
                    space: key.space,
                };
                if let Some(mut agents) = self.space_index.get_mut(&space_key) {
                    agents.retain(|a| *a != key.agent);
                }
            }
        }

        // Remove empty space indices
        self.space_index.retain(|_, agents| !agents.is_empty());

        // Kitsune2-protocol entries age out on the same sweep (the mem backend
        // prunes; the mongo backend is a TTL no-op).
        let k2_now = super::k2::current_time_micros();
        removed + self.k2.cleanup(k2_now).await
    }

    /// Get stats about the store
    pub fn stats(&self) -> BootstrapStats {
        BootstrapStats {
            total_agents: self.agents.len(),
            total_spaces: self.space_index.len(),
        }
    }
}

impl Default for BootstrapStore {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Statistics about the bootstrap store
#[derive(Debug, Clone)]
pub struct BootstrapStats {
    pub total_agents: usize,
    pub total_spaces: usize,
}

/// Spawn a background task to periodically cleanup expired entries
pub fn spawn_cleanup_task(store: Arc<BootstrapStore>) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(60); // Run every minute
        loop {
            tokio::time::sleep(interval).await;
            let removed = store.cleanup().await;
            if removed > 0 {
                debug!("Bootstrap cleanup: removed {} expired entries", removed);
            }
            let stats = store.stats();
            debug!(
                "Bootstrap stats: {} agents in {} spaces",
                stats.total_agents, stats.total_spaces
            );
        }
    });
    info!("Bootstrap cleanup task started");
}

#[cfg(test)]
mod tests {
    use super::super::types::KITSUNE_BIN_LEN;
    use super::*;

    #[test]
    fn test_agent_key_display() {
        let key = AgentKey {
            network: "tx5".to_string(),
            space: [1u8; KITSUNE_BIN_LEN],
            agent: [2u8; KITSUNE_BIN_LEN],
        };
        let display = key.display();
        assert!(display.starts_with("tx5:"));
    }

    #[test]
    fn test_store_stats() {
        let store = BootstrapStore::new(None);
        let stats = store.stats();
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.total_spaces, 0);
    }

    #[tokio::test]
    async fn bootstrap_store_selects_mem_when_no_mongo() {
        // Safe-by-default: with no mongo client the k2 backend is MemK2Store,
        // matching the pre-mongo behavior exactly (env-gated mongo path is off).
        let s = BootstrapStore::new(None);
        assert_eq!(s.k2().stats().await, (0, 0));
    }
}
