//! Configuration for elohim-storage

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default storage directory
pub fn default_storage_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elohim-storage")
}

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Storage directory for blobs
    #[serde(default = "default_storage_dir")]
    pub storage_dir: PathBuf,

    /// Holochain admin websocket URL
    #[serde(default = "default_admin_url")]
    pub holochain_admin_url: String,

    /// App ID to connect to
    #[serde(default = "default_app_id")]
    pub app_id: String,

    /// DNA role name
    #[serde(default = "default_role_name")]
    pub role_name: String,

    /// Zome name for blob operations
    #[serde(default = "default_zome_name")]
    pub zome_name: String,

    /// Maximum storage size in bytes (0 = unlimited)
    #[serde(default)]
    pub max_storage_bytes: u64,

    /// Enable LRU eviction when max storage reached
    #[serde(default = "default_true")]
    pub enable_eviction: bool,

    /// Minimum replicas before considering eviction
    #[serde(default = "default_min_replicas")]
    pub min_replicas_for_eviction: u32,

    /// Sync interval in seconds (register new blobs in DNA)
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u64,

    /// P2P port for direct blob transfers
    #[serde(default = "default_p2p_port")]
    pub p2p_port: u16,

    /// HTTP API port for shard storage
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// P2P bootstrap nodes for content discovery
    /// Format: /ip4/1.2.3.4/tcp/9876/p2p/12D3KooW...
    #[serde(default)]
    pub p2p_bootstrap_nodes: Vec<String>,

    /// Enable mDNS for local network discovery
    #[serde(default = "default_true")]
    pub enable_mdns: bool,
}

fn default_http_port() -> u16 {
    8090
}

fn default_admin_url() -> String {
    "ws://localhost:4444".to_string()
}

fn default_app_id() -> String {
    "elohim".to_string()
}

fn default_role_name() -> String {
    "elohim".to_string()
}

fn default_zome_name() -> String {
    "content_store".to_string()
}

fn default_true() -> bool {
    true
}

fn default_min_replicas() -> u32 {
    2
}

fn default_sync_interval() -> u64 {
    60
}

fn default_p2p_port() -> u16 {
    9876
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage_dir: default_storage_dir(),
            holochain_admin_url: default_admin_url(),
            app_id: default_app_id(),
            role_name: default_role_name(),
            zome_name: default_zome_name(),
            max_storage_bytes: 0,
            enable_eviction: true,
            min_replicas_for_eviction: 2,
            sync_interval_secs: 60,
            p2p_port: 9876,
            http_port: 8090,
            p2p_bootstrap_nodes: Vec::new(),
            enable_mdns: true,
        }
    }
}

impl Config {
    /// Load config from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Get blobs directory
    pub fn blobs_dir(&self) -> PathBuf {
        self.storage_dir.join("blobs")
    }

    /// Get metadata database path
    pub fn metadata_db_path(&self) -> PathBuf {
        self.storage_dir.join("metadata.sled")
    }

    /// Get config file path
    pub fn config_path(&self) -> PathBuf {
        self.storage_dir.join("config.toml")
    }
}
