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

use super::agent_key::{is_agent_key_form, normalize_agent_key};
use tracing::{info, warn};

/// Count DISTINCT hosted agents per conductor from `(conductor_id, agent_key)` rows.
///
/// # Why the unit is the NORMALIZED agent key
///
/// The registry deliberately holds **several keys per agent** — the canonical
/// `uhCAk…` HoloHash form plus the bare base64 encodings a pre-canonical JWT or
/// Mongo row can still carry — so a person routes whatever spelling they hold.
/// Counting raw key strings therefore counts SPELLINGS, and inflates every
/// conductor's load by the alias fan-out. Normalizing first collapses every
/// spelling of one key onto one identity, which is what "one hosted human"
/// actually means.
///
/// # What this replaced, and why
///
/// This used to count distinct `(conductor_id, app_id)` pairs. `app_id` was only
/// ever a PROXY for identity, adopted because no normalizer existed: the
/// provisioner mints a deterministic per-user app id, so two encodings of one
/// agent shared one app id and collapsed correctly. The proxy leaked in the
/// other direction, though — every row that carries the bare default `"elohim"`
/// app id (a legacy Mongo row, a `ConductorRouter` miss-path auto-registration,
/// a hand-driven `/admin/conductors/assign`) collapsed onto ONE count no matter
/// how many real humans it represented. That is an UNDERCOUNT on the same
/// surface that enforces `DOORWAY_MAX_AGENTS_PER_CONDUCTOR`, i.e. a cap that
/// silently stops biting. With [`normalize_agent_key`] available, the proxy is
/// no longer needed and the real unit can be used directly.
///
/// A string that is not a key in any encoding is counted as itself — an
/// unrecognized identity is still an identity, and dropping it would undercount
/// exactly the way the app_id proxy did.
pub fn count_distinct_agents_by_conductor<I>(rows: I) -> HashMap<String, usize>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (conductor_id, agent_key) in rows {
        if conductor_id.is_empty() {
            continue;
        }
        if seen.insert((conductor_id.clone(), normalize_agent_key(&agent_key))) {
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
    /// Deduplicates via [`count_distinct_agents_by_conductor`] — the SAME
    /// predicate `register_agent` recounts through, so a restart can neither
    /// halve nor inflate a live count.
    pub fn seed_capacity_from_agents(&self) -> Vec<(String, usize)> {
        let counts = count_distinct_agents_by_conductor(self.agent_rows());

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

        self.agents.insert(agent_pub_key.to_string(), entry);

        // Capacity is RECOUNTED from the agent map, never incremented.
        //
        // An unconditional `+= 1` counted string FORMS, not humans: the
        // provisioner and the startup discovery walk each register one agent
        // under several encodings so a JWT of any vintage still routes, so the
        // live number ran at ~2x the real cell count while
        // `seed_capacity_from_agents` — which has always deduped by install —
        // halved it again on every restart. A pool with
        // DOORWAY_MAX_AGENTS_PER_CONDUCTOR=50 was really admitting ~25, and the
        // cap moved when nothing about the conductor had.
        //
        // Recounting through the SAME predicate the restart path uses makes the
        // live count and the seeded count equal by construction rather than by
        // two agreeing implementations, and makes re-registering an agent (or
        // any of its aliases) idempotent.
        self.recount_capacity(conductor_id);

        Ok(())
    }

    /// Set one conductor's `capacity_used` to its DISTINCT install count.
    ///
    /// The unit is the normalized agent key — see
    /// [`count_distinct_agents_by_conductor`]. Idempotent and order-free: it is
    /// a projection of the agent map, so replaying it, or registering another
    /// spelling of an agent already counted, changes nothing.
    fn recount_capacity(&self, conductor_id: &str) {
        let count = count_distinct_agents_by_conductor(self.agent_rows())
            .get(conductor_id)
            .copied()
            .unwrap_or(0);
        if let Some(mut conductor) = self.conductors.get_mut(conductor_id) {
            conductor.capacity_used = count;
        }
    }

    /// `(conductor_id, agent_key)` for every registered mapping — the single
    /// shape both the live recount and the restart seed count over, so the two
    /// cannot drift into disagreeing implementations.
    fn agent_rows(&self) -> Vec<(String, String)> {
        self.agents
            .iter()
            .map(|e| (e.value().conductor_id.clone(), e.key().clone()))
            .collect()
    }

    /// Look up which conductor hosts an agent, in ANY string form of its key.
    ///
    /// Exact hit first (the overwhelmingly common case, since every form is
    /// registered as an alias at write time). The fallbacks exist for rows this
    /// process did not write: a Mongo row persisted by a pre-canonical doorway
    /// is loaded under the bare form it was stored in, while a freshly issued
    /// JWT carries the canonical one. Answering "not found" there would not be
    /// an honest absence — the agent IS hosted, under a different spelling — and
    /// on the router's path a miss does not even fail loudly: it auto-registers
    /// the key against a DEFAULT conductor and mis-routes from then on.
    ///
    /// The scan is miss-path only and is skipped outright for anything that is
    /// not a key in any encoding, so a genuine miss stays O(1).
    pub fn get_conductor_for_agent(&self, agent_pub_key: &str) -> Option<ConductorEntry> {
        if let Some(entry) = self.agents.get(agent_pub_key) {
            return Some(entry.clone());
        }
        if !is_agent_key_form(agent_pub_key) {
            return None;
        }
        let canonical = normalize_agent_key(agent_pub_key);
        if canonical != agent_pub_key {
            if let Some(entry) = self.agents.get(&canonical) {
                return Some(entry.clone());
            }
        }
        self.agents
            .iter()
            .find(|e| normalize_agent_key(e.key()) == canonical)
            .map(|e| e.value().clone())
    }

    /// Look up conductor info by ID
    pub fn get_conductor_info(&self, conductor_id: &str) -> Option<ConductorInfo> {
        self.conductors.get(conductor_id).map(|c| c.clone())
    }

    /// Find the conductor with the most available capacity
    pub fn find_least_loaded(&self) -> Option<ConductorInfo> {
        self.find_least_loaded_excluding(&[])
    }

    /// Find the conductor with the most available capacity, skipping any whose
    /// id appears in `exclude`.
    ///
    /// The exclusion set exists so a caller that has just watched a conductor
    /// drop its admin socket mid-call can re-offer the SAME work to a different
    /// member of the pool instead of surfacing a pool-wide refusal. Without it,
    /// `find_least_loaded` hands the caller the same unhealthy conductor on
    /// every retry, which is the "first-reachable-wins" degeneracy one seam out
    /// (museum trap #12).
    pub fn find_least_loaded_excluding(&self, exclude: &[String]) -> Option<ConductorInfo> {
        self.conductors
            .iter()
            .filter(|entry| !exclude.iter().any(|id| id == entry.key()))
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
        // Remove EVERY alias of this agent, not just the spelling the caller
        // happened to hold. Leaving a sibling encoding behind would keep the
        // install counted and keep the deprovisioned agent routable.
        let canonical = normalize_agent_key(agent_pub_key);
        let aliases: Vec<String> = if is_agent_key_form(agent_pub_key) {
            self.agents
                .iter()
                .filter(|e| normalize_agent_key(e.key()) == canonical)
                .map(|e| e.key().clone())
                .collect()
        } else {
            vec![agent_pub_key.to_string()]
        };

        let mut touched: HashSet<String> = HashSet::new();
        for alias in aliases {
            if let Some((_, entry)) = self.agents.remove(&alias) {
                touched.insert(entry.conductor_id);
            }
        }

        // Recount rather than decrement, for the same reason register_agent
        // does: the removed aliases were one install, not one each.
        for conductor_id in &touched {
            self.recount_capacity(conductor_id);
            info!(
                agent = %agent_pub_key,
                conductor = %conductor_id,
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

    // ---- capacity accounting (DOORWAY_MAX_AGENTS_PER_CONDUCTOR support) ----

    fn row(conductor: &str, agent: &str) -> (String, String) {
        (conductor.to_string(), agent.to_string())
    }

    /// A REAL agent key, in whichever spelling the caller asks for. Fake keys
    /// cannot exercise a normalizer, so the dedupe fixtures must be real.
    fn key_forms(seed: u8) -> Vec<String> {
        let raw = holo_hash::AgentPubKey::from_raw_32(vec![seed; 32])
            .get_raw_39()
            .to_vec();
        super::super::agent_key::lookup_forms(&raw)
    }

    #[test]
    fn distinct_agents_dedupes_the_alias_fan_out() {
        // The shape the provisioner and `discover_existing_agents` produce: ONE
        // agent registered under every string form of its key.
        let adam = key_forms(1);
        let eve = key_forms(2);
        let mut rows: Vec<(String, String)> = Vec::new();
        for form in adam.iter().chain(eve.iter()) {
            rows.push(row("conductor-0", form));
        }
        assert!(
            adam.len() > 1,
            "fixture must actually fan out, or it proves nothing"
        );
        assert_eq!(
            count_distinct_agents_by_conductor(rows)
                .get("conductor-0")
                .copied(),
            Some(2),
            "two humans registered under every encoding each must count as 2, not {}",
            adam.len() + eve.len()
        );
    }

    #[test]
    fn distinct_agents_partitions_by_conductor() {
        let a = key_forms(3).remove(0);
        let b = key_forms(4).remove(0);
        let c = key_forms(5).remove(0);
        let counts = count_distinct_agents_by_conductor(vec![
            row("conductor-0", &a),
            row("conductor-0", &b),
            row("conductor-1", &c),
            // The same agent on a different conductor is a distinct hosting.
            row("conductor-1", &a),
        ]);
        assert_eq!(counts.get("conductor-0").copied(), Some(2));
        assert_eq!(counts.get("conductor-1").copied(), Some(2));
        assert_eq!(counts.get("conductor-2").copied(), None);
    }

    #[test]
    fn distinct_agents_handles_empty_and_default_app_id_rows() {
        assert!(count_distinct_agents_by_conductor(vec![]).is_empty());

        // Rows with no conductor_id are skipped entirely.
        let a = key_forms(6).remove(0);
        let counts = count_distinct_agents_by_conductor(vec![row("", &a), row("conductor-0", &a)]);
        assert_eq!(counts.get("conductor-0").copied(), Some(1));
        assert_eq!(counts.get("").copied(), None);

        // THE REGRESSION the app_id proxy carried: three humans whose rows all
        // carry the bare default app id. The proxy collapsed these to 1 and the
        // cap stopped biting; counted by identity they are 3.
        let three: Vec<(String, String)> = (10u8..13)
            .map(|seed| row("conductor-0", &key_forms(seed).remove(0)))
            .collect();
        assert_eq!(
            count_distinct_agents_by_conductor(three)
                .get("conductor-0")
                .copied(),
            Some(3),
            "distinct humans must not collapse onto a shared app id"
        );
    }

    #[tokio::test]
    async fn registering_one_agent_under_every_encoding_costs_one_capacity() {
        // THE double-count this fix closes. The provisioner registers each agent
        // under the canonical form AND its legacy spellings so any JWT vintage
        // routes; an unconditional `capacity_used += 1` counted each spelling,
        // so a pool capped at 50 really admitted about half that — and a restart
        // (which always deduped) silently halved the number again.
        let registry = ConductorRegistry::new(None).await;
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-0".to_string(),
            conductor_url: "ws://c0:4445".to_string(),
            admin_url: "ws://c0:4444".to_string(),
            capacity_used: 0,
            capacity_max: 32,
        });

        let forms = key_forms(21);
        assert!(forms.len() >= 2, "fixture must fan out across encodings");
        for form in &forms {
            registry
                .register_agent(form, "conductor-0", "elohim-conductor-0-abc123")
                .await
                .unwrap();
        }

        assert_eq!(
            registry
                .get_conductor_info("conductor-0")
                .unwrap()
                .capacity_used,
            1,
            "one human is one cell, however many spellings of their key route to it"
        );

        // And the restart path agrees with the live number BY CONSTRUCTION.
        let seeded = registry.seed_capacity_from_agents();
        assert_eq!(seeded, vec![("conductor-0".to_string(), 1)]);
        assert_eq!(
            registry
                .get_conductor_info("conductor-0")
                .unwrap()
                .capacity_used,
            1,
            "a restart must neither halve nor inflate the live count"
        );
    }

    #[tokio::test]
    async fn any_spelling_of_a_key_finds_the_conductor_and_deprovisions_it() {
        // A Mongo row persisted by a pre-canonical doorway loads under the bare
        // form; the JWT that arrives next carries the canonical one. Answering
        // "not found" there is not an honest absence — and on the router's path
        // a miss auto-registers against a DEFAULT conductor and mis-routes.
        let registry = ConductorRegistry::new(None).await;
        registry.register_conductor(ConductorInfo {
            conductor_id: "conductor-7".to_string(),
            conductor_url: "ws://c7:4445".to_string(),
            admin_url: "ws://c7:4444".to_string(),
            capacity_used: 0,
            capacity_max: 32,
        });

        let forms = key_forms(33);
        let canonical = forms[0].clone();
        let legacy = forms[1].clone();
        assert_ne!(canonical, legacy);

        // Only the LEGACY spelling is stored, as an old Mongo row would be.
        registry
            .register_agent(&legacy, "conductor-7", "elohim-conductor-7-legacy")
            .await
            .unwrap();

        for spelling in [&canonical, &legacy] {
            assert_eq!(
                registry
                    .get_conductor_for_agent(spelling)
                    .expect("every spelling of one key must resolve")
                    .conductor_id,
                "conductor-7"
            );
        }

        // A string that is not a key in any encoding still misses honestly.
        assert!(registry
            .get_conductor_for_agent("uhCAk-dev-mode-agent-key")
            .is_none());

        // Deprovisioning by the canonical spelling must clear the legacy alias
        // too, or the agent stays routable and the install stays counted.
        registry.unregister_agent(&canonical);
        assert!(registry.get_conductor_for_agent(&legacy).is_none());
        assert_eq!(
            registry
                .get_conductor_info("conductor-7")
                .unwrap()
                .capacity_used,
            0
        );
    }

    #[tokio::test]
    async fn seed_capacity_counts_persisted_agents_not_process_lifetime() {
        let registry = ConductorRegistry::new(None).await;

        // Simulate load_from_db: agent mappings exist BEFORE any conductor is
        // registered, and two SPELLINGS of adam's one key are both persisted.
        let adam = key_forms(41);
        let eve = key_forms(42);
        for (key, app) in [
            (&adam[0], "elohim-conductor-0-adam"),
            (&adam[1], "elohim-conductor-0-adam"),
            (&eve[0], "elohim-conductor-0-eve"),
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
            "2 distinct humans on conductor-0 (not 3 key spellings); conductor-1 seeded to 0"
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
