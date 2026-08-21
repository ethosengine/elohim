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
                result.errors.push(format!("Admin connection failed: {e}"));
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

    /// Discover import config from a cell.
    ///
    /// Not yet implemented — returns None. Import config comes from
    /// steward self-registration via build_manifest(), not DNA introspection.
    async fn discover_import_config(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<ImportConfig>, String> {
        Ok(None)
    }

    /// Discover routes from a cell via __doorway_routes.
    ///
    /// Not yet implemented — returns None. Routes come from steward
    /// self-registration via build_manifest(), not DNA introspection.
    async fn discover_routes(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<DoorwayRoutes>, String> {
        Ok(None)
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

// ── Discovery retry (boot-order self-heal) ───────────────────────────────────
//
// Doorway and conductor restart independently (pod restarts, deploys, the local
// hc-mesh, which starts doorways at step 1 and conductors at step 2). Discovery
// used to be a ONE-SHOT: sleep 2s, call `discover()` once, and — if the
// conductor was not listening yet — log "readiness NOT signaled" and EXIT. The
// role map (`zome_configs`) then stayed empty for the life of the process, so
// every `get_zome_config_by_role` answered `No zome config found for role
// 'imagodei'. Available: []` and every hosted registration failed HTTP 500
// AGENT_KEY_ERROR until an operator restarted the doorway. This mirrors the
// storage-side shape fixed in 77dd6b7b6: retry only WHILE the connect fails,
// and return the instant one succeeds, is exactly half a self-heal.

/// First retry delay after a failed discovery attempt.
pub(crate) const DISCOVERY_RETRY_BASE: Duration = Duration::from_secs(1);

/// Ceiling for the retry ladder. Bounded so a conductor that is down for hours
/// costs one admin dial every 30s instead of a hot loop — and never silent: a
/// doorway with an empty role map cannot serve zome calls at all, so the retry
/// keeps saying so.
pub(crate) const DISCOVERY_RETRY_MAX: Duration = Duration::from_secs(30);

/// What to do after one discovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryVerdict {
    /// The role map is populated — signal readiness and STOP retrying.
    Ready,
    /// Nothing usable was discovered — back off and try again.
    Retry,
}

/// Judge one discovery attempt. Pure, so the stop condition is testable without
/// a conductor.
///
/// `Ready` requires cells AND no errors: a partial discovery leaves a role map
/// that answers some roles and not others, which is the same AGENT_KEY_ERROR
/// with a harder-to-read cause. The retry is cheap; a half-map is not.
pub fn discovery_verdict(result: &DiscoveryResult) -> DiscoveryVerdict {
    if result.cells_discovered > 0 && result.errors.is_empty() {
        DiscoveryVerdict::Ready
    } else {
        DiscoveryVerdict::Retry
    }
}

/// Next delay on the bounded-exponential ladder.
pub fn next_discovery_delay(prev: Duration) -> Duration {
    (prev * 2).min(DISCOVERY_RETRY_MAX)
}

/// Spawn discovery as a background task with completion signal.
///
/// The `ready_tx` channel is set to `true` when discovery completes successfully
/// (all cells discovered and stored in zome_configs). Routes can wait on the
/// corresponding receiver before attempting conductor operations.
///
/// Retries on a bounded-exponential ladder until the role map is non-empty, so
/// a doorway that booted before its conductor heals itself instead of serving
/// AGENT_KEY_ERROR until an operator restarts it. Once discovery succeeds the
/// loop RETURNS — the self-heal must not become its own flap.
pub fn spawn_discovery_task_with_signal(
    config: DiscoveryConfig,
    zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
    import_config_store: Arc<ImportConfigStore>,
    ready_tx: tokio::sync::watch::Sender<bool>,
) -> tokio::task::JoinHandle<DiscoveryResult> {
    tokio::spawn(async move {
        // Wait a bit for conductor to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        let service = DiscoveryService::new(config, zome_configs.clone(), import_config_store);
        let started = std::time::Instant::now();
        let mut delay = DISCOVERY_RETRY_BASE;
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;
            let result = service.discover().await;

            if discovery_verdict(&result) == DiscoveryVerdict::Ready {
                let _ = ready_tx.send(true);
                info!(
                    attempt,
                    cells = result.cells_discovered,
                    elapsed_secs = started.elapsed().as_secs(),
                    "Discovery ready signal sent — role map populated, retry loop stopping"
                );
                return result;
            }

            // Another path (a second discovery task, a test) populated the map
            // while we were dialing: the condition this loop exists to satisfy
            // is met, so stop rather than spin.
            if !zome_configs.is_empty() {
                let _ = ready_tx.send(true);
                info!(
                    attempt,
                    roles = zome_configs.len(),
                    "Role map populated by another path — discovery retry loop stopping"
                );
                return result;
            }

            // NEVER SILENT: an empty role map means every zome call fails
            // AGENT_KEY_ERROR, so each attempt says so at WARN with the cause
            // and when it will try again.
            warn!(
                attempt,
                cells = result.cells_discovered,
                errors = result.errors.len(),
                first_error = result.errors.first().map(String::as_str).unwrap_or(""),
                retry_in_secs = delay.as_secs(),
                "Discovery found no usable cells — role map still EMPTY (every zome call \
                 will fail AGENT_KEY_ERROR until it fills); retrying"
            );

            tokio::time::sleep(delay).await;
            delay = next_discovery_delay(delay);
        }
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

    fn result(cells: usize, errors: &[&str]) -> DiscoveryResult {
        DiscoveryResult {
            cells_discovered: cells,
            import_configs_found: 0,
            routes_found: 0,
            errors: errors.iter().map(|e| (*e).to_string()).collect(),
        }
    }

    /// THE REGRESSION THIS CLOSES (local mesh, 2026-08-21). `hc-mesh.sh
    /// start_all` starts doorways at step 1 and conductors at step 2, so the
    /// single discovery attempt (boot + 2s) found no admin interface and
    /// returned `Admin connection failed`. The old code logged
    /// "readiness NOT signaled" and the task EXITED — the role map stayed empty
    /// for the life of the process and every hosted registration answered HTTP
    /// 500 AGENT_KEY_ERROR until an operator restarted the doorway.
    ///
    /// A failed attempt must be a RETRY, never a terminal state.
    #[test]
    fn a_conductorless_attempt_retries_rather_than_giving_up() {
        assert_eq!(
            discovery_verdict(&result(0, &["Admin connection failed: connection refused"])),
            DiscoveryVerdict::Retry,
            "the boot-order case — doorway up before conductor"
        );
        assert_eq!(
            discovery_verdict(&result(0, &[])),
            DiscoveryVerdict::Retry,
            "a clean run that found nothing is still an empty role map"
        );
    }

    /// The stop condition: once the role map is populated the loop RETURNS.
    /// A self-heal that keeps running after it has healed is just a new flap.
    #[test]
    fn a_populated_role_map_stops_the_retry_loop() {
        assert_eq!(discovery_verdict(&result(3, &[])), DiscoveryVerdict::Ready);
    }

    /// A PARTIAL discovery is a retry, not a success. Signalling ready on a
    /// half-filled map produces the same AGENT_KEY_ERROR for whichever role is
    /// missing, with a much harder-to-read cause than an empty map.
    #[test]
    fn a_partial_discovery_is_not_ready() {
        assert_eq!(
            discovery_verdict(&result(2, &["DNA uhC0k-x: import config failed"])),
            DiscoveryVerdict::Retry
        );
    }

    /// Bounded: a conductor that is down for hours costs one admin dial every
    /// 30s, not a hot loop and not an unbounded wait.
    #[test]
    fn the_retry_ladder_is_bounded() {
        let mut d = DISCOVERY_RETRY_BASE;
        let mut steps = 0;
        while d < DISCOVERY_RETRY_MAX && steps < 100 {
            let next = next_discovery_delay(d);
            assert!(next > d, "the ladder must climb");
            d = next;
            steps += 1;
        }
        assert_eq!(d, DISCOVERY_RETRY_MAX, "the ladder reaches its ceiling");
        assert_eq!(
            next_discovery_delay(DISCOVERY_RETRY_MAX),
            DISCOVERY_RETRY_MAX,
            "and never climbs past it"
        );
    }
}
