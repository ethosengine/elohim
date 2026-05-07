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

use crate::auth::{extract_token_from_header, JwtValidator};
use crate::bootstrap::{self, BootstrapStore};
use crate::cache::{
    self, spawn_tiered_cleanup_task, AppFileCacheService, CacheConfig, CacheRuleStore,
    ContentCache, DeliveryRelay, DoorwayResolver, TieredBlobCache, TieredCacheConfig,
};
use crate::conductor::{ConductorRegistry, ConductorRouter};
use crate::config::Args;
use crate::db::MongoClient;
use crate::nats::{HostRouter, NatsClient};
use crate::orchestrator::OrchestratorState;
use crate::projection::{ProjectionConfig, ProjectionStore};
use crate::routes;
use crate::server::websocket;
use crate::services::{
    spawn_health_probe_task, CustodianService, CustodianServiceConfig, RouteRegistry,
    VerificationService, VerifyBlobRequest,
};
use crate::signal::{self, SignalStore, DEFAULT_MAX_CLIENTS};
use crate::signing::{SigningConfig, SigningService};
use crate::types::DoorwayError;
use crate::worker::{WorkerPool, ZomeCallConfig};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

/// Outcome of dispatching an unmatched request through the route registry.
///
/// Computed by `classify_dispatch` — separates the routing decision from the
/// handler invocation so the decision logic can be unit-tested without spinning
/// up an HTTP server or a real storage proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Disposition {
    /// Registry matched a `StorageProxy` route — caller forwards to `endpoint`.
    StorageProxy { endpoint: String },
    /// Registry matched but the target type is not yet handled by dispatch
    /// (BlobProxy, StreamProxy, ZomeCall, AgentProxy). Caller returns 404.
    RegistryUnhandled,
    /// No registry match, GET method, root_app_slug is configured —
    /// caller falls through to the root SPA bootstrap handler.
    RootApp,
    /// No registry match and no SPA fallback applies — caller returns 404.
    NotFound,
}

/// Classify how an unmatched request should be dispatched.
///
/// Called from `handle_request` after all explicit match arms have been tried.
/// Replaces the previous hard-coded `/api/v1/ || /account/` prefix guard, which
/// failed every time elohim-storage's manifest added a new top-level path
/// family (`blob_proxy`, `stream_proxy`, …) — `/blob/<hash>` requests skipped
/// the registry entirely and fell into the SPA bootstrap, breaking thumbnails.
///
/// The contract: if the registry has any compiled route matching `(method, path)`,
/// the registry decides. Otherwise, GET requests with a configured root SPA
/// slug fall through to the SPA; anything else is 404.
async fn classify_dispatch(
    registry: &crate::services::RouteRegistry,
    root_app_slug: Option<&str>,
    method: &Method,
    path: &str,
) -> Disposition {
    let http_method = match *method {
        Method::GET => doorway_client::HttpMethod::Get,
        Method::POST => doorway_client::HttpMethod::Post,
        Method::PUT => doorway_client::HttpMethod::Put,
        Method::DELETE => doorway_client::HttpMethod::Delete,
        Method::PATCH => doorway_client::HttpMethod::Patch,
        Method::HEAD => doorway_client::HttpMethod::Head,
        _ => doorway_client::HttpMethod::Get,
    };

    let matches = registry.match_request(http_method, path).await;
    if let Some(route) = matches.first() {
        if let Some(endpoint) = route.storage_endpoint() {
            return Disposition::StorageProxy {
                endpoint: endpoint.to_string(),
            };
        }
        // Future: handle ZomeCall, AgentProxy, BlobProxy, StreamProxy targets.
        // For now any non-StorageProxy registry hit returns 404.
        return Disposition::RegistryUnhandled;
    }

    if *method == Method::GET && root_app_slug.is_some() {
        return Disposition::RootApp;
    }

    Disposition::NotFound
}

/// Shared application state
pub struct AppState {
    pub args: Args,
    pub mongo: Option<MongoClient>,
    pub nats: Option<NatsClient>,
    pub router: HostRouter,
    /// Worker pool for APP interface (zome calls) - connects to port 4445
    pub pool: Option<Arc<WorkerPool>>,
    /// Worker pool for ADMIN interface (generate_agent_pub_key, list_apps, etc.) - connects to admin port
    pub admin_pool: Option<Arc<WorkerPool>>,
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
    /// Note: Write batching is handled by agent-side elohim-cache-core, NOT here
    pub delivery_relay: Arc<DeliveryRelay>,
    /// Import config discovered from DNAs (zome-declared routes)
    pub import_config_store: Option<Arc<crate::services::ImportConfigStore>>,
    /// Zome call configs by DNA hash (discovered from conductor)
    pub zome_configs: Arc<dashmap::DashMap<String, ZomeCallConfig>>,
    /// Discovery completion signal. Routes that need zome_configs wait on this.
    /// `false` = discovery not yet complete, `true` = discovery succeeded and zome_configs populated.
    pub discovery_ready: tokio::sync::watch::Receiver<bool>,
    /// Single-connection import client for batch operations
    /// Uses ONE connection to conductor to avoid overwhelming during imports
    pub import_client: Option<Arc<crate::services::ImportClient>>,
    /// Debug event hub for real-time debugging via WebSocket
    pub debug_hub: Arc<routes::DebugHub>,
    /// Conductor pool registry — maps agents to conductors, available on ALL instances
    pub conductor_registry: Option<Arc<ConductorRegistry>>,
    /// Per-request conductor routing (agent → conductor pool)
    /// When set, authenticated requests route to the conductor hosting that agent.
    /// When None, all requests use the default pool (backwards compat).
    pub conductor_router: Option<Arc<ConductorRouter>>,
    /// Node Ed25519 verifying (public) key for federation signing
    /// Generated at startup, used in DID document and JWKS endpoint
    pub node_verifying_key: Option<ed25519_dalek::VerifyingKey>,
    /// ZomeCaller for federation and service registration
    /// Shared by federation service, heartbeat task, and federation routes
    pub zome_caller: Option<Arc<crate::services::ZomeCaller>>,
    /// Cache of peer doorways discovered via HTTP federation
    pub peer_cache: crate::services::federation::PeerCache,
    /// Mutable list of federation peer URLs (seeded from env, mutable via admin API)
    pub peer_url_list: crate::services::federation::PeerUrlList,
    /// Cached P2P health from elohim-storage sidecar (polled every 30s)
    pub p2p_health: Arc<tokio::sync::RwLock<Option<crate::routes::health::P2PHealth>>>,
    /// CORS configuration (origin allowlist, dev-mode flag)
    pub cors_config: crate::cors::CorsConfig,
    /// Dynamic route registry — steward peer routes + external agent routes
    pub route_registry: Arc<RouteRegistry>,
    /// Journal inference mode — determined at boot by sidecar availability
    pub journal_inference_available: bool,
    /// Per-peer projection subscriber health (shared crate)
    pub peer_health: Arc<elohim_compute::PeerHealthRegistry>,
    /// Request throughput counters for /api/v1/cache/ (shared crate)
    pub request_counters: Arc<elohim_compute::RequestCounters>,
    /// Service boot time (for uptime in ComputeReport)
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Cached MongoDB projection stats (per-instance, not global static)
    pub projection_stats_cache: Arc<tokio::sync::Mutex<(std::time::Instant, u64, u64)>>,
    /// App file projection cache (MongoDB-backed, for HTML5 app assets)
    pub app_file_cache: Option<Arc<AppFileCacheService>>,
    /// Admin-controlled cache bypass flag.
    /// When false, all /apps/* requests skip the projection cache and proxy
    /// directly to elohim-storage. Used by delivery diagnostics tests to prove
    /// fallback behavior. Set via POST /admin/cache/disable and /admin/cache/enable.
    pub cache_enabled: Arc<std::sync::atomic::AtomicBool>,
    /// Observable warmup retry state — populated when spawn_stream_task starts.
    /// Read by /health/startup to expose attempt count and completion status.
    pub warmup_state: Option<Arc<crate::projection::warm_stream::WarmupState>>,
    /// Angular SSR renderer — present when SSR_BUNDLE_PATH env var is set at startup.
    /// Used by the /render-test/* hardcoded route to prove the architecture end-to-end.
    /// None in all other constructors (renderer: None is the safe default).
    pub renderer: Option<Arc<dyn elohim_render::Renderer>>,
}

/// Initialize the SSR renderer from the `SSR_BUNDLE_PATH` environment variable.
///
/// Returns `Some(renderer)` if the env var is set and the bundle path exists.
/// Returns `None` silently if the var is unset.
/// Logs a warning and returns `None` if the path is set but the bundle fails to load.
fn init_renderer() -> Option<Arc<dyn elohim_render::Renderer>> {
    match std::env::var("SSR_BUNDLE_PATH") {
        Ok(path) => match elohim_render::AngularRenderer::new(std::path::PathBuf::from(path)) {
            Ok(r) => {
                tracing::info!(target: "doorway::ssr", "SSR renderer ready");
                Some(Arc::new(r) as Arc<dyn elohim_render::Renderer>)
            }
            Err(e) => {
                tracing::warn!(target: "doorway::ssr", "SSR disabled: {}", e);
                None
            }
        },
        Err(_) => None,
    }
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
        let projection = Some(Arc::new(ProjectionStore::memory_only(
            ProjectionConfig::default(),
        )));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection only (no pool in this mode)
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), None, None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        let peer_url_list =
            crate::services::federation::new_peer_url_list(args.federation_peers.clone());
        let cors_config = crate::cors::CorsConfig::from_args(&args);

        Self {
            args,
            mongo: None,
            nats: None,
            router: HostRouter::new(None),
            pool: None,
            admin_pool: None,
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
            discovery_ready: tokio::sync::watch::channel(false).1,
            import_client: None, // Set later via set_import_client()
            debug_hub: Arc::new(routes::DebugHub::new(true)),
            conductor_registry: None,
            conductor_router: None,
            node_verifying_key: None,
            zome_caller: None,
            peer_cache: crate::services::federation::new_peer_cache(),
            peer_url_list,
            p2p_health: Arc::new(tokio::sync::RwLock::new(None)),
            cors_config,
            route_registry: Arc::new(RouteRegistry::with_defaults()),
            journal_inference_available: false,
            peer_health: Arc::new(elohim_compute::PeerHealthRegistry::new()),
            request_counters: Arc::new(elohim_compute::RequestCounters::new()),
            started_at: chrono::Utc::now(),
            projection_stats_cache: Arc::new(tokio::sync::Mutex::new((
                std::time::Instant::now() - std::time::Duration::from_secs(60),
                0,
                0,
            ))),
            app_file_cache: None,
            cache_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            warmup_state: None,
            renderer: init_renderer(),
        }
    }

    /// Create AppState with services but no worker pool (direct proxy mode)
    ///
    /// Projection store is initialized in memory-only mode. Use `init_projection()`
    /// to upgrade to MongoDB-backed projection after async initialization.
    pub fn with_services(args: Args, mongo: Option<MongoClient>, nats: Option<NatsClient>) -> Self {
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
        let projection = Some(Arc::new(ProjectionStore::memory_only(
            ProjectionConfig::default(),
        )));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection only (no pool in this mode)
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), None, None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        let peer_url_list =
            crate::services::federation::new_peer_url_list(args.federation_peers.clone());
        let cors_config = crate::cors::CorsConfig::from_args(&args);

        Self {
            args,
            mongo,
            nats,
            router,
            pool: None,
            admin_pool: None,
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
            discovery_ready: tokio::sync::watch::channel(false).1,
            import_client: None, // Set later via set_import_client()
            debug_hub: Arc::new(routes::DebugHub::new(true)),
            conductor_registry: None,
            conductor_router: None,
            node_verifying_key: None,
            zome_caller: None,
            peer_cache: crate::services::federation::new_peer_cache(),
            peer_url_list,
            p2p_health: Arc::new(tokio::sync::RwLock::new(None)),
            cors_config,
            route_registry: Arc::new(RouteRegistry::with_defaults()),
            journal_inference_available: false,
            peer_health: Arc::new(elohim_compute::PeerHealthRegistry::new()),
            request_counters: Arc::new(elohim_compute::RequestCounters::new()),
            started_at: chrono::Utc::now(),
            projection_stats_cache: Arc::new(tokio::sync::Mutex::new((
                std::time::Instant::now() - std::time::Duration::from_secs(60),
                0,
                0,
            ))),
            app_file_cache: None,
            cache_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            warmup_state: None,
            renderer: init_renderer(),
        }
    }

    /// Create AppState with worker pools (pooled connection mode)
    ///
    /// - `app_pool`: Connects to APP interface (port 4445) for zome calls
    /// - `admin_pool`: Connects to ADMIN interface for admin commands (generate_agent_pub_key, list_apps, etc.)
    ///
    /// Projection store is initialized in memory-only mode. Use `init_projection()`
    /// to upgrade to MongoDB-backed projection after async initialization.
    pub fn with_pool(
        args: Args,
        mongo: Option<MongoClient>,
        nats: Option<NatsClient>,
        app_pool: Arc<WorkerPool>,
        admin_pool: Option<Arc<WorkerPool>>,
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
        let projection = Some(Arc::new(ProjectionStore::memory_only(
            ProjectionConfig::default(),
        )));
        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with both projection and conductor fallback
        // Note: zome_config is discovered at runtime when conductor connection is established
        let resolver = Arc::new(DoorwayResolver::new(
            projection.clone(),
            Some(Arc::clone(&app_pool)),
            None,
        ));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        // Note: Write batching is handled by agent's elohim-cache-core WriteBuffer, NOT here
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        let peer_url_list =
            crate::services::federation::new_peer_url_list(args.federation_peers.clone());
        let cors_config = crate::cors::CorsConfig::from_args(&args);

        Self {
            args,
            mongo,
            nats,
            router,
            pool: Some(app_pool),
            admin_pool,
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
            discovery_ready: tokio::sync::watch::channel(false).1,
            import_client: None, // Set later via set_import_client()
            debug_hub: Arc::new(routes::DebugHub::new(true)),
            conductor_registry: None,
            conductor_router: None,
            node_verifying_key: None,
            zome_caller: None,
            peer_cache: crate::services::federation::new_peer_cache(),
            peer_url_list,
            p2p_health: Arc::new(tokio::sync::RwLock::new(None)),
            cors_config,
            route_registry: Arc::new(RouteRegistry::with_defaults()),
            journal_inference_available: false,
            peer_health: Arc::new(elohim_compute::PeerHealthRegistry::new()),
            request_counters: Arc::new(elohim_compute::RequestCounters::new()),
            started_at: chrono::Utc::now(),
            projection_stats_cache: Arc::new(tokio::sync::Mutex::new((
                std::time::Instant::now() - std::time::Duration::from_secs(60),
                0,
                0,
            ))),
            app_file_cache: None,
            cache_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            warmup_state: None,
            renderer: init_renderer(),
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
        let projection_store =
            ProjectionStore::new(mongo.clone(), ProjectionConfig::default()).await?;
        let projection = Some(Arc::new(projection_store));

        let signing = Arc::new(SigningService::new(SigningConfig::default()));
        let tiered_cache = Arc::new(TieredBlobCache::new(TieredCacheConfig::from_env()));
        let custodian = Arc::new(CustodianService::new(CustodianServiceConfig::default()));
        let verification = Arc::new(VerificationService::default());

        // Create resolver with projection and optional conductor fallback
        // Note: zome_config is discovered at runtime when conductor connection is established
        let resolver = Arc::new(DoorwayResolver::new(projection.clone(), pool.clone(), None));

        // Delivery relay for CDN-style caching (complements agent-side cache-core)
        // Note: Write batching is handled by agent's elohim-cache-core WriteBuffer, NOT here
        let delivery_relay = Arc::new(DeliveryRelay::with_defaults());

        let peer_url_list =
            crate::services::federation::new_peer_url_list(args.federation_peers.clone());
        let cors_config = crate::cors::CorsConfig::from_args(&args);

        // Initialize app file projection cache (MongoDB-backed)
        let app_file_cache = {
            let svc = AppFileCacheService::new(&mongo, "self-negotiated".to_string());
            // Pre-populate the slug index (slug -> blob_hash) from projection store
            svc.load_slug_index().await;
            info!("App file projection cache initialized");
            Some(Arc::new(svc))
        };

        Ok(Self {
            args,
            mongo: Some(mongo),
            nats,
            router,
            pool,
            admin_pool: None,
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
            discovery_ready: tokio::sync::watch::channel(false).1,
            import_client: None, // Set later via set_import_client()
            debug_hub: Arc::new(routes::DebugHub::new(true)),
            conductor_registry: None,
            conductor_router: None,
            node_verifying_key: None,
            zome_caller: None,
            peer_cache: crate::services::federation::new_peer_cache(),
            peer_url_list,
            p2p_health: Arc::new(tokio::sync::RwLock::new(None)),
            cors_config,
            route_registry: Arc::new(RouteRegistry::with_defaults()),
            journal_inference_available: false,
            peer_health: Arc::new(elohim_compute::PeerHealthRegistry::new()),
            request_counters: Arc::new(elohim_compute::RequestCounters::new()),
            started_at: chrono::Utc::now(),
            projection_stats_cache: Arc::new(tokio::sync::Mutex::new((
                std::time::Instant::now() - std::time::Duration::from_secs(60),
                0,
                0,
            ))),
            app_file_cache,
            cache_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            warmup_state: None,
            renderer: init_renderer(),
        })
    }

    /// Set orchestrator state (called from main after orchestrator is started)
    pub fn set_orchestrator(&mut self, state: Arc<OrchestratorState>) {
        self.orchestrator = Some(state);
    }

    /// Upgrade projection store from memory-only to MongoDB-backed.
    ///
    /// Called after AppState construction when MongoDB is available.
    /// Creates indexes, then replaces the memory-only projection and rebuilds
    /// the DoorwayResolver to use the new persistent store.
    pub async fn init_projection(&mut self, mongo: &MongoClient) -> Result<(), DoorwayError> {
        let projection_store =
            ProjectionStore::new(mongo.clone(), ProjectionConfig::default()).await?;
        let projection = Some(Arc::new(projection_store));

        // Rebuild resolver with the new MongoDB-backed projection
        self.resolver = Arc::new(DoorwayResolver::new(
            projection.clone(),
            self.pool.clone(),
            None,
        ));

        self.projection = projection;
        info!("Projection store upgraded to MongoDB-backed");

        // Initialize app file projection cache now that MongoDB is available
        let svc = AppFileCacheService::new(mongo, "self-negotiated".to_string());
        svc.load_slug_index().await;
        info!("App file projection cache initialized");
        self.app_file_cache = Some(Arc::new(svc));

        Ok(())
    }
}

/// Resolve the calling agent's `agent_cid` from the request's bearer token.
///
/// Returns `Some(human_id)` when:
/// - the request carries a `Bearer <token>` Authorization header AND
/// - the token validates against the configured JWT validator (or the dev-mode
///   validator when `args.dev_mode` is set)
///
/// Returns `None` for Session Visitors (no bearer), invalid/expired tokens, or
/// when no JWT secret is configured in production mode. Storage handlers fall
/// back to their `local_sessions`-based resolution when this header is absent.
///
/// **Alpha-substrate equivalence:** `agent_cid` is sourced from `claims.human_id`
/// today. The seeder authors AgentPeerBinding entries with
/// `agent_cid: human.id`; tests use slug-style strings. CIDv1 dag-cbor sha256
/// derivation of the Agent EPR is not enforced anywhere yet. When CIDv1
/// enforcement lands, doorway will resolve `human_id → agent_cid` once at user
/// creation, persist on UserDoc, and source from there. The wire shape
/// (`X-Agent-Cid` header) does not change.
fn resolve_agent_cid_from_request<B>(state: &AppState, req: &Request<B>) -> Option<String> {
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let token = extract_token_from_header(auth_header)?;

    let validator = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        let secret = state.args.jwt_secret.as_ref()?;
        JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds).ok()?
    };

    let result = validator.verify_token(token);
    if !result.valid {
        return None;
    }
    result.claims.map(|c| c.human_id)
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
        info!(
            "Signal service enabled at /signal/{{pubkey}} (max {} clients)",
            max
        );
        let _ = signal_store; // suppress unused warning
    }

    // Start cache cleanup task
    cache::store::spawn_cleanup_task(Arc::clone(&state.cache));
    info!(
        "Cache service enabled (max {} entries)",
        state.cache.config().max_entries
    );

    // Start tiered blob cache cleanup task (every 60 seconds)
    spawn_tiered_cleanup_task(
        Arc::clone(&state.tiered_cache),
        std::time::Duration::from_secs(60),
    );
    info!(
        "Tiered blob cache enabled (blob max: {} MB, chunk max: {} GB)",
        state.tiered_cache.config().blob_max_bytes / (1024 * 1024),
        state.tiered_cache.config().chunk_max_bytes / (1024 * 1024 * 1024)
    );

    // Start custodian health probe task (every 60 seconds)
    spawn_health_probe_task(
        Arc::clone(&state.custodian),
        std::time::Duration::from_secs(60),
    );
    info!("Custodian service enabled for P2P blob distribution");

    // Start P2P health polling task (every 30 seconds)
    {
        let p2p_health = Arc::clone(&state.p2p_health);
        let storage_url = state
            .args
            .storage_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8090".to_string());
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let url = format!("{}/p2p/status", storage_url.trim_end_matches('/'));
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let health = crate::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: body["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: body["peerId"].as_str().map(String::from),
                            };
                            *p2p_health.write().await = Some(health);
                        }
                    }
                    _ => {
                        // P2P not available — clear cached health
                        *p2p_health.write().await = None;
                    }
                }
            }
        });
    }

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);

                    let service = service_fn(move |req: Request<Incoming>| {
                        let state = Arc::clone(&state);
                        async move {
                            // Extract CORS context at the outermost level so every
                            // response — including early returns — gets CORS headers.
                            let request_origin: Option<String> = req
                                .headers()
                                .get(hyper::header::ORIGIN)
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string());
                            let cors_config = state.cors_config.clone();

                            if req.method() == hyper::Method::OPTIONS {
                                let resp: Result<Response<BoxBody>, hyper::Error> =
                                    Ok(to_boxed(crate::cors::preflight_response(
                                        &cors_config,
                                        request_origin.as_deref(),
                                    )));
                                return resp;
                            }

                            let response = handle_request(state, addr, req).await?;
                            Ok(crate::cors::apply_cors_headers(
                                &cors_config,
                                request_origin.as_deref(),
                                response,
                            ))
                        }
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

/// Wisdom-as-system-auth gate check for every state-changing HTTP request.
///
/// Mirrors the logic in `gate_client::tower_layer()` (which is axum-native) for
/// doorway's hyper `service_fn` architecture. Returns:
/// - `None`                  → gate allows or path is unmapped; caller continues routing.
/// - `Some(Response<BoxBody>)` → gate declined or escalated; caller must short-circuit.
///
/// GET/HEAD/OPTIONS always return `None` (read-only, exempt from gate).
/// POST/PUT/PATCH/DELETE are checked; unmapped paths fall through (Phase 1 contract).
///
/// In DevContext the gate always returns Allow, so this function always returns `None`
/// for real traffic until ElohimActive phase is enabled.
async fn apply_gate_check(method: &Method, path: &str) -> Option<Response<BoxBody>> {
    use gate_client::{check, GateStatus};

    // Only gate state-changing verbs.
    let is_state_changing = matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    );
    if !is_state_changing {
        return None;
    }

    // Path-based event inference (Phase 1 — see gate_client::tower module).
    // Unmapped paths fall through without a gate check.
    let event = infer_gate_event(path)?;

    let decision = match check(event).await {
        Ok(d) => d,
        Err(e) => {
            // Gate transport error — fail open with a log (Phase 2 will make this
            // configurable as fail-open vs fail-closed).
            tracing::warn!(error = %e, "gate-client transport error; failing open");
            return None;
        }
    };

    match &decision.status {
        GateStatus::Allow { .. } | GateStatus::Verdict(_) => {
            // Pass-through — caller adds x-gate-verdict header after routing.
            None
        }

        GateStatus::Decline { grounds } => {
            let body_json = serde_json::json!({
                "gate": "declined",
                "grounds": {
                    "category": grounds.category,
                    "summary": grounds.summary,
                    "principleRefs": grounds.principle_refs,
                }
            });
            let body_bytes = serde_json::to_vec(&body_json).unwrap_or_else(|_| b"{}".to_vec());
            Some(to_boxed(
                Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header("content-type", "application/json")
                    .header("x-gate-verdict", "decline")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .expect("infallible 403 gate response"),
            ))
        }

        GateStatus::Escalate { target, severity } => {
            let body_json = serde_json::json!({
                "gate": "escalated",
                "target": target,
                "severity": severity,
            });
            let body_bytes = serde_json::to_vec(&body_json).unwrap_or_else(|_| b"{}".to_vec());
            Some(to_boxed(
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("content-type", "application/json")
                    .header("x-gate-verdict", "escalate")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .expect("infallible 202 gate response"),
            ))
        }
    }
}

/// Infer a [`gate_client::RelationalImpactEvent`] from an HTTP path for the gate check.
///
/// Delegates to [`gate_client::infer_event_from_path`], which is the authoritative
/// source for path-to-event mapping shared by both the tower layer and this hyper
/// `service_fn` proxy. The mapping table lives in exactly one place.
///
/// Returns `None` for unmapped paths. Unknown paths fall through to the inner
/// handler without a gate check (Phase 1 contract).
fn infer_gate_event(path: &str) -> Option<gate_client::RelationalImpactEvent> {
    gate_client::infer_event_from_path(path)
}

/// Route incoming HTTP requests
async fn handle_request(
    state: Arc<AppState>,
    addr: SocketAddr,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, hyper::Error> {
    let method = req.method().clone();
    let method_str = method.to_string();
    let path = req.uri().path().to_string();

    // ── Wisdom-as-system-auth gate ─────────────────────────────────────────────
    // Fires before any routing for state-changing methods on gate-mapped paths.
    // In DevContext (Phase 1) the gate always allows — this is transparent to
    // existing callers. In ElohimActive phase it enforces relational wisdom on
    // every write that carries impact on others.
    if let Some(gate_resp) = apply_gate_check(&method, &path).await {
        return Ok(gate_resp);
    }

    // Extract observation session ID before req is consumed by the match block.
    // Used to fire-and-forget doorway-originated error contributions to storage.
    let observation_id: Option<String> = req
        .headers()
        .get("x-observation-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Snapshot storage URL before state is moved into match arms.
    let storage_url_for_obs: Option<String> = state.args.storage_url.clone();

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
                let resp = handle_signal_request(state, req, &path, addr).await;
                return Ok(resp);
            } else {
                return Ok(to_boxed(bad_request_response(
                    "Signal endpoint requires WebSocket upgrade",
                )));
            }
        }
    }

    // Handle auth routes (/auth/*) - these consume the request
    if path.starts_with("/auth") {
        if let Some(response) = routes::handle_auth_request(req, Arc::clone(&state)).await {
            return Ok(response);
        }
        return Ok(to_boxed(not_found_response(&path)));
    }

    let response = match (method, path.as_str()) {
        // Startup progress probe - reports identity/storage/projection/root-app readiness
        (Method::GET, "/health/startup") => {
            to_boxed(routes::startup_check(Arc::clone(&state)).await)
        }

        // Liveness probe - returns 200 if doorway is running
        (Method::GET, "/health") | (Method::GET, "/healthz") => {
            to_boxed(routes::health_check(Arc::clone(&state)))
        }

        // Readiness probe - returns 200 only if conductor is connected
        // Use this for seeder pre-flight checks
        (Method::GET, "/ready") | (Method::GET, "/readyz") => {
            to_boxed(routes::readiness_check(Arc::clone(&state)))
        }

        // Version info for deployment verification
        (Method::GET, "/version") => to_boxed(routes::version_info()),

        // Comprehensive status (runtime stats, cluster health, storage diagnostics)
        (Method::GET, "/status") => to_boxed(routes::status_page(req, Arc::clone(&state)).await),
        (Method::GET, "/status.json") => to_boxed(routes::status_check(Arc::clone(&state)).await),

        // Debug stream WebSocket for real-time debugging
        (Method::GET, "/debug/stream") if hyper_tungstenite::is_upgrade_request(&req) => {
            return Ok(to_boxed(
                routes::handle_debug_stream(
                    req,
                    Arc::clone(&state.debug_hub),
                    state.args.storage_url.clone(),
                )
                .await,
            ));
        }

        // DID Document for federation discovery (W3C standard path)
        (Method::GET, "/.well-known/did.json") => {
            to_boxed(routes::handle_did_document(Arc::clone(&state)))
        }

        // DID Document at explicit path (alternative)
        (Method::GET, "/identity/did") | (Method::GET, "/identity/did.json") => {
            to_boxed(routes::handle_did_endpoint(Arc::clone(&state)))
        }

        // Doorway public signing keys (JWKS format) for federation
        (Method::GET, "/.well-known/doorway-keys") => {
            to_boxed(routes::handle_doorway_keys(Arc::clone(&state)))
        }

        // Federation doorway listing
        (Method::GET, "/api/v1/federation/doorways") => {
            to_boxed(routes::handle_federation_doorways(Arc::clone(&state)).await)
        }

        // P2P peer discovery for desktop steward bootstrap
        (Method::GET, "/api/v1/federation/p2p-peers") => {
            to_boxed(routes::handle_federation_p2p_peers(Arc::clone(&state)).await)
        }

        // ====================================================================
        // Threshold (operator dashboard) - Angular SPA at /threshold/*
        // ====================================================================
        (Method::GET, p) if p.starts_with("/threshold") => {
            to_boxed(routes::handle_threshold_request(req, &state.args.threshold_url, p).await)
        }

        // ====================================================================
        // Holochain conductor WebSocket proxies
        // New paths: /hc/admin, /hc/app/{port}
        // Legacy paths: /, /admin, /app/{port} (kept for backwards compatibility)
        // ====================================================================

        // Chaperone: server-side Holochain connection setup (replaces admin WS dance)
        (Method::POST, "/hc/connect") => {
            return Ok(to_boxed(
                crate::conductor::chaperone::handle_hc_connect(req, Arc::clone(&state)).await,
            ));
        }

        // WebSocket upgrade for admin interface (NEW: /hc/admin)
        // Gated to dev-mode only — production clients use POST /hc/connect (Chaperone)
        (Method::GET, "/hc/admin") => {
            if !state.args.dev_mode {
                to_boxed(
                    Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error":"Admin WebSocket disabled in production. Use POST /hc/connect."}"#,
                        )))
                        .unwrap(),
                )
            } else if hyper_tungstenite::is_upgrade_request(&req) {
                to_boxed(websocket::handle_admin_upgrade(state, req).await)
            } else {
                to_boxed(bad_request_response(
                    "WebSocket upgrade required for /hc/admin",
                ))
            }
        }

        // WebSocket upgrade for app interface (NEW: /hc/app/{port})
        (Method::GET, p) if p.starts_with("/hc/app/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                // Extract port from /hc/app/:port
                let port_str = p.strip_prefix("/hc/app/").unwrap_or("");
                match port_str.parse::<u16>() {
                    Ok(port) if state.args.is_valid_app_port(port) => {
                        to_boxed(websocket::handle_app_upgrade(state, req, port).await)
                    }
                    _ => to_boxed(bad_request_response("Invalid app port")),
                }
            } else {
                to_boxed(bad_request_response(
                    "WebSocket upgrade required for /hc/app/{port}",
                ))
            }
        }

        // Root path: serve root SPA if configured, otherwise redirect to /threshold.
        // Preserve WebSocket upgrade for admin in dev mode (legacy path).
        (Method::GET, "/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                if state.args.dev_mode {
                    debug!("Legacy WebSocket path used - consider migrating to /hc/admin");
                    to_boxed(websocket::handle_admin_upgrade(state, req).await)
                } else {
                    to_boxed(
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .header("Content-Type", "application/json")
                            .body(Full::new(Bytes::from(
                                r#"{"error":"Admin WebSocket disabled in production. Use POST /hc/connect."}"#,
                            )))
                            .unwrap(),
                    )
                }
            } else {
                to_boxed(routes::handle_root_app_request(Arc::clone(&state), "/").await)
            }
        }

        // /admin: always redirect to /threshold (operator dashboard moved)
        (Method::GET, "/admin") => to_boxed(
            Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::from(
                    r#"<html><body>Redirecting to <a href="/threshold">/threshold</a></body></html>"#,
                )))
                .unwrap(),
        ),

        // WebSocket upgrade for app interface (LEGACY: /app/{port} - deprecated, use /hc/app/{port})
        (Method::GET, p) if p.starts_with("/app/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                debug!("Legacy WebSocket path used - consider migrating to /hc/app/{{port}}");
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

        // ====================================================================
        // Admin API endpoints for Shefa compute resources dashboard
        // ====================================================================

        // Feature flags — which optional services are active on this instance
        // No auth required: clients use this for upfront capability discovery
        (Method::GET, "/admin/capabilities") => {
            to_boxed(routes::handle_capabilities(Arc::clone(&state)).await)
        }

        // Conductor pool visibility (available on ALL instances)
        (Method::GET, "/admin/conductors") => {
            to_boxed(routes::handle_list_conductors(Arc::clone(&state)).await)
        }

        // Route registry visibility — which routes are live and from which source
        (Method::GET, "/admin/routes") => {
            to_boxed(routes::handle_route_registry(Arc::clone(&state)).await)
        }

        // Conductor agents listing
        (Method::GET, p) if p.starts_with("/admin/conductors/") && p.ends_with("/agents") => {
            let conductor_id = p
                .strip_prefix("/admin/conductors/")
                .and_then(|s| s.strip_suffix("/agents"))
                .unwrap_or("");
            to_boxed(routes::handle_conductor_agents(Arc::clone(&state), conductor_id).await)
        }

        // Manual agent→conductor assignment
        (Method::POST, "/admin/conductors/assign") => {
            return Ok(to_boxed(
                routes::handle_assign_agent(req, Arc::clone(&state)).await,
            ));
        }

        // Hosted users — manual provisioning
        (Method::POST, "/admin/hosted-users") => {
            return Ok(to_boxed(
                routes::handle_provision_user(req, Arc::clone(&state)).await,
            ));
        }

        // Hosted users — list users with conductor assignments
        (Method::GET, "/admin/hosted-users") => {
            to_boxed(routes::handle_list_hosted_users(Arc::clone(&state)).await)
        }

        // Hosted users — deprovision an agent
        (Method::DELETE, p) if p.starts_with("/admin/hosted-users/") => {
            let agent_key = p.strip_prefix("/admin/hosted-users/").unwrap_or("");
            to_boxed(routes::handle_deprovision_user(Arc::clone(&state), agent_key).await)
        }

        // Pipeline — user lifecycle funnel counts
        (Method::GET, "/admin/pipeline") => {
            to_boxed(routes::handle_admin_pipeline(Arc::clone(&state)).await)
        }

        // Dashboard topology — operator panel aggregate (Phase 5 T35)
        // DoorwayDashboardView: storage stewards + federation peers +
        // projection coverage + public surface. Composed per request from
        // RouteRegistry / PeerCache / ContentCache snapshots.
        (Method::GET, "/admin/dashboard/topology") => {
            to_boxed(routes::handle_admin_dashboard_topology(Arc::clone(&state)).await)
        }

        // Graduation endpoints — conductor retirement for steward users
        (Method::GET, "/admin/graduation/pending") => {
            to_boxed(routes::handle_graduation_pending(Arc::clone(&state)).await)
        }
        (Method::GET, "/admin/graduation/completed") => {
            to_boxed(routes::handle_graduation_completed(Arc::clone(&state)).await)
        }
        (Method::POST, p) if p.starts_with("/admin/graduation/force/") => {
            let agent_key = p.strip_prefix("/admin/graduation/force/").unwrap_or("");
            to_boxed(routes::handle_force_graduation(Arc::clone(&state), agent_key).await)
        }

        // Federation peer admin — configured peer URLs with add/remove/refresh
        (Method::GET, "/admin/federation/peers") => {
            to_boxed(routes::handle_admin_federation_peers(Arc::clone(&state)).await)
        }
        (Method::POST, "/admin/federation/peers/refresh") => {
            to_boxed(routes::handle_admin_refresh_federation_peers(Arc::clone(&state)).await)
        }
        (Method::POST, "/admin/federation/peers") => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    warn!("Federation peer add body error: {}", e);
                    return Ok(to_boxed(bad_request_response(
                        "Failed to read request body",
                    )));
                }
            };
            return Ok(to_boxed(
                routes::handle_admin_add_federation_peer(Arc::clone(&state), body).await,
            ));
        }
        (Method::DELETE, "/admin/federation/peers") => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    warn!("Federation peer remove body error: {}", e);
                    return Ok(to_boxed(bad_request_response(
                        "Failed to read request body",
                    )));
                }
            };
            return Ok(to_boxed(
                routes::handle_admin_remove_federation_peer(Arc::clone(&state), body).await,
            ));
        }

        // Agent conductor lookup
        (Method::GET, p) if p.starts_with("/admin/agents/") && p.ends_with("/conductor") => {
            let agent_key = p
                .strip_prefix("/admin/agents/")
                .and_then(|s| s.strip_suffix("/conductor"))
                .unwrap_or("");
            to_boxed(routes::handle_agent_conductor(Arc::clone(&state), agent_key).await)
        }

        // List all nodes with detailed resource and social metrics
        (Method::GET, "/admin/nodes") => to_boxed(routes::handle_nodes(Arc::clone(&state)).await),

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
                to_boxed(bad_request_response(
                    "WebSocket upgrade required for /admin/ws",
                ))
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
        // Admin cache control API — observe and toggle projection/app-file caches
        // Used by delivery diagnostics tests to prove fallback behavior
        (Method::GET, "/admin/cache/stats") => {
            to_boxed(routes::admin_cache::cache_stats(Arc::clone(&state)).await)
        }
        (Method::POST, "/admin/cache/disable") => {
            to_boxed(routes::admin_cache::cache_disable(Arc::clone(&state)).await)
        }
        (Method::POST, "/admin/cache/enable") => {
            to_boxed(routes::admin_cache::cache_enable(Arc::clone(&state)).await)
        }
        (Method::POST, p) if p.starts_with("/admin/cache/clear/") => {
            let slug = p.strip_prefix("/admin/cache/clear/").unwrap_or("");
            to_boxed(routes::admin_cache::cache_clear_slug(Arc::clone(&state), slug).await)
        }
        (Method::POST, "/admin/cache/warm") => {
            to_boxed(routes::admin_cache::cache_warm(Arc::clone(&state)).await)
        }

        // ====================================================================
        // Admin User Management API
        // Requires Admin permission via JWT token
        // ====================================================================
        (_, p) if p.starts_with("/admin/users") => {
            to_boxed(routes::handle_admin_users_request(req, Arc::clone(&state), p).await)
        }

        // Bootstrap service routes (X-Op header protocol)
        // POST /bootstrap with X-Op header, or legacy path-based routing
        (Method::POST, p) if p == "/bootstrap" || p.starts_with("/bootstrap/") => {
            handle_bootstrap_request(state, req, &path).await
        }

        // Bootstrap ping (GET for health check)
        (Method::GET, "/bootstrap") => to_boxed(
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(Full::new(Bytes::from("OK")))
                .unwrap(),
        ),

        // Signal service WebSocket (SBD protocol)
        (Method::GET, p) if p.starts_with("/signal/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                handle_signal_request(state, req, &path, addr).await
            } else {
                to_boxed(bad_request_response(
                    "Signal endpoint requires WebSocket upgrade",
                ))
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
            let base_url = format!("{scheme}://{host}");
            to_boxed(routes::handle_stream_request(state, p, &base_url).await)
        }

        // Blob verification endpoint
        (Method::POST, "/api/blob/verify") => handle_blob_verify(state, req).await,

        // Note: GET/HEAD /blob/{hash} is handled by the wildcard `classify_dispatch`
        // arm via the storage manifest's blob_proxy registration. Legacy /store/{hash}
        // and /api/blob/{hash} dispatch arms were removed in the 2026-04-30 vocabulary
        // cleanup. See genesis/graphos/vocabulary.md.

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
            to_boxed(
                routes::handle_api_request(state, p, query, Some(remote_ip), auth_header).await,
            )
        }

        // WebSocket import progress (proxy to elohim-storage)
        // GET /import/progress - WebSocket upgrade for real-time progress
        (Method::GET, "/import/progress") if hyper_tungstenite::is_upgrade_request(&req) => {
            info!("WebSocket upgrade request for /import/progress");
            return Ok(to_boxed(
                routes::handle_import_progress_ws(req, state.args.storage_url.clone()).await,
            ));
        }

        // Dynamic import routes (forwarded to elohim-storage)
        // POST /import/{batch_type} - queue import → forward to storage
        // GET /import/{batch_type}/{batch_id} - get status → forward to storage
        (method, p)
            if matches!(method, Method::POST | Method::GET) && p.starts_with("/import/") =>
        {
            // Parse import path: /import/{batch_type} or /import/{batch_type}/{batch_id}
            let remainder = p.strip_prefix("/import/").unwrap_or("");
            if remainder.is_empty() {
                return Ok(to_boxed(
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Missing batch_type in path"}"#,
                        )))
                        .unwrap(),
                ));
            }

            let parts: Vec<&str> = remainder.splitn(2, '/').collect();
            let batch_type = parts[0].to_string();
            let batch_id = parts.get(1).map(|s| s.to_string());

            info!(
                batch_type = %batch_type,
                batch_id = ?batch_id,
                storage_url = ?state.args.storage_url,
                "Forwarding import request to elohim-storage"
            );

            return Ok(to_boxed(
                routes::handle_import_request(
                    req,
                    state.args.storage_url.clone(),
                    batch_type,
                    batch_id,
                )
                .await,
            ));
        }

        // Database API routes (proxied to elohim-storage)
        // GET/POST/DELETE /db/content[/{id}], /db/stats, etc.
        // Required for browser clients since they can't access elohim-storage directly (CORS)
        (_, p) if p.starts_with("/db/") => {
            debug!(path = %p, "Forwarding database request to elohim-storage");
            return Ok(to_boxed(
                routes::handle_db_request(req, state.args.storage_url.clone(), p).await,
            ));
        }

        // Account API routes — handled by dynamic registry fallback below

        // Delivery capability probe (must match before GET /apps/)
        // HEAD /apps/{app_id}/_capability — lightweight probe, no body
        (Method::HEAD, p) if p.starts_with("/apps/") && p.ends_with("/_capability") => {
            debug!(path = %p, "Handling app capability probe");
            return Ok(to_boxed(
                routes::handle_app_capability(Arc::clone(&state), p).await,
            ));
        }

        // HTML5 App serving routes (projection cache → elohim-storage fallback)
        // GET /apps/{app_id}/{path} - Serve files from HTML5 app ZIPs
        (Method::GET, p) if p.starts_with("/apps/") => {
            debug!(path = %p, "Handling app request (projection cache)");
            return Ok(to_boxed(
                routes::handle_app_request(Arc::clone(&state), p).await,
            ));
        }

        // EPR Head proxy routes (proxied to elohim-storage)
        // GET/PUT /epr-head/{id} - Three-pillar metadata envelope (DAG-CBOR)
        (method, p)
            if matches!(method, Method::GET | Method::PUT) && p.starts_with("/epr-head/") =>
        {
            let id = p.strip_prefix("/epr-head/").unwrap_or("");
            debug!(path = %p, id = %id, "Forwarding EPR Head request to elohim-storage");
            let storage_url = match &state.args.storage_url {
                Some(url) => url,
                None => {
                    return Ok(to_boxed(
                        Response::builder()
                            .status(StatusCode::SERVICE_UNAVAILABLE)
                            .header("Content-Type", "application/json")
                            .body(Full::new(Bytes::from(
                                r#"{"error":"Storage URL not configured"}"#,
                            )))
                            .unwrap(),
                    ));
                }
            };
            match routes::handle_epr_head_request(req, storage_url, id).await {
                Ok(resp) => to_boxed(resp),
                Err(e) => {
                    error!(error = %e, "EPR Head proxy failed");
                    to_boxed(
                        Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .header("Content-Type", "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error":"EPR Head proxy failed: {e}"}}"#
                            ))))
                            .unwrap(),
                    )
                }
            }
        }

        // ====================================================================
        // Domain API Routes (v1) — business logic layer
        // ====================================================================

        // Collective governance
        (_, p) if p.starts_with("/api/v1/collectives") => {
            return Ok(to_boxed(
                routes::handle_collectives_request(req, Arc::clone(&state), p).await,
            ));
        }

        // Human-scoped collectives (cross-domain route)
        (Method::GET, p) if p.starts_with("/api/v1/humans/") && p.ends_with("/collectives") => {
            return Ok(to_boxed(
                routes::handle_collectives_request(req, Arc::clone(&state), p).await,
            ));
        }

        // Journal routing (intent analysis + suggestion generation)
        (Method::POST, "/api/v1/journal/analyze") => {
            return Ok(to_boxed(
                routes::handle_journal_analyze(req, Arc::clone(&state)).await,
            ));
        }

        (Method::POST, "/api/v1/journal/suggest") => {
            return Ok(to_boxed(
                routes::handle_journal_suggest(req, Arc::clone(&state)).await,
            ));
        }

        // Elohim Agent invocation
        (_, p) if p.starts_with("/api/v1/elohim") => {
            return Ok(to_boxed(
                routes::handle_elohim_agent_request(req, Arc::clone(&state), p).await,
            ));
        }

        // Identity API proxy — must come before /identity/did to avoid shadowing
        (_, p) if p.starts_with("/api/v1/identity") => {
            return Ok(to_boxed(
                routes::handle_identity_api_request(req, Arc::clone(&state), p).await,
            ));
        }

        // ====================================================================
        // SSR smoke-test route — proves doorway → elohim-render end-to-end.
        //
        // TEMPORARY: Replaced by manifest-driven SSR dispatch in Task 13
        //   (genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md).
        // The hardcoded /render-test/* arm exists only as the smallest first
        // slice that proves the architecture; once the storage manifest gains
        // a `render` field and the registry honours it, this arm is removed.
        //
        // GET /render-test/{url} — delegates to the Angular SSR renderer when
        // SSR_BUNDLE_PATH is set at startup.  Falls back to a static SPA shell
        // when the renderer is absent (SSR_BUNDLE_PATH unset) or returns an
        // error.  This arm is intentionally hardcoded (not manifest-driven)
        // because SSR is a doorway-specific concern: the renderer lives in
        // doorway's process, not in elohim-storage.
        // ====================================================================
        (Method::GET, p) if p.starts_with("/render-test/") => {
            if let Some(renderer) = state.renderer.as_ref() {
                let stripped = p.strip_prefix("/render-test/").unwrap_or("");
                let url = format!("/{}", stripped);
                let ctx = elohim_render::RenderContext {
                    spec: elohim_render::RenderSpec::AngularSsr,
                    url: url.clone(),
                    data_fetcher: Arc::new(NoopFetcher),
                    limits: Default::default(),
                };
                match renderer.render(ctx).await {
                    Ok(out) => {
                        tracing::debug!(target: "doorway::ssr", url = %url, "SSR render ok");
                        return Ok(to_boxed(ssr_html_response(out.html)));
                    }
                    Err(e) => {
                        tracing::warn!(target: "doorway::ssr", url = %url, error = %e, "SSR render error — falling back to SPA shell");
                        return Ok(to_boxed(ssr_spa_shell_fallback()));
                    }
                }
            }
            // No renderer configured — return SPA shell so CSR can hydrate.
            to_boxed(ssr_spa_shell_fallback())
        }

        // ====================================================================
        // Dynamic Route Registry + SPA fallback — all remaining requests.
        //
        // The registry is consulted on every otherwise-unmatched request.
        // Any path elohim-storage declared in its manifest (routes,
        // blob_proxy, stream_proxy, …) is dispatched without doorway-side
        // path-prefix changes — this is the contract written in
        // doorway/CLAUDE.md ("Adding a new endpoint to elohim-storage
        // automatically makes it routable through doorway").
        //
        // Registry miss + GET + slug configured → SPA bootstrap (Angular
        // client-side routing). Anything else → 404.
        // ====================================================================
        (_, p) => {
            let dispo = classify_dispatch(
                &state.route_registry,
                state.args.root_app_slug.as_deref(),
                req.method(),
                p,
            )
            .await;

            match dispo {
                Disposition::StorageProxy { endpoint } => {
                    debug!(path = %p, %endpoint, "Registry-routed to storage proxy");
                    // Resolve agent_cid from the bearer's claims (alpha-substrate:
                    // claims.human_id IS agent_cid). Storage's view services use
                    // X-Agent-Cid for identity-scoped views (`/cluster`,
                    // `/peer-topology`, `/reciprocity`) and reach gating on private
                    // blobs. Visitor / invalid-bearer paths get None and storage
                    // falls back to its local_sessions resolution or visitor branch.
                    let agent_cid_owned = resolve_agent_cid_from_request(&state, &req);
                    let ctx = routes::ForwardCtx {
                        agent_cid: agent_cid_owned.as_deref(),
                    };
                    // Blob paths get cache-aware forwarding; all other registry
                    // routes use the generic forwarder unchanged.
                    if p.starts_with("/blob/") {
                        return Ok(to_boxed(
                            routes::forward_blob_to_storage(
                                req,
                                &endpoint,
                                p,
                                Arc::clone(&state.cache),
                                ctx,
                            )
                            .await,
                        ));
                    }
                    return Ok(to_boxed(
                        routes::forward_to_storage(req, &endpoint, p, ctx).await,
                    ));
                }
                Disposition::RegistryUnhandled => {
                    debug!(path = %p, "Registry matched but target type not yet handled");
                    to_boxed(not_found_response(p))
                }
                Disposition::RootApp => {
                    to_boxed(routes::handle_root_app_request(Arc::clone(&state), p).await)
                }
                Disposition::NotFound => {
                    debug!(path = %p, "No registry match and no SPA fallback");
                    to_boxed(not_found_response(p))
                }
            }
        }
    };

    // Fire-and-forget: contribute doorway-originated errors to the observation session.
    // Only fires when the client sent X-Observation-Id AND doorway itself produced a 4xx/5xx
    // (i.e. errors before the request ever reached elohim-storage, such as registry misses,
    // auth failures at the gateway level, or conductor unavailability).
    if let Some(ref obs_id) = observation_id {
        let status = response.status().as_u16();
        if status >= 400 {
            if let Some(ref storage_url) = storage_url_for_obs {
                maybe_contribute_observation(
                    obs_id,
                    storage_url,
                    "route",
                    if status >= 500 { "error" } else { "warning" },
                    &method_str,
                    &path,
                    status,
                    &format!("Doorway returned {} before reaching storage", status),
                );
            }
        }
    }

    Ok(response)
}

/// Fire-and-forget: contribute a doorway observation entry to storage.
///
/// Called when doorway itself produces a 4xx/5xx response — i.e. errors that occur
/// before the request reaches elohim-storage (registry misses, auth failures, conductor
/// unavailability). Doorway observes things storage can't see; this surfaces them into
/// the same observation session the client is tracking.
///
/// The spawn is intentionally detached — we never await it. A failure to deliver the
/// observation entry must never affect the original response.
#[allow(clippy::too_many_arguments)]
fn maybe_contribute_observation(
    observation_id: &str,
    storage_url: &str,
    category: &str,
    severity: &str,
    method: &str,
    path: &str,
    status_code: u16,
    message: &str,
) {
    let url = format!(
        "{}/api/v1/observations/{}/entries",
        storage_url.trim_end_matches('/'),
        observation_id
    );
    let body = serde_json::json!({
        "origin": "doorway",
        "category": category,
        "severity": severity,
        "method": method,
        "path": path,
        "statusCode": status_code,
        "message": message
    });

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await;
    });
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

    debug!(
        "Bootstrap request: op={}, network={}, x_op={:?}",
        op, network, x_op
    );

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
                r#"{{"error": "Unknown bootstrap operation: {op}"}}"#
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
async fn handle_blob_verify(state: Arc<AppState>, req: Request<Incoming>) -> Response<BoxBody> {
    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Blob verify request body error: {}", e);
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

    // Parse JSON request
    let request: VerifyBlobRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Blob verify JSON parse error: {}", e);
            return to_boxed(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid JSON: {e}"}}"#
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
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json_body)))
            .unwrap(),
    )
}

/// Convert a Full<Bytes> body to BoxBody
fn to_boxed(response: Response<Full<Bytes>>) -> Response<BoxBody> {
    response.map(|body| body.map_err(|never| match never {}).boxed())
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

// ─── SSR helpers ──────────────────────────────────────────────────────────────

/// `DataFetcher` that returns 404 for every fetch.
///
/// TEMPORARY: Replaced in Task 13 by `ResolverFetcher`, which dispatches
/// fetches through `DoorwayResolver` so SSR data calls hit the projection
/// cache.  This stub exists only for the `/render-test/*` smoke route.
///
/// Used by the `/render-test/*` route so Angular SSR can call the renderer
/// without a real elohim-storage sidecar.  Any `fetch()` inside the bundle
/// will receive a 404 body rather than crashing the render worker.
struct NoopFetcher;

#[async_trait::async_trait]
impl elohim_render::DataFetcher for NoopFetcher {
    async fn fetch(
        &self,
        _request: elohim_render::FetchRequest,
    ) -> elohim_render::Result<elohim_render::FetchResponse> {
        Ok(elohim_render::FetchResponse {
            status: 404,
            headers: Default::default(),
            body: b"not found".to_vec(),
            content_hash: None,
        })
    }
}

/// Build a `text/html` response from SSR-rendered HTML.
fn ssr_html_response(html: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Minimal SPA shell returned when SSR is unavailable or the renderer errors.
///
/// The shell contains an `<app-root>` placeholder so the Angular CSR bundle
/// can hydrate the page without a full server-rendered document.
fn ssr_spa_shell_fallback() -> Response<Full<Bytes>> {
    const SHELL: &str = concat!(
        "<!doctype html><html><body>",
        "<app-root></app-root>",
        "<script>console.log('SSR fallback \u{2014} CSR will hydrate')</script>",
        "</body></html>",
    );
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(SHELL)))
        .unwrap()
}

// ─── Gate layer tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod gate_layer_tests {
    use super::*;

    // Path-inference correctness tests live in gate_client::tower::path_tests.
    // doorway delegates to gate_client::infer_event_from_path — no duplication needed.

    // ── apply_gate_check: transparency in DevContext ──────────────────────────

    #[tokio::test]
    async fn get_requests_bypass_gate() {
        // GET is read-only — gate must never fire.
        let result = apply_gate_check(&Method::GET, "/content").await;
        assert!(
            result.is_none(),
            "GET /content must pass through gate (read-only)"
        );
    }

    #[tokio::test]
    async fn post_to_unmapped_path_passes_through() {
        // POST on a path the gate does not recognise falls through (Phase 1 contract).
        let result = apply_gate_check(&Method::POST, "/db/content/bulk").await;
        assert!(
            result.is_none(),
            "POST to unmapped path must fall through gate"
        );
    }

    #[tokio::test]
    async fn post_to_mapped_path_passes_through_in_dev_context() {
        // In DevContext the gate always returns Allow.
        // apply_gate_check must return None so routing continues unchanged.
        let result = apply_gate_check(&Method::POST, "/content").await;
        assert!(
            result.is_none(),
            "POST /content must pass through in DevContext (gate always allows)"
        );
    }

    #[tokio::test]
    async fn put_to_mapped_path_passes_through_in_dev_context() {
        let result = apply_gate_check(&Method::PUT, "/attestation").await;
        assert!(
            result.is_none(),
            "PUT /attestation must pass through in DevContext"
        );
    }

    #[tokio::test]
    async fn post_to_content_does_not_short_circuit_in_dev_context() {
        // Regression: the gate must not produce a 403 for normal writes in DevContext.
        // This is the key transparency assertion — existing callers are unaffected.
        let result = apply_gate_check(&Method::POST, "/content").await;
        assert!(
            result.is_none(),
            "DevContext gate must not short-circuit POST /content"
        );
    }

    // ── Phase 7 E2E: Decline path through doorway apply_gate_check ────────────
    //
    // These tests exercise the full Decline/Escalate code paths in doorway's
    // `apply_gate_check` using the gate-client testing override.  They prove
    // that the doorway seam correctly translates gate decisions into HTTP
    // responses without requiring a live conductor.
    //
    // What IS verified:
    //   - Decline → 403, x-gate-verdict: decline, JSON body with grounds
    //   - Escalate → 202, x-gate-verdict: escalate, JSON body with target
    //   - Path inference (POST /content → ContentPublish, POST /attestation → AttestationWrite)
    //
    // What IS NOT verified:
    //   - Real DHT commit (no conductor running)
    //   - Real elohim-agent wisdom evaluation (DevContext always returns Allow;
    //     these tests inject overrides via gate-client's testing shim)
    //   - HTTP proxy to elohim-storage (no storage instance)

    #[tokio::test]
    async fn decline_produces_403_with_gate_verdict_header_and_grounds() {
        // Inject a Decline via the gate-client testing override.
        gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
            "harmful-content",
            "content violates existential safety principle — Phase 7 E2E probe",
        )));

        let result = apply_gate_check(&Method::POST, "/content").await;

        let resp = result
            .expect("Decline must produce Some(response) — apply_gate_check must short-circuit");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Decline must produce 403 Forbidden"
        );

        let verdict = resp
            .headers()
            .get("x-gate-verdict")
            .expect("x-gate-verdict header must be present on Decline response");
        assert_eq!(
            verdict, "decline",
            "x-gate-verdict header must be 'decline'"
        );

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("body must be readable")
            .to_bytes();
        let body = String::from_utf8_lossy(&body_bytes);
        assert!(
            body.contains("harmful-content"),
            "body must carry the grounds category; got: {body}"
        );
        assert!(
            body.contains("\"gate\":\"declined\"") || body.contains("\"gate\": \"declined\""),
            "body must carry gate:declined marker; got: {body}"
        );
    }

    #[tokio::test]
    async fn escalate_produces_202_with_gate_verdict_header_and_target() {
        use gate_client::types::{EscalationTarget, Severity};

        gate_client::__test_set_decision_override(Some(gate_client::testing::mock_escalate(
            EscalationTarget::AppSteward {
                steward_id: "steward-phase7".to_string(),
            },
            Severity::High,
        )));

        let result = apply_gate_check(&Method::POST, "/attestation").await;

        let resp = result
            .expect("Escalate must produce Some(response) — apply_gate_check must short-circuit");
        assert_eq!(
            resp.status(),
            StatusCode::ACCEPTED,
            "Escalate must produce 202 Accepted"
        );

        let verdict = resp
            .headers()
            .get("x-gate-verdict")
            .expect("x-gate-verdict header must be present on Escalate response");
        assert_eq!(
            verdict, "escalate",
            "x-gate-verdict header must be 'escalate'"
        );

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("body must be readable")
            .to_bytes();
        let body = String::from_utf8_lossy(&body_bytes);
        assert!(
            body.contains("\"gate\":\"escalated\"") || body.contains("\"gate\": \"escalated\""),
            "body must carry gate:escalated marker; got: {body}"
        );
    }

    // ── Path-inference correctness: doorway delegates to gate_client ──────────

    #[tokio::test]
    async fn post_to_attestation_infers_attestation_write() {
        // Verify path inference works for the attestation path (POST /attestation
        // → AttestationWrite event).  DevContext returns Allow → None from
        // apply_gate_check, which means routing continues unchanged.
        let result = apply_gate_check(&Method::POST, "/attestation").await;
        assert!(
            result.is_none(),
            "POST /attestation must pass through in DevContext (gate infers AttestationWrite)"
        );
    }

    #[tokio::test]
    async fn decline_on_unmapped_path_is_never_reached_because_gate_skips_unmapped() {
        // Even if a Decline were injected, an unmapped path never calls check()
        // so the override is never consumed. apply_gate_check returns None.
        gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
            "should-not-fire",
            "gate skips unmapped paths",
        )));

        let result = apply_gate_check(&Method::POST, "/db/stats").await;
        assert!(
            result.is_none(),
            "Unmapped paths must skip gate entirely; decline override was not consumed"
        );

        // Clean up the unused override so it doesn't bleed into subsequent tests.
        gate_client::__test_set_decision_override(None);
    }
}

#[cfg(test)]
mod dispatch_classification_tests {
    //! Contract tests for `classify_dispatch`.
    //!
    //! These tests pin the dispatch contract that fixes the recurring
    //! /blob/<hash> regression: any path the registry has compiled is routed
    //! by the registry, regardless of prefix. Adding `blob_proxy` /
    //! `stream_proxy` / future manifest path families to elohim-storage's
    //! manifest must NOT require a doorway code change to make them routable.

    use super::{classify_dispatch, Disposition};
    use crate::services::RouteRegistry;
    use doorway_client::HttpMethod;
    use hyper::Method;

    /// Inject a `StorageProxy` route at `path` into a fresh registry.
    /// Mimics what `register_steward_peer` does internally after fetching
    /// the manifest — see `route_registry.rs:671-687`.
    async fn registry_with_storage_route(method: HttpMethod, path: &str) -> RouteRegistry {
        use crate::services::route_registry::{CompiledRoute, RouteSource, RouteTarget};
        let registry = RouteRegistry::with_defaults();
        let route = CompiledRoute {
            method,
            path: path.to_string(),
            source: RouteSource::StewardPeer {
                storage_url: "http://storage:8090".to_string(),
            },
            target: RouteTarget::StorageProxy {
                endpoint: "http://storage:8090".to_string(),
            },
            auth_required: false,
            cache_ttl_secs: 0,
            rate_limit_rpm: 0,
        };
        let mut compiled = registry.compiled_routes.write().await;
        compiled.push(route);
        drop(compiled);
        registry
    }

    #[tokio::test]
    async fn blob_path_dispatches_to_storage_proxy() {
        // Regression: the recurring thumbnail bug. /blob/<hash> must reach
        // the registry and be classified as StorageProxy, not fall through
        // to the SPA bootstrap.
        let registry = registry_with_storage_route(HttpMethod::Get, "/blob/:hash").await;
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::GET,
            "/blob/sha256-abcdef123456",
        )
        .await;
        assert!(
            matches!(&dispo, Disposition::StorageProxy { endpoint } if endpoint == "http://storage:8090"),
            "GET /blob/<hash> must classify as StorageProxy, got {dispo:?}"
        );
    }

    #[tokio::test]
    async fn arbitrary_new_prefix_reaches_registry() {
        // The durable contract: a hypothetical future manifest path family
        // (e.g. /thumbnails/, /shards/, anything outside /api/v1/+/account/)
        // must route through the registry without a doorway code change.
        let registry = registry_with_storage_route(HttpMethod::Get, "/future/:id").await;
        let dispo =
            classify_dispatch(&registry, Some("lamad"), &Method::GET, "/future/some-id").await;
        assert!(
            matches!(dispo, Disposition::StorageProxy { .. }),
            "Any registry-compiled path must route through the registry"
        );
    }

    #[tokio::test]
    async fn unregistered_get_with_slug_falls_through_to_root_app() {
        // SPA client-side routing: paths the registry doesn't know
        // (e.g. /learn/<id>) must serve the SPA bootstrap on GET when a
        // root_app_slug is configured.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::GET,
            "/learn/some-path-id",
        )
        .await;
        assert_eq!(
            dispo,
            Disposition::RootApp,
            "Unregistered GET with slug configured must fall through to SPA"
        );
    }

    #[tokio::test]
    async fn unregistered_post_returns_not_found() {
        // API misses must 404 (not serve HTML). Without this, an unknown
        // POST /api/v1/foo would render the SPA bootstrap to a JSON client.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::POST,
            "/api/v1/no-such-route",
        )
        .await;
        assert_eq!(
            dispo,
            Disposition::NotFound,
            "Unregistered non-GET must 404, never SPA"
        );
    }
}
