//! HTTP API for shard storage
//!
//! Provides REST endpoints for storing and retrieving shards:
//!
//! ## Shard API (Direct Shard Access)
//! - `PUT /shard/{shard_hash}` - Store a shard
//! - `GET /shard/{shard_hash}` - Retrieve a shard
//! - `HEAD /shard/{shard_hash}` - Check if shard exists
//!
//! ## Blob API (Convenience - Auto-Sharding)
//! - `PUT /blob/{blob_hash}` - Store blob, auto-create manifest
//! - `GET /blob/{blob_hash}` - Reassemble blob from shards
//! - `GET /manifest/{blob_hash}` - Get shard manifest
//!
//! ## Example Usage
//!
//! ```bash
//! # Store a shard
//! curl -X PUT -H "Content-Type: application/octet-stream" \
//!      --data-binary @video-chunk.bin \
//!      http://localhost:8090/shard/sha256-abc123
//!
//! # Retrieve a shard
//! curl http://localhost:8090/shard/sha256-abc123 > chunk.bin
//!
//! # Store a blob (auto-shards into manifest)
//! curl -X PUT -H "Content-Type: video/mp4" \
//!      --data-binary @video.mp4 \
//!      http://localhost:8090/blob/sha256-xyz789
//!
//! # Get manifest to see shards
//! curl http://localhost:8090/manifest/sha256-xyz789
//! ```

use crate::blob_store::BlobStore;
use crate::db::content_diesel::ContentQuery;
use crate::db::policy_cache::{
    ContentMetadata, PolicyDecision, PolicyEnforcement, PolicyEvent, PolicyEventType,
};
use crate::db::{self, AppContext, DbPool, PooledConn};
use crate::db::{
    collectives, content_mastery, contributor_presences, economic_events, human_relationships,
    humans, stewardship_allocations,
};
use crate::error::StorageError;
use crate::import_api::ImportApi;
use crate::progress_hub::ProgressHub;
use crate::progress_ws;
use crate::services::{response, Services};
use crate::sharding::{ShardEncoder, ShardManifest};
use crate::sync::SyncManager;
use crate::views::{
    validate_schema_versions,
    AccountIdentityView,
    AccountImportResultView,
    AccountPackageInputView,
    AccountPackageView,
    BeginObservationInputView,
    BeginObservationResponseView,
    CollectiveParticipationView,
    CollectiveSeedView,
    CollectiveView,
    ContentAssignmentView,
    ContentMasteryView,
    ContentStewardshipView,
    ContentView,
    ContentWithTagsView,
    ContributorPresenceView,
    CreateAllocationInputView,
    CreateCollectiveInputView,
    // InputView types for API boundary (camelCase with parsed JSON)
    CreateContentInputView,
    CreateContributorPresenceInputView,
    CreateEconomicEventInputView,
    CreateHumanRelationshipInputView,
    CreateMasteryInputView,
    CreateNodeStewardshipInputView,
    CreateRelationshipInputView,
    CreateScheduleInputView,
    CreateStewardedNodeInputView,
    EconomicEventView,
    EprHeadInputView,
    EprHeadView,
    HumanView,
    InitiateClaimInputView,
    LocalSessionView,
    NodeStewardshipView,
    ObservationDurationView,
    ObservationEntryInputView,
    ObservationEntryView,
    ObservationIssueView,
    ObservationReportView,
    ObservationSummaryView,
    ObservationSystemStateView,
    PackageManifestView,
    RelationshipSeedView,
    RelationshipView,
    ScheduleView,
    StewardedNodeView,
    StewardshipAllocationView,
    StewardshipSeedView,
    UpdateAllocationInputView,
    UpdateContentInputView,
    SUPPORTED_SCHEMA_VERSIONS,
};
use bytes::Bytes;
use doorway_client::{DoorwayRoutesBuilder, Route};
use elohim_cache_core::extraction::ExtractionCache;
use http_body_util::{BodyExt, Either, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};

/// Unified response body: Left for normal (buffered) responses, Right for SSE (streaming).
pub type ApiBody = Either<Full<Bytes>, crate::sse::SseBody>;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Maximum number of concurrent HTTP requests the server will handle.
/// Excess requests wait for a permit, preventing OOM under burst traffic
/// (e.g., HTML5 app iframe loading 30+ assets simultaneously).
const MAX_CONCURRENT_REQUESTS: usize = 64;

/// HTTP server state
pub struct HttpServer {
    blob_store: Arc<BlobStore>,
    manifests: Arc<RwLock<std::collections::HashMap<String, ShardManifest>>>,
    bind_addr: SocketAddr,
    /// Optional Import API for handling /import/* routes
    import_api: Option<Arc<RwLock<ImportApi>>>,
    /// Progress hub for WebSocket streaming
    progress_hub: Option<Arc<ProgressHub>>,
    /// Sync manager for CRDT document sync
    sync_manager: Option<Arc<SyncManager>>,
    /// Diesel connection pool for database operations
    db_pool: Option<DbPool>,
    /// Service layer for business logic
    services: Option<Arc<Services>>,
    /// Policy enforcement for stewardship content filtering
    policy_enforcement: Option<Arc<PolicyEnforcement>>,
    /// Node Registry API for tracking shards
    node_registry_api: Option<Arc<crate::node_registry_api::NodeRegistryApi>>,
    /// P2P handle for status endpoint (Send+Sync safe)
    #[cfg(feature = "p2p")]
    p2p_handle: Option<crate::p2p::P2PHandle>,
    /// Extraction cache for HTML5 app files (None = disabled)
    extraction_cache: Option<Arc<ExtractionCache>>,
    /// In-memory index: slug -> blobHash (avoids per-request SQLite scan)
    slug_index: Arc<RwLock<std::collections::HashMap<String, String>>>,
    /// Concurrency limiter: prevents OOM under burst traffic (e.g., HTML5 app loads)
    request_semaphore: Arc<Semaphore>,
}

/// Extract X-Schema-Version header from request and validate it.
/// Returns Ok(Some(version)) if present and valid, Ok(None) if absent, Err if unsupported.
fn validate_schema_version_header(req: &Request<Incoming>) -> Result<Option<u32>, String> {
    match req.headers().get("X-Schema-Version") {
        Some(val) => {
            let version = val
                .to_str()
                .map_err(|_| "Invalid X-Schema-Version header encoding".to_string())?
                .parse::<u32>()
                .map_err(|_| "X-Schema-Version must be a positive integer".to_string())?;
            if !SUPPORTED_SCHEMA_VERSIONS.contains(&version) {
                return Err(format!(
                    "Unsupported schema version: {}. Supported: {:?}",
                    version, SUPPORTED_SCHEMA_VERSIONS
                ));
            }
            Ok(Some(version))
        }
        None => {
            warn!("Bulk request missing X-Schema-Version header (deprecated: clients should send this header)");
            Ok(None)
        }
    }
}

/// Check whether an identifier looks like a content address (sha256-...) rather
/// than a human-readable slug. Used by `/apps/` routes to decide whether to
/// resolve via `slug_index` or treat the identifier as a direct blob hash.
fn is_content_address(identifier: &str) -> bool {
    identifier.starts_with("sha256-") && identifier.len() > 10
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(blob_store: Arc<BlobStore>, bind_addr: SocketAddr) -> Self {
        Self {
            blob_store,
            manifests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bind_addr,
            import_api: None,
            progress_hub: None,
            sync_manager: None,
            db_pool: None,
            services: None,
            policy_enforcement: None,
            node_registry_api: None,
            #[cfg(feature = "p2p")]
            p2p_handle: None,
            extraction_cache: None,
            slug_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            request_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        }
    }

    /// Set the Import API handler
    pub fn with_import_api(mut self, import_api: Arc<RwLock<ImportApi>>) -> Self {
        self.import_api = Some(import_api);
        self
    }

    /// Set the Progress Hub for WebSocket streaming
    pub fn with_progress_hub(mut self, hub: Arc<ProgressHub>) -> Self {
        self.progress_hub = Some(hub);
        self
    }

    /// Set the Node Registry API
    pub fn with_node_registry_api(
        mut self,
        api: Arc<crate::node_registry_api::NodeRegistryApi>,
    ) -> Self {
        self.node_registry_api = Some(api);
        self
    }

    /// Set the Sync Manager for CRDT document sync
    pub fn with_sync_manager(mut self, sync_manager: Arc<SyncManager>) -> Self {
        self.sync_manager = Some(sync_manager);
        self
    }

    /// Set the Service layer for business logic
    pub fn with_services(mut self, services: Arc<Services>) -> Self {
        self.services = Some(services);
        self
    }

    /// Set the Diesel connection pool for new entity endpoints
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Set the Policy Enforcement engine for stewardship content filtering
    pub fn with_policy_enforcement(mut self, enforcement: Arc<PolicyEnforcement>) -> Self {
        self.policy_enforcement = Some(enforcement);
        self
    }

    /// Set the P2P handle for status endpoint
    #[cfg(feature = "p2p")]
    pub fn with_p2p_handle(mut self, handle: crate::p2p::P2PHandle) -> Self {
        self.p2p_handle = Some(handle);
        self
    }

    /// Set the extraction cache
    pub fn with_extraction_cache(mut self, cache: Arc<ExtractionCache>) -> Self {
        self.extraction_cache = Some(cache);
        self
    }

    /// Load the slug index from database (call after db_pool is set)
    pub async fn load_slug_index(&self) {
        let mut conn = match self.get_conn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get DB connection for slug index: {}", e);
                return;
            }
        };
        let app_ctx = db::AppContext::default_lamad();
        // Startup slug index refresh. This cache backs external HTML5 app
        // routing, so it must only index content that has a provenance
        // marker (dht_anchor_hash or p2p_published_at). If the drain has
        // not run yet, the index will be empty — that is the desired
        // behavior: we must never serve undrained rows to browsers.
        let query = ContentQuery {
            content_format: Some("html5-app".to_string()),
            limit: 100,
            require_provenance: true,
            ..Default::default()
        };

        match db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
            Ok(items) => {
                let mut index = self.slug_index.write().await;
                index.clear();
                for item in items {
                    if let Some(ref content_body) = item.content.content_body {
                        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                            if let Some(slug) = obj.get("slug").and_then(|v| v.as_str()) {
                                let blob_hash = item.content.blob_hash.clone().unwrap_or_default();
                                if !blob_hash.is_empty() {
                                    info!(slug = %slug, blob_hash = %blob_hash, "Indexed HTML5 app");
                                    index.insert(slug.to_string(), blob_hash);
                                }
                            }
                        }
                    }
                }
                info!(count = index.len(), "Slug index loaded");
            }
            Err(e) => {
                warn!("Failed to load slug index: {}", e);
            }
        }
    }

    /// Get a connection from the Diesel pool
    fn get_conn(&self) -> Result<PooledConn, StorageError> {
        self.db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// Run the HTTP server
    pub async fn run(self: Arc<Self>) -> Result<(), StorageError> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!(
            addr = %self.bind_addr,
            max_concurrent = MAX_CONCURRENT_REQUESTS,
            "HTTP server listening"
        );

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let server = self.clone();
            let semaphore = self.request_semaphore.clone();

            tokio::spawn(async move {
                // Acquire a permit before processing — back-pressures under burst
                // traffic (e.g., 30+ concurrent HTML5 app asset requests).
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return, // semaphore closed — shutting down
                };

                let service = service_fn(move |req| {
                    let server = server.clone();
                    async move { server.handle_request(req).await }
                });

                // Enable HTTP upgrades for WebSocket support
                // Without .with_upgrades(), WebSocket handshakes fail immediately
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    warn!(addr = %remote_addr, error = %err, "Connection error");
                }
            });
        }
    }

    /// Route requests to handlers
    async fn handle_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ApiBody>, hyper::Error> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        // Extract observation session ID before req is consumed -- used by middleware aspect.
        let obs_session_id: Option<String> = req
            .headers()
            .get("X-Observation-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Capture method string for middleware aspect before method is moved into the match.
        let method_str = method.to_string();

        debug!(method = %method, path = %path, "Incoming request");

        let result = match (method, path.as_str()) {
            // CORS preflight for all routes
            (Method::OPTIONS, _) => Ok(Self::cors_preflight()),

            // Health check (supports ?detail=error|warn|info|debug|trace)
            (Method::GET, "/health") => {
                let query = req.uri().query().unwrap_or("");
                self.handle_health(query).await
            }

            // Build/version info
            (Method::GET, "/version") => {
                let info = elohim_compute::BuildInfo::new("elohim-storage");
                let body = serde_json::to_string(&info)
                    .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }

            // Shard API
            (Method::PUT, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_put_shard(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_get_shard(hash).await
            }
            (Method::HEAD, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_head_shard(hash).await
            }

            // Blob API (convenience with auto-sharding)
            (Method::PUT, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                self.handle_put_blob(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                let agent_id = Self::extract_agent_id(&req);
                self.handle_get_blob(hash, agent_id.as_deref()).await
            }

            // Manifest API
            (Method::GET, p) if p.starts_with("/manifest/") => {
                let hash = p.strip_prefix("/manifest/").unwrap_or("");
                self.handle_get_manifest(hash).await
            }

            // WebSocket upgrade for progress streaming
            (Method::GET, "/import/progress") if progress_ws::is_websocket_upgrade(&req) => {
                if let Some(ref hub) = self.progress_hub {
                    match progress_ws::handle_progress_upgrade(req, Arc::clone(hub)).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            error!(error = %e, "WebSocket upgrade failed");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Full::new(Bytes::from("WebSocket upgrade failed")))
                                .unwrap())
                        }
                    }
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Progress hub not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Import API (forwarded from doorway)
            (_, p) if p.starts_with("/import/") => {
                if let Some(ref import_api) = self.import_api {
                    // Lazy reconnection: if HcClient is not connected, attempt to reconnect
                    // This handles the case where elohim-storage starts before the hApp is installed
                    {
                        let api = import_api.read().await;
                        if api.needs_reconnect() {
                            drop(api); // Release read lock before acquiring write lock
                            let mut api_write = import_api.write().await;
                            if api_write.needs_reconnect() {
                                // Double-check after acquiring write lock
                                info!("Import API: Attempting lazy reconnection to conductor...");
                                match api_write.connect_conductor().await {
                                    Ok(_) => info!("Import API: Lazy reconnection successful"),
                                    Err(e) => {
                                        warn!(error = %e, "Import API: Lazy reconnection failed")
                                    }
                                }
                            }
                        }
                    }

                    let api = import_api.read().await;
                    api.handle_request(req, &path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Import API not enabled. Set ENABLE_IMPORT_API=true"}"#,
                        )))
                        .unwrap())
                }
            }

            // P2P Status endpoint
            #[cfg(feature = "p2p")]
            (Method::GET, "/p2p/status") => self.handle_p2p_status().await,

            // Delivery peers — discovered peers with delivery capabilities
            // Used by frontend for multi-peer app delivery scoring
            #[cfg(feature = "p2p")]
            (Method::GET, "/api/v1/peers/delivery") => self.handle_delivery_peers().await,

            // Sync API: /sync/v1/{h_app_id}/docs[/{doc_id}[/heads|/changes]]
            (method, p) if p.starts_with("/sync/v1/") => {
                if let Some(ref sync_manager) = self.sync_manager {
                    self.handle_sync_request(req, method, &path, sync_manager.clone())
                        .await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Sync API not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // SSE event stream — must be matched before the /api/v1/ catch-all
            (Method::GET, "/api/v1/events") => {
                if let Some(ref services) = self.services {
                    let response = crate::sse::create_sse_stream(&services.events);
                    return Ok(response.map(Either::Right));
                } else {
                    Ok(response::service_unavailable("Event bus not available"))
                }
            }

            // Cache stream for projection warm-up (SSE)
            (Method::GET, "/api/v1/cache/stream") => {
                if let Some(ref pool) = self.db_pool {
                    let response = crate::cache_stream::create_cache_stream(
                        pool.clone(),
                        "lamad", // Default app context for cache stream
                    );
                    return Ok(response.map(Either::Right));
                } else {
                    Ok(response::service_unavailable("Database not available"))
                }
            }

            // Observation Session API -- must be matched before the /api/v1/ catch-all
            (method, p) if p.starts_with("/api/v1/observations") => {
                if let Some(ref pool) = self.db_pool {
                    let sub_path = p.strip_prefix("/api/v1/observations").unwrap_or("");
                    self.handle_observation_request(req, method, sub_path, pool.clone())
                        .await
                } else {
                    Ok(response::service_unavailable("Database not available"))
                }
            }

            // Enriched API: Business logic endpoints
            (method, p) if p.starts_with("/api/v1/") => {
                if let Some(ref pool) = self.db_pool {
                    crate::api::handle_api_request(
                        req,
                        method,
                        &path,
                        pool.clone(),
                        self.services.clone(),
                    )
                    .await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "API not available - database pool not configured"}"#,
                        )))
                        .unwrap())
                }
            }

            // Database API: Content, Paths, Stats
            (method, p) if p.starts_with("/db/") => {
                if self.db_pool.is_some() {
                    self.handle_db_request(req, method, &path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Delivery capability probe: HEAD /apps/{slug}/_capability
            // Lightweight probe for delivery negotiation — no body, just headers.
            // Reports whether extraction cache is warm for this app.
            (Method::HEAD, p) if p.starts_with("/apps/") && p.ends_with("/_capability") => {
                self.handle_app_capability(p).await
            }

            // HTML5 App serving: /apps/{slug}/{file_path}
            (Method::GET, p) if p.starts_with("/apps/") => {
                if self.db_pool.is_some() {
                    self.handle_app_request(&path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Session API: Local session management for Tauri native handoff
            (Method::GET, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_get_session(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::POST, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_create_session(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::DELETE, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_delete_session(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::GET, "/session/all") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_list_sessions(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::POST, "/session/intent") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_set_session_intent(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // EPR Head API: DAG-CBOR encoded three-pillar metadata
            (Method::PUT, p) if p.starts_with("/epr-head/") => {
                let id = p.strip_prefix("/epr-head/").unwrap_or("");
                self.handle_put_epr_head(req, id).await
            }
            (Method::GET, p) if p.starts_with("/epr-head/") => {
                let id = p.strip_prefix("/epr-head/").unwrap_or("");
                self.handle_get_epr_head(req, id).await
            }

            // IPFS block API: raw block retrieval by CID
            (Method::GET, p) if p.starts_with("/ipfs/") => {
                let cid_str = p.strip_prefix("/ipfs/").unwrap_or("");
                self.handle_get_ipfs_block(cid_str).await
            }
            (Method::HEAD, p) if p.starts_with("/ipfs/") => {
                let cid_str = p.strip_prefix("/ipfs/").unwrap_or("");
                self.handle_head_ipfs_block(cid_str).await
            }

            // DAG API: decoded IPLD operations
            (Method::GET, p) if p.starts_with("/dag/") && p.ends_with("/links") => {
                let cid_str = p
                    .strip_prefix("/dag/")
                    .and_then(|s| s.strip_suffix("/links"))
                    .unwrap_or("");
                self.handle_get_dag_links(cid_str).await
            }
            (Method::GET, p) if p.starts_with("/dag/") => {
                let cid_str = p.strip_prefix("/dag/").unwrap_or("");
                self.handle_get_dag(cid_str).await
            }

            // Account API: Import/Export account packages
            (Method::POST, "/account/import") => {
                if let Some(ref pool) = self.db_pool {
                    self.do_account_import(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::GET, p) if p.starts_with("/account/export/") => {
                let human_id = p.strip_prefix("/account/export/").unwrap_or("");
                if let Some(ref pool) = self.db_pool {
                    self.do_account_export(human_id, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // Route manifest — declares the API surface for doorway dynamic discovery
            (Method::GET, "/manifest") => self.handle_manifest().await,

            // Admin: extraction cache stats
            (Method::GET, "/admin/extraction-cache/stats") => {
                self.handle_extraction_cache_stats().await
            }

            // Admin: evict app from extraction cache
            (Method::POST, p) if p.starts_with("/admin/extraction-cache/evict/") => {
                let slug = p
                    .strip_prefix("/admin/extraction-cache/evict/")
                    .unwrap_or("");
                self.handle_extraction_cache_evict(slug).await
            }

            // Not found
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap()),
        };

        match result {
            Ok(mut response) => {
                // Observation middleware aspect: record non-2xx failures when session is active.
                if let Some(ref session_id) = obs_session_id {
                    self.maybe_observe_request(
                        session_id,
                        &method_str,
                        &path,
                        response.status().as_u16(),
                    );
                }

                // Add CORS headers to ALL responses (not just preflight)
                let headers = response.headers_mut();
                headers.insert(
                    "Access-Control-Allow-Origin",
                    hyper::header::HeaderValue::from_static("*"),
                );
                headers.insert(
                    "Access-Control-Allow-Methods",
                    hyper::header::HeaderValue::from_static(
                        "GET, PUT, POST, PATCH, DELETE, HEAD, OPTIONS",
                    ),
                );
                headers.insert(
                    "Access-Control-Allow-Headers",
                    hyper::header::HeaderValue::from_static(
                        "Content-Type, Authorization, X-Agent-Id, X-Schema-Version, X-Observation-Id",
                    ),
                );
                Ok(response.map(Either::Left))
            }
            Err(e) => {
                error!(error = %e, "Request error");
                Ok(Self::with_cors_headers(Response::builder())
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Either::Left(Full::new(Bytes::from(format!(
                        "Error: {}",
                        e
                    )))))
                    .unwrap())
            }
        }
    }

    /// Health check endpoint with tiered detail via `?detail=` query parameter.
    ///
    /// Detail levels (cumulative):
    /// - `error` / `warn`: status + build info only
    /// - `info` (default): adds blob stats and import status
    /// - `debug`: adds manifest count, concurrency limit, app index size
    /// - `trace`: adds semaphore permits, db pool, extraction cache status
    async fn handle_health(&self, query: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        let detail = query
            .split('&')
            .find_map(|p| p.strip_prefix("detail="))
            .and_then(|v| v.parse::<elohim_compute::DetailLevel>().ok())
            .unwrap_or_default();

        let build = elohim_compute::BuildInfo::new("elohim-storage");

        // error + warn level: always present
        let mut body = serde_json::json!({
            "status": "ok",
            "build": build,
        });

        // info level (default): basic operational data
        if detail >= elohim_compute::DetailLevel::Info {
            let stats = self.blob_store.stats().await?;
            body["blobs"] = serde_json::json!(stats.total_blobs);
            body["bytes"] = serde_json::json!(stats.total_bytes);
            body["importEnabled"] = serde_json::json!(self.import_api.is_some());
        }

        // debug level: resource details
        if detail >= elohim_compute::DetailLevel::Debug {
            body["manifests"] = serde_json::json!(self.manifests.read().await.len());
            body["concurrencyLimit"] = serde_json::json!(MAX_CONCURRENT_REQUESTS);
            body["slugIndex"] = serde_json::json!(self.slug_index.read().await.len());
        }

        // trace level: full internal state
        if detail >= elohim_compute::DetailLevel::Trace {
            body["semaphorePermits"] =
                serde_json::json!(self.request_semaphore.available_permits());
            body["dbPoolEnabled"] = serde_json::json!(self.db_pool.is_some());
            body["extractionCacheEnabled"] = serde_json::json!(self.extraction_cache.is_some());
        }

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// GET /manifest - Declare the route surface for doorway dynamic discovery
    ///
    /// Returns a `DoorwayRoutes` JSON payload listing all `/api/v1/*` and `/db/*`
    /// routes that doorway should proxy to clients, plus the blob proxy config.
    /// Infrastructure endpoints (/health, /shard/*, /sync/*, /import/*) are
    /// intentionally omitted — doorway proxies only API routes.
    async fn handle_manifest(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let routes = build_manifest();
        let body = serde_json::to_string(&routes)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize manifest: {}", e)))?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap())
    }

    /// GET /admin/extraction-cache/stats — Extraction cache statistics
    ///
    /// Returns the number of warm apps, total cached bytes, budget, and TTL.
    /// Used by delivery diagnostics to observe the cache layer without evicting.
    async fn handle_extraction_cache_stats(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = if let Some(ref cache) = self.extraction_cache {
            let stats = cache.stats().await;
            serde_json::json!({
                "warmApps": stats.cached_apps,
                "totalCachedBytes": stats.total_cached_bytes,
                "budgetBytes": stats.budget_bytes,
                "ttlSecs": stats.ttl_secs,
            })
        } else {
            serde_json::json!({ "error": "extraction cache not configured" })
        };
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// POST /admin/extraction-cache/evict/{slug} — Evict an app from the extraction cache
    ///
    /// Forces re-extraction on the next request for this app.
    /// Used by delivery diagnostics tests to peel back the extraction layer
    /// and reveal the blob decompression layer beneath.
    async fn handle_extraction_cache_evict(
        &self,
        slug: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if slug.is_empty() {
            let body = serde_json::json!({ "error": "missing slug" });
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(body.to_string())))
                .unwrap());
        }
        let body = if let Some(ref cache) = self.extraction_cache {
            cache.evict_app(slug).await.map_err(|e| {
                StorageError::Internal(format!("Extraction cache evict failed: {}", e))
            })?;
            info!(slug = %slug, "Extraction cache evicted by admin");
            serde_json::json!({ "slug": slug, "evicted": true })
        } else {
            serde_json::json!({ "error": "extraction cache not configured" })
        };
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// PUT /shard/{hash} - Store a shard
    async fn handle_put_shard(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Read body
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes();

        // Verify hash - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash
            .strip_prefix("sha256-")
            .unwrap_or(&computed_hash);
        let expected_hex = expected_hash
            .strip_prefix("sha256-")
            .unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Store shard
        let result = self.blob_store.store(&data).await?;

        info!(
            hash = %result.hash,
            size = result.size_bytes,
            existed = result.already_existed,
            "Stored shard"
        );

        let body = serde_json::json!({
            "hash": result.hash,
            "sizeBytes": result.size_bytes,
            "alreadyExisted": result.already_existed,
        });

        Ok(Response::builder()
            .status(if result.already_existed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            })
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// GET /shard/{hash} - Retrieve a shard
    async fn handle_get_shard(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing shard hash")))
                .unwrap());
        }

        match self.blob_store.get(hash).await {
            Ok(data) => {
                info!(hash = %hash, size = data.len(), "Serving shard");

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CONTENT_LENGTH, data.len())
                    .header(header::ETAG, format!("\"{}\"", hash))
                    .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                    .body(Full::new(Bytes::from(data)))
                    .unwrap())
            }
            Err(StorageError::NotFound(_)) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Shard not found")))
                .unwrap()),
            Err(e) => Err(e),
        }
    }

    /// HEAD /shard/{hash} - Check if shard exists
    async fn handle_head_shard(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        match self.blob_store.size(hash).await {
            Ok(size) => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, size)
                .header(header::ETAG, format!("\"{}\"", hash))
                .body(Full::new(Bytes::new()))
                .unwrap()),
            Err(_) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap()),
        }
    }

    /// PUT /blob/{hash} - Store blob with auto-sharding
    async fn handle_put_blob(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Get content type from header
        let mime_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let agent_id =
            Self::extract_agent_id(&req).unwrap_or_else(|| "did:elohim:storage".to_string());

        // Read body
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes().to_vec();

        // Verify hash if provided - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash
            .strip_prefix("sha256-")
            .unwrap_or(&computed_hash);
        let expected_hex = expected_hash
            .strip_prefix("sha256-")
            .unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Create shard encoder and generate manifest
        let encoder = ShardEncoder::new(crate::sharding::ShardConfig::default());
        let manifest = encoder.create_manifest(&data, &mime_type, "commons");

        // Store each shard
        for (i, shard_hash) in manifest.shard_hashes.iter().enumerate() {
            // For "none" encoding, the whole blob is one shard
            let shard_data = if manifest.encoding == "none" {
                data.clone()
            } else {
                // For chunked encoding, split the data
                let start = i * manifest.shard_size as usize;
                let end = ((i + 1) * manifest.shard_size as usize).min(data.len());
                data[start..end].to_vec()
            };

            // Verify shard hash matches
            let actual_hash = BlobStore::compute_hash(&shard_data);
            if actual_hash != *shard_hash {
                warn!(
                    expected = %shard_hash,
                    actual = %actual_hash,
                    index = i,
                    "Shard hash mismatch during blob storage"
                );
            }

            self.blob_store.store(&shard_data).await?;

            // Register with Node Registry if available
            if let Some(ref nr_api) = self.node_registry_api {
                let assignment = crate::node_registry_api::ShardAssignment {
                    assignment_hash: None,
                    content_hash: expected_hex.to_string(), // The full content hash
                    // Ideally, use the actual agent ID (e.g. from X-Agent-Id header) if provided;
                    // fallback to a generic placeholder or the configured connection's role for now.
                    custodian_did: agent_id.clone(),
                    shard_index: i as u32,
                    strategy: crate::node_registry_api::ShardingStrategy::Geographic,
                    status: crate::node_registry_api::ShardStatus::Active,
                    verified_at: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = nr_api.create_shard_assignment(assignment).await {
                    warn!(
                        error = %e,
                        shard_index = i,
                        "Failed to register shard assignment with Node Registry"
                    );
                } else {
                    info!(
                        shard_index = i,
                        content_hash = %expected_hex,
                        "Registered shard assignment with Node Registry"
                    );
                }
            }
        }

        // Store manifest
        self.manifests
            .write()
            .await
            .insert(manifest.blob_hash.clone(), manifest.clone());

        info!(
            blob_hash = %manifest.blob_hash,
            total_size = manifest.total_size,
            shards = manifest.shard_hashes.len(),
            encoding = %manifest.encoding,
            "Stored blob with manifest"
        );

        let body =
            serde_json::to_string(&manifest).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap())
    }

    /// Extract agent ID from request headers (set by doorway)
    fn extract_agent_id(req: &Request<Incoming>) -> Option<String> {
        req.headers()
            .get("x-agent-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    /// CORS preflight response for cross-origin requests
    fn cors_preflight() -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header(
                "Access-Control-Allow-Methods",
                "GET, PUT, POST, PATCH, DELETE, HEAD, OPTIONS",
            )
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization, X-Agent-Id, X-Schema-Version",
            )
            .header("Access-Control-Max-Age", "86400")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    /// Add CORS headers to a response
    fn with_cors_headers(
        builder: hyper::http::response::Builder,
    ) -> hyper::http::response::Builder {
        builder
            .header("Access-Control-Allow-Origin", "*")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
    }

    /// GET /blob/{hash} - Reassemble blob from shards
    /// Checks policy enforcement if agent_id is provided
    async fn handle_get_blob(
        &self,
        hash: &str,
        agent_id: Option<&str>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        // Parse content address (accepts CID, sha256-prefixed hash, or raw hex)
        let normalized_hash = match crate::blob_store::BlobStore::parse_content_address(hash) {
            Ok(h) => format!("sha256-{}", h),
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from(format!(
                        "Invalid content address: {}",
                        hash
                    ))))
                    .unwrap());
            }
        };
        let hash = normalized_hash.as_str();

        // Check policy enforcement if enabled and agent_id is provided
        if let (Some(ref enforcement), Some(agent)) = (&self.policy_enforcement, agent_id) {
            // Create content metadata for policy check
            // For now, we only have the hash - in future, we could look up metadata
            let content = ContentMetadata {
                hash: hash.to_string(),
                categories: Vec::new(), // TODO: Could be looked up from db_pool
                age_rating: None,
                reach_level: None,
            };

            match enforcement.can_serve(agent, &content) {
                Ok(PolicyDecision::Allow) => {
                    debug!(hash = %hash, agent = %agent, "Policy check passed");
                }
                Ok(PolicyDecision::Block { reason }) => {
                    warn!(hash = %hash, agent = %agent, reason = %reason, "Content blocked by policy");

                    // Log the policy event
                    let _ = enforcement.cache().log_event(
                        agent,
                        None,
                        &PolicyEvent {
                            event_type: PolicyEventType::BlockedContent,
                            details: reason.clone(),
                            content_hash: Some(hash.to_string()),
                            feature_name: None,
                        },
                        30, // Default retention days
                    );

                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "blocked", "reason": "{}"}}"#,
                            reason
                        ))))
                        .unwrap());
                }
                Err(e) => {
                    // Policy check failed, but we shouldn't block content - log and proceed
                    warn!(error = %e, "Policy check failed, allowing access");
                }
            }
        }

        // Get manifest
        let manifest = self.manifests.read().await.get(hash).cloned();
        let manifest = match manifest {
            Some(m) => m,
            None => {
                // Try direct blob lookup (for non-sharded blobs)
                match self.blob_store.get(hash).await {
                    Ok(data) => {
                        return Ok(Self::with_cors_headers(Response::builder())
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/octet-stream")
                            .header(header::CONTENT_LENGTH, data.len())
                            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                            .body(Full::new(Bytes::from(data)))
                            .unwrap());
                    }
                    Err(_) => {
                        return Ok(Self::with_cors_headers(Response::builder())
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::from("Blob not found")))
                            .unwrap());
                    }
                }
            }
        };

        // Reassemble from shards
        let mut data = Vec::with_capacity(manifest.total_size as usize);

        for shard_hash in &manifest.shard_hashes {
            let shard_data = self.blob_store.get(shard_hash).await?;
            data.extend_from_slice(&shard_data);
        }

        // Truncate to actual size (last shard may be padded)
        data.truncate(manifest.total_size as usize);

        info!(
            hash = %hash,
            size = data.len(),
            shards = manifest.shard_hashes.len(),
            "Serving reassembled blob"
        );

        Ok(Self::with_cors_headers(Response::builder())
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &manifest.mime_type)
            .header(header::CONTENT_LENGTH, data.len())
            .header(header::ETAG, format!("\"{}\"", hash))
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(Full::new(Bytes::from(data)))
            .unwrap())
    }

    /// GET /manifest/{hash} - Get shard manifest
    async fn handle_get_manifest(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        let manifest = self.manifests.read().await.get(hash).cloned();

        match manifest {
            Some(m) => {
                let body =
                    serde_json::to_string(&m).map_err(|e| StorageError::Internal(e.to_string()))?;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Manifest not found")))
                .unwrap()),
        }
    }

    /// Handle P2P status request
    #[cfg(feature = "p2p")]
    async fn handle_p2p_status(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        if let Some(ref handle) = self.p2p_handle {
            let status = handle.status();
            let json = serde_json::to_string(&status).map_err(|e| {
                StorageError::Internal(format!("Failed to serialize P2P status: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "P2P networking not enabled"}"#,
                )))
                .unwrap())
        }
    }

    /// Handle delivery peers request — returns discovered peers with capabilities
    #[cfg(feature = "p2p")]
    async fn handle_delivery_peers(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        if let Some(ref handle) = self.p2p_handle {
            let peers = handle.delivery_peers();
            let json = serde_json::to_string(&peers).map_err(|e| {
                StorageError::Internal(format!("Failed to serialize delivery peers: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        } else {
            // No P2P — return empty array (not an error)
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from("[]")))
                .unwrap())
        }
    }

    /// Handle sync API requests
    ///
    /// Routes:
    /// - GET /sync/v1/{h_app_id}/docs - List documents
    /// - GET /sync/v1/{h_app_id}/docs/{doc_id}/heads - Get document heads
    /// - GET /sync/v1/{h_app_id}/docs/{doc_id}/changes?have={heads} - Get changes since heads
    /// - POST /sync/v1/{h_app_id}/docs/{doc_id}/changes - Apply changes
    async fn handle_sync_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Parse path: /sync/v1/{h_app_id}/docs[/{doc_id}[/heads|/changes]]
        let parts: Vec<&str> = path.trim_start_matches("/sync/v1/").split('/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Missing h_app_id"}"#)))
                .unwrap());
        }

        let h_app_id = parts[0];

        // /sync/v1/{h_app_id}/docs
        if parts.len() == 2 && parts[1] == "docs" {
            return self
                .handle_sync_list_docs(method, h_app_id, &req, sync_manager)
                .await;
        }

        // /sync/v1/{h_app_id}/docs/{doc_id}
        if parts.len() == 3 && parts[1] == "docs" {
            let doc_id = parts[2];
            return self
                .handle_sync_doc(method, h_app_id, doc_id, req, sync_manager)
                .await;
        }

        // /sync/v1/{h_app_id}/docs/{doc_id}/{action}
        if parts.len() == 4 && parts[1] == "docs" {
            let doc_id = parts[2];
            let action = parts[3];

            return match action {
                "heads" => {
                    self.handle_sync_heads(method, h_app_id, doc_id, sync_manager)
                        .await
                }
                "changes" => {
                    self.handle_sync_changes(method, h_app_id, doc_id, req, sync_manager)
                        .await
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Unknown action: {}"}}"#,
                        action
                    ))))
                    .unwrap()),
            };
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Invalid sync path"}"#)))
            .unwrap())
    }

    /// GET /sync/v1/{h_app_id}/docs - List documents
    async fn handle_sync_list_docs(
        &self,
        method: Method,
        h_app_id: &str,
        req: &Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        // Parse query params: ?prefix=&offset=&limit=
        let query = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

        let prefix = params.get("prefix").map(|s| s.as_str());
        let offset: u32 = params
            .get("offset")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let limit: u32 = params
            .get("limit")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        match sync_manager
            .list_documents(h_app_id, prefix, offset, limit)
            .await
        {
            Ok((docs, total)) => {
                let documents: Vec<serde_json::Value> = docs
                    .into_iter()
                    .map(|d| {
                        serde_json::json!({
                            "docId": d.doc_id,
                            "docType": d.doc_type,
                            "changeCount": d.change_count,
                            "lastModified": d.last_modified,
                            "heads": d.heads,
                        })
                    })
                    .collect();

                let body = serde_json::json!({
                    "hAppId": h_app_id,
                    "documents": documents,
                    "total": total,
                    "offset": offset,
                    "limit": limit,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(h_app_id = %h_app_id, error = %e, "Failed to list documents");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                    .unwrap())
            }
        }
    }

    /// Handle document-level requests
    async fn handle_sync_doc(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        _req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // Return document info
                match sync_manager.get_heads(h_app_id, doc_id).await {
                    Ok(heads) => {
                        if heads.is_empty() {
                            return Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error": "Document not found: {}"}}"#,
                                    doc_id
                                ))))
                                .unwrap());
                        }

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "heads": heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get document");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// GET /sync/v1/{h_app_id}/docs/{doc_id}/heads - Get document heads
    async fn handle_sync_heads(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        match sync_manager.get_heads(h_app_id, doc_id).await {
            Ok(heads) => {
                let body = serde_json::json!({
                    "hAppId": h_app_id,
                    "docId": doc_id,
                    "heads": heads,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get heads");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                    .unwrap())
            }
        }
    }

    /// GET/POST /sync/v1/{h_app_id}/docs/{doc_id}/changes
    async fn handle_sync_changes(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // GET changes since given heads
                let query = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                // Parse have_heads from comma-separated list
                let have_heads: Vec<String> = params
                    .get("have")
                    .map(|s| s.split(',').map(|h| h.to_string()).collect())
                    .unwrap_or_default();

                match sync_manager
                    .get_changes_since(h_app_id, doc_id, &have_heads)
                    .await
                {
                    Ok((changes, new_heads)) => {
                        // Encode changes as base64 for JSON transport
                        let changes_b64: Vec<String> = changes
                            .iter()
                            .map(|c| {
                                base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    c,
                                )
                            })
                            .collect();

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "changes": changes_b64,
                            "newHeads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            Method::POST => {
                // Apply changes from client
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                // Parse JSON body: { "changes": ["base64..."] }
                let payload: serde_json::Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                let changes_b64 = payload["changes"]
                    .as_array()
                    .ok_or_else(|| StorageError::Internal("Missing 'changes' array".to_string()))?;

                // Decode base64 changes
                let changes: Vec<Vec<u8>> = changes_b64
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| {
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok()
                    })
                    .collect();

                if changes.is_empty() {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(r#"{"error": "No valid changes"}"#)))
                        .unwrap());
                }

                match sync_manager.apply_changes(h_app_id, doc_id, changes).await {
                    Ok(new_heads) => {
                        info!(h_app_id = %h_app_id, doc_id = %doc_id, heads = ?new_heads, "Applied changes via HTTP");

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "newHeads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to apply changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Database API handlers
    // =========================================================================

    /// Handle database API requests
    ///
    /// Routes:
    /// - GET /db/stats - Database statistics
    /// - GET /db/content - List content (with query params)
    /// - GET /db/content/{id} - Get content by ID
    /// - POST /db/content - Create single content
    /// - POST /db/content/bulk - Bulk create content
    /// - GET /db/relationships - List relationships (with query params)
    /// - GET /db/relationships/{id} - Get relationship by ID
    /// - POST /db/relationships - Create relationship
    /// - POST /db/relationships/bulk - Bulk create relationships
    /// - GET /db/relationships/graph/{content_id} - Get content graph
    /// - GET /db/knowledge-maps - List knowledge maps
    /// - GET /db/knowledge-maps/{id} - Get knowledge map by ID
    /// - POST /db/knowledge-maps - Create knowledge map
    /// - PUT /db/knowledge-maps/{id} - Update knowledge map
    /// - DELETE /db/knowledge-maps/{id} - Delete knowledge map
    ///
    /// Extract app context from path, supporting both:
    /// - New: /db/{h_app_id}/content/... -> AppContext(h_app_id)
    /// - Legacy: /db/content/... -> AppContext("lamad") for backwards compatibility
    fn extract_app_context(sub_path: &str) -> (db::AppContext, &str) {
        // Check if path starts with a known resource type (legacy route)
        let legacy_prefixes = ["content", "stats", "schema"];
        for prefix in &legacy_prefixes {
            if sub_path == *prefix || sub_path.starts_with(&format!("{}/", prefix)) {
                // Legacy route: default to 'lamad' for learning content
                return (db::AppContext::default_lamad(), sub_path);
            }
        }

        // New route: /db/{h_app_id}/...
        if let Some(slash_pos) = sub_path.find('/') {
            let h_app_id = &sub_path[..slash_pos];
            let resource_path = &sub_path[slash_pos + 1..];
            return (db::AppContext::new(h_app_id), resource_path);
        }

        // Just h_app_id with no resource (e.g., /db/lamad -> stats for that app)
        if !sub_path.is_empty() && !legacy_prefixes.contains(&sub_path) {
            return (db::AppContext::new(sub_path), "stats");
        }

        // Fallback to default
        (db::AppContext::default_lamad(), sub_path)
    }

    async fn handle_db_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Strip /db/ prefix
        let sub_path = path.strip_prefix("/db/").unwrap_or("");

        // Extract app context (supports both legacy and new routes)
        let (app_ctx, resource_path) = Self::extract_app_context(sub_path);
        debug!(h_app_id = %app_ctx.h_app_id, resource_path = %resource_path, "DB request routing");

        // Route to specific handlers (all use Diesel pool via self.get_conn())
        if resource_path == "stats" {
            return self.handle_db_stats(method, &app_ctx).await;
        }

        if resource_path == "schema" {
            if method != Method::GET {
                return Ok(response::method_not_allowed());
            }
            let deprecated: Vec<u32> = vec![];
            return Ok(response::ok(&serde_json::json!({
                "supportedVersions": SUPPORTED_SCHEMA_VERSIONS,
                "currentVersion": SUPPORTED_SCHEMA_VERSIONS.last().copied().unwrap_or(1),
                "deprecatedVersions": deprecated,
            })));
        }

        if resource_path == "content" {
            return self.handle_db_content_list(req, method).await;
        }

        if resource_path == "content/bulk" {
            return self.handle_db_content_bulk(req, method).await;
        }

        // Entity-nested schedule routes: /db/content/{cid}/schedule
        if let Some(rest) = resource_path.strip_prefix("content/") {
            if let Some(cid) = rest.strip_suffix("/schedule") {
                return self
                    .handle_content_schedule(req, method, cid, &app_ctx)
                    .await;
            }
        }

        if let Some(content_id) = resource_path.strip_prefix("content/") {
            return self.handle_db_content_by_id(req, method, content_id).await;
        }

        // Relationships routes
        if resource_path == "relationships" {
            return self.handle_db_relationships_list(req, method).await;
        }

        if resource_path == "relationships/bulk" {
            return self.handle_db_relationships_bulk(req, method).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/graph/") {
            return self.handle_db_content_graph(req, method, rel_id).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/") {
            return self.handle_db_relationship_by_id(req, method, rel_id).await;
        }

        // Knowledge maps routes
        if resource_path == "knowledge-maps" {
            return self.handle_db_knowledge_maps_list(req, method).await;
        }

        if let Some(map_id) = resource_path.strip_prefix("knowledge-maps/") {
            return self
                .handle_db_knowledge_map_by_id(req, method, map_id)
                .await;
        }

        // ============================================================================
        // Diesel-based entity routes (using db_pool)
        // ============================================================================

        // Human relationships routes (Diesel)
        if resource_path == "human-relationships" {
            return self
                .handle_human_relationships_list(req, method, &app_ctx)
                .await;
        }

        if let Some(rel_path) = resource_path.strip_prefix("human-relationships/") {
            // Check for action sub-paths first
            if let Some(rest) = rel_path.strip_suffix("/consent") {
                return self
                    .handle_human_relationship_consent(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = rel_path.strip_suffix("/custody") {
                return self
                    .handle_human_relationship_custody(req, method, rest, &app_ctx)
                    .await;
            }
            // Fall back to generic ID handler
            return self
                .handle_human_relationship_by_id(req, method, rel_path, &app_ctx)
                .await;
        }

        // Human directory route (Diesel)
        if resource_path == "humans" && method == Method::GET {
            return self.handle_list_humans(req, &app_ctx).await;
        }

        // Collective routes (Diesel)
        if resource_path == "collectives" {
            return self.handle_collectives_list(req, method, &app_ctx).await;
        }

        if let Some(coll_path) = resource_path.strip_prefix("collectives/") {
            // Check for participants sub-path
            if let Some(rest) = coll_path.strip_suffix("/participants") {
                return self
                    .handle_collective_participants(req, method, rest, &app_ctx)
                    .await;
            }
            // Check for participants/{human_id} delete pattern
            if coll_path.contains("/participants/") {
                let parts: Vec<&str> = coll_path.splitn(3, '/').collect();
                if parts.len() == 3 && parts[1] == "participants" {
                    return self
                        .handle_collective_participant_depart(
                            req, method, parts[0], parts[2], &app_ctx,
                        )
                        .await;
                }
            }
            // Fall back to generic ID handler
            return self
                .handle_collective_by_id(req, method, coll_path, &app_ctx)
                .await;
        }

        // Participations by human route
        if let Some(human_id) = resource_path.strip_prefix("participations/") {
            return self
                .handle_participations_by_human(req, method, human_id, &app_ctx)
                .await;
        }

        // Contributor presences routes (Diesel)
        if resource_path == "presences" {
            return self.handle_presences_list(req, method, &app_ctx).await;
        }

        if resource_path == "presences/bulk" {
            return self.handle_presences_bulk(req, method, &app_ctx).await;
        }

        if let Some(presence_path) = resource_path.strip_prefix("presences/") {
            // Check for action sub-paths first
            if let Some(rest) = presence_path.strip_suffix("/stewardship") {
                return self
                    .handle_presence_stewardship(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = presence_path.strip_suffix("/claim") {
                return self
                    .handle_presence_claim(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = presence_path.strip_suffix("/verify-claim") {
                return self
                    .handle_presence_verify_claim(req, method, rest, &app_ctx)
                    .await;
            }
            // Fall back to generic ID handler
            return self
                .handle_presence_by_id(req, method, presence_path, &app_ctx)
                .await;
        }

        // Economic events routes (Diesel)
        if resource_path == "events" {
            return self.handle_events_list(req, method, &app_ctx).await;
        }

        if resource_path == "events/bulk" {
            return self.handle_events_bulk(req, method, &app_ctx).await;
        }

        if let Some(event_id) = resource_path.strip_prefix("events/") {
            return self
                .handle_event_by_id(req, method, event_id, &app_ctx)
                .await;
        }

        // Content mastery routes (Diesel)
        if resource_path == "mastery" {
            return self.handle_mastery_list(req, method, &app_ctx).await;
        }

        if resource_path == "mastery/bulk" {
            return self.handle_mastery_bulk(req, method, &app_ctx).await;
        }

        if let Some(mastery_path) = resource_path.strip_prefix("mastery/") {
            // Support /mastery/human/{human_id} and /mastery/{id}
            if let Some(human_id) = mastery_path.strip_prefix("human/") {
                return self
                    .handle_mastery_for_human(req, method, human_id, &app_ctx)
                    .await;
            }
            return self
                .handle_mastery_by_id(req, method, mastery_path, &app_ctx)
                .await;
        }

        // Stewardship allocations routes (Diesel)
        if resource_path == "allocations" {
            return self.handle_allocations_list(req, method, &app_ctx).await;
        }

        if resource_path == "allocations/bulk" {
            return self.handle_allocations_bulk(req, method, &app_ctx).await;
        }

        if let Some(alloc_path) = resource_path.strip_prefix("allocations/") {
            // Support /allocations/content/{content_id} and /allocations/steward/{steward_id}
            if let Some(content_id) = alloc_path.strip_prefix("content/") {
                return self
                    .handle_allocations_for_content(req, method, content_id, &app_ctx)
                    .await;
            }
            if let Some(steward_id) = alloc_path.strip_prefix("steward/") {
                return self
                    .handle_allocations_for_steward(req, method, steward_id, &app_ctx)
                    .await;
            }
            // Check for action sub-paths
            if let Some(rest) = alloc_path.strip_suffix("/dispute") {
                return self
                    .handle_allocation_dispute(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = alloc_path.strip_suffix("/resolve") {
                return self
                    .handle_allocation_resolve(req, method, rest, &app_ctx)
                    .await;
            }
            return self
                .handle_allocation_by_id(req, method, alloc_path, &app_ctx)
                .await;
        }

        // Stewarded node routes (Diesel)
        if resource_path == "nodes" {
            return self.handle_nodes_list(req, method, &app_ctx).await;
        }

        if let Some(node_path) = resource_path.strip_prefix("nodes/") {
            // /db/nodes/{id}/stewardship
            if let Some(node_id) = node_path.strip_suffix("/stewardship") {
                return self
                    .handle_node_stewardship(req, method, node_id, &app_ctx)
                    .await;
            }
            // /db/nodes/{id}
            return self
                .handle_node_by_id(req, method, node_path, &app_ctx)
                .await;
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "Unknown database endpoint"}"#,
            )))
            .unwrap())
    }

    /// GET /db/stats - Database statistics
    async fn handle_db_stats(
        &self,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let scoped = db::AppScopedDb::new(pool.clone(), &app_ctx.h_app_id);
        Ok(response::from_result(scoped.stats()))
    }

    /// GET /db/content - List content, POST /db/content - Create content
    async fn handle_db_content_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        // Parse query params via serde — ContentQuery has #[serde(rename_all = "camelCase")]
        // so the compiler enforces camelCase param names (contentType, contentFormat, etc.)
        let query_str = req.uri().query().unwrap_or("");
        let query: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => match services.content.list(&query) {
                Ok(items) => {
                    // Reach-based filtering: unauthenticated requests only see commons/public
                    let has_auth = req.headers().get(header::AUTHORIZATION).is_some()
                        || req.headers().get("X-Agent-Id").is_some();
                    let views: Vec<ContentView> = items
                        .into_iter()
                        .map(Into::into)
                        .filter(|v: &ContentView| {
                            has_auth || v.reach == "commons" || v.reach == "public"
                        })
                        .collect();
                    let body = serde_json::json!({
                        "items": views,
                        "count": views.len(),
                        "limit": query.limit,
                        "offset": query.offset,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                let input_view: CreateContentInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                // Capture EPR-relevant data before consuming input_view
                #[cfg(feature = "p2p")]
                let epr_data = (
                    input_view.id.clone(),
                    input_view.title.clone(),
                    input_view
                        .content_type
                        .clone()
                        .unwrap_or_else(|| "concept".to_string()),
                    input_view.description.clone(),
                    input_view.content_format.clone(),
                    input_view.blob_cid.clone(),
                    input_view.reach.clone(),
                    input_view.created_by.clone(),
                    input_view.tags.clone(),
                );

                // Capture manifest-relevant data before consuming input_view
                let manifest_data = (
                    input_view.id.clone(),
                    input_view.blob_hash.clone(),
                    input_view.blob_cid.clone(),
                    input_view.content_format.clone().unwrap_or_default(),
                    input_view
                        .reach
                        .clone()
                        .unwrap_or_else(|| "commons".to_string()),
                );

                let input: db::content_diesel::CreateContentInput = input_view.into();
                let result = services.content.create(input);

                // Auto-publish EPR Head on successful create
                #[cfg(feature = "p2p")]
                if result.is_ok() {
                    if let Some(ref handle) = self.p2p_handle {
                        let handle = handle.clone();
                        let (id, title, content_type, desc, fmt, cid, reach, author, tags) =
                            epr_data;
                        tokio::spawn(async move {
                            let head = crate::epr_codec::EprHead {
                                version: 1,
                                id: id.clone(),
                                content: cid.unwrap_or_default(),
                                lamad: crate::epr_codec::EprLamadContext {
                                    title,
                                    content_type,
                                    description: desc,
                                    content_format: fmt,
                                    tags,
                                },
                                shefa: crate::epr_codec::EprShefaContext {
                                    stewards: vec![],
                                    allocations: vec![],
                                },
                                qahal: crate::epr_codec::EprQahalContext {
                                    reach,
                                    layer: None,
                                    attestation_requirements: vec![],
                                },
                                relationships: vec![],
                                author,
                                updated: Some(chrono::Utc::now().to_rfc3339()),
                            };
                            if let Ok(bytes) = rmp_serde::to_vec(&head) {
                                handle.publish_epr_head(id, bytes).await;
                            }
                        });
                    }
                }

                // Capture distribution data before manifest recording consumes manifest_data
                #[cfg(feature = "p2p")]
                let distribution_data = (manifest_data.0.clone(), manifest_data.1.clone());

                // Record shard manifest if content has a blob
                if result.is_ok() && manifest_data.1.is_some() {
                    if let Some(ref pool) = self.db_pool {
                        let blob_store = self.blob_store.clone();
                        let pool = pool.clone();
                        let (content_id, blob_hash, blob_cid, content_format, reach) =
                            manifest_data;
                        let blob_hash = blob_hash.unwrap(); // Safe: checked above
                        tokio::spawn(async move {
                            if let Ok(data) = blob_store.get(&blob_hash).await {
                                let encoder = crate::sharding::ShardEncoder::new(
                                    crate::sharding::ShardConfig::default(),
                                );
                                let manifest =
                                    encoder.create_manifest(&data, &content_format, &reach);
                                let shard_hashes_json =
                                    serde_json::to_string(&manifest.shard_hashes)
                                        .unwrap_or_else(|_| "[]".to_string());
                                if let Ok(mut conn) = pool.get() {
                                    let new_manifest = crate::db::models::NewShardManifest {
                                        content_id: &content_id,
                                        h_app_id: "lamad",
                                        blob_hash: &blob_hash,
                                        blob_cid: blob_cid.as_deref(),
                                        encoding: &manifest.encoding,
                                        data_shard_count: manifest.data_shards as i32,
                                        parity_shard_count: (manifest.total_shards
                                            - manifest.data_shards)
                                            as i32,
                                        shard_hashes_json: &shard_hashes_json,
                                        total_size_bytes: manifest.total_size as i64,
                                        shard_size_bytes: manifest.shard_size as i64,
                                        mime_type: &manifest.mime_type,
                                        reach: &reach,
                                    };
                                    if let Err(e) = crate::db::shard_manifests::upsert_manifest(
                                        &mut conn,
                                        &new_manifest,
                                    ) {
                                        tracing::warn!(
                                            content_id = %content_id,
                                            error = %e,
                                            "Failed to record shard manifest"
                                        );
                                    } else {
                                        tracing::debug!(
                                            content_id = %content_id,
                                            encoding = %manifest.encoding,
                                            "Recorded shard manifest"
                                        );
                                    }
                                }
                            }
                        });
                    }
                }

                // Auto-distribute shards to peers
                #[cfg(feature = "p2p")]
                if result.is_ok() {
                    if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.db_pool) {
                        if let (ref content_id, Some(ref blob_hash)) = distribution_data {
                            let handle = handle.clone();
                            let pool = pool.clone();
                            let content_id = content_id.clone();
                            let blob_store = self.blob_store.clone();
                            let blob_hash = blob_hash.clone();
                            tokio::spawn(async move {
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    match handle
                                        .distribute_shards(&content_id, &data, &pool, "lamad")
                                        .await
                                    {
                                        Ok(n) => tracing::info!(
                                            content_id = %content_id,
                                            shards = n,
                                            "Shard distribution complete"
                                        ),
                                        Err(e) => tracing::warn!(
                                            content_id = %content_id,
                                            error = %e,
                                            "Shard distribution failed"
                                        ),
                                    }
                                }
                            });
                        }
                    }
                }

                Ok(response::from_create_result(result))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/content/bulk - Bulk create content
    async fn handle_db_content_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let input_views: Vec<CreateContentInputView> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        // Capture EPR-relevant data before input_views are consumed
        #[cfg(feature = "p2p")]
        let epr_inputs: Vec<_> = input_views
            .iter()
            .map(|v| {
                (
                    v.id.clone(),
                    v.title.clone(),
                    v.content_type
                        .clone()
                        .unwrap_or_else(|| "concept".to_string()),
                    v.description.clone(),
                    v.content_format.clone(),
                    v.blob_cid.clone(),
                    v.reach.clone(),
                    v.created_by.clone(),
                    v.tags.clone(),
                )
            })
            .collect();

        // Capture manifest-relevant data before input_views are consumed
        let manifest_inputs: Vec<_> = input_views
            .iter()
            .map(|v| {
                (
                    v.id.clone(),
                    v.blob_hash.clone(),
                    v.blob_cid.clone(),
                    v.content_format.clone().unwrap_or_default(),
                    v.reach.clone().unwrap_or_else(|| "commons".to_string()),
                )
            })
            .collect();

        let items: Vec<db::content_diesel::CreateContentInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        let count = items.len();
        info!(count = count, "Bulk creating content");

        match services.content.bulk_create(items) {
            Ok(result) => {
                // Auto-publish EPR Heads to DHT for cross-peer discovery
                #[cfg(feature = "p2p")]
                if let Some(ref handle) = self.p2p_handle {
                    let handle = handle.clone();
                    let inserted = result.inserted;
                    if inserted > 0 {
                        let epr_data = epr_inputs;
                        tokio::spawn(async move {
                            for (id, title, content_type, desc, fmt, cid, reach, author, tags) in
                                epr_data
                            {
                                let head = crate::epr_codec::EprHead {
                                    version: 1,
                                    id: id.clone(),
                                    content: cid.unwrap_or_default(),
                                    lamad: crate::epr_codec::EprLamadContext {
                                        title,
                                        content_type,
                                        description: desc,
                                        content_format: fmt,
                                        tags,
                                    },
                                    shefa: crate::epr_codec::EprShefaContext {
                                        stewards: vec![],
                                        allocations: vec![],
                                    },
                                    qahal: crate::epr_codec::EprQahalContext {
                                        reach,
                                        layer: None,
                                        attestation_requirements: vec![],
                                    },
                                    relationships: vec![],
                                    author,
                                    updated: Some(chrono::Utc::now().to_rfc3339()),
                                };
                                if let Ok(bytes) = rmp_serde::to_vec(&head) {
                                    handle.publish_epr_head(id, bytes).await;
                                }
                            }
                            info!(count = inserted, "Published EPR Heads to DHT");
                        });
                    }
                }

                // Capture distribution data before manifest_inputs is consumed
                #[cfg(feature = "p2p")]
                let distribution_items: Vec<(String, String)> = manifest_inputs
                    .iter()
                    .filter_map(|(id, bh, _, _, _)| bh.as_ref().map(|h| (id.clone(), h.clone())))
                    .collect();

                // Record shard manifests for items with blobs
                if let Some(ref pool) = self.db_pool {
                    let blob_store = self.blob_store.clone();
                    let pool = pool.clone();
                    let items_with_blobs: Vec<_> = manifest_inputs
                        .into_iter()
                        .filter(|(_, blob_hash, _, _, _)| blob_hash.is_some())
                        .collect();
                    if !items_with_blobs.is_empty() {
                        tokio::spawn(async move {
                            for (content_id, blob_hash, blob_cid, content_format, reach) in
                                items_with_blobs
                            {
                                let blob_hash = blob_hash.unwrap(); // Safe: filtered above
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    let encoder = crate::sharding::ShardEncoder::new(
                                        crate::sharding::ShardConfig::default(),
                                    );
                                    let manifest =
                                        encoder.create_manifest(&data, &content_format, &reach);
                                    let shard_hashes_json =
                                        serde_json::to_string(&manifest.shard_hashes)
                                            .unwrap_or_else(|_| "[]".to_string());
                                    if let Ok(mut conn) = pool.get() {
                                        let new_manifest = crate::db::models::NewShardManifest {
                                            content_id: &content_id,
                                            h_app_id: "lamad",
                                            blob_hash: &blob_hash,
                                            blob_cid: blob_cid.as_deref(),
                                            encoding: &manifest.encoding,
                                            data_shard_count: manifest.data_shards as i32,
                                            parity_shard_count: (manifest.total_shards
                                                - manifest.data_shards)
                                                as i32,
                                            shard_hashes_json: &shard_hashes_json,
                                            total_size_bytes: manifest.total_size as i64,
                                            shard_size_bytes: manifest.shard_size as i64,
                                            mime_type: &manifest.mime_type,
                                            reach: &reach,
                                        };
                                        if let Err(e) = crate::db::shard_manifests::upsert_manifest(
                                            &mut conn,
                                            &new_manifest,
                                        ) {
                                            tracing::warn!(
                                                content_id = %content_id,
                                                error = %e,
                                                "Failed to record shard manifest"
                                            );
                                        }
                                    }
                                }
                            }
                            tracing::debug!("Bulk shard manifest recording complete");
                        });
                    }
                }

                // Auto-distribute shards to peers
                #[cfg(feature = "p2p")]
                if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.db_pool) {
                    if !distribution_items.is_empty() {
                        let handle = handle.clone();
                        let pool = pool.clone();
                        let blob_store = self.blob_store.clone();
                        tokio::spawn(async move {
                            for (content_id, blob_hash) in distribution_items {
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    let _ = handle
                                        .distribute_shards(&content_id, &data, &pool, "lamad")
                                        .await;
                                }
                            }
                            tracing::info!("Bulk shard distribution complete");
                        });
                    }
                }

                Ok(response::ok_with_schema_info(&result))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/content/{cid}/schedule - Entity-nested schedule convenience
    async fn handle_content_schedule(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match db::schedules::get_schedule(&mut conn, app_ctx, "content", content_id) {
                    Ok(schedule) => Ok(response::ok(&ScheduleView::from(schedule))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                use http_body_util::BodyExt;
                let body_bytes = req
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input_view: CreateScheduleInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                // Override entity_type/entity_id from URL path
                let input = db::schedules::CreateScheduleInput {
                    entity_type: "content".to_string(),
                    entity_id: content_id.to_string(),
                    scheduled_at: input_view.scheduled_at,
                    expires_at: input_view.expires_at,
                    rrule: input_view.rrule,
                };

                match db::schedules::create_schedule(&mut conn, app_ctx, input) {
                    Ok(schedule) => Ok(response::created(&ScheduleView::from(schedule))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/content/{id} - Get or delete content by ID
    async fn handle_db_content_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                let result = services
                    .content
                    .get(content_id)
                    .map(|opt| opt.map(ContentView::from));

                // Layer 1: Reach-based access control
                // commons/public serve without auth, restricted content requires authentication
                if let Ok(Some(ref view)) = result {
                    let is_public = view.reach == "commons" || view.reach == "public";
                    if !is_public {
                        let has_auth = req.headers().get(header::AUTHORIZATION).is_some()
                            || req.headers().get("X-Agent-Id").is_some();
                        if !has_auth {
                            return Ok(Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error":"Authentication required","requiredReach":"{}"}}"#,
                                    view.reach
                                ))))
                                .unwrap());
                        }
                    }

                    // Policy enforcement: check device policy ceiling
                    let agent_id = Self::extract_agent_id(&req);
                    if let (Some(ref enforcement), Some(ref agent)) =
                        (&self.policy_enforcement, &agent_id)
                    {
                        let reach_level_num = match view.reach.as_str() {
                            "commons" | "public" => 0u8,
                            "community" => 1,
                            "familiar" => 2,
                            "trusted" => 3,
                            "intimate" => 4,
                            "self" | "private" => 5,
                            _ => 0,
                        };
                        let content_meta = ContentMetadata {
                            hash: content_id.to_string(),
                            categories: Vec::new(),
                            age_rating: None,
                            reach_level: Some(reach_level_num),
                        };
                        match enforcement.can_serve(agent, &content_meta) {
                            Ok(PolicyDecision::Block { reason }) => {
                                return Ok(Response::builder()
                                    .status(StatusCode::FORBIDDEN)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(
                                        r#"{{"error":"Policy blocked","reason":"{}"}}"#,
                                        reason
                                    ))))
                                    .unwrap());
                            }
                            Ok(PolicyDecision::Allow) => {}
                            Err(_) => {} // Policy lookup failure is non-blocking
                        }
                    }

                    // Layer 2: Attestation gate — prerequisite mastery check
                    // EPR Heads (discovery) flow freely; body access may require attestations
                    if let Some(ref agent) = Self::extract_agent_id(&req) {
                        if let Ok(mut att_conn) = self.get_conn() {
                            let attestations =
                                crate::db::content_attestations::query_attestations_for_content(
                                    &mut att_conn,
                                    content_id,
                                );
                            if let Ok(atts) = attestations {
                                let prereq_atts: Vec<_> = atts
                                    .iter()
                                    .filter(|a| {
                                        a.attestation_type == "prerequisite-mastery"
                                            && a.is_revoked == 0
                                    })
                                    .collect();

                                if !prereq_atts.is_empty() {
                                    // Requester must have mastery of the prerequisite content
                                    let app_ctx = db::AppContext::default_lamad();
                                    let human = crate::db::humans::get_human_by_agent_key(
                                        &mut att_conn,
                                        agent,
                                    );
                                    if let Ok(Some(human)) = human {
                                        let mut has_all_prereqs = true;
                                        for prereq in &prereq_atts {
                                            // The prereq's content_id is the prerequisite content
                                            // evidence field stores the prerequisite content ID
                                            let prereq_content_id = prereq
                                                .evidence
                                                .as_deref()
                                                .unwrap_or(&prereq.content_id);
                                            let mastery =
                                                crate::db::content_mastery::get_mastery_for_content(
                                                    &mut att_conn,
                                                    &app_ctx,
                                                    &human.id,
                                                    prereq_content_id,
                                                );
                                            match mastery {
                                                Ok(Some(m)) if m.mastery_level != "not_started" => {
                                                }
                                                _ => {
                                                    has_all_prereqs = false;
                                                    break;
                                                }
                                            }
                                        }
                                        if !has_all_prereqs {
                                            return Ok(Response::builder()
                                                .status(StatusCode::FORBIDDEN)
                                                .header(
                                                    header::CONTENT_TYPE,
                                                    "application/json",
                                                )
                                                .body(Full::new(Bytes::from(format!(
                                                    r#"{{"error":"Prerequisite mastery required","contentId":"{}"}}"#,
                                                    content_id
                                                ))))
                                                .unwrap());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If content found locally or DB error, return immediately
                if !matches!(&result, Ok(None)) {
                    return Ok(response::from_option(
                        result,
                        &format!("Content not found: {}", content_id),
                    ));
                }

                // Content not found locally — try P2P EPR resolution + shard fetch
                #[cfg(feature = "p2p")]
                if let Some(ref handle) = self.p2p_handle {
                    debug!(id = %content_id, "Content not found locally, trying P2P resolve + fetch");
                    if let Some((head, content_bytes)) = handle.resolve_and_fetch(content_id).await
                    {
                        info!(id = %content_id, size = content_bytes.len(), "Content resolved via P2P");

                        // Store blob
                        let blob_result = self.blob_store.store(&content_bytes).await;

                        // Persist to local SQLite so future GETs are local
                        if let Some(ref svc) = self.services {
                            let body_str = String::from_utf8_lossy(&content_bytes).to_string();
                            let input = db::content_diesel::CreateContentInput {
                                id: content_id.to_string(),
                                title: head.lamad.title.clone(),
                                description: head.lamad.description.clone(),
                                content_type: head.lamad.content_type.clone(),
                                content_format: head
                                    .lamad
                                    .content_format
                                    .clone()
                                    .unwrap_or_else(|| "markdown".to_string()),
                                blob_hash: blob_result.as_ref().ok().map(|r| r.hash.clone()),
                                blob_cid: if head.content.is_empty() {
                                    None
                                } else {
                                    Some(head.content.clone())
                                },
                                content_size_bytes: Some(content_bytes.len() as i32),
                                metadata_json: Some(r#"{"resolved_via":"p2p"}"#.to_string()),
                                reach: head
                                    .qahal
                                    .reach
                                    .clone()
                                    .unwrap_or_else(|| "commons".to_string()),
                                created_by: head.author.clone(),
                                tags: head.lamad.tags.clone(),
                                content_body: Some(body_str),
                            };
                            match svc.content.create(input) {
                                Ok(content_with_tags) => {
                                    info!(id = %content_id, "P2P content persisted to local SQLite");

                                    // Distribute recognition through the pipeline (fire-and-forget)
                                    // Pipeline handles: normalize → resolve stewards → weight by affinity → limit → settle
                                    if let Ok(mut recog_conn) = self.get_conn() {
                                        let recog_ctx = db::AppContext::default_lamad();
                                        let trigger = crate::services::recognition_pipeline_service::RecognitionTrigger {
                                            content_id: content_id.to_string(),
                                            event_type: crate::db::models::lamad_event_types::CONTENT_DELIVERY.to_string(),
                                            raw_amount: 1.0,
                                            triggered_by: Some("p2p-epr-resolution".to_string()),
                                        };
                                        match crate::services::recognition_pipeline_service::distribute(
                                            &mut recog_conn,
                                            &recog_ctx,
                                            trigger,
                                        ) {
                                            Ok(result) => {
                                                debug!(
                                                    id = %content_id,
                                                    stewards = result.distributions.len(),
                                                    events = result.economic_event_ids.len(),
                                                    "Recognition distributed via pipeline"
                                                );
                                            }
                                            Err(e) => {
                                                debug!(id = %content_id, error = %e, "Recognition pipeline failed (non-fatal)");
                                            }
                                        }
                                    }

                                    let view = ContentView::from(content_with_tags);
                                    return Ok(response::ok(&view));
                                }
                                Err(e) => {
                                    warn!(id = %content_id, error = %e, "Failed to persist P2P content");
                                    let view = ContentView::from_epr_head(&head);
                                    return Ok(response::ok(&view));
                                }
                            }
                        }
                    }
                }

                Ok(response::from_option(
                    result,
                    &format!("Content not found: {}", content_id),
                ))
            }
            Method::PATCH => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                let view: UpdateContentInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                Ok(response::from_result(
                    services
                        .content
                        .update(content_id, view)
                        .map(ContentWithTagsView::from),
                ))
            }
            Method::DELETE => {
                let result = services.content.delete_cascade(content_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Content not found: {}", content_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Relationship handlers
    // =========================================================================

    /// GET /db/relationships - List relationships, POST - Create relationship
    async fn handle_db_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let query: db::relationships_diesel::RelationshipQuery =
            serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => match services.relationship.list(&query) {
                Ok(items) => {
                    let views: Vec<RelationshipView> =
                        items.into_iter().map(|r| r.into()).collect();
                    let body = serde_json::json!({
                        "items": views,
                        "count": views.len(),
                        "limit": query.limit,
                        "offset": query.offset,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input_view: CreateRelationshipInputView =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: db::relationships_diesel::CreateRelationshipInput = input_view.into();
                Ok(response::from_create_result(
                    services.relationship.create(input),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/relationships/bulk - Bulk create relationships
    async fn handle_db_relationships_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let input_views: Vec<CreateRelationshipInputView> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<db::relationships_diesel::CreateRelationshipInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match services.relationship.bulk_create(inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/relationships/graph/{content_id} - Get content graph
    async fn handle_db_content_graph(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect();

        let relationship_types: Option<Vec<String>> = params
            .get("types")
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

        Ok(response::from_result(
            services
                .relationship
                .get_graph(content_id, relationship_types.as_deref()),
        ))
    }

    /// GET/DELETE /db/relationships/{id} - Get or delete relationship by ID
    async fn handle_db_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        rel_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                let result = services.relationship.get(rel_id);
                Ok(response::from_option(
                    result,
                    &format!("Relationship not found: {}", rel_id),
                ))
            }
            Method::DELETE => {
                let result = services.relationship.delete(rel_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Relationship not found: {}", rel_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Knowledge Map handlers
    // =========================================================================

    /// GET /db/knowledge-maps - List knowledge maps, POST - Create knowledge map
    async fn handle_db_knowledge_maps_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let query: db::knowledge_maps_diesel::KnowledgeMapQuery =
            serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => match services.knowledge.list_knowledge_maps(&query) {
                Ok(items) => {
                    let body = serde_json::json!({
                        "items": items,
                        "count": items.len(),
                        "limit": query.limit,
                        "offset": query.offset,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input: db::knowledge_maps_diesel::CreateKnowledgeMapInput =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                Ok(response::from_create_result(
                    services.knowledge.create_knowledge_map(input),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/PUT/DELETE /db/knowledge-maps/{id} - Knowledge map by ID
    async fn handle_db_knowledge_map_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        map_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                let result = services.knowledge.get_knowledge_map(map_id);
                Ok(response::from_option(
                    result,
                    &format!("Knowledge map not found: {}", map_id),
                ))
            }
            Method::PUT => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input: db::knowledge_maps_diesel::CreateKnowledgeMapInput =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                Ok(response::from_result(
                    services.knowledge.update_knowledge_map(map_id, input),
                ))
            }
            Method::DELETE => {
                let result = services.knowledge.delete_knowledge_map(map_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Knowledge map not found: {}", map_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // HTML5 App serving handlers
    // =========================================================================

    /// Handle delivery capability probe for an HTML5 app.
    ///
    /// Route: HEAD /apps/{identifier}/_capability
    ///
    /// The identifier can be either a slug (human-readable name) or a content
    /// address (sha256-...). Returns an empty body with headers describing the
    /// delivery readiness of this storage node for the given app. Used by
    /// service workers and doorway to negotiate the optimal delivery path
    /// (extracted vs compressed).
    async fn handle_app_capability(
        &self,
        path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let identifier = path
            .strip_prefix("/apps/")
            .and_then(|s| s.strip_suffix("/_capability"))
            .unwrap_or("");

        if identifier.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        // Resolve identifier: content address bypasses slug_index lookup
        let is_cid = is_content_address(identifier);
        let (resolved_slug, blob_hash) = if is_cid {
            (None, Some(identifier.to_string()))
        } else {
            let hash = self.slug_index.read().await.get(identifier).cloned();
            (Some(identifier.to_string()), hash)
        };

        // Cache key: use slug when available, otherwise the CID itself
        let cache_key = resolved_slug.as_deref().unwrap_or(identifier);

        // Check extraction cache warmth
        let (ready, delivery_mode) = match (&self.extraction_cache, &blob_hash) {
            (Some(cache), Some(hash)) => {
                let is_warm = cache.is_current(cache_key, hash).await;
                (is_warm, if is_warm { "extracted" } else { "compressed" })
            }
            _ => (false, "compressed"),
        };

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Length", "0")
            .header("X-Delivery-Mode", delivery_mode)
            .header("X-Cache-Tier", "extraction")
            .header("X-Ready", if ready { "true" } else { "false" });

        if let Some(hash) = &blob_hash {
            builder = builder
                .header("X-Blob-Hash", hash.as_str())
                .header("X-Content-Address", hash.as_str());
        }
        if let Some(ref slug) = resolved_slug {
            builder = builder.header("X-Content-Slug", slug.as_str());
        }

        Ok(builder.body(Full::new(Bytes::new())).unwrap())
    }

    /// Handle HTML5 app file requests
    ///
    /// Route: GET /apps/{identifier}/{file_path}
    ///
    /// The identifier can be either a slug (human-readable name) or a content
    /// address (sha256-...). Slug-based paths resolve via `slug_index`; content
    /// address paths use the hash directly as the blob key.
    ///
    /// Fast path (cache hit): O(1) index lookup + disk read. No DB, no ZIP, no pool.
    /// Slow path (cache miss): DB query + ZIP extract + cache all files + serve.
    async fn handle_app_request(&self, path: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        use std::io::Read;
        use zip::ZipArchive;

        // Parse path: /apps/{identifier}/{file_path}
        let remainder = path.strip_prefix("/apps/").unwrap_or("");
        let (identifier, file_path) = match remainder.find('/') {
            Some(pos) => (&remainder[..pos], &remainder[pos + 1..]),
            None => (remainder, "index.html"),
        };

        if identifier.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Missing app identifier"}"#,
                )))
                .unwrap());
        }

        if file_path.contains("..") || file_path.contains('\0') || file_path.starts_with('/') {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Invalid file path"}"#)))
                .unwrap());
        }

        // Resolve identifier: content address bypasses slug_index lookup
        let is_cid = is_content_address(identifier);
        let (resolved_slug, cached_blob_hash) = if is_cid {
            (None, Some(identifier.to_string()))
        } else {
            let hash = {
                let index = self.slug_index.read().await;
                index.get(identifier).cloned()
            };
            (Some(identifier.to_string()), hash)
        };

        // Cache key: use slug when available, otherwise the CID itself
        let cache_key = resolved_slug.as_deref().unwrap_or(identifier);

        debug!(identifier = %identifier, file_path = %file_path, is_cid = %is_cid, "App file request");

        // --- Fast path: check extraction cache ---
        if let (Some(ref cache), Some(ref hash)) = (&self.extraction_cache, &cached_blob_hash) {
            if cache.is_current(cache_key, hash).await {
                if let Some(data) = cache.get_file(cache_key, file_path).await {
                    let content_type = Self::get_mime_type(file_path);
                    debug!(identifier = %identifier, file_path = %file_path, "Cache HIT");
                    let mut builder = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, data.len())
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .header("X-Cache", "HIT")
                        .header("X-Content-Address", hash.as_str());
                    if let Some(ref slug) = resolved_slug {
                        builder = builder
                            .header("X-Slug", slug.as_str())
                            .header("X-Content-Slug", slug.as_str());
                    }
                    return Ok(builder.body(Full::new(Bytes::from(data))).unwrap());
                }
            }
        }

        // --- Slow path: DB lookup + ZIP extraction ---
        // Thundering herd protection: if another request is already extracting
        // this app, wait for it to finish then retry from cache.
        if let Some(ref cache) = self.extraction_cache {
            if let Some(mut rx) = cache.begin_extraction(cache_key) {
                // Another request is extracting — wait for it
                debug!(identifier = %identifier, "Waiting for in-flight extraction");
                let _ = rx.recv().await; // ignore errors — extractor may have finished
                                         // Retry from cache
                if let Some(data) = cache.get_file(cache_key, file_path).await {
                    let content_type = Self::get_mime_type(file_path);
                    debug!(identifier = %identifier, file_path = %file_path, "Cache HIT (after wait)");
                    let mut builder = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, data.len())
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .header("X-Cache", "HIT-COALESCED");
                    if let Some(ref hash) = cached_blob_hash {
                        builder = builder.header("X-Content-Address", hash.as_str());
                    }
                    if let Some(ref slug) = resolved_slug {
                        builder = builder
                            .header("X-Slug", slug.as_str())
                            .header("X-Content-Slug", slug.as_str());
                    }
                    return Ok(builder.body(Full::new(Bytes::from(data))).unwrap());
                }
                // Extraction failed or file not found — fall through to extract ourselves
            }
            // We're first — create drop guard that calls finish_extraction on ALL exit paths
            // (including early returns for 404, empty hash, corrupt ZIP, etc.)
        }
        // Guard lives until end of function — any return triggers finish_extraction
        let _extraction_guard = self
            .extraction_cache
            .as_ref()
            .map(|c| c.extraction_guard(cache_key));

        debug!(identifier = %identifier, "Cache MISS — extracting from ZIP");

        let blob_hash = match cached_blob_hash {
            Some(h) => h,
            None => {
                if is_cid {
                    // CID was provided but blob not found — nothing to resolve
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "App not found for content address: {}"}}"#,
                            identifier
                        ))))
                        .unwrap());
                }
                match self.lookup_slug_blob_hash(identifier).await? {
                    Some(h) => h,
                    None => {
                        return Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "App not found: {}"}}"#,
                                identifier
                            ))))
                            .unwrap());
                    }
                }
            }
        };

        if blob_hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "App ZIP not available (no blob_hash)"}"#,
                )))
                .unwrap());
        }

        debug!(identifier = %identifier, blob_hash = %blob_hash, "Found blob hash");

        // Fetch ZIP from blob store
        let zip_data = match self.blob_store.get(&blob_hash).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "App ZIP blob not found: {}"}}"#,
                        blob_hash
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        debug!(identifier = %identifier, zip_size = zip_data.len(), "Fetched ZIP blob");

        // Extract ALL files from ZIP
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Invalid ZIP archive");
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid ZIP archive: {}"}}"#,
                        e
                    ))))
                    .unwrap());
            }
        };

        let mut all_files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut requested_file_data: Option<Vec<u8>> = None;
        let normalized_path = file_path.trim_start_matches('/');

        for i in 0..archive.len() {
            if let Ok(mut f) = archive.by_index(i) {
                if f.is_dir() {
                    continue;
                }
                let name = f.name().to_string();
                let mut contents = Vec::new();
                if f.read_to_end(&mut contents).is_ok() {
                    // Check if this is the requested file (exact or suffix match)
                    if requested_file_data.is_none()
                        && (name == normalized_path
                            || name.ends_with(normalized_path)
                            || name.ends_with(&format!("/{}", normalized_path)))
                    {
                        requested_file_data = Some(contents.clone());
                    }
                    all_files.push((name, contents));
                }
            }
        }

        // Cache the extracted files (non-fatal if caching fails)
        if let Some(ref cache) = self.extraction_cache {
            if let Err(e) = cache.put_app(cache_key, &blob_hash, all_files).await {
                warn!(error = %e, identifier = %identifier, "Failed to cache extraction (non-fatal)");
            }
            // Guard drop handles finish_extraction — no explicit call needed
        }

        // Serve the requested file
        let contents = match requested_file_data {
            Some(data) => data,
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "File not found in app: {}"}}"#,
                        normalized_path
                    ))))
                    .unwrap());
            }
        };

        let content_type = Self::get_mime_type(file_path);

        info!(
            identifier = %identifier,
            file_path = %file_path,
            content_type = %content_type,
            size = contents.len(),
            "Serving app file (extracted + cached)"
        );

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, contents.len())
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .header("X-Cache", "MISS")
            .header("X-Content-Address", &blob_hash);
        if let Some(ref slug) = resolved_slug {
            builder = builder
                .header("X-Slug", slug.as_str())
                .header("X-Content-Slug", slug.as_str());
        }
        Ok(builder.body(Full::new(Bytes::from(contents))).unwrap())
    }

    /// Look up blob hash for an app by querying DB and updating slug_index.
    async fn lookup_slug_blob_hash(&self, slug: &str) -> Result<Option<String>, StorageError> {
        let mut conn = self.get_conn()?;
        let app_ctx = db::AppContext::default_lamad();
        // External HTTP slug resolution — only consider rows that carry a
        // provenance marker (Holochain dht_anchor_hash or libp2p Kad publish).
        let query = ContentQuery {
            content_format: Some("html5-app".to_string()),
            limit: 100,
            require_provenance: true,
            ..Default::default()
        };

        let items = db::content_diesel::list_content(&mut conn, &app_ctx, &query)?;
        let mut found_hash = None;

        for item in items {
            if let Some(ref content_body) = item.content.content_body {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                    if let Some(content_slug) = obj.get("slug").and_then(|v| v.as_str()) {
                        let hash = item.content.blob_hash.clone().unwrap_or_default();
                        if !hash.is_empty() {
                            let mut index = self.slug_index.write().await;
                            index.insert(content_slug.to_string(), hash.clone());
                        }
                        if content_slug == slug {
                            found_hash = item.content.blob_hash.clone();
                        }
                    }
                }
            }
        }

        Ok(found_hash)
    }

    // ========================================================================
    // Diesel-based Entity Handlers
    // ========================================================================

    /// Helper to get a Diesel connection from the pool
    fn get_diesel_conn(&self) -> Result<crate::db::PooledConn, StorageError> {
        self.db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Diesel pool not configured".into()))?
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// GET/POST /db/human-relationships - List or create human relationships
    async fn handle_human_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: human_relationships::HumanRelationshipQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match human_relationships::list_human_relationships(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateHumanRelationshipInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: human_relationships::CreateHumanRelationshipInput = input_view.into();

                match human_relationships::create_human_relationship(&mut conn, ctx, input) {
                    Ok(rel) => Ok(response::created(&rel)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/human-relationships/{id}
    async fn handle_human_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match human_relationships::get_human_relationship(&mut conn, ctx, id) {
                Ok(Some(rel)) => Ok(response::ok(&rel)),
                Ok(None) => Ok(response::not_found(&format!(
                    "Human relationship {} not found",
                    id
                ))),
                Err(e) => Ok(response::error_response(e)),
            },
            Method::DELETE => {
                match human_relationships::delete_human_relationship(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!(
                        "Human relationship {} not found",
                        id
                    ))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/human-relationships/{id}/consent - Update consent on a relationship
    async fn handle_human_relationship_consent(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        #[derive(Deserialize)]
        struct ConsentInput {
            party_id: String,
            consent: bool,
        }

        let input: ConsentInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let consent_update = human_relationships::ConsentUpdate {
            consent_given: input.consent,
        };

        match human_relationships::update_consent(
            &mut conn,
            ctx,
            id,
            &input.party_id,
            &consent_update,
        ) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/human-relationships/{id}/custody - Update custody settings on a relationship
    async fn handle_human_relationship_custody(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        #[derive(Deserialize)]
        struct CustodyInput {
            party_id: String,
            enabled: bool,
            #[serde(default)]
            auto_custody: Option<bool>,
            #[serde(default)]
            emergency_access: Option<bool>,
        }

        let input: CustodyInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let custody_update = human_relationships::CustodyUpdate {
            custody_enabled: input.enabled,
            auto_custody_enabled: input.auto_custody,
            emergency_access_enabled: input.emergency_access,
        };

        match human_relationships::update_custody(
            &mut conn,
            ctx,
            id,
            &input.party_id,
            &custody_update,
        ) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/presences - List or create contributor presences
    async fn handle_presences_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: contributor_presences::ContributorPresenceQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match contributor_presences::list_contributor_presences(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<ContributorPresenceView> =
                            items.into_iter().map(|p| p.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateContributorPresenceInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: contributor_presences::CreateContributorPresenceInput =
                    input_view.into();

                match contributor_presences::create_contributor_presence(&mut conn, ctx, input) {
                    Ok(presence) => {
                        let view: ContributorPresenceView = presence.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/presences/{id}
    async fn handle_presence_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match contributor_presences::get_contributor_presence(&mut conn, ctx, id) {
                    Ok(Some(presence)) => {
                        let view: ContributorPresenceView = presence.into();
                        Ok(response::ok(&view))
                    }
                    Ok(None) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match contributor_presences::delete_contributor_presence(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/presences/{id}/stewardship - Initiate stewardship of a presence
    async fn handle_presence_stewardship(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        let input: contributor_presences::InitiateStewardshipInput =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match contributor_presences::initiate_stewardship(&mut conn, ctx, id, &input) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/claim - Initiate claim of a presence
    async fn handle_presence_claim(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView, convert to internal DB type
        let input_view: InitiateClaimInputView = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let input: contributor_presences::InitiateClaimInput = input_view.into();

        match contributor_presences::initiate_claim(&mut conn, ctx, id, &input) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/verify-claim - Verify a claim (sets state to claimed)
    async fn handle_presence_verify_claim(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match contributor_presences::verify_claim(&mut conn, ctx, id) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/events - List or record economic events
    async fn handle_events_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: economic_events::EconomicEventQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match economic_events::list_economic_events(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<EconomicEventView> =
                            items.into_iter().map(|e| e.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateEconomicEventInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: economic_events::CreateEconomicEventInput = input_view.into();

                match economic_events::record_event(&mut conn, ctx, input) {
                    Ok(event) => {
                        let view: EconomicEventView = event.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/events/{id}
    async fn handle_event_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match economic_events::get_economic_event(&mut conn, ctx, id) {
                Ok(Some(event)) => {
                    let view: EconomicEventView = event.into();
                    Ok(response::ok(&view))
                }
                Ok(None) => Ok(response::not_found(&format!("Event {} not found", id))),
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/POST /db/mastery - List or create content mastery records
    async fn handle_mastery_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: content_mastery::MasteryQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match content_mastery::list_mastery(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<ContentMasteryView> =
                            items.into_iter().map(|m| m.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let input: content_mastery::CreateMasteryInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match content_mastery::upsert_mastery(&mut conn, ctx, input) {
                    Ok(mastery) => {
                        let view: ContentMasteryView = mastery.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/{id}
    async fn handle_mastery_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match content_mastery::get_mastery(&mut conn, ctx, id) {
                Ok(Some(mastery)) => {
                    let view: ContentMasteryView = mastery.into();
                    Ok(response::ok(&view))
                }
                Ok(None) => Ok(response::not_found(&format!(
                    "Mastery record {} not found",
                    id
                ))),
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/human/{human_id} - Get all mastery records for a human
    async fn handle_mastery_for_human(
        &self,
        _req: Request<Incoming>,
        method: Method,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match content_mastery::get_mastery_for_human(&mut conn, ctx, human_id) {
                Ok(items) => {
                    let views: Vec<ContentMasteryView> =
                        items.into_iter().map(|m| m.into()).collect();
                    let count = views.len();
                    let body = serde_json::json!({
                        "items": views,
                        "count": count,
                        "humanId": human_id,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Bulk Endpoints (for seeding/import operations)
    // =========================================================================

    /// POST /db/presences/bulk - Bulk create contributor presences
    async fn handle_presences_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateContributorPresenceInputView> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<contributor_presences::CreateContributorPresenceInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match contributor_presences::bulk_create_presences(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/events/bulk - Bulk record economic events
    async fn handle_events_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateEconomicEventInputView> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<economic_events::CreateEconomicEventInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match economic_events::bulk_record_events(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/mastery/bulk - Bulk create/update mastery records
    async fn handle_mastery_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateMasteryInputView> = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<content_mastery::CreateMasteryInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match content_mastery::bulk_upsert_mastery(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    // =========================================================================
    // Stewardship Allocation handlers
    // =========================================================================

    /// GET/POST /db/allocations - List or create stewardship allocations
    async fn handle_allocations_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: stewardship_allocations::AllocationQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match stewardship_allocations::list_allocations(&mut conn, app_ctx, &query) {
                    Ok(allocations) => {
                        let views: Vec<StewardshipAllocationView> =
                            allocations.into_iter().map(|a| a.into()).collect();
                        Ok(response::ok(&views))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateAllocationInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                let input: stewardship_allocations::CreateAllocationInput = input_view.into();

                match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/{id}, DELETE /db/allocations/{id}
    async fn handle_allocation_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match stewardship_allocations::get_allocation_by_id(&mut conn, app_ctx, id) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::ok(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::PUT => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: UpdateAllocationInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                let input: stewardship_allocations::UpdateAllocationInput = input_view.into();

                match stewardship_allocations::update_allocation(&mut conn, app_ctx, id, &input) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::ok(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match stewardship_allocations::delete_allocation(&mut conn, app_ctx, id) {
                    Ok(()) => Ok(response::no_content()),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/content/{content_id} - Get content stewardship aggregate
    async fn handle_allocations_for_content(
        &self,
        _req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_content_stewardship(&mut conn, app_ctx, content_id) {
            Ok(stewardship) => {
                let view: ContentStewardshipView = stewardship.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/allocations/steward/{steward_id} - Get allocations for a steward
    async fn handle_allocations_for_steward(
        &self,
        _req: Request<Incoming>,
        method: Method,
        steward_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_allocations_for_steward(&mut conn, app_ctx, steward_id) {
            Ok(allocations) => {
                let views: Vec<StewardshipAllocationView> =
                    allocations.into_iter().map(|a| a.into()).collect();
                Ok(response::ok(&views))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/dispute - File a dispute on an allocation
    async fn handle_allocation_dispute(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct DisputeInput {
            dispute_id: String,
            disputed_by: String,
            reason: String,
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: DisputeInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::file_dispute(
            &mut conn,
            app_ctx,
            allocation_id,
            &input.dispute_id,
            &input.disputed_by,
            &input.reason,
        ) {
            Ok(allocation) => {
                let view: StewardshipAllocationView = allocation.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/resolve - Resolve a dispute (Elohim ratification)
    async fn handle_allocation_resolve(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ResolveInput {
            ratifier_id: String,
            new_state: String,
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: ResolveInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::resolve_dispute(
            &mut conn,
            app_ctx,
            allocation_id,
            &input.ratifier_id,
            &input.new_state,
        ) {
            Ok(allocation) => {
                let view: StewardshipAllocationView = allocation.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/bulk - Bulk create stewardship allocations
    async fn handle_allocations_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateAllocationInputView> = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<stewardship_allocations::CreateAllocationInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        let mut created = 0;
        let mut failed = 0;
        let mut errors: Vec<String> = Vec::new();

        for input in inputs {
            match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                Ok(_) => created += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", input.content_id, e));
                }
            }
        }

        #[derive(serde::Serialize)]
        struct BulkResult {
            created: usize,
            failed: usize,
            errors: Vec<String>,
        }

        Ok(response::ok_with_schema_info(&BulkResult {
            created,
            failed,
            errors,
        }))
    }

    // =========================================================================
    // Stewarded Node Handlers
    // =========================================================================

    /// GET /db/nodes — list nodes (optional ?claimStatus= filter)
    /// POST /db/nodes — register a new node
    async fn handle_nodes_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();
                let claim_status = params.get("claimStatus").map(|s| s.as_str());

                match crate::db::stewarded_nodes::list_stewarded_nodes(&mut conn, claim_status) {
                    Ok(nodes) => {
                        let views: Vec<StewardedNodeView> =
                            nodes.into_iter().map(|n| n.into()).collect();
                        Ok(response::ok(&views))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input_view: CreateStewardedNodeInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                // Inject h_app_id from context
                let mut input: crate::db::stewarded_nodes::CreateStewardedNodeInput =
                    input_view.into();
                input.h_app_id = app_ctx.h_app_id.clone();

                match crate::db::stewarded_nodes::create_stewarded_node(&mut conn, input) {
                    Ok(node) => {
                        let view: StewardedNodeView = node.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/nodes/{id} — get node with stewards joined
    async fn handle_node_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        _app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match crate::db::stewarded_nodes::get_stewarded_node_by_id(&mut conn, id) {
                    Ok(Some(node)) => {
                        let mut view: StewardedNodeView = node.into();
                        // Join stewards with human display names
                        match crate::db::stewarded_nodes::list_stewards_for_node(&mut conn, id) {
                            Ok(stewards) => {
                                view.stewards = stewards
                                    .into_iter()
                                    .map(|s| {
                                        let name = crate::db::humans::get_human_by_id(
                                            &mut conn,
                                            &s.human_id,
                                        )
                                        .ok()
                                        .flatten()
                                        .map(|h| h.display_name)
                                        .unwrap_or_else(|| s.human_id.clone());
                                        NodeStewardshipView::from_with_name(s, name)
                                    })
                                    .collect();
                            }
                            Err(e) => {
                                warn!(error = %e, node_id = id, "Failed to load stewards for node");
                            }
                        }
                        Ok(response::ok(&view))
                    }
                    Ok(None) => Ok(response::not_found("Node not found")),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/nodes/{id}/stewardship — add a stewardship relationship to a node
    async fn handle_node_stewardship(
        &self,
        req: Request<Incoming>,
        method: Method,
        node_id: &str,
        _app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let mut input_view: CreateNodeStewardshipInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                // Override node_id from URL path segment (authoritative)
                input_view.node_id = node_id.to_string();
                let input: crate::db::stewarded_nodes::CreateNodeStewardshipInput =
                    input_view.into();

                match crate::db::stewarded_nodes::create_node_stewardship(&mut conn, input) {
                    Ok(stewardship) => {
                        let name =
                            crate::db::humans::get_human_by_id(&mut conn, &stewardship.human_id)
                                .ok()
                                .flatten()
                                .map(|h| h.display_name)
                                .unwrap_or_else(|| stewardship.human_id.clone());
                        let view = NodeStewardshipView::from_with_name(stewardship, name);
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Session Handlers (Tauri Native Handoff)
    // =========================================================================

    /// GET /session - Get active local session
    ///
    /// Returns the currently active session for native app use.
    /// Returns 404 if no active session exists (first run).
    async fn handle_get_session(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        match db::local_sessions::get_active_session(&mut conn)? {
            Some(session) => Ok(response::ok(&LocalSessionView::from(session))),
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "No active session"}"#)))
                .unwrap()),
        }
    }

    /// POST /session - Create a new local session
    ///
    /// Called after OAuth handoff from doorway to store session locally.
    /// Automatically deactivates any existing sessions.
    async fn handle_create_session(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let bytes = body.to_bytes();

        let input: db::local_sessions::CreateLocalSessionInput = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        let session = db::local_sessions::create_session(&mut conn, input)?;

        info!(
            session_id = %session.id,
            human_id = %session.human_id,
            "Created local session"
        );

        Ok(response::created(&LocalSessionView::from(session)))
    }

    /// DELETE /session - Delete active session (logout)
    ///
    /// Deactivates and removes the currently active session.
    async fn handle_delete_session(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        // Get active session first to know what we're deleting
        let active = db::local_sessions::get_active_session(&mut conn)?;

        if let Some(session) = active {
            db::local_sessions::delete_session(&mut conn, &session.id)?;
            info!(session_id = %session.id, "Deleted local session");
            Ok(response::ok(&serde_json::json!({
                "deleted": true,
                "sessionId": session.id
            })))
        } else {
            Ok(response::ok(&serde_json::json!({
                "deleted": false,
                "message": "No active session to delete"
            })))
        }
    }

    /// GET /session/all - List all sessions (for debugging)
    ///
    /// Returns all sessions including inactive ones.
    async fn handle_list_sessions(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        let sessions = db::local_sessions::list_all_sessions(&mut conn)?;
        let views: Vec<LocalSessionView> =
            sessions.into_iter().map(LocalSessionView::from).collect();
        Ok(response::ok(&views))
    }

    /// POST /session/intent - Set session intent for drift detection
    ///
    /// Declares what the user plans to do this session. Creates a set-point
    /// for the ElohimGate to detect behavioral drift.
    async fn handle_set_session_intent(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let bytes = body.to_bytes();

        let input: crate::views::SetSessionIntentInputView = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        match db::local_sessions::get_active_session(&mut conn)? {
            Some(session) => {
                db::local_sessions::set_session_intent(&mut conn, &session.id, &input.intent)?;
                info!(session_id = %session.id, "Session intent set");
                Ok(response::ok(&serde_json::json!({ "status": "intent_set" })))
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "No active session"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // EPR Head Handlers
    // =========================================================================

    /// PUT /epr-head/{id} — Accept JSON, encode as DAG-CBOR, store blob, return CID.
    async fn handle_put_epr_head(
        &self,
        req: Request<Incoming>,
        id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
                .unwrap());
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes();

        // Parse JSON input
        let input: EprHeadInputView = match serde_json::from_slice(&data) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Invalid JSON: {e}"}}"#
                    ))))
                    .unwrap());
            }
        };

        // Convert to EprHead and encode as DAG-CBOR
        let mut head: crate::epr_codec::EprHead = input.into();
        // Override id from URL path
        head.id = id.to_string();

        let (cbor_bytes, cid) = match crate::epr_codec::encode_epr_head(&head) {
            Ok(result) => result,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Encoding failed: {e}"}}"#
                    ))))
                    .unwrap());
            }
        };

        // Store DAG-CBOR blob
        let store_result = self.blob_store.store_dag_cbor(&cbor_bytes).await?;

        // Build response
        let mut view: EprHeadView = head.into();
        view.cid = Some(cid.to_string());

        let response_body = serde_json::json!({
            "head": view,
            "cid": store_result.cid,
            "hash": store_result.hash,
            "sizeBytes": store_result.size_bytes,
            "alreadyExisted": store_result.already_existed,
        });

        Ok(Response::builder()
            .status(if store_result.already_existed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            })
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(response_body.to_string())))
            .unwrap())
    }

    /// GET /epr-head/{id} — Retrieve EPR Head.
    /// Content-negotiation via Accept header:
    /// - `application/vnd.ipld.dag-cbor` → raw CBOR bytes
    /// - `application/json` (default) → JSON
    async fn handle_get_epr_head(
        &self,
        req: Request<Incoming>,
        id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
                .unwrap());
        }

        // Look up EPR Head from content DB by ID
        if let Ok(mut conn) = self.get_conn() {
            let app_ctx = db::AppContext::default_lamad();
            // External HTTP handler for GET /epr-head/{id} — gate on provenance
            // so we never surface a row that has neither been notarized on
            // Holochain nor published to libp2p Kad.
            let content_opt =
                db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, id, true)?;

            if let Some(content_with_tags) = content_opt {
                let content = &content_with_tags.content;
                // Build an EprHead from the content record
                let head = crate::epr_codec::EprHead {
                    version: 1,
                    id: content.id.clone(),
                    content: content.blob_cid.clone().unwrap_or_default(),
                    lamad: crate::epr_codec::EprLamadContext {
                        title: content.title.clone(),
                        content_type: content.content_type.clone(),
                        description: content.description.clone(),
                        content_format: Some(content.content_format.clone()),
                        tags: content_with_tags.tags.clone(),
                    },
                    shefa: crate::epr_codec::EprShefaContext {
                        stewards: vec![],
                        allocations: vec![],
                    },
                    qahal: crate::epr_codec::EprQahalContext {
                        reach: Some(content.reach.clone()),
                        layer: None,
                        attestation_requirements: vec![],
                    },
                    relationships: vec![],
                    author: content.created_by.clone(),
                    updated: Some(content.updated_at.clone()),
                };

                // Check Accept header for content negotiation
                let wants_cbor = req
                    .headers()
                    .get(header::ACCEPT)
                    .and_then(|v| v.to_str().ok())
                    .map(|a| a.contains("application/vnd.ipld.dag-cbor"))
                    .unwrap_or(false);

                if wants_cbor {
                    match crate::epr_codec::encode_epr_head(&head) {
                        Ok((cbor_bytes, _cid)) => {
                            return Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/vnd.ipld.dag-cbor")
                                .body(Full::new(Bytes::from(cbor_bytes)))
                                .unwrap());
                        }
                        Err(e) => {
                            return Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error":"Encoding failed: {e}"}}"#
                                ))))
                                .unwrap());
                        }
                    }
                }

                // Default: JSON response
                let mut view: EprHeadView = head.clone().into();
                if let Ok((_bytes, cid)) = crate::epr_codec::encode_epr_head(&head) {
                    view.cid = Some(cid.to_string());
                }

                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(
                        serde_json::to_string(&view).unwrap(),
                    )))
                    .unwrap());
            }
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"EPR Head not found: {id}"}}"#
            ))))
            .unwrap())
    }

    // =========================================================================
    // IPFS Block API
    // =========================================================================

    /// GET /ipfs/{cid} — Raw block retrieval by CID.
    ///
    /// Returns the raw bytes of a block stored by its CID.
    async fn handle_get_ipfs_block(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, data.len().to_string())
                .body(Full::new(Bytes::from(data)))
                .unwrap()),
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap())
            }
            Err(e) => Err(e),
        }
    }

    /// HEAD /ipfs/{cid} — Block existence check.
    async fn handle_head_ipfs_block(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        match self.blob_store.exists_by_address(cid_str).await {
            Ok(true) => {
                let size = self.blob_store.size_by_address(cid_str).await.unwrap_or(0);
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CONTENT_LENGTH, size.to_string())
                    .body(Full::new(Bytes::new()))
                    .unwrap())
            }
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap()),
        }
    }

    // =========================================================================
    // DAG API
    // =========================================================================

    /// GET /dag/{cid} — Decode DAG-CBOR block to JSON.
    ///
    /// Retrieves a block by CID and decodes it from DAG-CBOR to JSON for display.
    async fn handle_get_dag(&self, cid_str: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        let data = match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        // Try decoding as DAG-CBOR → JSON
        match serde_ipld_dagcbor::from_slice::<serde_json::Value>(&data) {
            Ok(value) => {
                let json = serde_json::to_string_pretty(&value).unwrap_or_default();
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap())
            }
            Err(_) => {
                // Not DAG-CBOR, return error with hint
                Ok(Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block is not DAG-CBOR","cid":"{cid_str}","hint":"Use /ipfs/{cid_str} for raw bytes"}}"#
                    ))))
                    .unwrap())
            }
        }
    }

    /// GET /dag/{cid}/links — List CIDs linked from a DAG-CBOR block.
    async fn handle_get_dag_links(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        // Parse CID to determine codec
        let cid = match cid::Cid::from_str(cid_str) {
            Ok(c) => c,
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Invalid CID: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
        };

        let data = match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        let links = crate::dag_store::extract_links(&data, cid.codec());
        let link_strs: Vec<String> = links.iter().map(|c| c.to_string()).collect();

        let response = serde_json::json!({
            "cid": cid_str,
            "links": link_strs,
            "count": link_strs.len(),
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(response.to_string())))
            .unwrap())
    }

    // =========================================================================
    // Human Directory Handler (Qahal - Community Directory)
    // =========================================================================

    /// GET /db/humans - List all humans for the app
    async fn handle_list_humans(
        &self,
        _req: Request<Incoming>,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match humans::list_humans(&mut conn, &ctx.h_app_id) {
            Ok(items) => {
                let views: Vec<HumanView> = items.into_iter().map(HumanView::from).collect();
                let body = serde_json::json!({
                    "items": views,
                    "count": views.len(),
                });
                Ok(response::ok(&body))
            }
            Err(e) => Ok(response::error_response(StorageError::Internal(format!(
                "Failed to list humans: {}",
                e
            )))),
        }
    }

    // =========================================================================
    // Collective Handlers (Qahal - Governance Contexts)
    // =========================================================================

    /// GET/POST /db/collectives - List or create collectives
    async fn handle_collectives_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: collectives::CollectiveQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match collectives::list_collectives(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<CollectiveView> =
                            items.into_iter().map(CollectiveView::from).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let input_view: CreateCollectiveInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: collectives::CreateCollectiveInput = input_view.into();

                match collectives::create_collective(&mut conn, ctx, &input) {
                    Ok(c) => Ok(response::created(&CollectiveView::from(c))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/collectives/{id}
    async fn handle_collective_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::get_collective(&mut conn, ctx, id) {
            Ok(Some(c)) => Ok(response::ok(&CollectiveView::from(c))),
            Ok(None) => Ok(response::not_found(&format!("Collective {} not found", id))),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/collectives/{id}/participants
    async fn handle_collective_participants(
        &self,
        req: Request<Incoming>,
        method: Method,
        collective_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match collectives::get_participants_of_collective(&mut conn, ctx, collective_id) {
                    Ok(items) => {
                        let views: Vec<CollectiveParticipationView> = items
                            .into_iter()
                            .map(CollectiveParticipationView::from)
                            .collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct AddParticipantInput {
                    human_id: String,
                    #[serde(default)]
                    intimacy_level: Option<String>,
                    #[serde(default)]
                    role_context: Option<String>,
                }

                let input: AddParticipantInput = serde_json::from_slice(&body.to_bytes())
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                let participation_input = collectives::CreateParticipationInput {
                    id: None,
                    collective_id: collective_id.to_string(),
                    human_id: input.human_id,
                    intimacy_level: input
                        .intimacy_level
                        .unwrap_or_else(|| "recognition".to_string()),
                    role_context: input.role_context,
                    governance_weight: 1.0,
                    consent_state: "consented".to_string(),
                    metadata_json: None,
                };

                match collectives::create_participation(&mut conn, ctx, &participation_input) {
                    Ok(p) => Ok(response::created(&CollectiveParticipationView::from(p))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// DELETE /db/collectives/{id}/participants/{human_id} - Depart from collective
    async fn handle_collective_participant_depart(
        &self,
        _req: Request<Incoming>,
        method: Method,
        collective_id: &str,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::DELETE {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::depart_collective(&mut conn, ctx, collective_id, human_id) {
            Ok(true) => Ok(response::ok(&serde_json::json!({"departed": true}))),
            Ok(false) => Ok(response::not_found("Participation not found")),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/participations/{human_id} - All collectives for a human
    async fn handle_participations_by_human(
        &self,
        _req: Request<Incoming>,
        method: Method,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::get_participations_for_human(&mut conn, ctx, human_id) {
            Ok(items) => {
                let views: Vec<CollectiveParticipationView> = items
                    .into_iter()
                    .map(CollectiveParticipationView::from)
                    .collect();
                let body = serde_json::json!({
                    "items": views,
                    "count": views.len(),
                });
                Ok(response::ok(&body))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    // =========================================================================
    // Account Import/Export Handlers
    // =========================================================================

    /// POST /account/import - Import an account package
    ///
    /// Accepts an AccountPackageInputView and orchestrates:
    /// 1. Content reach updates (sets per-content reach levels)
    /// 2. Human relationship creation
    /// 3. Stewardship allocation creation
    /// 4. Collective participation creation
    ///
    /// This endpoint serves both genesis seeding (initial conditions) and
    /// account recovery (restoring a human's world from a backup).
    async fn do_account_import(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let package: AccountPackageInputView = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid account package JSON: {}", e)))?;

        let human_id = package.identity.human_id.clone();
        let ctx = AppContext::default_lamad();

        info!(
            human_id = %human_id,
            content_count = package.content.len(),
            relationship_count = package.relationships.len(),
            stewardship_count = package.stewardship.len(),
            "Importing account package"
        );

        let mut errors: Vec<String> = Vec::new();
        let mut content_updated: usize = 0;
        let mut relationships_created: usize = 0;
        let mut stewardship_created: usize = 0;

        // Phase 1: Update content reach levels
        // The content itself is already seeded — we're updating reach per-human's assignment
        if !package.content.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            use crate::db::diesel_schema::content;
            use diesel::prelude::*;

            for assignment in &package.content {
                let updated = diesel::update(
                    content::table
                        .filter(content::h_app_id.eq(&ctx.h_app_id))
                        .filter(content::id.eq(&assignment.content_id)),
                )
                .set(content::reach.eq(&assignment.reach))
                .execute(&mut *conn);

                match updated {
                    Ok(n) if n > 0 => content_updated += 1,
                    Ok(_) => {
                        // Content doesn't exist yet — not an error, just skip
                        debug!(content_id = %assignment.content_id, "Content not found for reach update, skipping");
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to update reach for {}: {}",
                            assignment.content_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                content_updated = content_updated,
                "Content reach updates complete"
            );
        }

        // Phase 2: Create human relationships
        if !package.relationships.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            for rel_seed in &package.relationships {
                let input = human_relationships::CreateHumanRelationshipInput {
                    id: None,
                    party_a_id: human_id.clone(),
                    party_b_id: rel_seed.target_id.clone(),
                    relationship_type: rel_seed.relationship_type.clone(),
                    intimacy_level: rel_seed.intimacy_level.clone(),
                    is_bidirectional: rel_seed.is_bidirectional,
                    consent_given_by_a: true,
                    consent_given_by_b: false, // Other party consents independently
                    initiated_by: human_id.clone(),
                    governance_layer: None,
                    reach: rel_seed
                        .reach
                        .clone()
                        .unwrap_or_else(|| "private".to_string()),
                    context_json: None,
                    expires_at: None,
                };

                match human_relationships::create_human_relationship(&mut conn, &ctx, input) {
                    Ok(_) => relationships_created += 1,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to create relationship {} -> {}: {}",
                            human_id, rel_seed.target_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                relationships_created = relationships_created,
                "Human relationship creation complete"
            );
        }

        // Phase 3: Create stewardship allocations
        // Match stewardship seeds to content by category, create allocations
        if !package.stewardship.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            // For each stewardship seed, we need a contributor presence for the human.
            // Look up or note that presences should already exist from seeding.
            for steward_seed in &package.stewardship {
                // Find content matching this category via content_type field
                use crate::db::diesel_schema::content;
                use diesel::prelude::*;

                let matching_content: Vec<String> = content::table
                    .filter(content::h_app_id.eq(&ctx.h_app_id))
                    .filter(content::content_type.eq(&steward_seed.content_category))
                    .select(content::id)
                    .load(&mut *conn)
                    .unwrap_or_default();

                // Create allocation for each matching content item
                // Note: This requires a presence_id for the human, which is resolved
                // by the seeder beforehand. For now, use human_id as presence reference.
                for content_id in &matching_content {
                    let input = stewardship_allocations::CreateAllocationInput {
                        content_id: content_id.clone(),
                        steward_presence_id: human_id.clone(),
                        allocation_ratio: steward_seed.allocation_ratio,
                        allocation_method: "computed".to_string(),
                        contribution_type: steward_seed
                            .contribution_type
                            .clone()
                            .unwrap_or_else(|| "inherited".to_string()),
                        contribution_evidence_json: None,
                        note: Some(format!(
                            "Account package import: {} stewardship",
                            steward_seed.content_category
                        )),
                        metadata_json: None,
                    };

                    match stewardship_allocations::create_allocation(&mut conn, &ctx, &input) {
                        Ok(_) => stewardship_created += 1,
                        Err(e) => {
                            // Duplicate allocations are expected on re-import
                            let err_msg = e.to_string();
                            if !err_msg.contains("UNIQUE constraint") {
                                errors.push(format!(
                                    "Failed to create allocation for {}: {}",
                                    content_id, e
                                ));
                            }
                        }
                    }
                }
            }

            info!(
                human_id = %human_id,
                stewardship_created = stewardship_created,
                "Stewardship allocation creation complete"
            );
        }

        // Phase 4: Create collective participations
        let mut collectives_joined: usize = 0;
        if !package.collectives.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            let qahal_ctx = AppContext::new("qahal");

            for coll_seed in &package.collectives {
                // Ensure the collective exists (create stub if needed for seeding)
                let _ =
                    collectives::get_collective(&mut conn, &qahal_ctx, &coll_seed.collective_id);

                let participation_input = collectives::CreateParticipationInput {
                    id: None,
                    collective_id: coll_seed.collective_id.clone(),
                    human_id: human_id.clone(),
                    intimacy_level: coll_seed
                        .intimacy_level
                        .clone()
                        .unwrap_or_else(|| "connection".to_string()),
                    role_context: coll_seed.role_context.clone(),
                    governance_weight: 1.0,
                    consent_state: "consented".to_string(),
                    metadata_json: None,
                };

                match collectives::create_participation(&mut conn, &qahal_ctx, &participation_input)
                {
                    Ok(_) => collectives_joined += 1,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to join collective {}: {}",
                            coll_seed.collective_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                collectives_joined = collectives_joined,
                "Collective participation creation complete"
            );
        }

        let result = AccountImportResultView {
            human_id,
            content_updated,
            relationships_created,
            stewardship_created,
            collectives_joined,
            errors,
        };

        Ok(response::ok(&result))
    }

    /// GET /account/export/{human_id} - Export an account package
    ///
    /// Assembles an account package from the current state of a human's world:
    /// - Content assignments (what they can see and at what reach)
    /// - Human relationships (who they know)
    /// - Stewardship allocations (what they steward)
    /// - Collective participations (what collectives they belong to)
    ///
    /// This serves both backup/recovery and migration between conductors.
    async fn do_account_export(
        &self,
        human_id: &str,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let ctx = AppContext::default_lamad();
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        info!(human_id = %human_id, "Exporting account package");

        // Phase 1: Gather content assignments
        // Content where this human has specific reach (via stewardship or relationships)
        let content_assignments: Vec<ContentAssignmentView> = {
            use crate::db::diesel_schema::content;
            use diesel::prelude::*;

            // Get all content in this app context — the reach field tells us the assignment
            let items: Vec<(String, String)> = content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .select((content::id, content::reach))
                .load(&mut *conn)
                .map_err(|e| StorageError::Internal(format!("Content query failed: {}", e)))?;

            items
                .into_iter()
                .map(|(id, reach)| ContentAssignmentView {
                    content_id: id,
                    reach,
                    reason: None,
                    steward_ratio: None,
                })
                .collect()
        };

        // Phase 2: Gather human relationships
        let relationship_seeds: Vec<RelationshipSeedView> = {
            let query = human_relationships::HumanRelationshipQuery {
                party_id: Some(human_id.to_string()),
                limit: 10000,
                ..Default::default()
            };

            let rels = human_relationships::list_human_relationships(&mut conn, &ctx, &query)
                .unwrap_or_default();

            rels.into_iter()
                .map(|r| {
                    // Determine which party is the "other" one
                    let target = if r.party_a_id == human_id {
                        r.party_b_id
                    } else {
                        r.party_a_id
                    };

                    RelationshipSeedView {
                        target_id: target,
                        relationship_type: r.relationship_type,
                        intimacy_level: r.intimacy_level,
                        is_bidirectional: r.is_bidirectional == 1,
                        reach: Some(r.reach),
                    }
                })
                .collect()
        };

        // Phase 3: Gather stewardship allocations
        let stewardship_seeds: Vec<StewardshipSeedView> = {
            let allocs =
                stewardship_allocations::get_allocations_for_steward(&mut conn, &ctx, human_id)
                    .unwrap_or_default();

            // Group by content_type (category) and compute aggregate ratios
            let mut category_map: std::collections::HashMap<String, (f32, String)> =
                std::collections::HashMap::new();

            for alloc in &allocs {
                // Look up content_type for this content
                use crate::db::diesel_schema::content;
                use diesel::prelude::*;

                let content_type: Option<String> = content::table
                    .filter(content::id.eq(&alloc.content_id))
                    .select(content::content_type)
                    .first(&mut *conn)
                    .optional()
                    .unwrap_or(None);

                if let Some(ct) = content_type {
                    let entry = category_map
                        .entry(ct)
                        .or_insert((0.0, alloc.contribution_type.clone()));
                    entry.0 += alloc.allocation_ratio;
                }
            }

            category_map
                .into_iter()
                .map(
                    |(category, (ratio, contribution_type))| StewardshipSeedView {
                        content_category: category,
                        allocation_ratio: ratio,
                        contribution_type: Some(contribution_type),
                    },
                )
                .collect()
        };

        // Phase 4: Gather collective participations
        let collective_seeds: Vec<CollectiveSeedView> = {
            let qahal_ctx = AppContext::new("qahal");
            let participations =
                collectives::get_participations_for_human(&mut conn, &qahal_ctx, human_id)
                    .unwrap_or_default();

            participations
                .into_iter()
                .map(|p| CollectiveSeedView {
                    collective_id: p.collective_id,
                    role_context: p.role_context,
                    intimacy_level: Some(p.intimacy_level),
                })
                .collect()
        };

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let package = AccountPackageView {
            identity: AccountIdentityView {
                human_id: human_id.to_string(),
                display_name: human_id.to_string(), // Will be enriched when identity service is integrated
                category: None,
                profile_reach: None,
                bio: None,
                location: None,
                affinities: vec![],
                organizations: vec![],
            },
            content: content_assignments,
            relationships: relationship_seeds,
            stewardship: stewardship_seeds,
            collectives: collective_seeds,
            manifest: PackageManifestView {
                version: "1.0.0".to_string(),
                generated_at: now,
                source_story: Some("export".to_string()),
                content_hash: None,
            },
        };

        info!(
            human_id = %human_id,
            content_count = package.content.len(),
            relationship_count = package.relationships.len(),
            stewardship_count = package.stewardship.len(),
            collectives_count = package.collectives.len(),
            "Account package exported"
        );

        Ok(response::ok(&package))
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Get MIME type for a file path based on extension
    fn get_mime_type(path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("html") | Some("htm") => "text/html; charset=utf-8",
            Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            Some("eot") => "application/vnd.ms-fontobject",
            Some("wasm") => "application/wasm",
            Some("mp3") => "audio/mpeg",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("ogg") => "audio/ogg",
            Some("wav") => "audio/wav",
            Some("txt") => "text/plain; charset=utf-8",
            Some("xml") => "application/xml",
            Some("pdf") => "application/pdf",
            Some("zip") => "application/zip",
            Some("map") => "application/json", // source maps
            _ => "application/octet-stream",
        }
    }

    // =========================================================================
    // Observation Session API
    // =========================================================================

    /// Dispatcher for `/api/v1/observations[/*]` requests.
    ///
    /// Routes:
    /// - `POST /api/v1/observations/begin`           -> begin a new session
    /// - `POST /api/v1/observations/{id}/entries`    -> append entries
    /// - `GET  /api/v1/observations/{id}/report`     -> generate/return report
    async fn handle_observation_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        sub_path: &str,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // POST /api/v1/observations/begin
        if sub_path == "/begin" && method == Method::POST {
            return self.handle_observation_begin(req, &pool).await;
        }

        // POST /api/v1/observations/{id}/entries
        // GET  /api/v1/observations/{id}/report
        if let Some(rest) = sub_path.strip_prefix('/') {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 {
                let session_id = parts[0];
                match (method.clone(), parts[1]) {
                    (Method::POST, "entries") => {
                        return self
                            .handle_observation_add_entries(req, session_id, &pool)
                            .await;
                    }
                    (Method::GET, "report") => {
                        return self.handle_observation_report(session_id, &pool).await;
                    }
                    _ => {}
                }
            }
        }

        Ok(response::not_found(&format!(
            "Unknown observation route: /api/v1/observations{}",
            sub_path
        )))
    }

    /// POST /api/v1/observations/begin
    ///
    /// Begin a new observation session. Returns the session ID and computed expiry.
    async fn handle_observation_begin(
        &self,
        req: Request<Incoming>,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body_bytes = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();

        let input: BeginObservationInputView = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let metadata_str = input
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(&m.0).unwrap_or_default());

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        let session = db::observation_sessions::begin_session(
            &mut conn,
            &input.source,
            input.ttl_seconds,
            metadata_str.as_deref(),
        )
        .map_err(|e| StorageError::Internal(format!("Failed to begin session: {}", e)))?;

        // Compute expiry from started_at + ttl_seconds
        let expires_at = chrono::DateTime::parse_from_rfc3339(&session.started_at)
            .map(|dt| {
                (dt + chrono::Duration::seconds(session.ttl_seconds as i64))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            })
            .unwrap_or_else(|_| session.started_at.clone());

        let view = BeginObservationResponseView {
            session_id: session.id,
            expires_at,
        };

        Ok(response::created(&view))
    }

    /// POST /api/v1/observations/{id}/entries
    ///
    /// Append one or an array of observation entries to an active session.
    /// Returns 201 with empty body on success.
    async fn handle_observation_add_entries(
        &self,
        req: Request<Incoming>,
        session_id: &str,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        // Guard: session must be active
        let is_active = db::observation_sessions::is_session_active(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Session check failed: {}", e)))?;

        if !is_active {
            return Ok(response::bad_request(&format!(
                "Observation session '{}' is not active or does not exist",
                session_id
            )));
        }

        let body_bytes = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();

        // Accept single entry or array
        let entries: Vec<ObservationEntryInputView> = if body_bytes.first() == Some(&b'[') {
            serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON array: {}", e)))?
        } else {
            let single: ObservationEntryInputView = serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
            vec![single]
        };

        for entry in &entries {
            let context_str = entry
                .context
                .as_ref()
                .map(|c| serde_json::to_string(&c.0).unwrap_or_default());

            db::observation_sessions::append_entry(
                &mut conn,
                session_id,
                &entry.origin,
                &entry.category,
                &entry.severity,
                entry.method.as_deref(),
                entry.path.as_deref(),
                entry.status_code,
                &entry.message,
                context_str.as_deref(),
            )
            .map_err(|e| StorageError::Internal(format!("Append entry failed: {}", e)))?;
        }

        // 201 with empty body -- entries accepted
        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from("{}")))
            .unwrap())
    }

    /// GET /api/v1/observations/{id}/report
    ///
    /// Generates a diagnostic report for the session, then closes the session
    /// and purges its entries. Idempotent: if a report_content_id is already
    /// recorded on the session, returns the existing report content ID.
    ///
    /// Content ID is stable: `obs-report-for-{scenarioId}` when scenarioId is
    /// present in session metadata, otherwise `obs-report-{sessionId}`.
    async fn handle_observation_report(
        &self,
        session_id: &str,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        let session = db::observation_sessions::get_session(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Session lookup failed: {}", e)))?
            .ok_or_else(|| {
                StorageError::NotFound(format!("Observation session '{}' not found", session_id))
            })?;

        // Idempotent: return existing content ID if already reported
        if let Some(ref existing_id) = session.report_content_id {
            let body = serde_json::json!({ "contentId": existing_id, "cached": true });
            return Ok(response::ok(&body));
        }

        // Read all entries for this session
        let entries = db::observation_sessions::get_entries(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Get entries failed: {}", e)))?;

        // Build summary counts
        let total_entries = entries.len();
        let mut by_origin: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut by_severity: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut by_category: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for entry in &entries {
            *by_origin.entry(entry.origin.clone()).or_insert(0) += 1;
            *by_severity.entry(entry.severity.clone()).or_insert(0) += 1;
            *by_category.entry(entry.category.clone()).or_insert(0) += 1;
        }

        // Correlate issues from entries
        let issues = correlate_issues(&entries);

        // Determine content ID -- stable per scenarioId if present in metadata
        let scenario_id = session
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.get("scenarioId")
                    .and_then(|s| s.as_str())
                    .map(String::from)
            });

        let content_id = match scenario_id {
            Some(ref sid) => format!("obs-report-for-{}", sid),
            None => format!("obs-report-{}", session_id),
        };

        // Compute duration
        let ended_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let duration_ms = chrono::DateTime::parse_from_rfc3339(&session.started_at)
            .map(|start| {
                chrono::Utc::now()
                    .signed_duration_since(start)
                    .num_milliseconds()
            })
            .unwrap_or(0);

        // System state snapshot
        let system_state = ObservationSystemStateView {
            storage_healthy: self.db_pool.is_some(),
            conductor_connected: self.import_api.is_some(),
            p2p_peer_count: 0,
        };

        let metadata = session
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .map(crate::views::JsonVal);

        let entry_views: Vec<ObservationEntryView> = entries
            .iter()
            .cloned()
            .map(ObservationEntryView::from)
            .collect();

        let report = ObservationReportView {
            content_id: content_id.clone(),
            session_id: session_id.to_string(),
            source: session.source.clone(),
            metadata,
            duration: ObservationDurationView {
                started_at: session.started_at.clone(),
                ended_at: ended_at.clone(),
                duration_ms,
            },
            summary: ObservationSummaryView {
                total_entries,
                by_origin,
                by_severity,
                by_category,
            },
            issues,
            system_state,
        };

        // Persist report as a content node so it is addressable as an EPR
        let report_body = serde_json::to_string(&serde_json::json!({
            "entries": entry_views,
            "report": report,
        }))
        .unwrap_or_default();

        if let Some(ref services) = self.services {
            let create_input = db::content_diesel::CreateContentInput {
                id: content_id.clone(),
                title: format!("Observation Report: {}", session.source),
                description: Some(format!(
                    "Diagnostic report for session {} ({})",
                    session_id, session.source
                )),
                content_type: "observation-report".to_string(),
                content_format: "json".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: Some(report_body.len() as i32),
                metadata_json: session.metadata_json.clone(),
                reach: "familiar".to_string(),
                created_by: Some("elohim-storage".to_string()),
                tags: vec!["observation".to_string(), "diagnostic".to_string()],
                content_body: Some(report_body),
            };

            // Non-fatal: if content already exists (idempotent scenario), just log
            match services.content.create(create_input) {
                Ok(_) => {
                    debug!(content_id = %content_id, "Observation report persisted as content node");
                }
                Err(e) => {
                    debug!(
                        content_id = %content_id,
                        error = %e,
                        "Observation report content node already exists or failed (non-fatal)"
                    );
                }
            }
        }

        // Close session and purge entries
        db::observation_sessions::close_session(&mut conn, session_id, Some(&content_id))
            .map_err(|e| StorageError::Internal(format!("Close session failed: {}", e)))?;

        db::observation_sessions::purge_entries(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Purge entries failed: {}", e)))?;

        Ok(response::ok(&report))
    }

    /// Observation middleware aspect.
    ///
    /// Called after the main request handler when `X-Observation-Id` is present.
    /// Appends an entry to the named session for any non-2xx response status.
    /// Successes (2xx) are not recorded -- only failures carry diagnostic signal.
    fn maybe_observe_request(&self, session_id: &str, method: &str, path: &str, status_code: u16) {
        // Only observe failures
        if status_code < 300 {
            return;
        }

        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return,
        };

        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Infer category from path
        let category = if path.starts_with("/db/content") || path.starts_with("/api/v1/content") {
            "content"
        } else if path.starts_with("/db/allocations") || path.starts_with("/api/v1/stewardship") {
            "stewardship"
        } else if path.starts_with("/api/v1/mastery") {
            "mastery"
        } else if path.starts_with("/api/v1/governance") {
            "governance"
        } else if path.starts_with("/api/v1/identity") || path.starts_with("/db/humans") {
            "identity"
        } else if path.starts_with("/blob/") || path.starts_with("/shard/") {
            "blob"
        } else {
            "api"
        };

        // Infer severity from status code
        let severity = if status_code >= 500 {
            "error"
        } else if status_code >= 400 {
            "warning"
        } else {
            "info"
        };

        let message = format!("HTTP {} {} -> {}", method, path, status_code);

        let _ = db::observation_sessions::append_entry(
            &mut conn,
            session_id,
            "elohim-storage",
            category,
            severity,
            Some(method),
            Some(path),
            Some(status_code as i32),
            &message,
            None,
        );
    }
}

/// Correlate observation entries into actionable issues.
///
/// Groups error/warning entries by failure pattern:
/// - 401 clusters → single auth issue
/// - 404 per content ID → not-found issue per content
/// - 405 per path → method-not-allowed issue per path
/// - 503 mentioning "imagodei" → identity/conductor issue
///
/// Content IDs are extracted from `/db/content/{id}` and
/// `/db/allocations/content/{id}` paths.
fn correlate_issues(entries: &[crate::db::models::ObservationEntry]) -> Vec<ObservationIssueView> {
    use std::collections::HashMap;

    let mut issues: HashMap<String, ObservationIssueView> = HashMap::new();

    for entry in entries {
        if entry.severity != "error" && entry.severity != "warning" {
            continue;
        }

        let path = entry.path.as_deref().unwrap_or("");
        let status = entry.status_code.unwrap_or(0);

        // Extract content ID from well-known path patterns
        let content_id_from_path = if let Some(rest) = path.strip_prefix("/db/content/") {
            let id = rest.split('/').next().unwrap_or("").to_string();
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        } else if let Some(rest) = path.strip_prefix("/db/allocations/content/") {
            let id = rest.split('/').next().unwrap_or("").to_string();
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        } else {
            None
        };

        match status {
            401 => {
                let key = "auth-401".to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "auth".to_string(),
                        severity: "error".to_string(),
                        title: "Authentication failures detected".to_string(),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause:
                            "Agent session may be missing or expired. Check X-Agent-Id header."
                                .to_string(),
                    });
                issue.entry_count += 1;
                if let Some(ref cid) = content_id_from_path {
                    if !issue.related_content_ids.contains(cid) {
                        issue.related_content_ids.push(cid.clone());
                    }
                }
            }
            404 => {
                let key = if let Some(ref cid) = content_id_from_path {
                    format!("not-found-{}", cid)
                } else {
                    format!("not-found-{}", path.replace('/', "-").trim_matches('-'))
                };
                let related = content_id_from_path
                    .clone()
                    .map(|s| vec![s])
                    .unwrap_or_default();
                let path_clone = path.to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "content".to_string(),
                        severity: "warning".to_string(),
                        title: format!("Content not found: {}", path_clone),
                        entry_count: 0,
                        related_content_ids: related,
                        suggested_cause:
                            "Content ID may not be seeded or may belong to a different h_app_id."
                                .to_string(),
                    });
                issue.entry_count += 1;
            }
            405 => {
                let key = format!(
                    "method-not-allowed-{}",
                    path.replace('/', "-").trim_matches('-')
                );
                let path_clone = path.to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "routing".to_string(),
                        severity: "warning".to_string(),
                        title: format!("Method not allowed on: {}", path_clone),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause:
                            "HTTP method is not registered for this route. Check API specification."
                                .to_string(),
                    });
                issue.entry_count += 1;
            }
            503 if entry.message.contains("imagodei") || path.contains("imagodei") => {
                let key = "conductor-imagodei".to_string();
                let issue = issues.entry(key.clone()).or_insert_with(|| ObservationIssueView {
                    id: key,
                    category: "infrastructure".to_string(),
                    severity: "error".to_string(),
                    title: "Imagodei (identity) conductor unreachable".to_string(),
                    entry_count: 0,
                    related_content_ids: vec![],
                    suggested_cause:
                        "The imagodei DNA conductor is not responding. Verify that elohim-storage                          is connected and the hApp is installed."
                            .to_string(),
                });
                issue.entry_count += 1;
            }
            503 => {
                let key = "service-unavailable".to_string();
                let issue =
                    issues.entry(key.clone()).or_insert_with(|| {
                        ObservationIssueView {
                    id: key,
                    category: "infrastructure".to_string(),
                    severity: "error".to_string(),
                    title: "Service unavailable responses detected".to_string(),
                    entry_count: 0,
                    related_content_ids: vec![],
                    suggested_cause:
                        "Backend service or database pool is not initialised. Check startup logs."
                            .to_string(),
                }
                    });
                issue.entry_count += 1;
            }
            _ => {
                // Other error/warning entries: group by severity + category
                let key = format!("{}-{}", entry.severity, entry.category);
                let sev = entry.severity.clone();
                let cat = entry.category.clone();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: cat,
                        severity: sev,
                        title: format!(
                            "Repeated {} in category: {}",
                            entry.severity, entry.category
                        ),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause: "Review log entries for details.".to_string(),
                    });
                issue.entry_count += 1;
                if let Some(ref cid) = content_id_from_path {
                    if !issue.related_content_ids.contains(cid) {
                        issue.related_content_ids.push(cid.clone());
                    }
                }
            }
        }
    }

    let mut result: Vec<ObservationIssueView> = issues.into_values().collect();
    // Sort: errors first, then warnings; within each severity sort by id
    result.sort_by(|a, b| {
        let sev_order = |s: &str| match s {
            "error" => 0u8,
            "warning" => 1,
            _ => 2,
        };
        sev_order(&a.severity)
            .cmp(&sev_order(&b.severity))
            .then(a.id.cmp(&b.id))
    });
    result
}

/// Build the static route manifest for doorway discovery.
///
/// Declares every `/api/v1/*` and `/db/*` route that elohim-storage serves.
/// Used by `GET /manifest` and consumed by doorway's dynamic route registry.
///
/// Design rule: list only routes that doorway should proxy to clients.
/// Infrastructure routes (/health, /shard/*, /blob/*, /sync/*, /import/*,
/// /p2p/*, /epr-head/*, /ipfs/*, /dag/*, /session, /account/*) are
/// intentionally omitted — doorway handles them independently or not at all.
pub fn build_manifest() -> doorway_client::DoorwayRoutes {
    DoorwayRoutesBuilder::new()
        // =====================================================================
        // /api/v1/mastery — Content mastery lifecycle
        // =====================================================================
        .route(
            Route::get("/api/v1/mastery")
                .handler("list_mastery")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery")
                .handler("initialize_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/engagement")
                .handler("record_engagement")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/assessment")
                .handler("record_assessment")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/batch")
                .handler("batch_query_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/check-privilege")
                .handler("check_privilege")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/stats")
                .handler("mastery_stats")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/pool/{id}")
                .handler("get_practice_pool")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/path/{path_id}")
                .handler("mastery_for_path")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/{id}")
                .handler("get_mastery")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/governance — Governance state and deliberation
        // =====================================================================
        .route(
            Route::get("/api/v1/governance/state")
                .handler("governance_state")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/states")
                .handler("list_governance_states")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/challenges")
                .handler("list_challenges")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/challenges/{id}")
                .handler("get_challenge")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges")
                .handler("file_challenge")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges/{id}/respond")
                .handler("respond_to_challenge")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges/{id}/appeal")
                .handler("file_appeal")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/appeals/{challengeId}")
                .handler("list_appeals")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals")
                .handler("list_proposals")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}")
                .handler("get_proposal")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/precedents")
                .handler("list_precedents")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/precedents/{id}")
                .handler("get_precedent")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/discussions")
                .handler("list_discussions")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/discussions/{id}")
                .handler("get_discussion")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals")
                .handler("create_proposal")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/votes")
                .handler("cast_vote")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/votes")
                .handler("list_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions")
                .handler("create_discussion")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions/{id}/messages")
                .handler("post_message")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/options")
                .handler("create_proposal_options")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/options")
                .handler("list_proposal_options")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/ranked-votes")
                .handler("cast_ranked_votes")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/ranked-votes")
                .handler("list_ranked_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/tally")
                .handler("compute_tally")
                .cache_ttl(10)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/signals")
                .handler("record_signal")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/signals/aggregate")
                .handler("aggregate_signals")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/signals")
                .handler("list_signals")
                .cache_ttl(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/governance/sensemaking — Sensemaking statements & clustering
        // =====================================================================
        .route(
            Route::post("/api/v1/governance/sensemaking/statements")
                .handler("create_statement")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/statements")
                .handler("list_statements")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/sensemaking/statements/{id}/vote")
                .handler("vote_on_statement")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/votes")
                .handler("list_statement_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/clusters")
                .handler("compute_clusters")
                .cache_ttl(10)
                .build(),
        )
        // =====================================================================
        // /api/v1/economic-events — REA economic events
        // =====================================================================
        .route(
            Route::get("/api/v1/economic-events")
                .handler("list_economic_events")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events")
                .handler("create_economic_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/bulk")
                .handler("bulk_create_economic_events")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/from-staged")
                .handler("economic_events_from_staged")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/agent/{agent_id}")
                .handler("economic_events_by_agent")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/content/{content_id}")
                .handler("economic_events_by_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/appreciations")
                .handler("list_appreciations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/appreciations")
                .handler("create_appreciation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/{id}")
                .handler("get_economic_event")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/stewardship — Stewardship allocations and policy
        // =====================================================================
        .route(
            Route::get("/api/v1/stewardship/policy")
                .handler("get_stewardship_policy")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/grants")
                .handler("list_stewardship_grants")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/grants")
                .handler("create_stewardship_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/grants/{id}")
                .handler("get_stewardship_grant")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/grants/{id}/delegate")
                .handler("delegate_stewardship_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/appeals")
                .handler("file_stewardship_appeal")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/activity")
                .handler("log_stewardship_activity")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations")
                .handler("list_allocations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations")
                .handler("create_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/content/{content_id}")
                .handler("allocations_for_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/steward/{steward_id}")
                .handler("allocations_for_steward")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/{id}")
                .handler("get_allocation")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::put("/api/v1/stewardship/allocations/{id}")
                .handler("update_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/stewardship/allocations/{id}")
                .handler("delete_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations/{id}/dispute")
                .handler("dispute_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations/{id}/resolve")
                .handler("resolve_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/policies")
                .handler("upsert_stewardship_policy")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies")
                .handler("list_stewardship_policies")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/me/chain")
                .handler("my_policy_chain")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}")
                .handler("get_stewardship_policy_by_id")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}/parent")
                .handler("policy_parent")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}/chain")
                .handler("policy_chain")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/access/time")
                .handler("check_time_access")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/resilience — Content resilience projection
        // =====================================================================
        .route(
            Route::get("/api/v1/resilience/{content_id}")
                .handler("get_resilience")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/resilience/{content_id}/verify")
                .handler("verify_resilience")
                .build(),
        )
        // =====================================================================
        // /api/v1/recognition — Recognition distribution pipeline
        // =====================================================================
        .route(
            Route::post("/api/v1/recognition/distribute")
                .handler("distribute_recognition")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/schedules — Kairos temporal schedules
        // =====================================================================
        .route(
            Route::get("/api/v1/schedules")
                .handler("list_schedules")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/schedules")
                .handler("create_schedule")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/schedules/{id}")
                .handler("get_schedule")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/schedules/{id}")
                .handler("update_schedule")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/schedules/{id}/advance")
                .handler("advance_schedule")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/places — Governed spatial entities (DHT projection)
        // =====================================================================
        .route(
            Route::get("/api/v1/places")
                .handler("list_places")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/places")
                .handler("create_place")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/places/{id}")
                .handler("get_place")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/spatial-contexts — Geospatial context (H3 indexed)
        // =====================================================================
        .route(
            Route::get("/api/v1/spatial-contexts")
                .handler("list_spatial_contexts")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/spatial-contexts")
                .handler("create_spatial_context")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/spatial-contexts/{id}")
                .handler("get_spatial_context")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/spatial-contexts/{id}")
                .handler("update_spatial_context")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/spatial-contexts/{id}")
                .handler("delete_spatial_context")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/steward-affinity — Steward affinity lifecycle
        // =====================================================================
        .route(
            Route::get("/api/v1/steward-affinity")
                .handler("list_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity")
                .handler("create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity/bulk")
                .handler("bulk_create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity/curation-event")
                .handler("steward_affinity_curation_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward-affinity/{id}")
                .handler("get_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/resources — REA resource management
        // =====================================================================
        .route(
            Route::get("/api/v1/resources")
                .handler("list_resources")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/resources")
                .handler("create_resource")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/dashboard/{id}")
                .handler("resource_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/constitutional-limits/{id}")
                .handler("constitutional_limits")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/resources/{id}/allocations")
                .handler("allocate_resource")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/resources/{id}/usage")
                .handler("record_resource_usage")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/{id}")
                .handler("get_resource")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/exchange — Requests and offers
        // =====================================================================
        .route(
            Route::get("/api/v1/exchange/requests")
                .handler("list_requests")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/requests")
                .handler("create_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/requests/{id}")
                .handler("get_request")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/exchange/requests/{id}")
                .handler("update_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/exchange/requests/{id}")
                .handler("archive_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/requests/{id}/match")
                .handler("match_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/offers")
                .handler("list_offers")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/offers")
                .handler("create_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/offers/{id}")
                .handler("get_offer")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/exchange/offers/{id}")
                .handler("update_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/exchange/offers/{id}")
                .handler("archive_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/offers/{id}/match")
                .handler("match_offer")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/custodians — Infrastructure custodian metrics
        // =====================================================================
        .route(
            Route::get("/api/v1/custodians/metrics")
                .handler("list_custodian_metrics")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/custodians/metrics")
                .handler("report_custodian_metrics")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/alerts")
                .handler("custodian_alerts")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/{id}")
                .handler("get_custodian_metrics")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/{id}/recommendations")
                .handler("custodian_recommendations")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/protection/{id}")
                .handler("custodian_protection")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/compute — Compute dashboard
        // =====================================================================
        .route(
            Route::get("/api/v1/compute/dashboard")
                .handler("compute_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/compute/dashboard/refresh")
                .handler("refresh_compute_dashboard")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/flow-planning — Resource flow planning
        // =====================================================================
        .route(
            Route::get("/api/v1/flow-planning")
                .handler("list_flow_plans")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/flow-planning")
                .handler("create_flow_plan")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/attestations — Contributor attestations
        // =====================================================================
        .route(
            Route::get("/api/v1/attestations")
                .handler("list_attestations")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/attestations")
                .handler("create_attestation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/attestations/{id}/revoke")
                .handler("revoke_attestation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/attestations/{id}")
                .handler("get_attestation")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/steward — Steward credentials, gates, grants, access
        // =====================================================================
        .route(
            Route::post("/api/v1/steward/credentials")
                .handler("create_steward_credential")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/credentials")
                .handler("list_steward_credentials")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/credentials/{id}")
                .handler("get_steward_credential")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward/gates")
                .handler("create_steward_gate")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/gates")
                .handler("list_steward_gates")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/gates/{id}")
                .handler("get_steward_gate")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/steward/grants")
                .handler("create_steward_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/grants")
                .handler("list_steward_grants")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/grants/{id}")
                .handler("get_steward_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/access")
                .handler("steward_access")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/revenue/{id}")
                .handler("steward_revenue")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/contributors — Contributor dashboards
        // =====================================================================
        .route(
            Route::get("/api/v1/contributors/me/dashboard")
                .handler("my_contributor_dashboard")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/dashboard")
                .handler("contributor_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/impact")
                .handler("contributor_impact")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/recognition")
                .handler("contributor_recognition")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/presence — Contributor presences
        // =====================================================================
        .route(
            Route::get("/api/v1/presence")
                .handler("list_presences")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/presence")
                .handler("create_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/presence/{id}")
                .handler("get_presence")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/api/v1/presence/{id}")
                .handler("delete_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/stewardship")
                .handler("begin_stewardship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/claim")
                .handler("initiate_claim")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/verify-claim")
                .handler("verify_claim")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/identity — Human identity registration
        // =====================================================================
        .route(
            Route::post("/api/v1/identity/register")
                .handler("register_human")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/identity/me")
                .handler("get_me")
                .auth_required()
                .build(),
        )
        .route(
            Route::put("/api/v1/identity/me")
                .handler("update_me")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/agreements — REA agreements
        // =====================================================================
        .route(
            Route::get("/api/v1/agreements")
                .handler("list_agreements")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/agreements")
                .handler("create_agreement")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/agreements/{id}")
                .handler("get_agreement")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/commitments — REA commitments
        // =====================================================================
        .route(
            Route::get("/api/v1/commitments")
                .handler("list_commitments")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/commitments")
                .handler("create_commitment")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/commitments/agent/{agent_id}")
                .handler("commitments_by_agent")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/commitments/{id}")
                .handler("get_commitment")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::patch("/api/v1/commitments/{id}")
                .handler("update_commitment_state")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /db/ — Structured local database (content, paths, relationships, etc.)
        // =====================================================================
        .route(
            Route::get("/db/stats")
                .handler("db_stats")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/schema")
                .handler("db_schema")
                .cache_ttl(3600)
                .build(),
        )
        // Content
        .route(
            Route::get("/db/content")
                .handler("list_content")
                .cache_ttl(300)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            Route::post("/db/content")
                .handler("create_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/content/bulk")
                .handler("bulk_create_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/content/{id}")
                .handler("get_content")
                .cache_ttl(300)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            Route::delete("/db/content/{id}")
                .handler("delete_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/content/{id}/schedule")
                .handler("get_content_schedule")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/content/{id}/schedule")
                .handler("create_content_schedule")
                .auth_required()
                .build(),
        )
        // Knowledge graph relationships
        .route(
            Route::get("/db/relationships")
                .handler("list_relationships")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/relationships")
                .handler("create_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/relationships/bulk")
                .handler("bulk_create_relationships")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/relationships/graph/{content_id}")
                .handler("content_graph")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/relationships/{id}")
                .handler("get_relationship")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::delete("/db/relationships/{id}")
                .handler("delete_relationship")
                .auth_required()
                .build(),
        )
        // Knowledge maps
        .route(
            Route::get("/db/knowledge-maps")
                .handler("list_knowledge_maps")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/knowledge-maps")
                .handler("create_knowledge_map")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/knowledge-maps/{id}")
                .handler("get_knowledge_map")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::put("/db/knowledge-maps/{id}")
                .handler("update_knowledge_map")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/db/knowledge-maps/{id}")
                .handler("delete_knowledge_map")
                .auth_required()
                .build(),
        )
        // Human relationships
        .route(
            Route::get("/db/human-relationships")
                .handler("list_human_relationships")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/human-relationships")
                .handler("create_human_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/human-relationships/{id}")
                .handler("get_human_relationship")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/human-relationships/{id}")
                .handler("delete_human_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/human-relationships/{id}/consent")
                .handler("update_relationship_consent")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/human-relationships/{id}/custody")
                .handler("update_relationship_custody")
                .auth_required()
                .build(),
        )
        // Human directory (qahal)
        .route(
            Route::get("/db/humans")
                .handler("list_humans")
                .cache_ttl(300)
                .build(),
        )
        // Collectives (qahal)
        .route(
            Route::get("/db/collectives")
                .handler("list_collectives")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/collectives")
                .handler("create_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/collectives/{id}")
                .handler("get_collective")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/collectives/{id}/participants")
                .handler("list_collective_participants")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/collectives/{id}/participants")
                .handler("join_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/db/collectives/{id}/participants/{human_id}")
                .handler("depart_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/participations/{human_id}")
                .handler("participations_by_human")
                .cache_ttl(60)
                .build(),
        )
        // Contributor presences
        .route(
            Route::get("/db/presences")
                .handler("list_db_presences")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/presences")
                .handler("create_db_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/bulk")
                .handler("bulk_create_presences")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/presences/{id}")
                .handler("get_db_presence")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/presences/{id}")
                .handler("delete_db_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/stewardship")
                .handler("db_begin_stewardship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/claim")
                .handler("db_initiate_claim")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/verify-claim")
                .handler("db_verify_claim")
                .auth_required()
                .build(),
        )
        // Economic events
        .route(
            Route::get("/db/events")
                .handler("list_db_events")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/events")
                .handler("create_db_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/events/bulk")
                .handler("bulk_create_events")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/events/{id}")
                .handler("get_db_event")
                .cache_ttl(60)
                .build(),
        )
        // Content mastery
        .route(
            Route::get("/db/mastery")
                .handler("list_db_mastery")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/mastery")
                .handler("create_db_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/mastery/bulk")
                .handler("bulk_create_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/mastery/human/{human_id}")
                .handler("mastery_for_human")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/mastery/{id}")
                .handler("get_db_mastery")
                .cache_ttl(60)
                .build(),
        )
        // Stewardship allocations
        .route(
            Route::get("/db/allocations")
                .handler("list_db_allocations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/allocations")
                .handler("create_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/bulk")
                .handler("bulk_create_allocations")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/allocations/content/{content_id}")
                .handler("db_allocations_for_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/allocations/steward/{steward_id}")
                .handler("db_allocations_for_steward")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/allocations/{id}")
                .handler("get_db_allocation")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/allocations/{id}")
                .handler("delete_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/{id}/dispute")
                .handler("dispute_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/{id}/resolve")
                .handler("resolve_db_allocation")
                .auth_required()
                .build(),
        )
        // Delivery peers — discovered peers with delivery capabilities
        // Used by frontend Service Worker for multi-peer app delivery scoring
        .route(
            Route::get("/api/v1/peers/delivery")
                .handler("delivery_peers")
                .cache_ttl(10)
                .build(),
        )
        // Cache stream for projection warm-up (SSE)
        .route(
            Route::get("/api/v1/cache/stream")
                .handler("cache_stream")
                .build(),
        )
        // =====================================================================
        // /api/v1/observations -- Observation Session API
        // =====================================================================
        .route(
            Route::post("/api/v1/observations/begin")
                .handler("observation_begin")
                .build(),
        )
        .route(
            Route::post("/api/v1/observations/{id}/entries")
                .handler("observation_add_entries")
                .build(),
        )
        .route(
            Route::get("/api/v1/observations/{id}/report")
                .handler("observation_report")
                .build(),
        )
        // Blob proxy: doorway caches blobs from /blob/{hash}
        .with_blobs_at("/blob")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_format() {
        let hash = BlobStore::compute_hash(b"test data");
        assert!(hash.starts_with("sha256-"));
        assert_eq!(hash.len(), 7 + 64); // "sha256-" + 64 hex chars
    }

    #[test]
    fn test_manifest_builds() {
        let manifest = build_manifest();
        assert!(!manifest.routes.is_empty());
        assert!(manifest.blob_proxy.is_some());
        // Spot-check a few known routes exist
        let paths: Vec<&str> = manifest.routes.iter().map(|r| r.path.as_str()).collect();
        assert!(paths.contains(&"/db/content"), "missing /db/content");
        assert!(
            paths.contains(&"/api/v1/mastery"),
            "missing /api/v1/mastery"
        );
        assert!(
            paths.contains(&"/api/v1/economic-events"),
            "missing /api/v1/economic-events"
        );
        assert!(
            paths.contains(&"/api/v1/presence"),
            "missing /api/v1/presence"
        );
        // Ensure infrastructure routes are NOT in the manifest
        assert!(
            !paths.contains(&"/health"),
            "health should not be in manifest"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("/shard")),
            "shard routes should not be in manifest"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("/sync")),
            "sync routes should not be in manifest"
        );
        // Verify blob proxy points to /blob
        assert_eq!(manifest.blob_proxy.unwrap().base_path, "/blob");
    }
}
