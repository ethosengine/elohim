//! Network registration and membership
//!
//! Tracks node's relationship to the network:
//! - Operator who manages this node
//! - Doorways the node is registered with
//! - Apps connecting and syncing to this node
//! - Sync state and progress

pub mod operator;
pub mod registration;
pub mod sync_state;

pub use operator::Operator;
pub use registration::RegistrationStatus;
pub use sync_state::{ConnectedApp, SyncProgress};

use serde::{Deserialize, Serialize};

/// Complete network membership state for this node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMembership {
    /// Current registration status
    pub status: RegistrationStatus,

    /// Operator who manages this node (from join key)
    pub operator: Option<Operator>,

    /// Cluster this node belongs to
    pub cluster: Option<ClusterInfo>,

    /// Doorways this node is registered with
    pub doorways: Vec<RegisteredDoorway>,

    /// Apps currently connected and syncing
    pub connected_apps: Vec<ConnectedApp>,

    /// Overall sync progress
    pub sync_progress: SyncProgress,

    /// When registration was completed
    pub registered_at: Option<u64>,

    /// Last heartbeat to primary doorway
    pub last_heartbeat: Option<u64>,
}

/// Information about the cluster this node belongs to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Cluster name (e.g., "johnson-family")
    pub name: String,

    /// Cluster key hash (for verification)
    pub key_hash: String,

    /// Role in the cluster
    pub role: ClusterRole,

    /// Other nodes in this cluster
    pub peer_nodes: Vec<ClusterPeer>,
}

/// Role of this node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterRole {
    /// Primary node (handles writes, can invite new nodes)
    Primary,
    /// Replica node (syncs from primary)
    Replica,
    /// Witness node (stores data but doesn't sync actively)
    Witness,
}

/// A peer node in the same cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPeer {
    pub node_id: String,
    pub hostname: Option<String>,
    pub role: ClusterRole,
    pub last_seen: u64,
    pub sync_status: PeerSyncStatus,
}

/// Sync status with a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerSyncStatus {
    /// Fully synced
    Synced,
    /// Syncing in progress
    Syncing { progress_percent: u8 },
    /// Behind, needs to catch up
    Behind { entries_behind: u64 },
    /// Peer unreachable
    Unreachable,
}

/// A doorway this node is registered with
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDoorway {
    /// Doorway URL
    pub url: String,

    /// Doorway's public key (for verification)
    pub pub_key: Option<String>,

    /// Whether this is the primary doorway
    pub is_primary: bool,

    /// Connection status
    pub status: DoorwayStatus,

    /// Last successful communication
    pub last_contact: u64,

    /// Capabilities provided by this doorway
    pub capabilities: Vec<String>,
}

/// Status of connection to a doorway
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DoorwayStatus {
    /// Connected and healthy
    Connected,
    /// Connecting
    Connecting,
    /// Connection failed
    Disconnected,
    /// Doorway is degraded
    Degraded,
}

impl Default for NetworkMembership {
    fn default() -> Self {
        Self {
            status: RegistrationStatus::Unregistered,
            operator: None,
            cluster: None,
            doorways: Vec::new(),
            connected_apps: Vec::new(),
            sync_progress: SyncProgress::default(),
            registered_at: None,
            last_heartbeat: None,
        }
    }
}

impl NetworkMembership {
    /// Create a new unregistered membership
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if node is part of a network
    pub fn is_registered(&self) -> bool {
        matches!(
            self.status,
            RegistrationStatus::Registered | RegistrationStatus::Active
        )
    }

    /// Get primary doorway URL
    #[allow(dead_code)]
    pub fn primary_doorway(&self) -> Option<&str> {
        self.doorways
            .iter()
            .find(|d| d.is_primary)
            .map(|d| d.url.as_str())
    }

    /// Get connected doorway count
    #[allow(dead_code)]
    pub fn connected_doorway_count(&self) -> usize {
        self.doorways
            .iter()
            .filter(|d| d.status == DoorwayStatus::Connected)
            .count()
    }

    /// Get count of actively syncing apps
    #[allow(dead_code)]
    pub fn syncing_app_count(&self) -> usize {
        self.connected_apps.iter().filter(|a| a.is_syncing).count()
    }
}
