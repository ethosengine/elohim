//! ContentServer Bridge - Register with infrastructure zome for P2P discovery
//!
//! This module provides the bridge between elohim-storage and the Holochain
//! infrastructure DNA's ContentServer registration system.
//!
//! ## How It Works
//!
//! 1. elohim-storage stores a blob locally
//! 2. Calls `register_content_server` in infrastructure zome
//! 3. Other nodes use `find_publishers` to discover who has the content
//! 4. Shards are transferred via libp2p (or HTTP fallback)
//!
//! ## ContentServer Entry (in DNA)
//!
//! ```ignore
//! ContentServer {
//!     content_hash: String,      // sha256-xxx
//!     capability: String,        // blob, html5_app, media_stream, etc.
//!     serve_url: Option<String>, // HTTP endpoint for legacy access
//!     online: bool,
//!     priority: u8,              // 0-100, higher = preferred
//!     region: Option<String>,
//!     bandwidth_mbps: Option<u32>,
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::conductor_client::ConductorClient;
use crate::error::StorageError;

#[cfg(feature = "p2p")]
use crate::identity::NodeIdentity;

/// Configuration for ContentServer bridge
#[derive(Debug, Clone)]
pub struct ContentServerConfig {
    /// Infrastructure DNA hash (base64)
    pub dna_hash: String,
    /// Zome name in infrastructure DNA
    pub zome_name: String,
    /// HTTP serve URL (if exposing HTTP API)
    pub serve_url: Option<String>,
    /// Priority for this node (0-100)
    pub priority: u8,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

impl Default for ContentServerConfig {
    fn default() -> Self {
        Self {
            dna_hash: String::new(),
            zome_name: "infrastructure".to_string(),
            serve_url: None,
            priority: 50,
            heartbeat_interval_secs: 60,
        }
    }
}

/// Storage endpoint for content serving (NEW)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEndpointInput {
    /// Base URL for fetching content (hash appended)
    pub url: String,
    /// Protocol type: "http", "https", "libp2p"
    pub protocol: String,
    /// Priority within this server (0-100, higher = preferred)
    pub priority: Option<u8>,
}

/// Input for registering a content server (matches zome input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterContentServerInput {
    /// Content hash this server can provide (e.g., "sha256-abc123")
    /// Use "*" for wildcard registration (can serve any content of this capability)
    pub content_hash: String,
    /// Capability: blob, html5_app, media_stream, learning_package, custom
    pub capability: String,
    /// URL where this server accepts content requests (DEPRECATED - use endpoints)
    pub serve_url: Option<String>,
    /// Multiple reachable endpoints for content fetching (NEW)
    pub endpoints: Option<Vec<StorageEndpointInput>>,
    /// Server priority (0-100, higher = preferred)
    pub priority: Option<u8>,
    /// Geographic region for latency-based routing
    pub region: Option<String>,
    /// Bandwidth capacity in Mbps
    pub bandwidth_mbps: Option<u32>,
}

/// Output from content server registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentServerOutput {
    /// Action hash of the ContentServer entry
    pub action_hash: Vec<u8>,
    /// The registered server info
    pub server: ContentServerInfo,
}

/// Storage endpoint info (matches DNA entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEndpointInfo {
    pub url: String,
    pub protocol: String,
    pub priority: u8,
}

/// ContentServer info (matches DNA entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentServerInfo {
    pub content_hash: String,
    pub capability: String,
    pub serve_url: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<StorageEndpointInfo>,
    pub online: bool,
    pub priority: u8,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

/// Input for finding publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersInput {
    /// Content hash to find publishers for
    pub content_hash: String,
    /// Optional: filter by capability
    pub capability: Option<String>,
    /// Optional: prefer publishers in this region
    pub prefer_region: Option<String>,
    /// Maximum number of publishers to return (default: 10)
    pub limit: Option<usize>,
    /// Only return online publishers (default: true)
    pub online_only: Option<bool>,
}

/// Output from finding publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersOutput {
    /// Content hash queried
    pub content_hash: String,
    /// Found publishers, sorted by priority
    pub publishers: Vec<ContentServerOutput>,
}

/// Bridge to infrastructure zome's ContentServer functions
pub struct ContentServerBridge {
    /// Conductor client for zome calls
    conductor_client: Arc<ConductorClient>,
    /// Configuration
    config: ContentServerConfig,
    /// Cell ID (dna_hash + agent_pubkey) - cached after first discovery
    cell_id: Option<Vec<u8>>,
    /// Agent public key
    agent_pubkey: String,
    /// Registered content servers (action_hash by content_hash)
    registrations: std::collections::HashMap<String, Vec<u8>>,
}

impl ContentServerBridge {
    /// Create a new bridge
    pub fn new(
        conductor_client: Arc<ConductorClient>,
        config: ContentServerConfig,
        agent_pubkey: String,
    ) -> Self {
        Self {
            conductor_client,
            config,
            cell_id: None,
            agent_pubkey,
            registrations: std::collections::HashMap::new(),
        }
    }

    /// Create from NodeIdentity (when p2p feature is enabled)
    #[cfg(feature = "p2p")]
    pub fn from_identity(
        conductor_client: Arc<ConductorClient>,
        config: ContentServerConfig,
        identity: &NodeIdentity,
    ) -> Self {
        Self::new(
            conductor_client,
            config,
            identity.agent_pubkey().to_string(),
        )
    }

    /// Set the cell ID for zome calls
    pub fn set_cell_id(&mut self, cell_id: Vec<u8>) {
        self.cell_id = Some(cell_id);
    }

    /// Get the cell ID (returns error if not set)
    fn get_cell_id(&self) -> Result<&[u8], StorageError> {
        self.cell_id
            .as_ref()
            .map(|v| v.as_slice())
            .ok_or_else(|| StorageError::Config("Cell ID not set".into()))
    }

    /// Register this node as serving a content hash (uses default serve_url)
    pub async fn register_content(
        &mut self,
        content_hash: &str,
        capability: &str,
    ) -> Result<Vec<u8>, StorageError> {
        self.register_content_with_endpoints(content_hash, capability, None).await
    }

    /// Register this node as serving a content hash with explicit endpoints
    ///
    /// If endpoints is None, falls back to config.serve_url for backwards compatibility.
    pub async fn register_content_with_endpoints(
        &mut self,
        content_hash: &str,
        capability: &str,
        endpoints: Option<Vec<StorageEndpointInput>>,
    ) -> Result<Vec<u8>, StorageError> {
        let cell_id = self.get_cell_id()?;

        // Build endpoints list: use provided, or create from serve_url
        let endpoints = endpoints.or_else(|| {
            self.config.serve_url.as_ref().map(|url| {
                let protocol = if url.starts_with("https://") { "https" } else { "http" };
                vec![StorageEndpointInput {
                    url: url.clone(),
                    protocol: protocol.to_string(),
                    priority: Some(self.config.priority),
                }]
            })
        });

        let input = RegisterContentServerInput {
            content_hash: content_hash.to_string(),
            capability: capability.to_string(),
            serve_url: self.config.serve_url.clone(), // Keep for backwards compat
            endpoints,
            priority: Some(self.config.priority),
            region: None, // TODO: Get from identity
            bandwidth_mbps: None,
        };

        let payload = rmp_serde::to_vec(&input)
            .map_err(|e| StorageError::Internal(format!("Failed to encode input: {}", e)))?;

        debug!(
            content_hash = %content_hash,
            capability = %capability,
            "Registering content server"
        );

        let result = self
            .conductor_client
            .call_zome(cell_id, &self.config.zome_name, "register_content_server", &payload)
            .await?;

        // Parse response to get action_hash
        let output: ContentServerOutput = rmp_serde::from_slice(&result)
            .map_err(|e| StorageError::Internal(format!("Failed to decode output: {}", e)))?;

        // Cache the registration
        self.registrations
            .insert(content_hash.to_string(), output.action_hash.clone());

        info!(
            content_hash = %content_hash,
            "Content server registered"
        );

        Ok(output.action_hash)
    }

    /// Find publishers for a content hash
    pub async fn find_publishers(
        &self,
        content_hash: &str,
        prefer_region: Option<&str>,
    ) -> Result<Vec<PublisherInfo>, StorageError> {
        let cell_id = self.get_cell_id()?;

        let input = FindPublishersInput {
            content_hash: content_hash.to_string(),
            capability: None,
            prefer_region: prefer_region.map(|s| s.to_string()),
            limit: Some(10),
            online_only: Some(true),
        };

        let payload = rmp_serde::to_vec(&input)
            .map_err(|e| StorageError::Internal(format!("Failed to encode input: {}", e)))?;

        debug!(content_hash = %content_hash, "Finding publishers");

        let result = self
            .conductor_client
            .call_zome(cell_id, &self.config.zome_name, "find_publishers", &payload)
            .await?;

        let output: FindPublishersOutput = rmp_serde::from_slice(&result)
            .map_err(|e| StorageError::Internal(format!("Failed to decode output: {}", e)))?;

        // Convert to PublisherInfo
        let publishers = output
            .publishers
            .into_iter()
            .map(|p| PublisherInfo::from(p.server))
            .collect();

        Ok(publishers)
    }

    /// Update heartbeat for a registered content
    pub async fn heartbeat(&self, content_hash: &str) -> Result<(), StorageError> {
        let action_hash = self
            .registrations
            .get(content_hash)
            .ok_or_else(|| {
                StorageError::Internal(format!("Content not registered: {}", content_hash))
            })?
            .clone();

        let cell_id = self.get_cell_id()?;

        // The zome expects the action_hash directly
        let payload = rmp_serde::to_vec(&action_hash)
            .map_err(|e| StorageError::Internal(format!("Failed to encode input: {}", e)))?;

        debug!(content_hash = %content_hash, "Sending heartbeat");

        self.conductor_client
            .call_zome(
                cell_id,
                &self.config.zome_name,
                "update_content_server_heartbeat",
                &payload,
            )
            .await?;

        Ok(())
    }

    /// Mark content as offline (graceful shutdown)
    pub async fn go_offline(&self, content_hash: &str) -> Result<(), StorageError> {
        let action_hash = self
            .registrations
            .get(content_hash)
            .ok_or_else(|| {
                StorageError::Internal(format!("Content not registered: {}", content_hash))
            })?
            .clone();

        let cell_id = self.get_cell_id()?;

        let payload = rmp_serde::to_vec(&action_hash)
            .map_err(|e| StorageError::Internal(format!("Failed to encode input: {}", e)))?;

        info!(content_hash = %content_hash, "Marking content offline");

        self.conductor_client
            .call_zome(
                cell_id,
                &self.config.zome_name,
                "mark_content_server_offline",
                &payload,
            )
            .await?;

        Ok(())
    }

    /// Mark all registered content as offline (shutdown)
    pub async fn go_offline_all(&self) -> Result<(), StorageError> {
        for content_hash in self.registrations.keys() {
            if let Err(e) = self.go_offline(content_hash).await {
                warn!(content_hash = %content_hash, error = %e, "Failed to mark offline");
            }
        }
        Ok(())
    }

    /// Start heartbeat loop (run in background)
    pub async fn run_heartbeat_loop(&self, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        let interval = std::time::Duration::from_secs(self.config.heartbeat_interval_secs);

        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    for content_hash in self.registrations.keys() {
                        if let Err(e) = self.heartbeat(content_hash).await {
                            warn!(content_hash = %content_hash, error = %e, "Heartbeat failed");
                        }
                    }
                }
                _ = shutdown.recv() => {
                    info!("Heartbeat loop shutting down");
                    break;
                }
            }
        }
    }
}

/// Publisher information for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    /// Agent public key of the publisher
    pub agent_pubkey: String,
    /// HTTP serve URL (if available)
    pub serve_url: Option<String>,
    /// Priority (0-100, higher = preferred)
    pub priority: u8,
    /// Region for latency-based routing
    pub region: Option<String>,
    /// Bandwidth capacity
    pub bandwidth_mbps: Option<u32>,
    /// Whether the publisher is online
    pub online: bool,
}

impl From<ContentServerInfo> for PublisherInfo {
    fn from(server: ContentServerInfo) -> Self {
        Self {
            // NOTE: ContentServerInfo doesn't have agent_pubkey directly,
            // the DNA entry is linked to the author. For now, leave empty.
            agent_pubkey: String::new(),
            serve_url: server.serve_url,
            priority: server.priority,
            region: server.region,
            bandwidth_mbps: server.bandwidth_mbps,
            online: server.online,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_input_serialization() {
        let input = RegisterContentServerInput {
            content_hash: "sha256-abc123".to_string(),
            capability: "blob".to_string(),
            serve_url: Some("http://localhost:8080".to_string()),
            endpoints: Some(vec![StorageEndpointInput {
                url: "http://localhost:8080/store".to_string(),
                protocol: "http".to_string(),
                priority: Some(100),
            }]),
            priority: Some(50),
            region: Some("us-west".to_string()),
            bandwidth_mbps: Some(100),
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: RegisterContentServerInput = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(decoded.content_hash, input.content_hash);
        assert_eq!(decoded.capability, input.capability);
        assert_eq!(decoded.priority, input.priority);
        assert!(decoded.endpoints.is_some());
        assert_eq!(decoded.endpoints.unwrap().len(), 1);
    }

    #[test]
    fn test_find_publishers_input() {
        let input = FindPublishersInput {
            content_hash: "sha256-abc123".to_string(),
            capability: None,
            prefer_region: Some("eu-central".to_string()),
            limit: Some(5),
            online_only: Some(true),
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: FindPublishersInput = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(decoded.content_hash, input.content_hash);
        assert_eq!(decoded.prefer_region, input.prefer_region);
    }
}
