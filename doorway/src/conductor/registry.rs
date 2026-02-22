//! Conductor Registry — maps agents to conductors in the pool
//!
//! Every doorway instance (writer or reader) holds a ConductorRegistry.
//! The registry tracks which conductor hosts which agent, enabling
//! future per-request routing based on JWT agent_pub_key claims.
//!
//! ## Data Flow
//!
//! 1. On startup, conductor URLs are loaded from CONDUCTOR_URLS config
//! 2. Each conductor is registered with a generated ID and capacity info
//! 3. Agent→conductor mappings are loaded from MongoDB (if available)
//! 4. On agent provisioning (future), new mappings are persisted to MongoDB
//!
//! ## Thread Safety
//!
//! Uses DashMap for lock-free concurrent reads — critical since every
//! HTTP request may need to look up which conductor handles an agent.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Registry of conductors and agent→conductor mappings
pub struct ConductorRegistry {
    /// agent_pub_key → conductor entry (which conductor hosts this agent)
    agents: DashMap<String, ConductorEntry>,
    /// conductor_id → conductor info (URL, capacity)
    conductors: DashMap<String, ConductorInfo>,
    /// MongoDB collection for persistent backing (None = memory-only)
    db: Option<mongodb::Collection<bson::Document>>,
}

/// An agent's assignment to a conductor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorEntry {
    /// Unique conductor identifier (e.g., "conductor-0")
    pub conductor_id: String,
    /// WebSocket URL for the conductor's app interface
    pub conductor_url: String,
    /// Holochain app ID installed for this agent
    pub app_id: String,
    /// When this agent was assigned to the conductor
    pub assigned_at: DateTime<Utc>,
}

/// Information about a conductor in the pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorInfo {
    /// Unique conductor identifier
    pub conductor_id: String,
    /// App interface URL (port 4445 by default)
    pub conductor_url: String,
    /// Admin interface URL (port 4444 by default)
    pub admin_url: String,
    /// Number of agents currently hosted
    pub capacity_used: usize,
    /// Maximum agents this conductor should host
    pub capacity_max: usize,
}

impl ConductorRegistry {
    /// Create a new registry with optional MongoDB backing
    pub async fn new(db: Option<mongodb::Collection<bson::Document>>) -> Self {
        let registry = Self {
            agents: DashMap::new(),
            conductors: DashMap::new(),
            db,
        };

        // Load persisted agent mappings if MongoDB is available
        if registry.db.is_some() {
            if let Err(e) = registry.load_from_db().await {
                warn!("Failed to load conductor registry from MongoDB: {}", e);
            }
        }

        registry
    }

    /// Load agent→conductor mappings from MongoDB
    pub async fn load_from_db(&self) -> anyhow::Result<()> {
        use futures::TryStreamExt;

        let Some(ref collection) = self.db else {
            return Ok(());
        };

        let mut cursor = collection.find(bson::doc! {}).await?;
        let mut count = 0u64;

        while let Some(doc) = cursor.try_next().await? {
            let agent_pub_key = doc.get_str("agent_pub_key").unwrap_or_default().to_string();
            let conductor_id = doc.get_str("conductor_id").unwrap_or_default().to_string();
            let conductor_url = doc.get_str("conductor_url").unwrap_or_default().to_string();
            let app_id = doc.get_str("app_id").unwrap_or("elohim").to_string();
            let assigned_at = doc
                .get_datetime("assigned_at")
                .map(|dt| dt.to_chrono())
                .unwrap_or_else(|_| Utc::now());

            if !agent_pub_key.is_empty() && !conductor_id.is_empty() {
                self.agents.insert(
                    agent_pub_key,
                    ConductorEntry {
                        conductor_id,
                        conductor_url,
                        app_id,
                        assigned_at,
                    },
                );
                count += 1;
            }
        }

        if count > 0 {
            info!("Loaded {} agent→conductor mappings from MongoDB", count);
        }

        Ok(())
    }

    /// Register a conductor in the pool
    pub fn register_conductor(&self, info: ConductorInfo) {
        info!(
            conductor_id = %info.conductor_id,
            url = %info.conductor_url,
            admin_url = %info.admin_url,
            capacity_max = info.capacity_max,
            "Registered conductor in pool"
        );
        self.conductors.insert(info.conductor_id.clone(), info);
    }

    /// Register an agent→conductor mapping
    pub async fn register_agent(
        &self,
        agent_pub_key: &str,
        conductor_id: &str,
        app_id: &str,
    ) -> anyhow::Result<()> {
        // Look up conductor URL
        let conductor_url = self
            .conductors
            .get(conductor_id)
            .map(|c| c.conductor_url.clone())
            .unwrap_or_default();

        let entry = ConductorEntry {
            conductor_id: conductor_id.to_string(),
            conductor_url,
            app_id: app_id.to_string(),
            assigned_at: Utc::now(),
        };

        // Persist to MongoDB if available
        if let Some(ref collection) = self.db {
            let doc = bson::doc! {
                "agent_pub_key": agent_pub_key,
                "conductor_id": conductor_id,
                "conductor_url": &entry.conductor_url,
                "app_id": app_id,
                "assigned_at": bson::DateTime::from_chrono(entry.assigned_at),
            };

            collection
                .update_one(
                    bson::doc! { "agent_pub_key": agent_pub_key },
                    bson::doc! { "$set": doc },
                )
                .upsert(true)
                .await?;
        }

        // Update capacity
        if let Some(mut conductor) = self.conductors.get_mut(conductor_id) {
            conductor.capacity_used += 1;
        }

        self.agents.insert(agent_pub_key.to_string(), entry);

        Ok(())
    }

    /// Look up which conductor hosts an agent
    pub fn get_conductor_for_agent(&self, agent_pub_key: &str) -> Option<ConductorEntry> {
        self.agents.get(agent_pub_key).map(|e| e.clone())
    }

    /// Look up conductor info by ID
    pub fn get_conductor_info(&self, conductor_id: &str) -> Option<ConductorInfo> {
        self.conductors.get(conductor_id).map(|c| c.clone())
    }

    /// Find the conductor with the most available capacity
    pub fn find_least_loaded(&self) -> Option<ConductorInfo> {
        self.conductors
            .iter()
            .max_by_key(|entry| entry.capacity_max.saturating_sub(entry.capacity_used))
            .map(|entry| entry.value().clone())
    }

    /// List all conductors in the pool
    pub fn list_conductors(&self) -> Vec<ConductorInfo> {
        self.conductors.iter().map(|e| e.value().clone()).collect()
    }

    /// List all agents assigned to a specific conductor
    pub fn list_agents_on_conductor(&self, conductor_id: &str) -> Vec<(String, ConductorEntry)> {
        self.agents
            .iter()
            .filter(|e| e.value().conductor_id == conductor_id)
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect()
    }

    /// Remove an agent→conductor mapping (for deprovisioning).
    pub fn unregister_agent(&self, agent_pub_key: &str) {
        if let Some((_, entry)) = self.agents.remove(agent_pub_key) {
            // Decrement capacity
            if let Some(mut conductor) = self.conductors.get_mut(&entry.conductor_id) {
                conductor.capacity_used = conductor.capacity_used.saturating_sub(1);
            }
            info!(
                agent = %agent_pub_key,
                conductor = %entry.conductor_id,
                "Removed agent from registry"
            );
        }
    }

    /// Get the number of registered conductors
    pub fn conductor_count(&self) -> usize {
        self.conductors.len()
    }

    /// Get the total number of registered agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_basic_operations() {
        let registry = ConductorRegistry::new(None).await;

        // Register conductors
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

        assert_eq!(registry.conductor_count(), 2);
        assert_eq!(registry.list_conductors().len(), 2);

        // Register an agent
        registry
            .register_agent("uhCAk_test_agent_1", "conductor-0", "elohim")
            .await
            .unwrap();

        assert_eq!(registry.agent_count(), 1);

        // Look up agent
        let entry = registry
            .get_conductor_for_agent("uhCAk_test_agent_1")
            .unwrap();
        assert_eq!(entry.conductor_id, "conductor-0");
        assert_eq!(entry.conductor_url, "ws://cond-0:4445");
        assert_eq!(entry.app_id, "elohim");

        // Agent not found
        assert!(registry.get_conductor_for_agent("unknown").is_none());

        // List agents on conductor
        let agents = registry.list_agents_on_conductor("conductor-0");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].0, "uhCAk_test_agent_1");

        let agents_1 = registry.list_agents_on_conductor("conductor-1");
        assert_eq!(agents_1.len(), 0);
    }

    #[tokio::test]
    async fn test_find_least_loaded() {
        let registry = ConductorRegistry::new(None).await;

        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://cond-0:4445".to_string(),
            admin_url: "ws://cond-0:4444".to_string(),
            capacity_used: 40,
            capacity_max: 50,
        });
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-1".to_string(),
            conductor_url: "ws://cond-1:4445".to_string(),
            admin_url: "ws://cond-1:4444".to_string(),
            capacity_used: 10,
            capacity_max: 50,
        });

        let least_loaded = registry.find_least_loaded().unwrap();
        assert_eq!(least_loaded.conductor_id, "conductor-1");
    }

    /// Multi-user isolation: two users connecting get independent state.
    /// Progress/mastery data doesn't bleed between users because each
    /// agent has its own conductor entry with distinct app_id + conductor mapping.
    #[tokio::test]
    async fn test_multi_user_isolation_independent_state() {
        let registry = ConductorRegistry::new(None).await;

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

        // User A assigned to conductor-0
        registry
            .register_agent("uhCAk_alice", "conductor-0", "elohim")
            .await
            .unwrap();

        // User B assigned to conductor-1
        registry
            .register_agent("uhCAk_bob", "conductor-1", "elohim")
            .await
            .unwrap();

        // Verify each user has their own independent assignment
        let alice_entry = registry.get_conductor_for_agent("uhCAk_alice").unwrap();
        let bob_entry = registry.get_conductor_for_agent("uhCAk_bob").unwrap();

        assert_eq!(alice_entry.conductor_id, "conductor-0");
        assert_eq!(bob_entry.conductor_id, "conductor-1");
        assert_ne!(alice_entry.conductor_id, bob_entry.conductor_id);

        // Verify agent lists are isolated per conductor
        let agents_on_0 = registry.list_agents_on_conductor("conductor-0");
        let agents_on_1 = registry.list_agents_on_conductor("conductor-1");
        assert_eq!(agents_on_0.len(), 1);
        assert_eq!(agents_on_1.len(), 1);
        assert_eq!(agents_on_0[0].0, "uhCAk_alice");
        assert_eq!(agents_on_1[0].0, "uhCAk_bob");
    }

    /// Verify that removing one user doesn't affect the other's state
    #[tokio::test]
    async fn test_multi_user_unregister_does_not_affect_other() {
        let registry = ConductorRegistry::new(None).await;

        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://cond-0:4445".to_string(),
            admin_url: "ws://cond-0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 50,
        });

        // Both users on same conductor
        registry
            .register_agent("uhCAk_alice", "conductor-0", "elohim")
            .await
            .unwrap();
        registry
            .register_agent("uhCAk_bob", "conductor-0", "elohim")
            .await
            .unwrap();

        assert_eq!(registry.agent_count(), 2);

        // Remove alice
        registry.unregister_agent("uhCAk_alice");

        // Bob's assignment must be unaffected
        assert!(registry.get_conductor_for_agent("uhCAk_alice").is_none());
        let bob_entry = registry.get_conductor_for_agent("uhCAk_bob").unwrap();
        assert_eq!(bob_entry.conductor_id, "conductor-0");
        assert_eq!(registry.agent_count(), 1);
    }

    /// Concurrent agent registrations should not interfere with each other
    #[tokio::test]
    async fn test_concurrent_agent_registration() {
        use std::sync::Arc;

        let registry = Arc::new(ConductorRegistry::new(None).await);

        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://cond-0:4445".to_string(),
            admin_url: "ws://cond-0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 100,
        });

        // Spawn 20 concurrent agent registrations
        let mut handles = Vec::new();
        for i in 0..20u32 {
            let reg = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                let agent = format!("uhCAk_agent_{}", i);
                reg.register_agent(&agent, "conductor-0", "elohim")
                    .await
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // All 20 agents should be registered
        assert_eq!(registry.agent_count(), 20);

        // Each agent should map to conductor-0
        for i in 0..20u32 {
            let agent = format!("uhCAk_agent_{}", i);
            let entry = registry.get_conductor_for_agent(&agent).unwrap();
            assert_eq!(entry.conductor_id, "conductor-0");
        }

        // Capacity should reflect all registrations
        let info = registry.get_conductor_info("conductor-0").unwrap();
        assert_eq!(info.capacity_used, 20);
    }
}
