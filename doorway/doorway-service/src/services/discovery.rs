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
use doorway_client::DoorwayRoutes;
use tracing::{debug, info, warn};

use super::{ImportConfig, ImportConfigStore, RouteRegistry};
use crate::conductor::typed_admin::TypedAdminClient;
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
    pub routes_found: usize,
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
    route_registry: Option<Arc<RouteRegistry>>,
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
            route_registry: None,
        }
    }

    /// Create with route registry for dynamic route discovery
    pub fn with_route_registry(
        config: DiscoveryConfig,
        zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
        import_config_store: Arc<ImportConfigStore>,
        route_registry: Arc<RouteRegistry>,
    ) -> Self {
        Self {
            config,
            zome_configs,
            import_config_store,
            route_registry: Some(route_registry),
        }
    }

    /// Run discovery - connects to conductor and discovers all configs
    pub async fn discover(&self) -> DiscoveryResult {
        let mut result = DiscoveryResult {
            cells_discovered: 0,
            import_configs_found: 0,
            routes_found: 0,
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
                warn!("Failed to get cells from admin interface: {}", e);
                warn!(
                    "Falling back to default import configuration for '{}'",
                    self.config.installed_app_id
                );

                // Fallback: Use default config when admin interface is unavailable
                // This allows seeding to work even when we can't enumerate cells
                // Use valid base64 placeholder (all zeros) to avoid decode errors
                let fallback_dna_hash = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();
                let fallback_agent = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();

                // Store a placeholder ZomeCallConfig
                self.zome_configs.insert(
                    fallback_dna_hash.clone(),
                    ZomeCallConfig {
                        dna_hash: fallback_dna_hash.clone(),
                        agent_pub_key: fallback_agent,
                        zome_name: self.config.zome_name.clone(),
                        app_id: self.config.installed_app_id.clone(),
                        role_name: "lamad".to_string(), // Default fallback role
                    },
                );

                // Store default import config
                let default_import_config = ImportConfig {
                    enabled: true,
                    base_route: "/import".to_string(),
                    batch_types: vec![doorway_client::ImportBatchType::new("content")
                        .queue_fn("queue_import")
                        .process_fn("process_import_chunk")
                        .status_fn("get_import_status")
                        .max_items(5000)
                        .chunk_size(50)],
                    require_auth: false,
                    allowed_agents: None,
                };
                self.import_config_store
                    .set_config(&fallback_dna_hash, default_import_config);

                result.errors.push(format!(
                    "Admin connection failed ({e}), using fallback config"
                ));
                result.import_configs_found = 1;

                info!("Fallback import config registered for /import/content");
                return result;
            }
        };

        info!("Found {} cells in app", cells.len());
        result.cells_discovered = cells.len();

        // Step 2: For each cell, store ZomeCallConfig and discover configs
        for cell in cells {
            let zome_config = ZomeCallConfig {
                dna_hash: cell.dna_hash.clone(),
                agent_pub_key: cell.agent_pub_key.clone(),
                zome_name: self.config.zome_name.clone(),
                app_id: self.config.installed_app_id.clone(),
                role_name: cell.role_name.clone(),
            };

            // Store zome config for later use
            self.zome_configs
                .insert(cell.dna_hash.clone(), zome_config.clone());
            debug!(dna = %cell.dna_hash, role = %cell.role_name, "Stored ZomeCallConfig");

            // Discover import config
            match self.discover_import_config(&cell, &zome_config).await {
                Ok(Some(import_config)) => {
                    self.import_config_store
                        .set_config(&cell.dna_hash, import_config);
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

            // Discover routes (if route registry is configured)
            if let Some(ref route_registry) = self.route_registry {
                match self.discover_routes(&cell, &zome_config).await {
                    Ok(Some(routes)) => {
                        route_registry
                            .register_dna_routes(
                                &cell.dna_hash,
                                &cell.role_name,
                                &zome_config.zome_name,
                                routes,
                            )
                            .await;
                        result.routes_found += 1;
                        info!(
                            dna = %cell.dna_hash,
                            role = %cell.role_name,
                            "Discovered and registered routes"
                        );
                    }
                    Ok(None) => {
                        debug!(dna = %cell.dna_hash, "No routes declared (function not found)");
                    }
                    Err(e) => {
                        warn!(dna = %cell.dna_hash, error = %e, "Failed to discover routes");
                        result
                            .errors
                            .push(format!("Routes {}: {}", cell.dna_hash, e));
                    }
                }
            }
        }

        info!(
            "Discovery complete: {} cells, {} import configs, {} routes, {} errors",
            result.cells_discovered,
            result.import_configs_found,
            result.routes_found,
            result.errors.len()
        );

        result
    }

    /// Get cells from conductor admin interface using TypedAdminClient.
    ///
    /// Uses the official `holochain_client::AdminWebsocket` via TypedAdminClient,
    /// which handles the conductor wire protocol correctly (typed enum matching
    /// for CellInfo::Provisioned, proper MessagePack framing).
    async fn get_cells(&self) -> Result<Vec<CellInfo>, String> {
        let admin = tokio::time::timeout(
            self.config.timeout,
            TypedAdminClient::connect(&self.config.admin_url),
        )
        .await
        .map_err(|_| "Timeout connecting to admin interface")?
        .map_err(|e| format!("Admin connect failed: {e}"))?;

        let app_info = tokio::time::timeout(
            self.config.timeout,
            admin.get_app_info(&self.config.installed_app_id),
        )
        .await
        .map_err(|_| "Timeout getting app info")?
        .map_err(|e| format!("get_app_info failed: {e}"))?;

        let cells: Vec<CellInfo> = app_info
            .cell_ids
            .into_iter()
            .map(|(role_name, (dna_hash, agent_pub_key))| CellInfo {
                dna_hash: encode_base64(&dna_hash),
                agent_pub_key: encode_base64(&agent_pub_key),
                role_name,
            })
            .collect();

        info!(
            "Discovered {} cells: [{}]",
            cells.len(),
            cells
                .iter()
                .map(|c| c.role_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

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
            batch_types: vec![doorway_client::ImportBatchType::new("content")
                .queue_fn("queue_import")
                .process_fn("process_import_chunk")
                .status_fn("get_import_status")
                .max_items(5000)
                .chunk_size(50)],
            require_auth: false,
            allowed_agents: None,
        }))
    }

    /// Discover routes from a cell via __doorway_routes
    async fn discover_routes(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<DoorwayRoutes>, String> {
        // TODO: Actually call __doorway_routes on the zome
        // For now, we'll return a default route config that matches what
        // the elohim DNA would declare.
        //
        // The actual implementation would:
        // 1. Build a zome call for __doorway_routes
        // 2. Send it via WorkerPool
        // 3. Parse the DoorwayRoutes response
        //
        // For now, return default elohim routes

        use doorway_client::{DoorwayRoutesBuilder, Route};

        Ok(Some(
            DoorwayRoutesBuilder::new()
                // Content API routes
                .route(
                    Route::get("/api/v1/content/{id}")
                        .handler("get_content")
                        .cache_ttl(3600)
                        .public_if_reach("commons")
                        .build(),
                )
                .route(
                    Route::get("/api/v1/content")
                        .handler("list_content")
                        .cache_ttl(300)
                        .build(),
                )
                .route(
                    Route::post("/api/v1/content")
                        .handler("create_content")
                        .auth_required()
                        .build(),
                )
                // Path API routes
                .route(
                    Route::get("/api/v1/paths/{id}")
                        .handler("get_path")
                        .cache_ttl(3600)
                        .public_if_reach("commons")
                        .build(),
                )
                .route(
                    Route::get("/api/v1/paths")
                        .handler("list_paths")
                        .cache_ttl(300)
                        .build(),
                )
                // Blob proxy - doorway caches, agent's elohim-storage is authoritative
                .with_blobs_at("/store")
                // Stream proxy for media
                .with_streaming()
                .build(),
        ))
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

/// Spawn discovery with route registry as a background task
pub fn spawn_discovery_task_with_routes(
    config: DiscoveryConfig,
    zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
    import_config_store: Arc<ImportConfigStore>,
    route_registry: Arc<RouteRegistry>,
) -> tokio::task::JoinHandle<DiscoveryResult> {
    tokio::spawn(async move {
        // Wait a bit for conductor to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        let service = DiscoveryService::with_route_registry(
            config,
            zome_configs,
            import_config_store,
            route_registry,
        );
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
