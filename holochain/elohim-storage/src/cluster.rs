//! Cluster Module - Trust-based replication topology
//!
//! Clusters are NOT defined by physical location (same LAN/house).
//! They are defined by human-configured trust relationships:
//!
//! - **Family**: Immediate family members (might live in different cities)
//! - **Community**: Broader community of practice
//! - **Association**: Organizational or interest-based groups
//!
//! The humans configure who belongs to their trust network, and this
//! guides the P2P replication and delivery topology.
//!
//! ## Physical vs Trust Topology
//!
//! ```text
//! Physical (mDNS discovers):           Trust (human configures):
//! ┌─────────────────────┐              ┌─────────────────────┐
//! │ Same LAN (house)    │              │ Trust Network       │
//! │   ├─ Mom's laptop   │              │   ├─ Mom (Seattle)  │
//! │   ├─ Dad's phone    │              │   ├─ Dad (Seattle)  │
//! │   └─ Kid's tablet   │              │   ├─ Kid (college)  │
//! └─────────────────────┘              │   ├─ Grandma (NYC)  │
//!                                      │   └─ Uncle (Austin) │
//!                                      └─────────────────────┘
//! ```
//!
//! mDNS helps with fast local discovery, but the trust topology
//! determines WHO gets replicated data, not WHERE they are.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::error::StorageError;
use crate::sovereignty::ClusterRole;

#[cfg(feature = "p2p")]
use libp2p::PeerId;

/// Trust level determines replication priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Immediate family - highest replication priority
    Family,
    /// Extended family or close friends
    Extended,
    /// Community members
    Community,
    /// Association/organization members
    Association,
    /// Public network (lowest priority)
    Network,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Network
    }
}

/// A member of the trust network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    /// Holochain agent public key
    pub agent_pubkey: String,
    /// libp2p PeerId (for P2P communication)
    #[cfg(feature = "p2p")]
    pub peer_id: Option<PeerId>,
    #[cfg(not(feature = "p2p"))]
    pub peer_id: Option<String>,
    /// Human-assigned display name
    pub display_name: Option<String>,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Role in the cluster
    pub role: ClusterRole,
    /// Storage capacity in bytes (if known)
    pub capacity_bytes: Option<u64>,
    /// Used storage in bytes (if known)
    pub used_bytes: Option<u64>,
    /// Whether currently online
    pub online: bool,
    /// Last seen timestamp (unix seconds)
    pub last_seen: u64,
}

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Unique cluster identifier
    pub cluster_id: String,
    /// Cluster display name
    pub display_name: String,
    /// This node's role
    pub local_role: ClusterRole,
    /// Target replicas within cluster
    pub internal_replicas: u8,
    /// Target replicas outside cluster (for resilience)
    pub external_replicas: u8,
    /// Auto-accept members at or above this trust level
    pub auto_accept_trust: TrustLevel,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_id: uuid::Uuid::new_v4().to_string(),
            display_name: "My Cluster".to_string(),
            local_role: ClusterRole::StorageNode,
            internal_replicas: 2,
            external_replicas: 1,
            auto_accept_trust: TrustLevel::Extended,
        }
    }
}

/// Manages trust-based cluster membership and replication
pub struct ClusterManager {
    /// Cluster configuration
    config: ClusterConfig,
    /// Members by agent pubkey
    members: HashMap<String, ClusterMember>,
    /// Local agent pubkey
    local_agent: String,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new(config: ClusterConfig, local_agent: String) -> Self {
        Self {
            config,
            members: HashMap::new(),
            local_agent,
        }
    }

    /// Get cluster ID
    pub fn cluster_id(&self) -> &str {
        &self.config.cluster_id
    }

    /// Get local role
    pub fn local_role(&self) -> ClusterRole {
        self.config.local_role
    }

    /// Add a member to the cluster
    pub fn add_member(&mut self, member: ClusterMember) -> Result<(), StorageError> {
        if member.agent_pubkey == self.local_agent {
            return Err(StorageError::Cluster("Cannot add self as member".into()));
        }

        info!(
            agent = %member.agent_pubkey,
            trust = ?member.trust_level,
            "Added cluster member"
        );

        self.members.insert(member.agent_pubkey.clone(), member);
        Ok(())
    }

    /// Remove a member from the cluster
    pub fn remove_member(&mut self, agent_pubkey: &str) -> Option<ClusterMember> {
        let member = self.members.remove(agent_pubkey);
        if member.is_some() {
            info!(agent = %agent_pubkey, "Removed cluster member");
        }
        member
    }

    /// Get a member by agent pubkey
    pub fn get_member(&self, agent_pubkey: &str) -> Option<&ClusterMember> {
        self.members.get(agent_pubkey)
    }

    /// Update member online status
    pub fn update_member_status(
        &mut self,
        agent_pubkey: &str,
        online: bool,
        last_seen: u64,
    ) {
        if let Some(member) = self.members.get_mut(agent_pubkey) {
            member.online = online;
            member.last_seen = last_seen;
        }
    }

    /// Get all members at or above a trust level
    pub fn members_at_trust(&self, min_trust: TrustLevel) -> Vec<&ClusterMember> {
        self.members
            .values()
            .filter(|m| m.trust_level <= min_trust) // Lower enum value = higher trust
            .collect()
    }

    /// Get online members at or above a trust level
    pub fn online_members_at_trust(&self, min_trust: TrustLevel) -> Vec<&ClusterMember> {
        self.members_at_trust(min_trust)
            .into_iter()
            .filter(|m| m.online)
            .collect()
    }

    /// Select replication targets for a blob
    ///
    /// Returns members sorted by trust level (highest first),
    /// limited to the configured replica count.
    pub fn select_replication_targets(&self) -> Vec<&ClusterMember> {
        let mut targets: Vec<_> = self.members.values()
            .filter(|m| m.online)
            .collect();

        // Sort by trust level (Family first, then Extended, etc.)
        targets.sort_by_key(|m| m.trust_level);

        // Take up to internal_replicas
        targets.truncate(self.config.internal_replicas as usize);
        targets
    }

    /// Select targets for recovery-critical data
    ///
    /// Recovery data should be replicated to ALL online family members,
    /// plus configured external replicas.
    pub fn select_recovery_targets(&self) -> Vec<&ClusterMember> {
        // All family members (regardless of replica limit)
        let family: Vec<_> = self.members.values()
            .filter(|m| m.online && m.trust_level == TrustLevel::Family)
            .collect();

        // Plus extended family up to reasonable limit
        let mut extended: Vec<_> = self.members.values()
            .filter(|m| m.online && m.trust_level == TrustLevel::Extended)
            .collect();
        extended.truncate(3); // Max 3 extended family

        let mut targets = family;
        targets.extend(extended);
        targets
    }

    /// Check if an agent is allowed to request content
    pub fn should_serve(&self, requester_agent: &str) -> bool {
        // Serve to any cluster member
        self.members.contains_key(requester_agent)
    }

    /// Get total cluster capacity
    pub fn total_capacity(&self) -> u64 {
        self.members.values()
            .filter_map(|m| m.capacity_bytes)
            .sum()
    }

    /// Get total used storage
    pub fn total_used(&self) -> u64 {
        self.members.values()
            .filter_map(|m| m.used_bytes)
            .sum()
    }

    /// Get member count by trust level
    pub fn member_count_by_trust(&self) -> HashMap<TrustLevel, usize> {
        let mut counts = HashMap::new();
        for member in self.members.values() {
            *counts.entry(member.trust_level).or_insert(0) += 1;
        }
        counts
    }

    /// Get online member count
    pub fn online_count(&self) -> usize {
        self.members.values().filter(|m| m.online).count()
    }

    /// Export cluster configuration for sharing
    pub fn export_config(&self) -> ClusterConfig {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(agent: &str, trust: TrustLevel, online: bool) -> ClusterMember {
        ClusterMember {
            agent_pubkey: agent.to_string(),
            peer_id: None,
            display_name: Some(agent.to_string()),
            trust_level: trust,
            role: ClusterRole::StorageNode,
            capacity_bytes: Some(10_000_000_000),
            used_bytes: Some(1_000_000_000),
            online,
            last_seen: 0,
        }
    }

    #[test]
    fn test_replication_targets_prioritize_family() {
        let config = ClusterConfig {
            internal_replicas: 2,
            ..Default::default()
        };
        let mut manager = ClusterManager::new(config, "local".to_string());

        // Add members at different trust levels
        manager.add_member(make_member("community1", TrustLevel::Community, true)).unwrap();
        manager.add_member(make_member("family1", TrustLevel::Family, true)).unwrap();
        manager.add_member(make_member("extended1", TrustLevel::Extended, true)).unwrap();
        manager.add_member(make_member("family2", TrustLevel::Family, true)).unwrap();

        let targets = manager.select_replication_targets();

        // Should prioritize family (2 replicas max)
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.trust_level == TrustLevel::Family));
    }

    #[test]
    fn test_recovery_targets_include_all_family() {
        let config = ClusterConfig::default();
        let mut manager = ClusterManager::new(config, "local".to_string());

        // Add 5 family members
        for i in 0..5 {
            manager.add_member(make_member(&format!("family{}", i), TrustLevel::Family, true)).unwrap();
        }

        let targets = manager.select_recovery_targets();

        // Should include ALL 5 family members (no limit)
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_serve_only_cluster_members() {
        let config = ClusterConfig::default();
        let mut manager = ClusterManager::new(config, "local".to_string());

        manager.add_member(make_member("family1", TrustLevel::Family, true)).unwrap();

        assert!(manager.should_serve("family1"));
        assert!(!manager.should_serve("stranger"));
    }
}
