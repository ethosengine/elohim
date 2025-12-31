//! HTTP server implementation
//!
//! Pattern adapted from holo-host/rust/holo-gateway/src/lib.rs
//! Uses hyper http1 with TokioIo for async handling.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

use crate::bootstrap::{self, BootstrapStore};
use crate::cache::{
    self, CacheConfig, CacheRuleStore, ContentCache, TieredBlobCache, TieredCacheConfig,
    spawn_tiered_cleanup_task, DoorwayResolver, DeliveryRelay,
};
use crate::orchestrator::OrchestratorState;
use crate::services::{
    CustodianService, CustodianServiceConfig, VerificationService, VerifyBlobRequest,
    spawn_health_probe_task,
};
use crate::config::Args;
use crate::db::MongoClient;
use crate::nats::{HostRouter, NatsClient};
use crate::projection::{ProjectionConfig, ProjectionStore};
use crate::routes;
use crate::server::websocket;
use crate::signal::{self, SignalStore, DEFAULT_MAX_CLIENTS};
use crate::signing::{SigningConfig, SigningService};
use crate::types::DoorwayError;
use crate::worker::{WorkerPool, ZomeCallConfig};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

/// Shared application state
pub struct AppState {
    pub args: Args,
    pub mongo: Option<MongoClient>,
    pub nats: Option<NatsClient>,
    pub router: HostRouter,
    /// Worker pool for request routing (always available)
    pub pool: Option<Arc<WorkerPool>>,
    /// Bootstrap store for agent discovery
    pub bootstrap: Option<Arc<BootstrapStore>>,
    /// Signal store for WebRTC signaling
    pub signal: Option<Arc<SignalStore>>,
    /// Content cache for REST API
    pub cache: Arc<ContentCache>,
    /// Cache rules discovered from DNAs
    pub cache_rules: Arc<CacheRuleStore>,
    /// Projection store for one-way DHT → cache projections
    pub projection: Option<Arc<ProjectionStore>>,
    /// Signing service for gateway-assisted human signing
    pub signing: Arc<SigningService>,
    /// Tiered blob cache for media streaming (metadata/blobs/chunks)
    pub tiered_cache: Arc<TieredBlobCache>,
    /// Custodian service for P2P blob distribution
    pub custodian: Arc<CustodianService>,
    /// Verification service for blob integrity
    pub verification: Arc<VerificationService>,
    /// Orchestrator state for cluster management (node health, provisioning)
    pub orchestrator: Option<Arc<OrchestratorState>>,
    /// Content resolver with tiered fallback (Projection → Conductor)
    pub resolver: Arc<DoorwayResolver>,
    /// Delivery relay for CDN-style content delivery (request coalescing, shard caching)
    /// Note: Write batching is handled by agent-side holochain-cache-core, NOT here
    pub delivery_relay: Arc<DeliveryRelay>,
    /// Import config discovered from DNAs (zome-declared routes)
    pub import_config_store: Option<Arc<crate::services::ImportConfigStore>>,
    /// Zome call configs by DNA hash (discovered from conductor)
    pub zome_configs: Arc<dashmap::DashMap<String, ZomeCallConfig>>,
}

impl AppState {
    /// Create AppState without external services (dev mode, direct proxy)
    pub fn new(args: Args) -> Self {
        let bootstrap = if args.bootstrap_enabled {
            Some(Arc::new(BootstrapStore::new()))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            Some(Arc::new(SignalStore::new(max_clients)))
        } else {
            None
        };
        let cache = Arc::new(ContentCache::new(CacheConfig::from_env()));
        let cache_rules = Arc::new(CacheRuleStore::new());
        // Projection store in memory-only mode (no MongoDB)
        let projection = Some(Arc::new(ProjectionStore::memory_only(ProjectionConfig::default())));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection only (no pool in this mode)
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), None, None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        Self {
            args,
            mongo: None,
            nats: None,
            router: HostRouter::new(None),
            pool: None,
            bootstrap,
            signal,
            cache,
            cache_rules,
            projection,
            signing,
            tiered_cache,
            custodian,
            verification,
            orchestrator: None,
            resolver,
            delivery_relay,
            import_config_store: Some(Arc::new(crate::services::ImportConfigStore::new())),
            zome_configs: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create AppState with services but no worker pool (direct proxy mode)
    ///
    /// Projection store is initialized in memory-only mode. Use `init_projection()`
    /// to upgrade to MongoDB-backed projection after async initialization.
    pub fn with_services(
        args: Args,
        mongo: Option<MongoClient>,
        nats: Option<NatsClient>,
    ) -> Self {
        let router = HostRouter::new(nats.clone());
        let bootstrap = if args.bootstrap_enabled {
            Some(Arc::new(BootstrapStore::new()))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            Some(Arc::new(SignalStore::new(max_clients)))
        } else {
            None
        };
        let cache = Arc::new(ContentCache::new(CacheConfig::from_env()));
        let cache_rules = Arc::new(CacheRuleStore::new());
        // Start with memory-only projection; upgrade to MongoDB via init_projection()
        let projection = Some(Arc::new(ProjectionStore::memory_only(ProjectionConfig::default())));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection only (no pool in this mode)
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), None, None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        Self {
            args,
            mongo,
            nats,
            router,
            pool: None,
            bootstrap,
            signal,
            cache,
            cache_rules,
            projection,
            signing,
            tiered_cache,
            custodian,
            verification,
            orchestrator: None,
            resolver,
            delivery_relay,
            import_config_store: Some(Arc::new(crate::services::ImportConfigStore::new())),
            zome_configs: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create AppState with worker pool (pooled connection mode)
    ///
    /// Projection store is initialized in memory-only mode. Use `init_projection()`
    /// to upgrade to MongoDB-backed projection after async initialization.
    pub fn with_pool(
        args: Args,
        mongo: Option<MongoClient>,
        nats: Option<NatsClient>,
        pool: Arc<WorkerPool>,
    ) -> Self {
        let router = HostRouter::new(nats.clone());
        let bootstrap = if args.bootstrap_enabled {
            Some(Arc::new(BootstrapStore::new()))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            Some(Arc::new(SignalStore::new(max_clients)))
        } else {
            None
        };
        let cache = Arc::new(ContentCache::new(CacheConfig::from_env()));
        let cache_rules = Arc::new(CacheRuleStore::new());
        // Start with memory-only projection; upgrade to MongoDB via init_projection()
        let projection = Some(Arc::new(ProjectionStore::memory_only(ProjectionConfig::default())));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with both projection and conductor fallback
        // Note: zome_config is discovered at runtime when conductor connection is established
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), Some(Arc::clone(&pool)), None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        // Note: Write batching is handled by agent's holochain-cache-core WriteBuffer, NOT here
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        Self {
            args,
            mongo,
            nats,
            router,
            pool: Some(pool),
            bootstrap,
            signal,
            cache,
            cache_rules,
            projection,
            signing,
            tiered_cache,
            custodian,
            verification,
            orchestrator: None,
            resolver,
            delivery_relay,
            import_config_store: Some(Arc::new(crate::services::ImportConfigStore::new())),
            zome_configs: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create a new AppState with MongoDB-backed projection store
    ///
    /// This is the preferred constructor when MongoDB is available,
    /// as it properly initializes the projection store with persistence.
    pub async fn with_projection(
        args: Args,
        mongo: MongoClient,
        nats: Option<NatsClient>,
        pool: Option<Arc<WorkerPool>>,
    ) -> Result<Self, DoorwayError> {
        let router = HostRouter::new(nats.clone());
        let bootstrap = if args.bootstrap_enabled {
            Some(Arc::new(BootstrapStore::new()))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            Some(Arc::new(SignalStore::new(max_clients)))
        } else {
            None
        };
        let cache = Arc::new(ContentCache::new(CacheConfig::from_env()));
        let cache_rules = Arc::new(CacheRuleStore::new());

        // Initialize projection store with MongoDB
        let projection_store = ProjectionStore::new(
            mongo.clone(),
            ProjectionConfig::default(),
        ).await?;
        let projection = Some(Arc::new(projection_store));

        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection and optional conductor fallback
        // Note: zome_config is discovered at runtime when conductor connection is established
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), pool.clone(), None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        // Note: Write batching is handled by agent's holochain-cache-core WriteBuffer, NOT here
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        Ok(Self {
            args,
            mongo: Some(mongo),
            nats,
            router,
            pool,
            bootstrap,
            signal,
            cache,
            cache_rules,
            projection,
            signing,
            tiered_cache,
            custodian,
            verification,
            orchestrator: None,
            resolver,
            delivery_relay,
            import_config_store: Some(Arc::new(crate::services::ImportConfigStore::new())),
            zome_configs: Arc::new(dashmap::DashMap::new()),
        })
    }

    /// Set orchestrator state (called from main after orchestrator is started)
    pub fn set_orchestrator(&mut self, state: Arc<OrchestratorState>) {
        self.orchestrator = Some(state);
    }
}

/// Start the HTTP server
pub async fn run(state: Arc<AppState>) -> Result<(), DoorwayError> {
    let listener = TcpListener::bind(state.args.listen).await?;

    info!(
        "Doorway listening on {} as node {}",
        state.args.listen, state.args.node_id
    );

    if state.args.dev_mode {
        warn!("Development mode enabled - authentication disabled");
    }

    // Start bootstrap cleanup task if enabled
    if let Some(ref bootstrap) = state.bootstrap {
        bootstrap::store::spawn_cleanup_task(Arc::clone(bootstrap));
        info!("Bootstrap service enabled at /bootstrap/*");
    }

    // Log signal service status
    if let Some(ref signal_store) = state.signal {
        let max = state.args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
        info!("Signal service enabled at /signal/{{pubkey}} (max {} clients)", max);
        let _ = signal_store; // suppress unused warning
    }

    // Start cache cleanup task
    cache::store::spawn_cleanup_task(Arc::clone(&state.cache));
    info!("Cache service enabled (max {} entries)", state.cache.config().max_entries);

    // Start tiered blob cache cleanup task (every 60 seconds)
    spawn_tiered_cleanup_task(Arc::clone(&state.tiered_cache), std::time::Duration::from_secs(60));
    info!(
        "Tiered blob cache enabled (blob max: {} MB, chunk max: {} GB)",
        state.tiered_cache.config().blob_max_bytes / (1024 * 1024),
        state.tiered_cache.config().chunk_max_bytes / (1024 * 1024 * 1024)
    );

    // Start custodian health probe task (every 60 seconds)
    spawn_health_probe_task(Arc::clone(&state.custodian), std::time::Duration::from_secs(60));
    info!("Custodian service enabled for P2P blob distribution");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);

                    let service = service_fn(move |req| {
                        let state = Arc::clone(&state);
                        async move { handle_request(state, addr, req).await }
                    });

                    if let Err(err) = http1::Builder::new()
                        .preserve_header_case(true)
                        .title_case_headers(true)
                        .serve_connection(io, service)
                        .with_upgrades()
                        .await
                    {
                        error!("Error serving connection from {}: {:?}", addr, err);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {:?}", e);
            }
        }
    }
}

/// Route incoming HTTP requests
async fn handle_request(
    state: Arc<AppState>,
    addr: SocketAddr,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Check if this is a signal subdomain request (signal.*.elohim.host)
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let is_signal_host = host.starts_with("signal.") || host.contains(".signal.");

    info!("[{}] {} {} (host: {})", addr, method, path, host);

    // Signal subdomain: route /{pubkey} to signal handler (tx5 protocol)
    // Path should be /{pubkey} where pubkey has no additional slashes
    if is_signal_host && method == Method::GET && path.len() > 1 {
        let after_slash = &path[1..]; // Skip leading /
        if !after_slash.is_empty() && !after_slash.contains('/') {
            if hyper_tungstenite::is_upgrade_request(&req) {
                return Ok(handle_signal_request(state, req, &path, addr).await);
            } else {
                return Ok(to_boxed(bad_request_response("Signal endpoint requires WebSocket upgrade")));
            }
        }
    }

    // Handle auth routes (/auth/*) - these consume the request
    if path.starts_with("/auth") {
        if let Some(response) = routes::handle_auth_request(req, Arc::clone(&state)).await {
            return Ok(response);
        }
        // If handle_auth_request returns None, it didn't handle the request
        // This shouldn't happen since we checked the path prefix, but handle gracefully
        return Ok(to_boxed(not_found_response(&path)));
    }

    let response = match (method, path.as_str()) {
        // Liveness probe - returns 200 if doorway is running
        (Method::GET, "/health") | (Method::GET, "/healthz") => {
            to_boxed(routes::health_check(Arc::clone(&state)))
        }

        // Readiness probe - returns 200 only if conductor is connected
        // Use this for seeder pre-flight checks
        (Method::GET, "/ready") | (Method::GET, "/readyz") => {
            to_boxed(routes::readiness_check(Arc::clone(&state)))
        }

        // CORS preflight
        (Method::OPTIONS, _) => to_boxed(preflight_response()),

        // WebSocket upgrade for admin interface
        (Method::GET, "/") | (Method::GET, "/admin") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                to_boxed(websocket::handle_admin_upgrade(state, req).await)
            } else {
                to_boxed(not_found_response(&path))
            }
        }

        // WebSocket upgrade for app interface
        (Method::GET, p) if p.starts_with("/app/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                // Extract port from /app/:port
                let port_str = p.strip_prefix("/app/").unwrap_or("");
                match port_str.parse::<u16>() {
                    Ok(port) if state.args.is_valid_app_port(port) => {
                        to_boxed(websocket::handle_app_upgrade(state, req, port).await)
                    }
                    _ => to_boxed(bad_request_response("Invalid app port")),
                }
            } else {
                to_boxed(not_found_response(p))
            }
        }

        // Status endpoint with runtime info
        (Method::GET, "/status") => to_boxed(routes::status_check(Arc::clone(&state)).await),

        // ====================================================================
        // Admin API endpoints for Shefa compute resources dashboard
        // ====================================================================

        // List all nodes with detailed resource and social metrics
        (Method::GET, "/admin/nodes") => {
            to_boxed(routes::handle_nodes(Arc::clone(&state)).await)
        }

        // Get specific node details
        (Method::GET, p) if p.starts_with("/admin/nodes/") => {
            let node_id = p.strip_prefix("/admin/nodes/").unwrap_or("");
            to_boxed(routes::handle_node_by_id(Arc::clone(&state), node_id).await)
        }

        // Cluster-wide aggregated metrics
        (Method::GET, "/admin/cluster") => {
            to_boxed(routes::handle_cluster_metrics(Arc::clone(&state)).await)
        }

        // Resource utilization summary
        (Method::GET, "/admin/resources") => {
            to_boxed(routes::handle_resources(Arc::clone(&state)).await)
        }

        // Custodian network overview
        (Method::GET, "/admin/custodians") => {
            to_boxed(routes::handle_custodians(Arc::clone(&state)).await)
        }

        // Real-time WebSocket feed for dashboard
        (Method::GET, "/admin/ws") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                to_boxed(routes::handle_dashboard_ws(Arc::clone(&state), req).await)
            } else {
                to_boxed(bad_request_response("WebSocket upgrade required for /admin/ws"))
            }
        }

        // Admin seed routes for bulk upload
        // PUT /admin/seed/blob - Upload blob to projection cache
        (Method::PUT, "/admin/seed/blob") => {
            to_boxed(routes::handle_seed_blob(req, Arc::clone(&state)).await)
        }
        // HEAD /admin/seed/blob/{hash} - Check if blob exists
        (Method::HEAD, p) if p.starts_with("/admin/seed/blob/") => {
            let hash = p.strip_prefix("/admin/seed/blob/").unwrap_or("");
            to_boxed(routes::handle_check_blob(hash, Arc::clone(&state)).await)
        }

        // Bootstrap service routes (X-Op header protocol)
        // POST /bootstrap with X-Op header, or legacy path-based routing
        (Method::POST, p) if p == "/bootstrap" || p.starts_with("/bootstrap/") => {
            handle_bootstrap_request(state, req, &path).await
        }

        // Bootstrap ping (GET for health check)
        (Method::GET, "/bootstrap") => {
            to_boxed(
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap(),
            )
        }

        // Signal service WebSocket (SBD protocol)
        (Method::GET, p) if p.starts_with("/signal/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                handle_signal_request(state, req, &path, addr).await
            } else {
                to_boxed(bad_request_response("Signal endpoint requires WebSocket upgrade"))
            }
        }

        // Streaming API routes (HLS/DASH)
        (Method::GET, p) if p.starts_with("/api/stream/") => {
            // Construct base URL from host header
            let host = req
                .headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");
            let scheme = if host.contains("localhost") || host.starts_with("127.") {
                "http"
            } else {
                "https"
            };
            let base_url = format!("{}://{}", scheme, host);
            to_boxed(routes::handle_stream_request(state, p, &base_url).await)
        }

        // Blob verification endpoint
        (Method::POST, "/api/blob/verify") => {
            handle_blob_verify(state, req).await
        }

        // Content store streaming with Range support (HTTP 206)
        // GET /store/{hash} - Stream entire content or byte range
        // HEAD /store/{hash} - Get content metadata
        (Method::GET, p) if p.starts_with("/store/") => {
            match routes::blob::handle_blob_request(req, Arc::clone(&state.cache)).await {
                Ok(resp) => to_boxed(resp),
                Err(err) => to_boxed(routes::blob::error_response(err)),
            }
        }
        (Method::HEAD, p) if p.starts_with("/store/") => {
            match routes::blob::handle_blob_request(req, Arc::clone(&state.cache)).await {
                Ok(resp) => to_boxed(resp),
                Err(err) => to_boxed(routes::blob::error_response(err)),
            }
        }

        // Cache API routes: GET /api/v1/cache/{type}/{id?}
        (Method::GET, p) if p.starts_with("/api/v1/cache/") => {
            let query = req.uri().query();
            // Extract auth header and remote IP for reach-aware serving
            let auth_header = req
                .headers()
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let remote_ip = addr.ip();
            to_boxed(routes::handle_api_request(state, p, query, Some(remote_ip), auth_header).await)
        }

        // Dynamic import routes (zome-declared via __doorway_import_config)
        // POST /{base_route}/{batch_type} - queue import
        // GET /{base_route}/{batch_type}/{batch_id} - get status
        (method, p) if matches!(method, Method::POST | Method::GET) => {
            // Try to match against discovered import routes
            if let Some(ref import_store) = state.import_config_store {
                if let Some((dna_hash, batch_type, batch_id)) = routes::match_import_route(p, import_store) {
                    // Need worker pool and zome config to make the call
                    if let Some(ref pool) = state.pool {
                        // Get ZomeCallConfig for this DNA
                        if let Some(zome_config) = state.zome_configs.get(&dna_hash) {
                            info!(
                                dna = %dna_hash,
                                batch_type = %batch_type,
                                batch_id = ?batch_id,
                                "Handling import request"
                            );

                            return Ok(to_boxed(
                                routes::handle_import_request(
                                    req,
                                    Arc::clone(import_store),
                                    Arc::clone(pool),
                                    zome_config.clone(),
                                    dna_hash,
                                    batch_type,
                                    batch_id,
                                ).await
                            ));
                        } else {
                            // Config discovered but zome connection not established yet
                            debug!(
                                dna = %dna_hash,
                                "Import route matched but ZomeCallConfig not yet available"
                            );
                            return Ok(to_boxed(
                                Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .header("Content-Type", "application/json")
                                    .header("Retry-After", "5")
                                    .body(Full::new(Bytes::from(format!(
                                        r#"{{"error": "Import route discovered but conductor connection for DNA {} pending. Retry after connection is established."}}"#,
                                        dna_hash
                                    ))))
                                    .unwrap(),
                            ));
                        }
                    } else {
                        return Ok(to_boxed(
                            Response::builder()
                                .status(StatusCode::SERVICE_UNAVAILABLE)
                                .header("Content-Type", "application/json")
                                .body(Full::new(Bytes::from(
                                    r#"{"error": "Worker pool not available"}"#,
                                )))
                                .unwrap(),
                        ));
                    }
                } else if p.starts_with("/import/") {
                    // Path looks like an import route but no config matched
                    // Check if discovery is still in progress
                    let discovered_dnas = import_store.get_import_enabled_dnas();
                    if discovered_dnas.is_empty() {
                        // No import configs discovered yet - discovery may still be in progress
                        debug!(
                            path = p,
                            "Import route requested but discovery not complete"
                        );
                        return Ok(to_boxed(
                            Response::builder()
                                .status(StatusCode::SERVICE_UNAVAILABLE)
                                .header("Content-Type", "application/json")
                                .header("Retry-After", "5")
                                .body(Full::new(Bytes::from(
                                    r#"{"error": "Import routes not yet discovered. Discovery in progress - retry in a few seconds.", "hint": "The doorway needs to connect to the conductor and discover import configurations before import routes are available."}"#,
                                )))
                                .unwrap(),
                        ));
                    }
                    // Discovery complete but route not found - return 404
                    debug!(
                        path = p,
                        discovered_count = discovered_dnas.len(),
                        "Import route not matched (discovery complete)"
                    );
                }
            }
            // Not an import route, fall through to not found
            to_boxed(not_found_response(&path))
        }

        // Not found
        _ => to_boxed(not_found_response(&path)),
    };

    Ok(response)
}

/// Handle bootstrap service requests
/// Supports both X-Op header protocol (POST /bootstrap) and legacy path-based routing
async fn handle_bootstrap_request(
    state: Arc<AppState>,
    req: Request<Incoming>,
    path: &str,
) -> Response<BoxBody> {
    // Check if bootstrap is enabled
    let store = match &state.bootstrap {
        Some(s) => Arc::clone(s),
        None => {
            return to_boxed(
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Bootstrap service not enabled"}"#,
                    )))
                    .unwrap(),
            );
        }
    };

    // Extract network type from query params (?net=tx5 or default to tx5)
    // Do this before consuming the request body
    let query_string = req.uri().query().map(|s| s.to_string());
    let network = query_string
        .as_ref()
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("net="))
                .and_then(|p| p.strip_prefix("net="))
        })
        .unwrap_or("tx5");

    // Determine operation: check X-Op header first, then fall back to path
    let x_op = req
        .headers()
        .get("X-Op")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase());

    let op = if let Some(ref header_op) = x_op {
        header_op.as_str()
    } else {
        // Legacy path-based routing: /bootstrap/put, /bootstrap/random, /bootstrap/now
        path.strip_prefix("/bootstrap/").unwrap_or("")
    };

    debug!("Bootstrap request: op={}, network={}, x_op={:?}", op, network, x_op);

    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Bootstrap request body error: {}", e);
            return to_boxed(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Failed to read request body"}"#,
                    )))
                    .unwrap(),
            );
        }
    };

    // Route to appropriate handler
    let response = match op {
        "put" => bootstrap::handle_put(store, body, network).await,
        "random" => bootstrap::handle_random(store, body, network).await,
        "now" => bootstrap::handle_now().await,
        "" => {
            // POST /bootstrap without X-Op header - invalid
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Missing X-Op header or path operation"}"#,
                )))
                .unwrap()
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error": "Unknown bootstrap operation: {}"}}"#,
                op
            ))))
            .unwrap(),
    };

    to_boxed(response)
}

/// Handle signal service WebSocket upgrade
async fn handle_signal_request(
    state: Arc<AppState>,
    req: Request<Incoming>,
    path: &str,
    addr: SocketAddr,
) -> Response<BoxBody> {
    // Check if signal is enabled
    let store = match &state.signal {
        Some(s) => Arc::clone(s),
        None => {
            return to_boxed(
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Signal service not enabled"}"#,
                    )))
                    .unwrap(),
            );
        }
    };

    // Extract pubkey from path: /signal/{pubkey} or /{pubkey} (signal subdomain)
    let pub_key_str = path
        .strip_prefix("/signal/")
        .or_else(|| path.strip_prefix("/"))
        .unwrap_or("");
    if pub_key_str.is_empty() {
        return to_boxed(bad_request_response("Missing public key in path"));
    }

    // Handle the WebSocket upgrade
    to_boxed(signal::handle_signal_upgrade(store, req, pub_key_str, addr, &state.args).await)
}

/// Handle blob verification request (POST /api/blob/verify)
///
/// This endpoint provides server-side SHA256 verification as part of defense-in-depth:
/// - Primary: Client uses WASM or SubtleCrypto for local verification
/// - Fallback: Client sends blob to server for authoritative verification
async fn handle_blob_verify(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Response<BoxBody> {
    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Blob verify request body error: {}", e);
            return to_boxed(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Failed to read request body"}"#,
                    )))
                    .unwrap(),
            );
        }
    };

    // Parse JSON request
    let request: VerifyBlobRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Blob verify JSON parse error: {}", e);
            return to_boxed(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid JSON: {}"}}"#,
                        e
                    ))))
                    .unwrap(),
            );
        }
    };

    debug!(
        expected_hash = %request.expected_hash,
        has_data = request.data_base64.is_some(),
        has_url = request.fetch_url.is_some(),
        content_id = ?request.content_id,
        "Processing blob verification request"
    );

    // Process verification
    let response = state.verification.handle_request(request).await;

    // Serialize response
    let json_body = match serde_json::to_string(&response) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize verification response: {}", e);
            return to_boxed(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Internal serialization error"}"#,
                    )))
                    .unwrap(),
            );
        }
    };

    to_boxed(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json_body)))
            .unwrap(),
    )
}

/// Convert a Full<Bytes> body to BoxBody
fn to_boxed(response: Response<Full<Bytes>>) -> Response<BoxBody> {
    response.map(|body| body.map_err(|never| match never {}).boxed())
}

/// CORS preflight response
fn preflight_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Not found response
fn not_found_response(path: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "Not Found",
        "path": path,
        "hint": "Use WebSocket connection to /admin or /app/:port"
    });

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// Bad request response
fn bad_request_response(message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "Bad Request",
        "message": message
    });

    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

