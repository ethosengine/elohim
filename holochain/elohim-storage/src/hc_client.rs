//! Holochain Client Wrapper
//!
//! Uses the official holochain_client crate for proper signed zome calls.
//! Holochain 0.6+ requires all zome calls to be signed with nonce, expires_at,
//! and ed25519 signature. This module handles:
//!
//! 1. Connecting to admin and app websockets
//! 2. Authorizing signing credentials via admin API
//! 3. Making signed zome calls that the conductor will accept
//!
//! ## Usage
//!
//! ```ignore
//! let client = HcClient::connect(HcClientConfig {
//!     admin_url: "localhost:4444".to_string(),
//!     app_url: "localhost:4445".to_string(),
//!     app_id: "elohim".to_string(),
//!     role: Some("lamad".to_string()),
//! }).await?;
//! let result = client.call_zome("content_store", "process_import_chunk", payload).await?;
//! ```

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use holochain_client::{
    AdminWebsocket, AllowedOrigins, AppWebsocket, ClientAgentSigner,
    AuthorizeSigningCredentialsPayload, CellId, ExternIO, ZomeCallTarget,
};

use crate::error::StorageError;

/// Conductor health information for backpressure decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorHealth {
    /// Storage information
    pub storage: Option<StorageHealth>,
    /// Network statistics
    pub network: Option<NetworkHealth>,
    /// Raw responses for debugging (JSON strings)
    pub raw_storage: Option<String>,
    pub raw_network_stats: Option<String>,
    pub raw_network_metrics: Option<String>,
}

/// Storage health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Total bytes used by entries
    pub bytes_used: u64,
    /// Number of entries
    pub entry_count: u64,
}

/// Network health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    /// Number of connected peers
    pub peer_count: Option<u64>,
    /// Any additional stats we can extract
    pub details: String,
}

/// Configuration for HcClient
#[derive(Debug, Clone)]
pub struct HcClientConfig {
    /// Admin websocket URL (e.g., "ws://localhost:4444")
    pub admin_url: String,
    /// App websocket URL (e.g., "ws://localhost:4445")
    pub app_url: String,
    /// Installed app ID
    pub app_id: String,
    /// Role to use for the cell (e.g., "lamad")
    pub role: Option<String>,
}

/// Holochain client with proper signing support
pub struct HcClient {
    config: HcClientConfig,
    /// Admin websocket connection
    admin_ws: AdminWebsocket,
    /// App websocket connection with signing
    app_ws: AppWebsocket,
    /// The cell ID for zome calls
    cell_id: CellId,
    /// Signing credentials
    signer: Arc<ClientAgentSigner>,
}

impl HcClient {
    /// Strip ws:// or wss:// prefix from URL to get socket address
    /// holochain_client expects "host:port" format, not "ws://host:port"
    fn to_socket_addr(url: &str) -> String {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("wss://") {
            rest.to_string()
        } else if let Some(rest) = url.strip_prefix("ws://") {
            rest.to_string()
        } else {
            url.to_string()
        }
    }

    /// Connect to Holochain conductor and set up signing credentials
    pub async fn connect(config: HcClientConfig) -> Result<Self, StorageError> {
        // Convert URLs to socket addresses (strip ws:// prefix)
        let admin_addr = Self::to_socket_addr(&config.admin_url);
        let app_addr = Self::to_socket_addr(&config.app_url);

        info!(
            admin_addr = %admin_addr,
            app_addr = %app_addr,
            app_id = %config.app_id,
            "Connecting to Holochain conductor with signing support"
        );

        // Connect to admin interface (socket_addr, origin)
        let admin_ws = AdminWebsocket::connect(&admin_addr, None)
            .await
            .map_err(|e| StorageError::Connection(format!("Admin connect failed: {}", e)))?;

        info!("Connected to admin interface");

        // Ensure an app interface exists on the expected port.
        // The embedded conductor (tauri-plugin-holochain) uses random ports,
        // so we attach one on our expected port via admin API.
        let app_port: u16 = app_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4445);
        match admin_ws
            .attach_app_interface(app_port, None, AllowedOrigins::Any, None)
            .await
        {
            Ok(port) => info!(port, "Attached app interface"),
            Err(e) => {
                // May already be attached from a previous call — continue
                warn!("attach_app_interface on port {}: {} (may already exist)", app_port, e);
            }
        }

        // Get app info to find cell
        let apps = admin_ws
            .list_apps(None)
            .await
            .map_err(|e| StorageError::Connection(format!("list_apps failed: {}", e)))?;

        let app_info = apps
            .iter()
            .find(|a| a.installed_app_id == config.app_id)
            .ok_or_else(|| StorageError::NotFound(format!("App '{}' not found", config.app_id)))?;

        // Find the cell for the specified role
        let cell_id = if let Some(role) = &config.role {
            app_info
                .cell_info
                .get(role)
                .and_then(|cells| cells.first())
                .and_then(|cell| match cell {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
                .ok_or_else(|| StorageError::NotFound(format!("Role '{}' not found", role)))?
        } else {
            // Use first available cell
            app_info
                .cell_info
                .values()
                .next()
                .and_then(|cells| cells.first())
                .and_then(|cell| match cell {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
                .ok_or_else(|| StorageError::NotFound("No cells found".to_string()))?
        };

        info!(
            dna_hash = %hex::encode(&cell_id.dna_hash().get_raw_39()[..8]),
            "Found cell"
        );

        // Create signing credentials
        let signer = ClientAgentSigner::default();

        // Authorize signing credentials for this cell
        let credentials = admin_ws
            .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                cell_id: cell_id.clone(),
                functions: None, // All functions
            })
            .await
            .map_err(|e| StorageError::Connection(format!("authorize_signing_credentials failed: {}", e)))?;

        // Add credentials to signer
        signer.add_credentials(cell_id.clone(), credentials);
        info!("Signing credentials authorized");

        // Get app auth token
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: config.app_id.clone(),
                expiry_seconds: 3600, // 1 hour
                single_use: false,
            })
            .await
            .map_err(|e| StorageError::Connection(format!("issue_app_auth_token failed: {}", e)))?;

        // Connect to app interface with signer (socket_addr, token, signer, origin)
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);
        let app_ws = AppWebsocket::connect(
            &app_addr,
            token.token,
            signer_arc.clone(),
            None,
        )
        .await
        .map_err(|e| StorageError::Connection(format!("App connect failed: {}", e)))?;

        info!("Connected to app interface with signing");

        Ok(Self {
            config,
            admin_ws,
            app_ws,
            cell_id,
            signer: signer_arc,
        })
    }

    /// Make a signed zome call
    pub async fn call_zome(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        debug!(
            zome = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            "Making signed zome call"
        );

        // The holochain_client handles signing automatically
        // Use ExternIO::from() for raw bytes - payload is already MessagePack encoded
        let result = self.app_ws
            .call_zome(
                ZomeCallTarget::CellId(self.cell_id.clone()),
                zome_name.into(),
                fn_name.into(),
                ExternIO::from(payload),
            )
            .await
            .map_err(|e| StorageError::Conductor(format!("Zome call failed: {}", e)))?;

        // Return raw bytes - caller will deserialize as needed
        Ok(result.into_vec())
    }

    /// Get the cell ID
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    /// Get DNA hash bytes
    pub fn dna_hash(&self) -> Vec<u8> {
        self.cell_id.dna_hash().get_raw_39().to_vec()
    }

    /// Get agent pubkey bytes
    pub fn agent_pub_key(&self) -> Vec<u8> {
        self.cell_id.agent_pubkey().get_raw_39().to_vec()
    }

    /// Get conductor health metrics for backpressure decisions
    ///
    /// Fetches storage info, network stats, and network metrics from the conductor.
    /// Returns raw JSON responses for evaluation - we can parse specific fields later
    /// once we understand the response structure.
    pub async fn get_health(&self) -> ConductorHealth {
        let mut health = ConductorHealth {
            storage: None,
            network: None,
            raw_storage: None,
            raw_network_stats: None,
            raw_network_metrics: None,
        };

        // Fetch storage info
        match self.admin_ws.storage_info().await {
            Ok(storage_info) => {
                // Convert to JSON for inspection
                let json = serde_json::to_string_pretty(&storage_info)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_storage = Some(json.clone());

                // Try to extract key metrics
                // StorageInfo has blobs.used_by_entries field
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let bytes_used = parsed["blobs"]["used_by_entries"]
                        .as_u64()
                        .unwrap_or(0);
                    let entry_count = parsed["blobs"]["used_by_entries_count"]
                        .as_u64()
                        .or_else(|| parsed["entries_count"].as_u64())
                        .unwrap_or(0);

                    health.storage = Some(StorageHealth {
                        bytes_used,
                        entry_count,
                    });
                }

                info!(
                    bytes_used = health.storage.as_ref().map(|s| s.bytes_used).unwrap_or(0),
                    "📊 STORAGE_INFO: Fetched conductor storage metrics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch storage_info");
                health.raw_storage = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network stats
        match self.admin_ws.dump_network_stats().await {
            Ok(network_stats) => {
                let json = serde_json::to_string_pretty(&network_stats)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_stats = Some(json.clone());

                // Extract what we can
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let peer_count = parsed["peer_count"]
                        .as_u64()
                        .or_else(|| parsed["peers"].as_array().map(|a| a.len() as u64));

                    health.network = Some(NetworkHealth {
                        peer_count,
                        details: format!("Keys: {:?}",
                            parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
                        ),
                    });
                }

                info!(
                    peer_count = health.network.as_ref().and_then(|n| n.peer_count),
                    "📊 NETWORK_STATS: Fetched conductor network statistics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_stats");
                health.raw_network_stats = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network metrics (more detailed DHT info)
        // dump_network_metrics takes optional DNA hash filter and DHT summary flag
        match self.admin_ws.dump_network_metrics(None, true).await {
            Ok(network_metrics) => {
                let json = serde_json::to_string_pretty(&network_metrics)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_metrics = Some(json);

                info!("📊 NETWORK_METRICS: Fetched conductor DHT metrics");
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_metrics");
                health.raw_network_metrics = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        health
    }

    /// Quick health check - just verify conductor is responsive
    pub async fn ping(&self) -> Result<(), StorageError> {
        // Use list_apps as a simple ping
        self.admin_ws
            .list_apps(None)
            .await
            .map_err(|e| StorageError::Connection(format!("Conductor ping failed: {}", e)))?;
        Ok(())
    }
}
