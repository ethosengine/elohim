//! Host document schema
//!
//! Stores registered Holochain host/operator information for multi-operator support.

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Collection name for hosts
pub const HOST_COLLECTION: &str = "hosts";

/// Host status
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostStatus {
    /// Host is online and accepting connections
    #[default]
    Online,
    /// Host is offline or unreachable
    Offline,
    /// Host is in maintenance mode
    Maintenance,
    /// Host has been deregistered
    Deregistered,
}

/// Host document stored in MongoDB
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HostDoc {
    /// MongoDB document ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,

    /// Common metadata
    #[serde(default)]
    pub metadata: Metadata,

    /// Unique node ID (UUID) for this host
    pub node_id: String,

    /// Human-readable name for the host
    pub name: String,

    /// Host URL for NATS routing (e.g., "host.example.com")
    pub host_url: String,

    /// Holochain conductor admin WebSocket URL
    pub conductor_url: String,

    /// Minimum app interface port
    pub app_port_min: u16,

    /// Maximum app interface port
    pub app_port_max: u16,

    /// Current status
    #[serde(default)]
    pub status: HostStatus,

    /// Last heartbeat time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<DateTime>,

    /// Operator/owner user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<ObjectId>,

    /// Geographic region (for routing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Current number of active connections
    #[serde(default)]
    pub active_connections: i32,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: i32,

    /// Number of agents currently hosted across all conductors
    #[serde(default)]
    pub agent_count: i32,

    /// Number of conductors in the pool
    #[serde(default)]
    pub conductor_count: i32,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version of the Doorway software
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

fn default_max_connections() -> i32 {
    1000
}

impl HostDoc {
    /// Create a new host document
    pub fn new(
        node_id: String,
        name: String,
        host_url: String,
        conductor_url: String,
        app_port_min: u16,
        app_port_max: u16,
    ) -> Self {
        Self {
            _id: None,
            metadata: Metadata::new(),
            node_id,
            name,
            host_url,
            conductor_url,
            app_port_min,
            app_port_max,
            status: HostStatus::Online,
            last_heartbeat: Some(DateTime::now()),
            operator_id: None,
            region: None,
            active_connections: 0,
            max_connections: default_max_connections(),
            agent_count: 0,
            conductor_count: 0,
            description: None,
            version: None,
        }
    }

    /// Check if the host is available for connections
    pub fn is_available(&self) -> bool {
        self.status == HostStatus::Online && self.active_connections < self.max_connections
    }

    /// Update the heartbeat timestamp
    pub fn touch_heartbeat(&mut self) {
        self.last_heartbeat = Some(DateTime::now());
    }
}

impl IntoIndexes for HostDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Unique index on node_id
            (
                doc! { "node_id": 1 },
                Some(
                    IndexOptions::builder()
                        .unique(true)
                        .name("node_id_unique".to_string())
                        .build(),
                ),
            ),
            // Index on status for finding available hosts
            (
                doc! { "status": 1 },
                Some(
                    IndexOptions::builder()
                        .name("status_index".to_string())
                        .build(),
                ),
            ),
            // Index on region for geographic routing
            (
                doc! { "region": 1 },
                Some(
                    IndexOptions::builder()
                        .name("region_index".to_string())
                        .sparse(true)
                        .build(),
                ),
            ),
            // Index on operator_id
            (
                doc! { "operator_id": 1 },
                Some(
                    IndexOptions::builder()
                        .name("operator_id_index".to_string())
                        .sparse(true)
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for HostDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}
