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
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

/// Count DISTINCT hosted installs per conductor from `(conductor_id, app_id)` pairs.
///
/// # Why `app_id` and not the agent key
///
/// The registry deliberately holds **two keys per discovered agent** — the same
/// `AgentPubKey` under base64-STANDARD (the provisioner's format) and under
/// base64-URL-SAFE-NO-PAD (Holochain's display format), so a JWT in either
/// encoding routes. Counting agent keys therefore double-counts every discovered
/// agent. Both encodings of one agent share a single `installed_app_id`, and the
/// provisioner mints a deterministic per-user app id
/// (`generate_app_id(app_id, conductor_id, user_identifier)`), so `app_id` is the
/// per-install unique proxy for "one hosted human."
///
/// # Known undercount
///
/// `load_from_db` coerces a legacy Mongo row with no `app_id` field to the bare
/// `"elohim"` default, so several such rows on one conductor collapse to a single
/// count. The seeded count is therefore a floor, never an overcount — it can only
/// make the cap more permissive, never spuriously refuse provisioning.
pub fn count_distinct_installs_by_conductor<I>(pairs: I) -> HashMap<String, usize>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (conductor_id, app_id) in pairs {
        if conductor_id.is_empty() {
            continue;
        }
        if seen.insert((conductor_id.clone(), app_id)) {
            *counts.entry(conductor_id).or_insert(0) += 1;
        }
    }
    counts
}

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

    /// Re-seed every registered conductor's `capacity_used` from the persisted
    /// agent mappings. Returns the seeded `(conductor_id, count)` pairs, sorted.
    ///
    /// # Why this is required for the cap to bite
    ///
    /// `register_conductor` hard-sets `capacity_used: 0`, and `load_from_db`
    /// restores agent→conductor mappings WITHOUT touching capacity. Without this
    /// seed, `capacity_used` counts only agents newly registered during the
    /// current process lifetime — a doorway that restarts several times a day
    /// would never observe its true hosted population, and
    /// `DOORWAY_MAX_AGENTS_PER_CONDUCTOR` would degrade into "N new
    /// registrations per process lifetime" rather than a population ceiling.
    ///
    /// # Call ordering
    ///
    /// Call ONCE at startup, AFTER the `register_conductor` loop (which would
    /// otherwise reset the seeded value to 0) and BEFORE
    /// `discover_existing_agents` (whose `register_agent` calls increment on top
    /// of the seed). Conductors with no persisted agents are explicitly seeded to
    /// 0 so a stale in-memory value can never survive a re-seed.
    ///
    /// Deduplicates via [`count_distinct_installs_by_conductor`] — see its docs
    /// for why raw agent-key counts double-count.
    pub fn seed_capacity_from_agents(&self) -> Vec<(String, usize)> {
        let pairs: Vec<(String, String)> = self
            .agents
            .iter()
            .map(|e| (e.value().conductor_id.clone(), e.value().app_id.clone()))
            .collect();
        let counts = count_distinct_installs_by_conductor(pairs);

        let mut seeded: Vec<(String, usize)> = Vec::new();
        for mut entry in self.conductors.iter_mut() {
            let count = counts.get(entry.key()).copied().unwrap_or(0);
            let conductor_id = entry.key().clone();
            entry.value_mut().capacity_used = count;
            seeded.push((conductor_id, count));
        }
        seeded.sort();
        seeded
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

    // ---- capacity seeding (DOORWAY_MAX_AGENTS_PER_CONDUCTOR support) ----

    fn pair(conductor: &str, app: &str) -> (String, String) {
        (conductor.to_string(), app.to_string())
    }

    #[test]
    fn distinct_installs_dedupes_dual_encoded_agent_keys() {
        // The shape `discover_existing_agents` produces: ONE agent registered
        // twice (base64-std + base64-url keys) sharing one installed_app_id.
        let counts = count_distinct_installs_by_conductor(vec![
            pair("conductor-0", "elohim-conductor-0-adam"),
            pair("conductor-0", "elohim-conductor-0-adam"),
            pair("conductor-0", "elohim-conductor-0-eve"),
            pair("conductor-0", "elohim-conductor-0-eve"),
        ]);
        assert_eq!(
            counts.get("conductor-0").copied(),
            Some(2),
            "two humans registered under two encodings each must count as 2, not 4"
        );
    }

    #[test]
    fn distinct_installs_partitions_by_conductor() {
        let counts = count_distinct_installs_by_conductor(vec![
            pair("conductor-0", "app-a"),
            pair("conductor-0", "app-b"),
            pair("conductor-1", "app-c"),
            // Same app_id on a different conductor is a distinct install.
            pair("conductor-1", "app-a"),
        ]);
        assert_eq!(counts.get("conductor-0").copied(), Some(2));
        assert_eq!(counts.get("conductor-1").copied(), Some(2));
        assert_eq!(counts.get("conductor-2").copied(), None);
    }

    #[test]
    fn distinct_installs_handles_empty_and_legacy_rows() {
        assert!(count_distinct_installs_by_conductor(vec![]).is_empty());

        // Rows with no conductor_id are skipped entirely.
        let counts = count_distinct_installs_by_conductor(vec![
            pair("", "app-a"),
            pair("conductor-0", "app-a"),
        ]);
        assert_eq!(counts.get("conductor-0").copied(), Some(1));
        assert_eq!(counts.get("").copied(), None);

        // Documented undercount: legacy rows coerced to the bare "elohim"
        // app_id collapse to 1. Asserted so the floor-not-overcount property
        // is a contract, not an accident.
        let legacy = count_distinct_installs_by_conductor(vec![
            pair("conductor-0", "elohim"),
            pair("conductor-0", "elohim"),
            pair("conductor-0", "elohim"),
        ]);
        assert_eq!(legacy.get("conductor-0").copied(), Some(1));
    }

    #[tokio::test]
    async fn seed_capacity_counts_persisted_agents_not_process_lifetime() {
        let registry = ConductorRegistry::new(None).await;

        // Simulate load_from_db: agent mappings exist BEFORE any conductor is
        // registered, and two encodings of one agent share one app_id.
        for (key, app) in [
            ("uhCAk_adam_std", "elohim-conductor-0-adam"),
            ("uhCAk_adam_url", "elohim-conductor-0-adam"),
            ("uhCAk_eve_std", "elohim-conductor-0-eve"),
        ] {
            registry
                .register_agent(key, "conductor-0", app)
                .await
                .unwrap();
        }

        // register_conductor resets capacity_used to 0 — the bug the seed fixes.
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://c0:4445".to_string(),
            admin_url: "ws://c0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 32,
        });
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-1".to_string(),
            conductor_url: "ws://c1:4445".to_string(),
            admin_url: "ws://c1:4444".to_string(),
            capacity_used: 0,
            capacity_max: 32,
        });
        assert_eq!(
            registry
                .get_conductor_info("conductor-0")
                .unwrap()
                .capacity_used,
            0,
            "precondition: register_conductor zeroes capacity_used"
        );

        let seeded = registry.seed_capacity_from_agents();
        assert_eq!(
            seeded,
            vec![
                ("conductor-0".to_string(), 2),
                ("conductor-1".to_string(), 0)
            ],
            "2 distinct installs on conductor-0 (not 3 agent keys); conductor-1 seeded to 0"
        );
        assert_eq!(
            registry
                .get_conductor_info("conductor-0")
                .unwrap()
                .capacity_used,
            2
        );
        assert_eq!(
            registry
                .get_conductor_info("conductor-1")
                .unwrap()
                .capacity_used,
            0
        );
    }

    #[tokio::test]
    async fn seeded_capacity_at_or_over_cap_refuses_further_growth() {
        let registry = ConductorRegistry::new(None).await;
        for i in 0..3u32 {
            registry
                .register_agent(
                    &format!("uhCAk_h{i}"),
                    "conductor-0",
                    &format!("elohim-conductor-0-h{i}"),
                )
                .await
                .unwrap();
        }
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://c0:4445".to_string(),
            admin_url: "ws://c0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 3,
        });
        registry.seed_capacity_from_agents();

        // This is the condition AgentProvisioner::provision_agent checks before
        // installing a NEW app; at the seeded population it must be true.
        let info = registry.find_least_loaded().unwrap();
        assert!(
            info.capacity_used >= info.capacity_max,
            "a cap at the seeded population must gate further provisioning"
        );
    }
}
