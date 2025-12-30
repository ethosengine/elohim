//! Configuration Discovery Service
//!
//! Discovers zome capabilities from the Holochain conductor:
//! - Import configuration via `__doorway_import_config`
//! - Cache rules via `__doorway_cache_rules`
//!
//! ## Discovery Flow
//!
//! 1. Connect to conductor admin interface
//! 2. List installed apps to get cell info (dna_hash, agent_pub_key)
//! 3. For each cell, call discovery functions
//! 4. Store discovered configs in appropriate stores

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use super::{ImportConfigStore, ImportConfig};
use crate::worker::ZomeCallConfig;

// =============================================================================
// Types
// =============================================================================

/// Discovery service configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Conductor admin URL
    pub admin_url: String,
    /// Installed app ID to discover
    pub installed_app_id: String,
    /// Default zome name for calls
    pub zome_name: String,
    /// Timeout for discovery calls
    pub timeout: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            installed_app_id: "elohim".to_string(),
            zome_name: "content_store".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Cell info from conductor
#[derive(Debug, Clone)]
pub struct CellInfo {
    pub dna_hash: String,
    pub agent_pub_key: String,
    pub role_name: String,
}

/// Discovery result
#[derive(Debug)]
pub struct DiscoveryResult {
    pub cells_discovered: usize,
    pub import_configs_found: usize,
    pub errors: Vec<String>,
}

// =============================================================================
// Discovery Service
// =============================================================================

/// Discovers zome configurations from conductor
pub struct DiscoveryService {
    config: DiscoveryConfig,
    zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
    import_config_store: Arc<ImportConfigStore>,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(
        config: DiscoveryConfig,
        zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
        import_config_store: Arc<ImportConfigStore>,
    ) -> Self {
        Self {
            config,
            zome_configs,
            import_config_store,
        }
    }

    /// Run discovery - connects to conductor and discovers all configs
    pub async fn discover(&self) -> DiscoveryResult {
        let mut result = DiscoveryResult {
            cells_discovered: 0,
            import_configs_found: 0,
            errors: vec![],
        };

        info!(
            "Starting discovery for app '{}' at {}",
            self.config.installed_app_id, self.config.admin_url
        );

        // Step 1: Get cell info from admin interface
        let cells = match self.get_cells().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get cells: {}", e);
                result.errors.push(format!("Failed to get cells: {}", e));
                return result;
            }
        };

        info!("Found {} cells in app", cells.len());
        result.cells_discovered = cells.len();

        // Step 2: For each cell, store ZomeCallConfig and discover import config
        for cell in cells {
            let zome_config = ZomeCallConfig {
                dna_hash: cell.dna_hash.clone(),
                agent_pub_key: cell.agent_pub_key.clone(),
                zome_name: self.config.zome_name.clone(),
                app_id: self.config.installed_app_id.clone(),
            };

            // Store zome config for later use
            self.zome_configs.insert(cell.dna_hash.clone(), zome_config.clone());
            debug!(dna = %cell.dna_hash, role = %cell.role_name, "Stored ZomeCallConfig");

            // Discover import config
            match self.discover_import_config(&cell, &zome_config).await {
                Ok(Some(import_config)) => {
                    self.import_config_store.set_config(&cell.dna_hash, import_config);
                    result.import_configs_found += 1;
                    info!(
                        dna = %cell.dna_hash,
                        role = %cell.role_name,
                        "Discovered import config"
                    );
                }
                Ok(None) => {
                    self.import_config_store.mark_discovered(&cell.dna_hash);
                    debug!(dna = %cell.dna_hash, "No import config (function not found or disabled)");
                }
                Err(e) => {
                    warn!(dna = %cell.dna_hash, error = %e, "Failed to discover import config");
                    result.errors.push(format!("DNA {}: {}", cell.dna_hash, e));
                }
            }
        }

        info!(
            "Discovery complete: {} cells, {} import configs, {} errors",
            result.cells_discovered, result.import_configs_found, result.errors.len()
        );

        result
    }

    /// Get cells from conductor admin interface
    async fn get_cells(&self) -> Result<Vec<CellInfo>, String> {
        // Connect to admin interface
        let host = self.config.admin_url
            .split("//")
            .last()
            .unwrap_or("localhost:4444");

        let request = Request::builder()
            .uri(&self.config.admin_url)
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| format!("Failed to build request: {}", e))?;

        let (ws_stream, _) = tokio::time::timeout(
            self.config.timeout,
            connect_async_with_config(request, None, false),
        )
        .await
        .map_err(|_| "Timeout connecting to admin interface")?
        .map_err(|e| format!("WebSocket connect failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Send list_apps request
        let list_apps = rmpv::Value::Map(vec![
            (Value::String("type".into()), Value::String("list_apps".into())),
            (Value::String("data".into()), Value::Nil),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &list_apps)
            .map_err(|e| format!("Failed to encode: {}", e))?;

        write.send(Message::Binary(buf)).await
            .map_err(|e| format!("Failed to send: {}", e))?;

        // Read response
        let response = tokio::time::timeout(self.config.timeout, read.next())
            .await
            .map_err(|_| "Timeout waiting for response")?
            .ok_or("Connection closed")?
            .map_err(|e| format!("Read error: {}", e))?;

        let response_bytes = match response {
            Message::Binary(b) => b,
            _ => return Err("Unexpected response type".to_string()),
        };

        // Parse response to extract cell info
        self.parse_list_apps_response(&response_bytes)
    }

    /// Parse list_apps response to extract cell info
    fn parse_list_apps_response(&self, response: &[u8]) -> Result<Vec<CellInfo>, String> {
        let value: Value = rmpv::decode::read_value(&mut &response[..])
            .map_err(|e| format!("Failed to decode response: {}", e))?;

        // Response format: { type: "list_apps", data: [{ installed_app_id, cell_info: [...] }] }
        let data = value.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("data")))
            .map(|(_, v)| v)
            .ok_or("Missing 'data' field")?;

        let apps = data.as_array().ok_or("Expected array of apps")?;

        let mut cells = Vec::new();

        for app in apps {
            let app_map = app.as_map().ok_or("Expected app object")?;

            // Check if this is our app
            let app_id = app_map.iter()
                .find(|(k, _)| k.as_str() == Some("installed_app_id"))
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("");

            if app_id != self.config.installed_app_id {
                continue;
            }

            // Extract cell_info
            let cell_info = app_map.iter()
                .find(|(k, _)| k.as_str() == Some("cell_info"))
                .map(|(_, v)| v)
                .ok_or("Missing cell_info")?;

            // cell_info is an array of [role_name, cell_info_variant]
            if let Some(arr) = cell_info.as_array() {
                for item in arr {
                    if let Some(pair) = item.as_array() {
                        if pair.len() >= 2 {
                            let role_name = pair[0].as_str().unwrap_or("unknown").to_string();

                            // cell_info_variant is typically { provisioned: { cell_id: [dna_hash, agent] } }
                            // or { stem: ... } etc.
                            if let Some(variant_map) = pair[1].as_map() {
                                for (variant_type, variant_data) in variant_map {
                                    if variant_type.as_str() == Some("provisioned") {
                                        if let Some(data_map) = variant_data.as_map() {
                                            if let Some(cell_id) = data_map.iter()
                                                .find(|(k, _)| k.as_str() == Some("cell_id"))
                                                .map(|(_, v)| v)
                                            {
                                                if let Some(id_arr) = cell_id.as_array() {
                                                    if id_arr.len() >= 2 {
                                                        let dna_hash = encode_base64(id_arr[0].as_slice().unwrap_or(&[]));
                                                        let agent_pub_key = encode_base64(id_arr[1].as_slice().unwrap_or(&[]));

                                                        cells.push(CellInfo {
                                                            dna_hash,
                                                            agent_pub_key,
                                                            role_name: role_name.clone(),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(cells)
    }

    /// Discover import config from a cell
    async fn discover_import_config(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<ImportConfig>, String> {
        // TODO: Actually call __doorway_import_config on the zome
        // For now, we'll use a hardcoded config that matches what the zome declares
        //
        // The actual implementation would:
        // 1. Build a zome call using ZomeCallBuilder
        // 2. Send it via WorkerPool
        // 3. Parse the ImportConfig response
        //
        // For now, assume the elohim DNA has the standard import config

        // Return hardcoded config for elohim DNA
        Ok(Some(ImportConfig {
            enabled: true,
            base_route: "/import".to_string(),
            batch_types: vec![
                doorway_client::ImportBatchType::new("content")
                    .queue_fn("queue_import")
                    .process_fn("process_import_chunk")
                    .status_fn("get_import_status")
                    .max_items(5000)
                    .chunk_size(50),
            ],
            require_auth: false,
            allowed_agents: None,
        }))
    }
}

/// Encode bytes to base64
fn encode_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Spawn discovery as a background task
pub fn spawn_discovery_task(
    config: DiscoveryConfig,
    zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
    import_config_store: Arc<ImportConfigStore>,
) -> tokio::task::JoinHandle<DiscoveryResult> {
    tokio::spawn(async move {
        // Wait a bit for conductor to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        let service = DiscoveryService::new(config, zome_configs, import_config_store);
        service.discover().await
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.installed_app_id, "elohim");
        assert_eq!(config.zome_name, "content_store");
    }
}
