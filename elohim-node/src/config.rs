//! Node configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::update::UpdateConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub sync: SyncConfig,
    pub cluster: ClusterConfig,
    pub p2p: P2PConfig,
    pub storage: StorageConfig,
    pub api: ApiConfig,
    #[serde(default)]
    pub update: UpdateConfig,
    #[serde(default)]
    pub pod: PodConfig,
}

/// Pod (cluster operator) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodConfig {
    /// Whether the pod is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Decision interval in seconds
    #[serde(default = "default_decision_interval")]
    pub decision_interval_secs: u64,

    /// Path to rules file (optional)
    #[serde(default)]
    pub rules_file: Option<String>,

    /// Maximum actions per hour
    #[serde(default = "default_max_actions")]
    pub max_actions_per_hour: u32,

    /// Dry run mode (don't execute actions)
    #[serde(default)]
    pub dry_run: bool,
}

impl Default for PodConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            decision_interval_secs: default_decision_interval(),
            rules_file: None,
            max_actions_per_hour: default_max_actions(),
            dry_run: false,
        }
    }
}

fn default_decision_interval() -> u64 { 10 }
fn default_max_actions() -> u32 { 20 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Unique node identifier
    pub id: String,

    /// Data directory
    pub data_dir: PathBuf,

    /// Cluster name this node belongs to
    pub cluster_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Maximum document size in bytes
    #[serde(default = "default_max_doc_size")]
    pub max_document_size: usize,

    /// Sync interval in milliseconds
    #[serde(default = "default_sync_interval")]
    pub sync_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Enable mDNS discovery
    #[serde(default = "default_true")]
    pub mdns_enabled: bool,

    /// Shared secret for cluster membership
    pub cluster_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Listen addresses
    #[serde(default = "default_listen_addrs")]
    pub listen_addrs: Vec<String>,

    /// Bootstrap nodes for peer discovery
    #[serde(default)]
    pub bootstrap_nodes: Vec<String>,

    /// Doorway bootstrap URL for peer discovery via doorway
    #[serde(default)]
    pub doorway_bootstrap_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum storage capacity
    #[serde(default = "default_max_capacity")]
    pub max_capacity: String,

    /// Reed-Solomon shard redundancy
    #[serde(default = "default_shard_redundancy")]
    pub shard_redundancy: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// HTTP API port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// gRPC API port
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
}

// Defaults
fn default_max_doc_size() -> usize { 10 * 1024 * 1024 } // 10MB
fn default_sync_interval() -> u64 { 1000 }
fn default_true() -> bool { true }
fn default_listen_addrs() -> Vec<String> {
    vec![
        "/ip4/0.0.0.0/tcp/4001".to_string(),
        "/ip4/0.0.0.0/udp/4001/quic-v1".to_string(),
    ]
}
fn default_max_capacity() -> String { "500GB".to_string() }
fn default_shard_redundancy() -> u8 { 3 }
fn default_http_port() -> u16 { 8080 }
fn default_grpc_port() -> u16 { 9090 }

impl Default for Config {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                id: "node-1".to_string(),
                data_dir: PathBuf::from("/var/lib/elohim"),
                cluster_name: "default".to_string(),
            },
            sync: SyncConfig {
                max_document_size: default_max_doc_size(),
                sync_interval_ms: default_sync_interval(),
            },
            cluster: ClusterConfig {
                mdns_enabled: true,
                cluster_key: None,
            },
            p2p: P2PConfig {
                listen_addrs: default_listen_addrs(),
                bootstrap_nodes: vec![],
                doorway_bootstrap_url: None,
            },
            storage: StorageConfig {
                max_capacity: default_max_capacity(),
                shard_redundancy: default_shard_redundancy(),
            },
            api: ApiConfig {
                http_port: default_http_port(),
                grpc_port: default_grpc_port(),
            },
            update: UpdateConfig::default(),
            pod: PodConfig::default(),
        }
    }
}
