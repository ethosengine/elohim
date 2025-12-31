//! Sovereignty Mode - Scale storage behavior based on device capabilities
//!
//! The Elohim Protocol recognizes different sovereignty stages, each with
//! different storage and replication behavior:
//!
//! | Mode | Storage | Replication | Serve |
//! |------|---------|-------------|-------|
//! | Laptop | Local (capped) | Selective | No |
//! | HomeNode | Local (full) | To family | Family only |
//! | HomeCluster | Coordinated | Within cluster + external | Cluster |
//! | Network | Local (full) | To network | Anyone |
//!
//! Mobile mode is deferred - mobile uses doorway for now.

use serde::{Deserialize, Serialize};

/// Sovereignty mode determines storage behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SovereigntyMode {
    /// Laptop: Intermittent connectivity, personal storage
    Laptop {
        /// Maximum local storage in bytes
        max_storage_bytes: u64,
        /// Enable background replication to trusted nodes
        replicate: bool,
    },

    /// HomeNode: Always-on, serves family members
    HomeNode {
        /// Willing to serve family members' requests
        serve_family: bool,
        /// Family member agent public keys
        family_members: Vec<String>,
    },

    /// HomeCluster: Coordinated family storage pool
    HomeCluster {
        /// Unique cluster identifier
        cluster_id: String,
        /// This node's role in the cluster
        cluster_role: ClusterRole,
    },

    /// Network: Full P2P participation
    Network {
        /// Doorway URL for web access (if proxying)
        doorway_url: Option<String>,
        /// Advertise to network for discovery
        advertise: bool,
    },
}

impl Default for SovereigntyMode {
    fn default() -> Self {
        Self::Laptop {
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            replicate: true,
        }
    }
}

/// Role within a home cluster
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClusterRole {
    /// Coordinates cluster, manages metadata
    Coordinator,
    /// Stores data, serves requests
    StorageNode,
    /// Lightweight node, delegates storage
    Satellite,
}

impl SovereigntyMode {
    /// Create laptop mode with default settings
    pub fn laptop() -> Self {
        Self::Laptop {
            max_storage_bytes: 10 * 1024 * 1024 * 1024,
            replicate: true,
        }
    }

    /// Create home node mode
    pub fn home_node(family_members: Vec<String>) -> Self {
        Self::HomeNode {
            serve_family: true,
            family_members,
        }
    }

    /// Create home cluster mode
    pub fn home_cluster(cluster_id: String, role: ClusterRole) -> Self {
        Self::HomeCluster {
            cluster_id,
            cluster_role: role,
        }
    }

    /// Create network mode
    pub fn network(advertise: bool) -> Self {
        Self::Network {
            doorway_url: None,
            advertise,
        }
    }

    /// Check if this mode should replicate content
    pub fn should_replicate(&self) -> bool {
        match self {
            Self::Laptop { replicate, .. } => *replicate,
            Self::HomeNode { .. } => true,
            Self::HomeCluster { .. } => true,
            Self::Network { .. } => true,
        }
    }

    /// Check if this mode should serve requests from others
    pub fn should_serve(&self, requester: Option<&str>) -> bool {
        match self {
            Self::Laptop { .. } => false,
            Self::HomeNode {
                serve_family,
                family_members,
            } => {
                if !serve_family {
                    return false;
                }
                // Serve if requester is in family list
                if let Some(req) = requester {
                    family_members.contains(&req.to_string())
                } else {
                    false
                }
            }
            Self::HomeCluster { .. } => {
                // Cluster mode serves cluster members (handled by cluster module)
                true
            }
            Self::Network { .. } => true,
        }
    }

    /// Get the maximum storage bytes (if limited)
    pub fn max_storage_bytes(&self) -> Option<u64> {
        match self {
            Self::Laptop {
                max_storage_bytes, ..
            } => Some(*max_storage_bytes),
            _ => None,
        }
    }

    /// Check if this mode is always-on
    pub fn is_always_on(&self) -> bool {
        matches!(
            self,
            Self::HomeNode { .. } | Self::HomeCluster { .. } | Self::Network { .. }
        )
    }

    /// Get cluster ID if in cluster mode
    pub fn cluster_id(&self) -> Option<&str> {
        match self {
            Self::HomeCluster { cluster_id, .. } => Some(cluster_id),
            _ => None,
        }
    }

    /// Get cluster role if in cluster mode
    pub fn cluster_role(&self) -> Option<ClusterRole> {
        match self {
            Self::HomeCluster { cluster_role, .. } => Some(*cluster_role),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laptop_mode() {
        let mode = SovereigntyMode::laptop();
        assert!(mode.should_replicate());
        assert!(!mode.should_serve(Some("anyone")));
        assert!(mode.max_storage_bytes().is_some());
        assert!(!mode.is_always_on());
    }

    #[test]
    fn test_home_node_mode() {
        let family = vec!["uhCAk_alice".to_string(), "uhCAk_bob".to_string()];
        let mode = SovereigntyMode::home_node(family);

        assert!(mode.should_replicate());
        assert!(mode.should_serve(Some("uhCAk_alice")));
        assert!(!mode.should_serve(Some("uhCAk_stranger")));
        assert!(mode.is_always_on());
    }

    #[test]
    fn test_cluster_mode() {
        let mode = SovereigntyMode::home_cluster("family-123".to_string(), ClusterRole::Coordinator);

        assert!(mode.should_replicate());
        assert_eq!(mode.cluster_id(), Some("family-123"));
        assert_eq!(mode.cluster_role(), Some(ClusterRole::Coordinator));
        assert!(mode.is_always_on());
    }

    #[test]
    fn test_network_mode() {
        let mode = SovereigntyMode::network(true);

        assert!(mode.should_replicate());
        assert!(mode.should_serve(Some("anyone")));
        assert!(mode.is_always_on());
    }
}
