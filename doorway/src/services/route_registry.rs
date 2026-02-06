//! Route Registry Service
//!
//! Manages dynamic routes discovered from DNAs and external agent registrations.
//! This replaces hard-coded routes with DNA-driven, auto-discovered configuration.
//!
//! ## Route Sources
//!
//! 1. **DNA Discovery**: Routes declared via `__doorway_routes` zome function
//! 2. **External Registration**: Agents that can't run doorway register via API
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │  RouteRegistry                                                            │
//! │                                                                          │
//! │  dna_routes: HashMap<dna_hash, DoorwayRoutes>     (from DNA discovery)   │
//! │  agent_routes: HashMap<agent_pubkey, AgentRouteEntry>  (external agents) │
//! │  compiled_routes: Vec<CompiledRoute>             (merged, ready to serve)│
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use doorway_client::{
    AgentCapability, AgentRegistration, AgentRegistrationResponse, BlobProxyConfig,
    DoorwayRoutes, HttpMethod, Route, StreamProxyConfig,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// =============================================================================
// Types
// =============================================================================

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RouteRegistryConfig {
    /// Maximum external agent registrations
    pub max_external_agents: usize,
    /// Default TTL for registrations without explicit TTL (seconds)
    pub default_registration_ttl_secs: u64,
    /// How often to cleanup expired registrations
    pub cleanup_interval: Duration,
    /// Maximum age for timestamp in registration (replay protection)
    pub max_timestamp_age_secs: u64,
}

impl Default for RouteRegistryConfig {
    fn default() -> Self {
        Self {
            max_external_agents: 100,
            default_registration_ttl_secs: 86400, // 24 hours
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_timestamp_age_secs: 300, // 5 minutes
        }
    }
}

/// An external agent's route entry
#[derive(Debug, Clone)]
pub struct AgentRouteEntry {
    /// Registration details
    pub registration: AgentRegistration,
    /// When this registration was accepted
    pub registered_at: Instant,
    /// When this registration expires (None = permanent)
    pub expires_at: Option<Instant>,
    /// Unique registration ID
    pub registration_id: String,
}

/// A compiled route ready for the HTTP router
#[derive(Debug, Clone)]
pub struct CompiledRoute {
    /// HTTP method
    pub method: HttpMethod,
    /// Full path pattern (e.g., "/api/v1/content/{id}")
    pub path: String,
    /// Source of this route
    pub source: RouteSource,
    /// Target zome function or agent endpoint
    pub target: RouteTarget,
    /// Whether auth is required
    pub auth_required: bool,
    /// Cache TTL (0 = no cache)
    pub cache_ttl_secs: u64,
    /// Rate limit (0 = no limit)
    pub rate_limit_rpm: u32,
}

/// Source of a route (for debugging/auditing)
#[derive(Debug, Clone)]
pub enum RouteSource {
    /// Discovered from a DNA
    Dna { dna_hash: String, role_name: String },
    /// Registered by an external agent
    ExternalAgent { agent_pubkey: String, registration_id: String },
    /// Built-in doorway route
    Builtin,
}

/// Target of a route
#[derive(Debug, Clone)]
pub enum RouteTarget {
    /// Call a zome function via conductor
    ZomeCall {
        dna_hash: String,
        zome_name: String,
        fn_name: String,
    },
    /// Proxy to an external agent's HTTP endpoint
    AgentProxy {
        agent_pubkey: String,
        endpoint: String,
        path_suffix: Option<String>,
    },
    /// Serve blobs from elohim-storage
    BlobProxy {
        config: BlobProxyConfig,
    },
    /// Serve media streams
    StreamProxy {
        config: StreamProxyConfig,
    },
}

/// Route registry statistics
#[derive(Debug, Clone, Default)]
pub struct RouteRegistryStats {
    pub dna_route_sources: usize,
    pub external_agents: usize,
    pub total_routes: usize,
    pub blob_proxies: usize,
    pub stream_proxies: usize,
}

// =============================================================================
// Route Registry
// =============================================================================

/// Registry for dynamic routes from DNAs and external agents
pub struct RouteRegistry {
    config: RouteRegistryConfig,
    /// Routes from DNA discovery (dna_hash -> routes)
    dna_routes: RwLock<HashMap<String, DnaRouteEntry>>,
    /// Routes from external agent registration (agent_pubkey -> entry)
    agent_routes: RwLock<HashMap<String, AgentRouteEntry>>,
    /// Compiled routes (recalculated when sources change)
    compiled_routes: RwLock<Vec<CompiledRoute>>,
    /// When routes were last compiled
    last_compiled: RwLock<Option<Instant>>,
}

/// Entry for DNA-discovered routes
#[derive(Debug, Clone)]
struct DnaRouteEntry {
    /// DNA hash
    dna_hash: String,
    /// Role name from conductor
    role_name: String,
    /// Zome name for calls
    zome_name: String,
    /// Discovered routes
    routes: DoorwayRoutes,
    /// When discovered
    discovered_at: Instant,
}

impl RouteRegistry {
    /// Create a new route registry
    pub fn new(config: RouteRegistryConfig) -> Self {
        Self {
            config,
            dna_routes: RwLock::new(HashMap::new()),
            agent_routes: RwLock::new(HashMap::new()),
            compiled_routes: RwLock::new(Vec::new()),
            last_compiled: RwLock::new(None),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RouteRegistryConfig::default())
    }

    // =========================================================================
    // DNA Route Discovery
    // =========================================================================

    /// Register routes discovered from a DNA
    pub async fn register_dna_routes(
        &self,
        dna_hash: &str,
        role_name: &str,
        zome_name: &str,
        routes: DoorwayRoutes,
    ) {
        let entry = DnaRouteEntry {
            dna_hash: dna_hash.to_string(),
            role_name: role_name.to_string(),
            zome_name: zome_name.to_string(),
            routes,
            discovered_at: Instant::now(),
        };

        let mut dna_routes = self.dna_routes.write().await;
        dna_routes.insert(dna_hash.to_string(), entry);

        info!(
            dna_hash = %dna_hash,
            role = %role_name,
            "Registered DNA routes"
        );

        // Invalidate compiled routes
        drop(dna_routes);
        self.recompile_routes().await;
    }

    /// Unregister a DNA's routes
    pub async fn unregister_dna(&self, dna_hash: &str) {
        let mut dna_routes = self.dna_routes.write().await;
        if dna_routes.remove(dna_hash).is_some() {
            info!(dna_hash = %dna_hash, "Unregistered DNA routes");
            drop(dna_routes);
            self.recompile_routes().await;
        }
    }

    // =========================================================================
    // External Agent Registration
    // =========================================================================

    /// Register an external agent's routes
    pub async fn register_external_agent(
        &self,
        registration: AgentRegistration,
    ) -> AgentRegistrationResponse {
        // Validate registration
        if let Err(e) = self.validate_registration(&registration) {
            return AgentRegistrationResponse {
                success: false,
                registration_id: None,
                base_url: None,
                error: Some(e),
                expires_at: None,
            };
        }

        // Check capacity
        let agent_routes = self.agent_routes.read().await;
        if agent_routes.len() >= self.config.max_external_agents {
            return AgentRegistrationResponse {
                success: false,
                registration_id: None,
                base_url: None,
                error: Some("Maximum external agents reached".to_string()),
                expires_at: None,
            };
        }
        drop(agent_routes);

        // Calculate expiration
        let ttl = if registration.ttl_secs == 0 {
            self.config.default_registration_ttl_secs
        } else {
            registration.ttl_secs
        };

        let now = Instant::now();
        let expires_at = if ttl == 0 {
            None
        } else {
            Some(now + Duration::from_secs(ttl))
        };

        let registration_id = generate_registration_id(&registration.agent_pubkey);

        // Calculate expires_at as unix timestamp for response
        let expires_at_unix = expires_at.map(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + ttl
        });

        let entry = AgentRouteEntry {
            registration: registration.clone(),
            registered_at: now,
            expires_at,
            registration_id: registration_id.clone(),
        };

        // Store registration
        let mut agent_routes = self.agent_routes.write().await;
        agent_routes.insert(registration.agent_pubkey.clone(), entry);

        info!(
            agent = %registration.agent_pubkey,
            endpoint = %registration.endpoint,
            capabilities = ?registration.capabilities,
            ttl_secs = ttl,
            "Registered external agent"
        );

        // Recompile routes
        drop(agent_routes);
        self.recompile_routes().await;

        // Build base URL for this agent
        let base_url = format!("/agent/{}", &registration.agent_pubkey[..12.min(registration.agent_pubkey.len())]);

        AgentRegistrationResponse {
            success: true,
            registration_id: Some(registration_id),
            base_url: Some(base_url),
            error: None,
            expires_at: expires_at_unix,
        }
    }

    /// Revoke an external agent's registration
    pub async fn revoke_registration(&self, agent_pubkey: &str) -> bool {
        let mut agent_routes = self.agent_routes.write().await;
        if agent_routes.remove(agent_pubkey).is_some() {
            info!(agent = %agent_pubkey, "Revoked agent registration");
            drop(agent_routes);
            self.recompile_routes().await;
            true
        } else {
            false
        }
    }

    /// Validate a registration request
    fn validate_registration(&self, reg: &AgentRegistration) -> Result<(), String> {
        // Check timestamp freshness (replay protection)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > reg.timestamp + self.config.max_timestamp_age_secs {
            return Err("Registration timestamp too old".to_string());
        }

        if reg.timestamp > now + 60 {
            return Err("Registration timestamp in future".to_string());
        }

        // Validate endpoint URL
        if !reg.endpoint.starts_with("http://") && !reg.endpoint.starts_with("https://") {
            return Err("Endpoint must be HTTP(S) URL".to_string());
        }

        // TODO: Verify signature
        // The signature should sign: "{agent_pubkey}:{endpoint}:{timestamp}"
        // We need to verify using the agent's public key
        if reg.signature.is_empty() {
            return Err("Signature required".to_string());
        }

        Ok(())
    }

    // =========================================================================
    // Route Compilation
    // =========================================================================

    /// Recompile all routes from DNA and agent sources
    async fn recompile_routes(&self) {
        let mut compiled = Vec::new();

        // Compile DNA routes
        let dna_routes = self.dna_routes.read().await;
        for entry in dna_routes.values() {
            compiled.extend(self.compile_dna_routes(entry));
        }
        drop(dna_routes);

        // Compile agent routes
        let agent_routes = self.agent_routes.read().await;
        for entry in agent_routes.values() {
            compiled.extend(self.compile_agent_routes(entry));
        }
        drop(agent_routes);

        let count = compiled.len();

        let mut routes = self.compiled_routes.write().await;
        *routes = compiled;

        let mut last = self.last_compiled.write().await;
        *last = Some(Instant::now());

        debug!(count = count, "Recompiled route table");
    }

    /// Compile routes from a DNA entry
    fn compile_dna_routes(&self, entry: &DnaRouteEntry) -> Vec<CompiledRoute> {
        let mut compiled = Vec::new();

        // Compile explicit routes
        for route in &entry.routes.routes {
            compiled.push(CompiledRoute {
                method: route.method,
                path: route.path.clone(),
                source: RouteSource::Dna {
                    dna_hash: entry.dna_hash.clone(),
                    role_name: entry.role_name.clone(),
                },
                target: RouteTarget::ZomeCall {
                    dna_hash: entry.dna_hash.clone(),
                    zome_name: entry.zome_name.clone(),
                    fn_name: route.handler.clone(),
                },
                auth_required: route.auth_required,
                cache_ttl_secs: route.cache_ttl_secs,
                rate_limit_rpm: route.rate_limit_rpm,
            });
        }

        // Compile blob proxy if enabled
        if let Some(ref blob_config) = entry.routes.blob_proxy {
            if blob_config.enabled {
                compiled.push(CompiledRoute {
                    method: HttpMethod::Get,
                    path: format!("{}/:hash", blob_config.base_path),
                    source: RouteSource::Dna {
                        dna_hash: entry.dna_hash.clone(),
                        role_name: entry.role_name.clone(),
                    },
                    target: RouteTarget::BlobProxy {
                        config: blob_config.clone(),
                    },
                    auth_required: false,
                    cache_ttl_secs: blob_config.cache_ttl_secs,
                    rate_limit_rpm: 0,
                });
            }
        }

        // Compile stream proxy if enabled
        if let Some(ref stream_config) = entry.routes.stream_proxy {
            if stream_config.enabled {
                compiled.push(CompiledRoute {
                    method: HttpMethod::Get,
                    path: format!("{}/:id/*path", stream_config.base_path),
                    source: RouteSource::Dna {
                        dna_hash: entry.dna_hash.clone(),
                        role_name: entry.role_name.clone(),
                    },
                    target: RouteTarget::StreamProxy {
                        config: stream_config.clone(),
                    },
                    auth_required: false,
                    cache_ttl_secs: 300, // 5 min for stream manifests
                    rate_limit_rpm: 0,
                });
            }
        }

        compiled
    }

    /// Compile routes from an external agent entry
    fn compile_agent_routes(&self, entry: &AgentRouteEntry) -> Vec<CompiledRoute> {
        let mut compiled = Vec::new();
        let base_path = format!("/agent/{}", &entry.registration.agent_pubkey[..12.min(entry.registration.agent_pubkey.len())]);

        // If agent provided explicit routes, use those
        if let Some(ref routes) = entry.registration.routes {
            for route in &routes.routes {
                compiled.push(CompiledRoute {
                    method: route.method,
                    path: format!("{}{}", base_path, route.path),
                    source: RouteSource::ExternalAgent {
                        agent_pubkey: entry.registration.agent_pubkey.clone(),
                        registration_id: entry.registration_id.clone(),
                    },
                    target: RouteTarget::AgentProxy {
                        agent_pubkey: entry.registration.agent_pubkey.clone(),
                        endpoint: entry.registration.endpoint.clone(),
                        path_suffix: Some(route.path.clone()),
                    },
                    auth_required: route.auth_required,
                    cache_ttl_secs: route.cache_ttl_secs,
                    rate_limit_rpm: route.rate_limit_rpm,
                });
            }
        } else {
            // Generate routes based on declared capabilities
            for cap in &entry.registration.capabilities {
                match cap {
                    AgentCapability::Content => {
                        // Generic content proxy
                        compiled.push(CompiledRoute {
                            method: HttpMethod::Get,
                            path: format!("{}/api/*path", base_path),
                            source: RouteSource::ExternalAgent {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                registration_id: entry.registration_id.clone(),
                            },
                            target: RouteTarget::AgentProxy {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                endpoint: entry.registration.endpoint.clone(),
                                path_suffix: None,
                            },
                            auth_required: true,
                            cache_ttl_secs: 300,
                            rate_limit_rpm: 100,
                        });
                    }
                    AgentCapability::Blobs => {
                        compiled.push(CompiledRoute {
                            method: HttpMethod::Get,
                            path: format!("{}/store/:hash", base_path),
                            source: RouteSource::ExternalAgent {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                registration_id: entry.registration_id.clone(),
                            },
                            target: RouteTarget::AgentProxy {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                endpoint: entry.registration.endpoint.clone(),
                                path_suffix: Some("/store".to_string()),
                            },
                            auth_required: false,
                            cache_ttl_secs: 86400,
                            rate_limit_rpm: 0,
                        });
                    }
                    AgentCapability::Streaming => {
                        compiled.push(CompiledRoute {
                            method: HttpMethod::Get,
                            path: format!("{}/stream/*path", base_path),
                            source: RouteSource::ExternalAgent {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                registration_id: entry.registration_id.clone(),
                            },
                            target: RouteTarget::AgentProxy {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                endpoint: entry.registration.endpoint.clone(),
                                path_suffix: Some("/stream".to_string()),
                            },
                            auth_required: false,
                            cache_ttl_secs: 300,
                            rate_limit_rpm: 0,
                        });
                    }
                    AgentCapability::Import => {
                        compiled.push(CompiledRoute {
                            method: HttpMethod::Post,
                            path: format!("{}/import/*path", base_path),
                            source: RouteSource::ExternalAgent {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                registration_id: entry.registration_id.clone(),
                            },
                            target: RouteTarget::AgentProxy {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                endpoint: entry.registration.endpoint.clone(),
                                path_suffix: Some("/import".to_string()),
                            },
                            auth_required: true,
                            cache_ttl_secs: 0,
                            rate_limit_rpm: 10,
                        });
                    }
                    AgentCapability::Custom(name) => {
                        compiled.push(CompiledRoute {
                            method: HttpMethod::Get,
                            path: format!("{}/{}/*path", base_path, name),
                            source: RouteSource::ExternalAgent {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                registration_id: entry.registration_id.clone(),
                            },
                            target: RouteTarget::AgentProxy {
                                agent_pubkey: entry.registration.agent_pubkey.clone(),
                                endpoint: entry.registration.endpoint.clone(),
                                path_suffix: Some(format!("/{}", name)),
                            },
                            auth_required: true,
                            cache_ttl_secs: 300,
                            rate_limit_rpm: 100,
                        });
                    }
                }
            }
        }

        compiled
    }

    // =========================================================================
    // Query Interface
    // =========================================================================

    /// Get all compiled routes
    pub async fn get_routes(&self) -> Vec<CompiledRoute> {
        self.compiled_routes.read().await.clone()
    }

    /// Get routes for a specific path (for debugging)
    pub async fn find_routes_for_path(&self, path: &str) -> Vec<CompiledRoute> {
        self.compiled_routes
            .read()
            .await
            .iter()
            .filter(|r| path_matches(&r.path, path))
            .cloned()
            .collect()
    }

    /// Get registry statistics
    pub async fn stats(&self) -> RouteRegistryStats {
        let dna_routes = self.dna_routes.read().await;
        let agent_routes = self.agent_routes.read().await;
        let compiled = self.compiled_routes.read().await;

        let blob_proxies = compiled
            .iter()
            .filter(|r| matches!(r.target, RouteTarget::BlobProxy { .. }))
            .count();

        let stream_proxies = compiled
            .iter()
            .filter(|r| matches!(r.target, RouteTarget::StreamProxy { .. }))
            .count();

        RouteRegistryStats {
            dna_route_sources: dna_routes.len(),
            external_agents: agent_routes.len(),
            total_routes: compiled.len(),
            blob_proxies,
            stream_proxies,
        }
    }

    // =========================================================================
    // Cleanup
    // =========================================================================

    /// Clean up expired agent registrations
    pub async fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut agent_routes = self.agent_routes.write().await;

        let before = agent_routes.len();
        agent_routes.retain(|pubkey, entry| {
            if let Some(expires_at) = entry.expires_at {
                if now >= expires_at {
                    info!(agent = %pubkey, "Agent registration expired");
                    return false;
                }
            }
            true
        });

        let removed = before - agent_routes.len();
        if removed > 0 {
            drop(agent_routes);
            self.recompile_routes().await;
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a unique registration ID
fn generate_registration_id(agent_pubkey: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    agent_pubkey.hash(&mut hasher);
    Instant::now().hash(&mut hasher);
    format!("reg-{:016x}", hasher.finish())
}

/// Simple path pattern matching (supports :param and *wildcard)
fn path_matches(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    let mut pi = 0;
    for (i, pp) in pattern_parts.iter().enumerate() {
        if pp.starts_with('*') {
            // Wildcard matches rest of path
            return true;
        }
        if pi >= path_parts.len() {
            return false;
        }
        if pp.starts_with(':') {
            // Parameter matches any segment
            pi += 1;
            continue;
        }
        if *pp != path_parts[pi] {
            return false;
        }
        pi += 1;
    }

    pi == path_parts.len()
}

/// Spawn cleanup task for expired registrations
pub fn spawn_cleanup_task(
    registry: Arc<RouteRegistry>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            registry.cleanup_expired().await;
        }
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use doorway_client::{DoorwayRoutesBuilder, Route as DoorwayRoute};

    #[tokio::test]
    async fn test_register_dna_routes() {
        let registry = RouteRegistry::with_defaults();

        let routes = DoorwayRoutesBuilder::new()
            .route(
                DoorwayRoute::get("/api/content/{id}")
                    .handler("get_content")
                    .cache_ttl(3600)
                    .build(),
            )
            .with_blobs()
            .build();

        registry
            .register_dna_routes("dna-hash-123", "elohim", "content_store", routes)
            .await;

        let compiled = registry.get_routes().await;
        assert!(compiled.len() >= 2); // At least 1 route + blob proxy

        let stats = registry.stats().await;
        assert_eq!(stats.dna_route_sources, 1);
        assert_eq!(stats.blob_proxies, 1);
    }

    #[tokio::test]
    async fn test_external_agent_registration() {
        let registry = RouteRegistry::with_defaults();

        let registration = AgentRegistration {
            agent_pubkey: "uhCAk12345678901234567890".to_string(),
            endpoint: "https://my-device.local:8080".to_string(),
            capabilities: vec![AgentCapability::Content, AgentCapability::Blobs],
            signature: "valid-signature".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            routes: None,
            ttl_secs: 3600,
        };

        let response = registry.register_external_agent(registration).await;
        assert!(response.success);
        assert!(response.registration_id.is_some());
        assert!(response.base_url.is_some());

        let stats = registry.stats().await;
        assert_eq!(stats.external_agents, 1);
        assert!(stats.total_routes >= 2); // content + blobs
    }

    #[tokio::test]
    async fn test_registration_validation() {
        let registry = RouteRegistry::with_defaults();

        // Test expired timestamp
        let old_registration = AgentRegistration {
            agent_pubkey: "uhCAk12345678901234567890".to_string(),
            endpoint: "https://my-device.local:8080".to_string(),
            capabilities: vec![],
            signature: "valid-signature".to_string(),
            timestamp: 1000, // Very old timestamp
            routes: None,
            ttl_secs: 0,
        };

        let response = registry.register_external_agent(old_registration).await;
        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_path_matching() {
        assert!(path_matches("/api/content/:id", "/api/content/abc"));
        assert!(path_matches("/api/*path", "/api/content/abc/def"));
        assert!(!path_matches("/api/content/:id", "/api/other/abc"));
        assert!(!path_matches("/api/content", "/api/content/extra"));
    }
}
