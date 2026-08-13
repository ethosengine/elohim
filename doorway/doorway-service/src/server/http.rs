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
use crate::config::{Args, OpGateMode};
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
    /// Registry matched a `StorageProxy` route that carries a render spec
    /// (manifest `render` field set). Caller dispatches through the in-process
    /// Renderer; falls back to `StorageProxy` if the renderer is absent.
    SsrRoute { spec: String, endpoint: String },
    /// Registry matched but the target type is not yet handled by dispatch
    /// (BlobProxy, StreamProxy, ZomeCall, AgentProxy). Caller returns 404.
    RegistryUnhandled,
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
/// the registry decides. Otherwise, anything unmatched returns 404. (Root-path
/// dispatch is now handled by the EPR router — `state.epr_router.dispatch(&path)`
/// — which fires before this function, consulting the active projections whose
/// `url_path == "/"` rather than a hardcoded slug env var.)
async fn classify_dispatch(
    registry: &crate::services::RouteRegistry,
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
            // SSR-eligible routes (manifest `render` field set) get their own
            // disposition so the caller can dispatch through the in-process
            // Renderer.  Non-SSR StorageProxy routes use the normal forwarder.
            if let Some(spec) = route.render_spec.as_deref() {
                return Disposition::SsrRoute {
                    spec: spec.to_string(),
                    endpoint: endpoint.to_string(),
                };
            }
            return Disposition::StorageProxy {
                endpoint: endpoint.to_string(),
            };
        }
        // Future: handle ZomeCall, AgentProxy, BlobProxy, StreamProxy targets.
        // For now any non-StorageProxy registry hit returns 404.
        return Disposition::RegistryUnhandled;
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
    /// Private half of the node key above. Retained (T2.2) so the JWT validator
    /// can MINT EdDSA tokens verifiable by siblings when the operator flips
    /// `DOORWAY_JWT_SIGN_ALG=eddsa`; unused on the HS256 default.
    ///
    /// CAVEAT (honest, not hidden): the pair is generated fresh at boot, so a
    /// restart rotates it and any EdDSA token minted before the restart becomes
    /// unverifiable once siblings refresh their JWKS. That is why `hs256`
    /// remains the default — flipping to `eddsa` should wait on a persisted
    /// node key (operator menu item 3, the dual-alg migration window).
    pub node_signing_key: Option<ed25519_dalek::SigningKey>,
    /// Shared `kid` → Ed25519 public key cache for verifying sibling-minted
    /// JWTs (Task 2.1's `PeerJwksCache`). Refreshed by
    /// `spawn_peer_jwks_refresh_task` off the existing peer-discovery cache —
    /// never fetched on the request path.
    pub peer_jwks_cache: crate::services::federation::PeerJwksCache,
    /// ZomeCaller for federation and service registration
    /// Shared by federation service, heartbeat task, and federation routes
    pub zome_caller: Option<Arc<crate::services::ZomeCaller>>,
    /// Cache of peer doorways discovered via HTTP federation
    pub peer_cache: crate::services::federation::PeerCache,
    /// Mutable list of federation peer URLs (seeded from env, mutable via admin API)
    pub peer_url_list: crate::services::federation::PeerUrlList,
    /// Cached P2P health from elohim-storage sidecar (polled every 30s)
    pub p2p_health: Arc<tokio::sync::RwLock<Option<crate::routes::health::P2PHealthSnapshot>>>,
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
    /// Per-app SSR renderers (manifest-driven dispatch, Task 13 → multi-app).
    /// Routes declared `render = "angular-ssr"` select the renderer FOR THE
    /// ROUTE'S PROJECTED APP through `RendererRegistry::select` — one seam for
    /// both the mismatch gate and the lookup, so a renderer is never applied
    /// to another app's route (the /lamad-dark failure: the landing bundle
    /// rendered for lamad URLs, then compose refused the cross-app splice on
    /// every request — one wasted V8 render each). Empty registry = SSR off;
    /// SSR-eligible routes fall back to the normal storage proxy.
    pub renderer_registry: crate::render::registry::RendererRegistry,
    /// Cached render-capability claim derived at startup.
    ///
    /// Served by `GET /admin/capability` so elohim-storage can pull the profile
    /// when polling doorway peers (Task 7: `load_render_capability_from_url`).
    ///
    /// `None` means doorway has no SSR runtime configured or the deriver
    /// returned `None` (e.g. `SSR_BUNDLES_DIR` unset, storage manifest
    /// unreachable, or the bundles directory is empty).
    pub render_capability: Option<crate::render::types::RenderCapabilityProfile>,

    /// Concurrency limiter for SSR rendering. Sized to
    /// `render_capability.max_concurrent_renders` at startup. The dispatch
    /// arm calls `try_acquire_owned()`; on failure (limit reached) the
    /// request returns a CSR-shell fallback with `x-ssr-skipped: overflow`
    /// rather than queueing — fallback is always faster than waiting.
    ///
    /// `None` means no SSR claim was published, so no limiter needed.
    pub render_semaphore: Option<Arc<tokio::sync::Semaphore>>,

    /// In-process aggregate of render-trace outcomes (terminal classification +
    /// latency). The feed-forward seam: served by `GET /admin/render-stats` for
    /// the compute-commitment scorer / capability tuner to read this peer's
    /// `degenerateRate` and `avgWallMs`. Always present (zeroed until first render).
    pub render_trace_stats: Arc<elohim_render::RenderTraceStats>,

    /// SSR render-failure circuit breaker. After `FAILURE_THRESHOLD` consecutive
    /// render errors it OPENS for a cooldown window: `serve_ssr_route` skips the
    /// V8 diversion entirely and serves the fallback tagged
    /// `x-ssr-skipped: error-backoff`, so a systemic render failure (e.g. a
    /// server bundle the runtime cannot load) degrades to today's projected-bundle
    /// serving at full speed instead of taxing every request with a doomed render
    /// first. A single successful render closes it. One global breaker matches the
    /// one-renderer reality — see `crate::render::breaker`.
    pub ssr_render_breaker: crate::render::breaker::SsrRenderBreaker,

    /// Self-hostable pkarr relay service (cutover gate #10).
    /// When `Some`, serves `GET /pkarr/{key}` and `PUT /pkarr/{key}`.
    /// Enabled via `DOORWAY_PKARR_RESOLVER_ENABLED=true`.
    /// `None` means all `/pkarr/*` requests return 404.
    pub pkarr_resolver: Option<Arc<crate::services::pkarr_resolver::PkarrResolverService>>,

    /// EPR projection router — routes incoming URLs to projected EPRs.
    ///
    /// Populated at boot from storage's `/db/rea_commitments?action=project-epr`
    /// endpoint and refreshed by SSE events (`projection.registered` /
    /// `projection.revoked`). Consulted before the main route match block so
    /// pillar URLs (e.g. `/lamad`) serve their cached build without requiring
    /// an explicit match arm per EPR.
    ///
    /// Starts empty. Never fails boot if storage is unreachable — the router
    /// remains empty and SSE events repopulate it when storage comes up.
    ///
    /// See `src/projection/epr_router.rs` (B11) and `src/main.rs` (B12).
    pub epr_router: Arc<crate::projection::EprRouter>,

    /// Materialized `/sitemap.xml` cache (spec §7.5): `(generation, xml)`.
    /// Served when the cached generation still matches `epr_router.generation()`;
    /// otherwise re-materialized from the routing table + commons ids and
    /// recached. Event-invalidated — never time-expired — so it cannot drift
    /// from dispatch.
    pub sitemap_cache: Arc<tokio::sync::RwLock<Option<(u64, String)>>>,

    /// **DEV/FIXTURE ONLY** portal-host health override (host_url → healthy).
    ///
    /// Doorway-local OPERATIONAL state — NOT a projection of any notarized DHT
    /// entry. `portal_hosts` is Category A (the DHT entry is the source of truth);
    /// this map never touches it. It exists purely so a2o fixtures can assert a
    /// portal host as reachable/unreachable WITHOUT standing up a real server at a
    /// non-resolving `.example` origin, which the live HEAD probe could never reach.
    ///
    /// Consulted by `probe_first_portal_host` ONLY when `args.dev_mode` is true:
    ///   - host present, healthy=true  → return the host (no live HEAD)
    ///   - host present, healthy=false → skip the host (no live HEAD)
    ///   - host absent                 → fall back to the real HEAD probe
    ///
    /// When `dev_mode` is false the map is ignored entirely, so production
    /// behaviour is byte-for-byte unchanged. Written via the dev-mode-gated
    /// `PUT /admin/dev/portal-health` route (`routes::admin_dev`).
    pub portal_health_override: Arc<tokio::sync::RwLock<std::collections::HashMap<String, bool>>>,

    /// Global inbound admission limiter (Pillar 2 layer 2). Bounds total
    /// in-flight requests; `try_acquire_owned()` sheds 503+Retry-After when
    /// full. NON-Option (always present, unlike render_semaphore). Sized from
    /// DOORWAY_MAX_INFLIGHT at boot (main.rs). Liveness/version/WS are exempt
    /// (admission_exempt) so /health stays answerable while shedding.
    pub inbound_semaphore: Arc<tokio::sync::Semaphore>,

    /// Read-admission pool (GET/HEAD) — separate from `inbound_semaphore`
    /// (writes) so a write burst can't starve projection reads (genesis #1272
    /// `/epr/*` shed). Sized from DOORWAY_MAX_INFLIGHT_READ at boot (main.rs).
    pub read_semaphore: Arc<tokio::sync::Semaphore>,

    /// ONE pooled HTTP client for the storage proxy (connect 3s / request 12s).
    /// Replaces per-request `reqwest::Client::new()` in forward_to_storage.
    pub storage_proxy_client: Arc<reqwest::Client>,

    /// Per-upstream circuit breakers for the storage proxy (Pillar 2 layer 4).
    pub upstream_breakers: Arc<crate::routes::UpstreamBreakers>,

    // ── Membrane policy (Pillar 2 layer 3) ────────────────────────────────────
    /// Per-source rate-limit store — evicting in-memory implementation.
    /// Guarded by a std Mutex so the non-async `assess` call can hold it;
    /// the lock is ALWAYS dropped before any `.await` (Shape sleep).
    pub membrane_guard: std::sync::Mutex<crate::server::membrane::EdgeGuardStore>,

    /// Threshold config for the membrane stage (shape / challenge / ban).
    pub membrane_cfg: elohim_peer_fabric::guard::GuardConfig,

    /// Number of trusted proxy hops from `DOORWAY_TRUSTED_PROXY_HOPS`
    /// (default 1 for single-ingress topology). Used to pick the rightmost-
    /// untrusted IP from X-Forwarded-For.
    pub trusted_proxy_hops: usize,
}

/// Retry-After (seconds) advertised when doorway sheds an inbound request at
/// the admission ceiling. Short — admission saturation is a transient burst.
const DOORWAY_ADMISSION_RETRY_AFTER_SECS: u64 = 2;

/// Inbound admission ceiling default (≈4× storage's 64 permits for a 4-worker
/// projection edge) and floor (anti-deadlock: a ceiling can never be 0). HOME
/// is here (not main.rs): the AppState ctors below reference DEFAULT_MAX_INFLIGHT,
/// and a lib cannot import a const from the bin. main.rs `use`s these.
pub const DEFAULT_MAX_INFLIGHT: usize = 256;
pub const MIN_MAX_INFLIGHT: usize = 8;
/// Read-admission ceiling default — larger than the write pool: reads are cheap
/// projection serves and must not shed while a write burst fills the write pool.
pub const DEFAULT_MAX_INFLIGHT_READ: usize = 512;

/// Build the ONE pooled client the storage proxy uses for forward_to_storage /
/// forward_blob_to_storage. Replaces the per-request `reqwest::Client::new()`
/// (untimed) so a hung upstream cannot block a worker indefinitely.
fn init_storage_proxy_client() -> Arc<reqwest::Client> {
    Arc::new(
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(
                crate::routes::storage_proxy::STORAGE_PROXY_CONNECT_TIMEOUT_SECS,
            ))
            .timeout(std::time::Duration::from_secs(
                crate::routes::storage_proxy::STORAGE_PROXY_REQUEST_TIMEOUT_SECS,
            ))
            .build()
            .unwrap_or_default(),
    )
}

/// Return the instance-scoped identity used to suppress signal-bus echo.
///
/// `doorway_id` identifies the interchangeable logical doorway and is shared by
/// its replicas. Using it here would make sibling replicas mistake each
/// other's frames for their own. `node_id` is minted per gateway instance, so
/// it is the correct origin identity for the operational signal bus.
fn signal_relay_id(args: &Args) -> String {
    args.node_id.to_string()
}

#[cfg(test)]
mod signal_relay_identity_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn sibling_replicas_of_one_doorway_have_distinct_signal_origins() {
        let first = Args::try_parse_from([
            "doorway",
            "--node-id",
            "00000000-0000-0000-0000-000000000001",
            "--doorway-id",
            "doorway:shared",
        ])
        .expect("first replica args must parse");
        let second = Args::try_parse_from([
            "doorway",
            "--node-id",
            "00000000-0000-0000-0000-000000000002",
            "--doorway-id",
            "doorway:shared",
        ])
        .expect("second replica args must parse");

        assert_eq!(first.doorway_id, second.doorway_id);
        assert_ne!(signal_relay_id(&first), signal_relay_id(&second));
        assert_eq!(
            signal_relay_id(&first),
            "00000000-0000-0000-0000-000000000001"
        );
    }
}

impl AppState {
    /// Build a **federation-aware** `JwtValidator` — the live wiring seam for
    /// Task 2.1's cross-doorway trust core.
    ///
    /// Every builder method is opt-in, so this returns exactly today's
    /// behaviour when nothing federation-related is configured:
    /// - `with_doorway_id` — enables self-vs-foreign `kid` comparison. Without
    ///   it every `kid`-bearing token reads as foreign.
    /// - `with_eddsa_signing_key` / `with_eddsa_verifying_key` — the node key
    ///   already published at `/.well-known/doorway-keys`; never a fresh key.
    /// - `with_sign_alg` — `DOORWAY_JWT_SIGN_ALG`, default `hs256`. Verification
    ///   is dual-algorithm regardless; this only controls MINTING.
    /// - `with_peer_key_lookup` — the `PeerJwksCache`. Without it, a foreign
    ///   `kid` fails closed to `UnknownIssuer` rather than falling through to
    ///   the HS256 secret.
    ///
    /// Returns `None` when JWT is not configured at all (no dev mode, no
    /// secret) so callers keep their existing "JWT not configured" branch.
    pub fn federation_jwt_validator(&self) -> Option<crate::auth::JwtValidator> {
        let base = if self.args.dev_mode {
            crate::auth::JwtValidator::new_dev()
        } else {
            let secret = self.args.jwt_secret.as_ref()?;
            crate::auth::JwtValidator::new(secret.clone(), self.args.jwt_expiry_seconds).ok()?
        };

        let mut validator = base
            .with_sign_alg(crate::auth::jwt::JwtSignAlg::from_process_env())
            .with_peer_key_lookup(std::sync::Arc::new(self.peer_jwks_cache.clone()));

        if let Some(ref id) = self.args.doorway_id {
            validator = validator.with_doorway_id(id.clone());
        }
        match (
            self.node_signing_key.as_ref(),
            self.node_verifying_key.as_ref(),
        ) {
            (Some(sk), Some(vk)) => {
                validator = validator.with_eddsa_signing_key(sk.clone(), *vk);
            }
            (None, Some(vk)) => {
                validator = validator.with_eddsa_verifying_key(*vk);
            }
            _ => {}
        }

        Some(validator)
    }

    /// Create AppState without external services (dev mode, direct proxy)
    pub fn new(args: Args) -> Self {
        let bootstrap = if args.bootstrap_enabled {
            // Dev/direct-proxy mode: no mongo client → always the mem k2 store.
            Some(Arc::new(BootstrapStore::new(None)))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            let relay_id = signal_relay_id(&args);
            Some(Arc::new(SignalStore::new(max_clients, None, relay_id)))
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
            node_signing_key: None,
            peer_jwks_cache: crate::services::federation::new_peer_jwks_cache(),
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
            renderer_registry: crate::render::registry::RendererRegistry::from_env(),
            render_trace_stats: Arc::new(elohim_render::RenderTraceStats::new()),
            ssr_render_breaker: crate::render::breaker::SsrRenderBreaker::new(),
            render_capability: None,
            render_semaphore: None,
            pkarr_resolver: None,
            epr_router: Arc::new(crate::projection::EprRouter::new()),
            sitemap_cache: Arc::new(tokio::sync::RwLock::new(None)),
            portal_health_override: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            inbound_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT)),
            read_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT_READ)),
            storage_proxy_client: init_storage_proxy_client(),
            upstream_breakers: Arc::new(crate::routes::UpstreamBreakers::default()),
            membrane_cfg: crate::server::membrane::edge_guard_config(),
            membrane_guard: std::sync::Mutex::new(crate::server::membrane::EdgeGuardStore::new(
                crate::server::membrane::edge_guard_config().window_secs,
            )),
            trusted_proxy_hops: crate::server::membrane::trusted_proxy_hops_from_env(),
        }
    }

    /// Create AppState with services but no worker pool (direct proxy mode)
    ///
    /// Projection store is initialized in memory-only mode. Use `init_projection()`
    /// to upgrade to MongoDB-backed projection after async initialization.
    pub fn with_services(args: Args, mongo: Option<MongoClient>, nats: Option<NatsClient>) -> Self {
        let router = HostRouter::new(nats.clone());
        let bootstrap = if args.bootstrap_enabled {
            Some(Arc::new(BootstrapStore::new(mongo.as_ref())))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            let relay_id = signal_relay_id(&args);
            Some(Arc::new(SignalStore::new(
                max_clients,
                mongo.as_ref(),
                relay_id,
            )))
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
            node_signing_key: None,
            peer_jwks_cache: crate::services::federation::new_peer_jwks_cache(),
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
            renderer_registry: crate::render::registry::RendererRegistry::from_env(),
            render_trace_stats: Arc::new(elohim_render::RenderTraceStats::new()),
            ssr_render_breaker: crate::render::breaker::SsrRenderBreaker::new(),
            render_capability: None,
            render_semaphore: None,
            pkarr_resolver: None,
            epr_router: Arc::new(crate::projection::EprRouter::new()),
            sitemap_cache: Arc::new(tokio::sync::RwLock::new(None)),
            portal_health_override: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            inbound_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT)),
            read_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT_READ)),
            storage_proxy_client: init_storage_proxy_client(),
            upstream_breakers: Arc::new(crate::routes::UpstreamBreakers::default()),
            membrane_cfg: crate::server::membrane::edge_guard_config(),
            membrane_guard: std::sync::Mutex::new(crate::server::membrane::EdgeGuardStore::new(
                crate::server::membrane::edge_guard_config().window_secs,
            )),
            trusted_proxy_hops: crate::server::membrane::trusted_proxy_hops_from_env(),
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
            Some(Arc::new(BootstrapStore::new(mongo.as_ref())))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            let relay_id = signal_relay_id(&args);
            Some(Arc::new(SignalStore::new(
                max_clients,
                mongo.as_ref(),
                relay_id,
            )))
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
            node_signing_key: None,
            peer_jwks_cache: crate::services::federation::new_peer_jwks_cache(),
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
            renderer_registry: crate::render::registry::RendererRegistry::from_env(),
            render_trace_stats: Arc::new(elohim_render::RenderTraceStats::new()),
            ssr_render_breaker: crate::render::breaker::SsrRenderBreaker::new(),
            render_capability: None,
            render_semaphore: None,
            pkarr_resolver: None,
            epr_router: Arc::new(crate::projection::EprRouter::new()),
            sitemap_cache: Arc::new(tokio::sync::RwLock::new(None)),
            portal_health_override: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            inbound_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT)),
            read_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT_READ)),
            storage_proxy_client: init_storage_proxy_client(),
            upstream_breakers: Arc::new(crate::routes::UpstreamBreakers::default()),
            membrane_cfg: crate::server::membrane::edge_guard_config(),
            membrane_guard: std::sync::Mutex::new(crate::server::membrane::EdgeGuardStore::new(
                crate::server::membrane::edge_guard_config().window_secs,
            )),
            trusted_proxy_hops: crate::server::membrane::trusted_proxy_hops_from_env(),
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
            Some(Arc::new(BootstrapStore::new(Some(&mongo))))
        } else {
            None
        };
        let signal = if args.signal_enabled {
            let max_clients = args.signal_max_clients.unwrap_or(DEFAULT_MAX_CLIENTS);
            let relay_id = signal_relay_id(&args);
            Some(Arc::new(SignalStore::new(
                max_clients,
                Some(&mongo),
                relay_id,
            )))
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
            node_signing_key: None,
            peer_jwks_cache: crate::services::federation::new_peer_jwks_cache(),
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
            renderer_registry: crate::render::registry::RendererRegistry::from_env(),
            render_trace_stats: Arc::new(elohim_render::RenderTraceStats::new()),
            ssr_render_breaker: crate::render::breaker::SsrRenderBreaker::new(),
            render_capability: None,
            render_semaphore: None,
            pkarr_resolver: None,
            epr_router: Arc::new(crate::projection::EprRouter::new()),
            sitemap_cache: Arc::new(tokio::sync::RwLock::new(None)),
            portal_health_override: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            inbound_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT)),
            read_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT_READ)),
            storage_proxy_client: init_storage_proxy_client(),
            upstream_breakers: Arc::new(crate::routes::UpstreamBreakers::default()),
            membrane_cfg: crate::server::membrane::edge_guard_config(),
            membrane_guard: std::sync::Mutex::new(crate::server::membrane::EdgeGuardStore::new(
                crate::server::membrane::edge_guard_config().window_secs,
            )),
            trusted_proxy_hops: crate::server::membrane::trusted_proxy_hops_from_env(),
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
/// Auth posture of an incoming request — what kind of session backs it
/// (or `Anonymous` if unauthenticated). Mirrors the protocol's `authModes`
/// enum exactly so the auth-mode check is a direct membership test against
/// `RenderCapabilityProfile.auth_modes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthPosture {
    Anonymous,
    DoorwayHosted,
    StewardPresence,
}

impl AuthPosture {
    /// Wire-shape string matching the protocol enum.
    pub fn as_claim_str(&self) -> &'static str {
        match self {
            AuthPosture::Anonymous => "anonymous",
            AuthPosture::DoorwayHosted => "doorway-hosted",
            AuthPosture::StewardPresence => "steward-presence",
        }
    }
}

/// Determine an incoming request's auth posture from its headers.
/// Authorization (Bearer / API key) → `DoorwayHosted` (the protocol JWT
/// flow today). Cookie containing `steward_attestation=` → `StewardPresence`
/// (forward-compat for M5 auth-portal-convergence). Cookie containing
/// `doorway_session=` → `DoorwayHosted`. Anything else → `Anonymous`.
pub fn determine_auth_posture<B>(req: &Request<B>) -> AuthPosture {
    if req.headers().get(hyper::header::AUTHORIZATION).is_some() {
        return AuthPosture::DoorwayHosted;
    }
    if let Some(cookie) = req.headers().get(hyper::header::COOKIE) {
        if let Ok(s) = cookie.to_str() {
            if s.contains("steward_attestation=") {
                return AuthPosture::StewardPresence;
            }
            if s.contains("doorway_session=") {
                return AuthPosture::DoorwayHosted;
            }
        }
    }
    AuthPosture::Anonymous
}

/// Build a `UserCredential` for the V8 SSR fetch shim from the originating
/// HTTP request's headers. Returns None for anonymous requests.
///
/// Order: Authorization header (Bearer / API key) wins; falls back to a
/// Cookie header containing a known doorway-session marker.
/// `steward_attestation=` is also recognised as a forward-compat hook for
/// the M5 auth-portal-convergence sprint; today the session layer doesn't
/// produce that cookie shape but the shim is wire-ready.
fn build_ssr_user_credential<B>(req: &Request<B>) -> Option<crate::ssr::UserCredential> {
    if let Some(auth) = req.headers().get(hyper::header::AUTHORIZATION) {
        if let Ok(value) = auth.to_str() {
            return Some(crate::ssr::UserCredential {
                header_name: "Authorization".into(),
                header_value: value.into(),
            });
        }
    }
    if let Some(cookie) = req.headers().get(hyper::header::COOKIE) {
        if let Ok(value) = cookie.to_str() {
            if value.contains("doorway_session=") || value.contains("steward_attestation=") {
                return Some(crate::ssr::UserCredential {
                    header_name: "Cookie".into(),
                    header_value: value.into(),
                });
            }
        }
    }
    None
}

/// Build the inline JSON context the native omnibar element reads (the doorway
/// inject path). Constructs and serializes a [`elohim_render::ChromeContext`]
/// — the single typed producer struct shared with the element's `ctx.<field>`
/// consumer contract (`elohim/elohim-chrome-asset/src/omni-element.js`),
/// guarded by that crate's structural contract test. The doorway supplies what
/// the element cannot infer or wants authoritatively client-side:
/// - `slug` — the EPR address, derived from the request path (root → the landing
///   slug `elohim-host-landing`, else the last non-empty path segment), so the
///   element doesn't have to re-derive it from `window.location`.
/// - `authenticated` — whether this request carried a session/credential; the
///   element cannot determine this client-side, and it gates the account link.
///
/// `theme`/`envTier`/`buildMarker`/`nav` are intentionally LEFT to the element's
/// own client-side resolution — the doorway SSR path does not yet plumb
/// env-tier/build-marker, and inventing it here would be speculative. The
/// element fetches `/db/content/{slug}` for the theme and — on first expand —
/// resolves its NAV and the EPR's backing claims from the Category-C EPR
/// projections `GET /api/v1/epr/{slug}/{nav-context,raw}` (proxied by this same
/// doorway). Nav is SETTLED there, not a gap: the doorway once deferred nav to
/// the element while the element deferred it back to the doorway, so the links
/// never rendered; client-side resolution wins. The
/// element's defaults are sensible for the unspecified fields, so a minimal,
/// honest island is correct — `ChromeContext`'s `Option` fields stay `None` and
/// are omitted from the wire JSON (`skip_serializing_if`), keeping the shape
/// identical to the hand-built `serde_json::json!` this replaces.
fn build_chrome_context_json<B>(path: &str, req: &Request<B>) -> String {
    const LANDING_SLUG: &str = "elohim-host-landing";
    let trimmed = path.trim_end_matches('/');
    let slug = if trimmed.is_empty() {
        LANDING_SLUG.to_string()
    } else {
        trimmed
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or(LANDING_SLUG)
            .to_string()
    };
    let authenticated = build_ssr_user_credential(req).is_some();
    elohim_render::ChromeContext {
        slug,
        authenticated,
        ..Default::default()
    }
    .to_json()
}

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

/// Extract the raw Bearer token string from an `Authorization: Bearer <token>` header.
///
/// Returns `None` when the header is absent or not a Bearer scheme.
/// Used by the op-gate to forward the user's own credential to storage.
fn extract_bearer_from_req<B>(req: &Request<B>) -> Option<String> {
    let header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(|t| t.trim().to_string())
}

// ── delegates-compute op-gate ─────────────────────────────────────────────────
//
// Pre-dispatch gate for `POST /db/content` and `POST /db/content/bulk`
// (both routes invoke `distribute_shards` — the compute-commitment surface).
//
// Gate performs a direct call to storage's internal `POST /api/v1/authorize-operation`
// (NOT proxied through doorway's public surface — that would create a verdict-oracle
// DoS vector per Review C6). The endpoint is reached on the same `endpoint` URL the
// StorageProxy disposition carries (the internal sidecar address).
//
// Decision matrix:
//   Off     → forward without calling storage (no latency, no I/O)
//   Observe → call storage, log result, always forward
//   Enforce → call storage; allowed → forward; denied|error → 403 (fail-closed)

/// Response body from storage's `POST /api/v1/authorize-operation`.
///
/// camelCase per the Rust-to-TS boundary (storage uses `#[serde(rename_all = "camelCase")]`).
/// We only need `allowed` and `reason`; `commitmentCid` is intentionally ignored here.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizeVerdict {
    allowed: bool,
    reason: String,
}

/// Outcome of the pre-dispatch op-gate decision for one request.
///
/// Pure over `(mode, method, path, verdict)` — extracted so the full matrix is
/// unit-testable without any I/O or running services.
#[derive(Debug, PartialEq, Eq)]
enum OpGateDecision {
    /// Allow the request to proceed to storage.
    Allow,
    /// Allow, but emit a tracing warning (Observe mode: would-have-denied).
    AllowWithWarn,
    /// Deny with generic 403 (Enforce mode: denied verdict or infra failure).
    Deny403,
}

/// Compute the op-gate decision from `(mode, method, path, verdict)`.
///
/// This is the pure decision kernel — no I/O, no state.
///
/// `verdict`: `Ok(true)` = allowed, `Ok(false)` = denied, `Err(_)` = infra failure.
/// In `Enforce` mode, both `Ok(false)` and `Err(_)` produce `Deny403` (fail-closed).
/// The gate only fires for `POST /db/content` and `POST /db/content/bulk`.
fn compute_op_gate_decision(
    mode: &OpGateMode,
    method: &Method,
    path: &str,
    verdict: &Result<bool, String>,
) -> OpGateDecision {
    // Only POST to the two content-write routes triggers the gate.
    let is_gated_write =
        method == Method::POST && (path == "/db/content" || path == "/db/content/bulk");
    if !is_gated_write || matches!(mode, OpGateMode::Off) {
        return OpGateDecision::Allow;
    }
    match mode {
        OpGateMode::Off => OpGateDecision::Allow, // unreachable — filtered above
        OpGateMode::Observe => OpGateDecision::AllowWithWarn,
        OpGateMode::Enforce => {
            if matches!(verdict, Ok(true)) {
                OpGateDecision::Allow
            } else {
                OpGateDecision::Deny403
            }
        }
    }
}

/// Outcome of the no-verified-credential branch of the op-gate.
///
/// `compute_op_gate_decision` never sees the credential — the credential check
/// is a control-flow decision in the dispatch path. This tiny enum makes that
/// branch unit-testable in isolation.
#[derive(Debug, PartialEq, Eq)]
enum NoCredentialDecision {
    /// Forward to storage without authorizing (no performer to authorize).
    Forward,
    /// Deny with a generic 403 (fail-closed).
    Deny,
}

/// Decide the no-credential case from the mode alone.
///
/// A `POST /db/content*` with no doorway-verified performer has nothing to
/// authorize. The mode still governs the outcome: `Observe` is a non-blocking
/// shadow stage (plan D2 — log the would-deny, never block) so it forwards;
/// `Enforce` fails closed with a 403. `Off` never reaches the gate (filtered by
/// the outer guard) but maps to `Forward` for totality.
fn no_credential_decision(mode: &OpGateMode) -> NoCredentialDecision {
    match mode {
        OpGateMode::Enforce => NoCredentialDecision::Deny,
        OpGateMode::Observe | OpGateMode::Off => NoCredentialDecision::Forward,
    }
}

/// Call storage's `POST /api/v1/authorize-operation` and return the verdict.
///
/// Uses the doorway's pooled `storage_proxy_client`. The `storage_endpoint` is the
/// internal base URL (e.g. `http://elohim-storage:8090`) — NOT the public doorway
/// proxy path (that would expose a verdict-oracle, Review C6).
///
/// The user's own Bearer token is forwarded (Review D6: storage can log the requester
/// identity on its side without the doorway re-issuing a service credential).
///
/// Any transport or parse failure is returned as `Err(String)`. The caller treats
/// this as a denial in Enforce mode (fail-closed) and a logged warning in Observe.
async fn call_authorize_operation(
    client: &reqwest::Client,
    bearer: Option<&str>,
    performer: &str,
    capability: &str,
    storage_endpoint: &str,
) -> Result<AuthorizeVerdict, String> {
    let url = format!(
        "{}/api/v1/authorize-operation",
        storage_endpoint.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "performer": performer,
        "capability": capability,
        // Synthetic/conservative reach for Slice 1: the content's actual reach
        // isn't plumbed through the gate yet, so we claim `commons` — the highest
        // rank, hence the strictest ceiling check (deny-safe until real reach lands).
        "reach": "commons",
        "targetEprId": null,
    });
    let mut req_builder = client.post(&url).json(&body);
    if let Some(token) = bearer {
        req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
    }
    let resp = req_builder
        .send()
        .await
        .map_err(|e| format!("transport error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "authorize-operation returned HTTP {}",
            resp.status()
        ));
    }
    resp.json::<AuthorizeVerdict>()
        .await
        .map_err(|e| format!("parse error: {e}"))
}

/// Generic 403 response for op-gate denials.
///
/// The body is intentionally vague — the denial reason is logged server-side only
/// (Review C5/security: commitment internals must not be echoed to the client).
fn make_op_gate_forbidden() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            r#"{"error":"operation not authorized"}"#,
        )))
        .unwrap()
}

// ── end op-gate helpers ───────────────────────────────────────────────────────

/// Heartbeat interval (ms): how often the MAIN runtime stamps the liveness
/// heartbeat. Must be well below `HEALTH_STALE_MS_DEFAULT` so a healthy runtime
/// keeps the heartbeat fresh between watchdog reads.
const HEARTBEAT_INTERVAL_MS: u64 = 2_000;

/// Default staleness threshold (ms): how long the MAIN runtime may go without
/// stamping the heartbeat before the watchdog declares it WEDGED and fails the
/// liveness probe (so kubelet restarts the pod). Generous enough to ride out a
/// transient full-pool park, far below the kubelet liveness budget. Override via
/// `DOORWAY_HEALTH_STALE_MS`.
const HEALTH_STALE_MS_DEFAULT: u64 = 15_000;

fn health_stale_threshold_ms() -> u64 {
    std::env::var("DOORWAY_HEALTH_STALE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(HEALTH_STALE_MS_DEFAULT)
}

/// Pure liveness verdict: the MAIN runtime is WEDGED when it has not stamped the
/// heartbeat within `threshold_ms`. Unit-testable without a runtime.
fn main_runtime_wedged(heartbeat_age_ms: u64, threshold_ms: u64) -> bool {
    heartbeat_age_ms > threshold_ms
}

/// Spawn the liveness heartbeat task on the MAIN runtime. It stamps `heartbeat`
/// with elapsed-millis-since-`start` every `HEARTBEAT_INTERVAL_MS`. If the main
/// worker pool is fully wedged (e.g. every worker parked in a blocking syscall)
/// this task cannot run, the stamp goes stale, and the watchdog — running on its
/// own OS thread — fails the liveness probe so kubelet restarts the pod.
fn spawn_liveness_heartbeat(
    start: std::time::Instant,
    heartbeat: Arc<std::sync::atomic::AtomicU64>,
) {
    tokio::spawn(async move {
        let mut tick =
            tokio::time::interval(std::time::Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        loop {
            tick.tick().await;
            heartbeat.store(
                start.elapsed().as_millis() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    });
}

/// Build the watchdog liveness response from the heartbeat age: 200 while the
/// MAIN runtime is making progress, 503 once it is wedged past `threshold_ms`.
fn watchdog_liveness_response(
    start: std::time::Instant,
    heartbeat: &std::sync::atomic::AtomicU64,
    threshold_ms: u64,
) -> Response<Full<Bytes>> {
    let last = heartbeat.load(std::sync::atomic::Ordering::Relaxed);
    let age = (start.elapsed().as_millis() as u64).saturating_sub(last);
    let wedged = main_runtime_wedged(age, threshold_ms);
    let status = if wedged {
        // The proximate trigger of the doorway self-kill restart (RCA Theory 3:
        // WEDGED precedes each restart). Record it for the durable /metrics
        // surface AND leave a Loki trace — because during a FATAL wedge the main
        // listener can't answer /metrics, so this warn! is the only signal home
        // besides the scrape target's `up == 0`.
        crate::metrics::inc_watchdog_wedged();
        warn!(
            heartbeat_age_ms = age,
            threshold_ms, "watchdog WEDGED — MAIN runtime heartbeat stale (liveness 503)"
        );
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    let body = serde_json::json!({
        "healthy": !wedged,
        "check": "main-runtime-heartbeat",
        "heartbeatAgeMs": age,
        "thresholdMs": threshold_ms,
    })
    .to_string();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .expect("infallible watchdog liveness response")
}

/// The **honest layered degraded page** served at `/` when the doorway has no root
/// projection — i.e. the EprRouter is empty because the storage/conductor backend is
/// unreachable (the connect-refused outage).
///
/// Replaces both the prior `302 → /threshold` (which leaked the operator dashboard to
/// every public visitor) AND a blunt content-less 503. The doorway must be CLEAR about
/// what it can see *at which layer* (operator framing, 2026-06-22):
///   - **Layer 1 — this doorway / web2 projection** (federation peers, projection cache,
///     MongoDB): the doorway sees this directly and reports it.
///   - **Layer 2 — the substrate dataplane** (Holochain DHT / libp2p / iroh, via the
///     conductor): NOT VISIBLE when the conductor connection is refused. So
///     humans-served / content-available / peer-count are **UNKNOWN, not zero**
///     (unmeasured ≠ zero — `backlog-resilience-unmeasured-vs-zero-honest-denominators`).
///
/// Served 503 + `Retry-After` (the requested content is genuinely unavailable, so
/// clients/crawlers/caches back off) but with a rich honest body, not a bare 503. A true
/// content-less 503 is the last resort, for when even Layer 1 is broken.
fn root_unavailable_html(routes_loaded: usize, generation: u64, federation_peers: usize) -> String {
    let projection_line = if routes_loaded == 0 {
        "Projection cache: <strong>empty</strong> — no routes loaded from the storage backend yet."
            .to_string()
    } else {
        format!("Projection cache: {routes_loaded} route(s) loaded (generation {generation}).")
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>elohim.host — reconnecting to the substrate</title>
<style>
  :root {{ color-scheme: dark; }}
  html,body {{ height:100%; margin:0; }}
  body {{ display:flex; align-items:center; justify-content:center;
    background:#0e1726; color:#e8eef7;
    font:16px/1.6 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif; padding:2rem; }}
  main {{ max-width:40rem; }}
  .glyph {{ font-size:2.5rem; text-align:center; }}
  h1 {{ font-size:1.4rem; font-weight:600; margin:1rem 0 .25rem; color:#f3c98b; text-align:center; }}
  .sub {{ color:#aebacb; text-align:center; margin:0 0 1.75rem; }}
  .layer {{ border:1px solid #243349; border-radius:.6rem; padding:.85rem 1.1rem; margin:.6rem 0; }}
  .layer h2 {{ font-size:.78rem; text-transform:uppercase; letter-spacing:.05em; margin:0 0 .4rem; color:#7f8fa6; font-weight:600; }}
  .up {{ color:#8fd19e; }} .unknown {{ color:#f0a868; }}
  .layer p {{ margin:.18rem 0; color:#cdd6e4; font-size:.95rem; }}
  .op {{ margin-top:1.75rem; font-size:.85rem; color:#5d6b80; text-align:center; }}
  .op a {{ color:#7f8fa6; }}
</style>
</head>
<body>
<main>
  <div class="glyph">✺</div>
  <h1>This corner of the network is catching its breath</h1>
  <p class="sub">The doorway is up &mdash; but it can't see the substrate right now.
     Here's exactly what is, and isn't, visible from here.</p>

  <div class="layer">
    <h2>This doorway &middot; web2 projection <span class="up">&#9679; up</span></h2>
    <p>Federated doorway peers: {federation_peers}</p>
    <p>{projection_line}</p>
  </div>

  <div class="layer">
    <h2>Substrate dataplane &middot; Holochain DHT / libp2p / iroh <span class="unknown">&#9679; not visible</span></h2>
    <p>The conductor connection could not be established, so the peer-to-peer dataplane
       cannot be read from this doorway.</p>
    <p><strong>Humans served, content available, peer count: unknown</strong> &mdash;
       unmeasured, <em>not zero</em>. This doorway cannot see the substrate right now;
       these are unseen from here, not absent.</p>
  </div>

  <p class="op">Reconnecting automatically&hellip; &nbsp;&middot;&nbsp;
     <a href="/threshold">operators &rarr; doorway dashboard</a></p>
</main>
<script>setTimeout(function(){{location.reload();}}, 30000);</script>
</body>
</html>"#
    )
}

fn root_unavailable_response(
    routes_loaded: usize,
    generation: u64,
    federation_peers: usize,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Retry-After", "30")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(root_unavailable_html(
            routes_loaded,
            generation,
            federation_peers,
        ))))
        .expect("infallible root-unavailable response")
}

#[cfg(test)]
mod root_unavailable_tests {
    use super::*;

    #[test]
    fn root_unavailable_is_503_designed_page_not_a_dashboard_redirect() {
        let resp = root_unavailable_response(0, 0, 0);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers()
                .get("Content-Type")
                .map(|v| v.to_str().unwrap()),
            Some("text/html; charset=utf-8")
        );
        assert!(resp.headers().get("Retry-After").is_some());
        // Crucially NOT a redirect to the operator dashboard (the bug this replaces).
        assert!(resp.headers().get("Location").is_none());
    }

    #[test]
    fn root_unavailable_html_is_layered_and_honest_not_zero() {
        let html = root_unavailable_html(0, 0, 0);
        // Layered framing present.
        assert!(html.contains("web2 projection"));
        assert!(html.contains("Holochain DHT / libp2p / iroh"));
        assert!(html.contains("not visible"));
        // The unmeasured-≠-zero honesty (the whole point — never claims "0").
        assert!(html.contains("unknown"));
        assert!(html.contains("not zero"));
        // An empty projection cache is named as empty, not silently a count.
        assert!(html.contains("empty"));
    }
}

/// Minimal health-probe dispatch for the dedicated health WATCHDOG listener.
///
/// Serves ONLY the liveness/readiness/startup probes, with no CORS, no gate, no
/// route-registry lookup, and no blocking work. `/health` (liveness) reflects
/// whether the MAIN runtime is scheduling (heartbeat-gated), NOT this listener's
/// own liveness — so it tells the truth even though it answers from a separate
/// OS-thread runtime. `/ready` and `/startup` delegate to the normal handlers
/// (cached-state reads). Anything else 404s.
async fn handle_watchdog_probe(
    state: Arc<AppState>,
    start: std::time::Instant,
    heartbeat: Arc<std::sync::atomic::AtomicU64>,
    threshold_ms: u64,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/health") | (&Method::GET, "/healthz") => {
            watchdog_liveness_response(start, &heartbeat, threshold_ms)
        }
        (&Method::GET, "/health/startup") => routes::startup_check(Arc::clone(&state)).await,
        (&Method::GET, "/ready") | (&Method::GET, "/readyz") => {
            routes::readiness_check(Arc::clone(&state))
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from_static(
                br#"{"error":"health watchdog serves only /health, /ready, /health/startup"}"#,
            )))
            .expect("infallible 404"),
    }
}

#[cfg(test)]
mod watchdog_tests {
    use super::*;

    #[test]
    fn wedged_only_past_threshold() {
        assert!(!main_runtime_wedged(0, 15_000));
        assert!(!main_runtime_wedged(15_000, 15_000)); // boundary ⇒ alive
        assert!(main_runtime_wedged(15_001, 15_000));
    }

    #[test]
    fn liveness_200_when_heartbeat_fresh() {
        let start = std::time::Instant::now();
        let hb = std::sync::atomic::AtomicU64::new(start.elapsed().as_millis() as u64);
        assert_eq!(
            watchdog_liveness_response(start, &hb, 60_000).status(),
            StatusCode::OK
        );
    }

    #[test]
    fn liveness_503_when_heartbeat_stale() {
        let start = std::time::Instant::now();
        let hb = std::sync::atomic::AtomicU64::new(0); // never stamped
        std::thread::sleep(std::time::Duration::from_millis(5));
        // 5ms elapsed, heartbeat still 0, threshold 1ms ⇒ wedged.
        assert_eq!(
            watchdog_liveness_response(start, &hb, 1).status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}

/// Spawn the dedicated health WATCHDOG: a probe listener on its OWN OS thread
/// with its OWN pinned tokio runtime, isolated from the main worker pool. Opt-in
/// via `DOORWAY_HEALTH_PORT` (unset ⇒ probes stay on the main listener).
///
/// This is the durable form the older shared-runtime listener's FOOTGUN comment
/// prescribed. Two properties make it a TRUE watchdog rather than a probe that
/// lies:
///   1. Its own OS-thread runtime means a FULL main-runtime worker stall (every
///      worker parked — e.g. blocking `getaddrinfo` during a conductor DNS flap,
///      the 2026-06-15 crashloop root) can never silence the probe listener.
///   2. The `/health` verdict is gated on a heartbeat the MAIN runtime stamps
///      (`spawn_liveness_heartbeat`). A genuinely wedged main runtime still FAILS
///      liveness → kubelet restarts it (restart-on-hang preserved); a transient
///      park the runtime recovers from does NOT trip a kill.
///
/// Point the manifest's livenessProbe at this port; keep readiness/startup on
/// the main `:8080` listener so they reflect real request-path serving.
fn spawn_health_listener(
    state: Arc<AppState>,
    start: std::time::Instant,
    heartbeat: Arc<std::sync::atomic::AtomicU64>,
) {
    let Some(port) = std::env::var("DOORWAY_HEALTH_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|&p| p > 0)
    else {
        return;
    };

    let threshold_ms = health_stale_threshold_ms();
    let mut health_addr = state.args.listen;
    health_addr.set_port(port);

    let spawned = std::thread::Builder::new()
        .name("doorway-health-wd".to_string())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    error!(
                        "Health watchdog runtime build failed: {e} — probes stay on main listener"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let listener = match TcpListener::bind(health_addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        error!(
                            "Health watchdog failed to bind {}: {} — probes stay on main listener",
                            health_addr, e
                        );
                        return;
                    }
                };
                info!(
                    "Health watchdog bound on {} (own OS-thread runtime; heartbeat-gated, stale>{}ms ⇒ wedged)",
                    health_addr, threshold_ms
                );
                loop {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            let state = Arc::clone(&state);
                            let heartbeat = Arc::clone(&heartbeat);
                            tokio::spawn(async move {
                                let io = TokioIo::new(stream);
                                let service = service_fn(move |req: Request<Incoming>| {
                                    let state = Arc::clone(&state);
                                    let heartbeat = Arc::clone(&heartbeat);
                                    async move {
                                        Ok::<_, hyper::Error>(
                                            handle_watchdog_probe(
                                                state, start, heartbeat, threshold_ms, req,
                                            )
                                            .await,
                                        )
                                    }
                                });
                                if let Err(err) =
                                    http1::Builder::new().serve_connection(io, service).await
                                {
                                    debug!(
                                        "Health watchdog connection error from {}: {:?}",
                                        addr, err
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            error!("Health watchdog accept error: {:?}", e);
                        }
                    }
                }
            });
        });

    if let Err(e) = spawned {
        error!("Failed to spawn health watchdog thread: {e} — probes stay on main listener");
    }
}

pub async fn run(state: Arc<AppState>) -> Result<(), DoorwayError> {
    let listener = TcpListener::bind(state.args.listen).await?;

    info!(
        "Doorway listening on {} as node {}",
        state.args.listen, state.args.node_id
    );

    // Liveness watchdog (opt-in via DOORWAY_HEALTH_PORT): stamp a heartbeat from
    // the MAIN runtime and serve the liveness probe from a dedicated OS-thread
    // runtime, so a full worker-pool stall (e.g. blocking getaddrinfo during a
    // conductor DNS flap — 2026-06-15 crashloop) can't silence the probe, yet a
    // genuine wedge still restarts the pod. Point livenessProbe at the watchdog
    // port; keep readiness/startup on this main listener. See the freeze RCA.
    let liveness_start = std::time::Instant::now();
    let liveness_heartbeat = Arc::new(std::sync::atomic::AtomicU64::new(0));
    // Register the /metrics collectors and hand the metrics module the SAME
    // heartbeat atomic the watchdog stamps, so `gather_text()` can derive
    // `doorway_heartbeat_age_ms` at scrape time without a sampler task.
    crate::metrics::register_all();
    crate::metrics::set_heartbeat_handle(liveness_start, Arc::clone(&liveness_heartbeat));
    spawn_liveness_heartbeat(liveness_start, Arc::clone(&liveness_heartbeat));
    spawn_health_listener(Arc::clone(&state), liveness_start, liveness_heartbeat);

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
        // Cross-relay signal-bus consumer (D2): deliver frames sibling relays
        // published for dests connected here. No-op on a mem bus (lone pod).
        signal::spawn_bus_consumer(Arc::clone(signal_store));
        info!(
            "Signal bus consumer started (backend: {})",
            signal_store.bus().backend_name()
        );
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
                            let recon = &body["projectionReconcile"];
                            let health = crate::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: body["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: body["peerId"].as_str().map(String::from),
                                caught_up: recon["caughtUp"].as_bool(),
                                converged: recon["converged"].as_bool(),
                                divergent_anchor: recon["divergentAnchor"]
                                    .as_u64()
                                    .map(|n| n as usize),
                                ..Default::default()
                            };
                            *p2p_health.write().await =
                                Some(crate::routes::health::P2PHealthSnapshot::new(health));
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

/// True for paths that belong to doorway's own service surface and must never
/// be intercepted by the EPR router — even when a projection is registered at `/`.
///
/// These prefixes are handled by explicit match arms above the EPR check, so they
/// are guaranteed to reach their own handlers. Keeping this list exhaustive avoids
/// the scenario where `/api/v1/content` silently serves an SPA because a root
/// projection (`url_path = "/"`) matches everything.
fn is_service_path(path: &str) -> bool {
    // Auth paths: only auth-layer-owned paths are service paths.
    // Unowned /auth/* paths (e.g. /auth/portal) fall through to the EPR router.
    if is_auth_owned_path(path) {
        return true;
    }
    // One-shot exhaustive check: each prefix belongs to a guaranteed explicit arm in
    // handle_request. Add new service prefixes here when new explicit arms land.
    // Note: "/auth" is intentionally absent — gated above via is_auth_owned_path.
    for prefix in &[
        "/health",
        "/ready",
        "/readyz",
        "/version",
        // /metrics has an explicit arm; without this the EPR router (GET +
        // !is_service_path) would shadow it whenever a root projection is
        // registered — the /auth/portal incident shape.
        "/metrics",
        // /chrome/* has an explicit arm (runtime chrome static assets); same
        // shadow-guard as /metrics — keep it out of the EPR router's reach.
        "/chrome/",
        "/status",
        "/debug",
        "/admin",
        "/hc/",
        "/app/",
        "/threshold",
        "/bootstrap",
        "/signal",
        "/api/",
        "/import/",
        "/db/",
        // /sync/* — Automerge doc-sync proxied to storage via the route
        // registry (storage build_manifest declares /sync/v1/*). Without this
        // the EPR router (GET + !is_service_path) would shadow GET /sync to the
        // SPA bundle — the /auth/portal incident shape. FLIPPED 2026-06-27.
        "/sync/",
        "/apps/",
        "/epr-head/",
        "/epr/",
        "/blob/",
        "/pkarr/",
        "/.well-known/",
        // /p2p/* — node-local P2P sync status (storage build_manifest declares
        // GET /p2p/status, proxied via the route registry). Without this the
        // EPR router (GET + !is_service_path) shadows it to the SPA bundle —
        // the /auth/portal incident shape (the lamad SyncStatusService's JSON
        // parse then fails on the HTML body).
        "/p2p/",
        // /1.0/identifiers/* — universal-resolver DID resolution has an explicit
        // arm (did:key local, did:elohim forwarded). Without this the EPR router
        // (GET + !is_service_path) would shadow it to the SPA bundle — the
        // /auth/portal incident shape. Reserves the whole /1.0/ resolver namespace.
        "/1.0/",
    ] {
        if path == *prefix || path.starts_with(prefix) {
            return true;
        }
    }
    // Exact-match service roots that don't carry a trailing slash in requests.
    // Note: "/" is intentionally NOT listed here so the EPR router (B13) can
    // match a root projection (url_path="/") before the explicit match arm below.
    // The explicit arm still handles WebSocket upgrades on "/" (dev-mode legacy path).
    //
    // The bare-root forms below are pinned by the shared route-claims fixture
    // (`elohim/sdk/fixtures/route-claims.vectors.json` reservedPrefixes) — the
    // two-layer guard with storage's alias validator (spec §4). They mirror the
    // trailing-slash prefixes above so a bare reserved root is never a legal
    // projection mount or alias target.
    matches!(
        path,
        "/admin"
            | "/status.json"
            | "/epr"
            | "/epr-head"
            | "/db"
            | "/api"
            | "/blob"
            | "/apps"
            // Bare /sync root — pinned reserved (mirrors the "/sync/" prefix
            // above and the route-claims fixture). Keeps a bare reserved root
            // from being a legal projection mount or alias target. FLIPPED 2026-06-27.
            | "/sync"
            // Bare /p2p root — pinned reserved (mirrors the "/p2p/" prefix
            // above). Keeps a bare reserved root from being a legal projection
            // mount or alias target.
            | "/p2p"
            // Only the DID service endpoints are doorway-owned under /identity.
            // The bare "/identity" and "/identity/*" SPA routes (imagodei pillar:
            // /identity/login, /identity/profile, …) must fall through to the EPR
            // router / SPA shell like /community and /account do. Claiming the
            // whole /identity subtree shadowed the entire pillar to the conductor
            // on cold load (the /auth/portal incident shape).
            | "/identity/did"
            | "/identity/did.json"
            | "/sitemap.xml"
    )
}

/// §12.1 reserved-prefix guard: a projection's `url_path` may never collide
/// with a doorway service surface (including the universal `/epr` address).
/// `"/"` is the sanctioned root mount. Note `/auth/portal` stays legal:
/// `is_service_path` only owns the exact AUTH_OWNED_PATHS under /auth.
pub(crate) fn is_reserved_url_path(url_path: &str) -> bool {
    url_path != "/" && is_service_path(url_path)
}

// ─────────────────────────────────────────────────────────────────────────────
// Routing-shakeout seams (2026-05-31 shift: doorway-routing-projection-shakeout)
//
// `is_auth_owned_path` and `derive_app_subpath` are the testable seams the
// shakeout shift hardens. They are the single source of truth for:
//   - which `/auth/*` paths the auth dispatch block (~line 1347) consumes vs.
//     which fall through to the EPR router (the `/auth/portal` un-shadow fix);
//     this predicate ALSO gates `is_service_path` so an unowned `/auth/*` is not
//     classified as a service path and the EPR router (~line 1365) gets to run.
//   - how an EPR projection `url_path` + the request path derive the storage
//     sub-path (extracted from `dispatch_to_projected_epr` so a cache-first
//     serve path can reuse identical derivation).
//
// The STUBS below are intentionally wrong (TDD red) at kickoff; the shift
// implements them. The `shakeout_tests` module is the FROZEN judge — it must not
// be edited mid-shift (agentic-developer principle 6: never edit the oracle).
// ─────────────────────────────────────────────────────────────────────────────

/// The exact set of `/auth/*` request paths the doorway auth layer owns.
/// Any `/auth/*` path NOT in this set must fall through to the EPR router so a
/// seeded projection (e.g. `imagodei-portal` at `/auth/portal`) can serve it.
///
/// MUST stay in sync with the match arms in
/// `routes::auth_routes::handle_auth_request`. (Follow-up: lift both off a shared
/// routing table so this can't drift — tracked in the shift's sprint result.)
const AUTH_OWNED_PATHS: &[&str] = &[
    "/auth/register",
    "/auth/login",
    "/auth/logout",
    "/auth/refresh",
    "/auth/me",
    "/auth/account",
    "/auth/authorize",
    "/auth/token",
    "/auth/native-handoff",
    "/auth/session-token",
    "/auth/exchange-session",
    "/auth/portal-host",
    "/auth/export-key",
    "/auth/confirm-stewardship",
    "/auth/confirm-sovereignty",
    "/auth/recover-custody",
    "/auth/check-recovery-status",
    "/auth/activate-recovery",
    "/auth/elohim-verify/start",
    "/auth/elohim-verify/answer",
];

/// True iff the doorway auth layer owns `path` (exact match, query stripped).
pub(crate) fn is_auth_owned_path(path: &str) -> bool {
    let bare = path.split('?').next().unwrap_or(path);
    AUTH_OWNED_PATHS.contains(&bare)
}

/// True iff `sub` is a ROUTE (SPA deep-link, fallback-eligible) rather than an
/// ASSET. The shared discrimination rule (§12.2 of the pillar-EPR decomposition
/// spec): a sub-path is a ROUTE iff its FINAL segment contains no `.`; otherwise
/// it is an ASSET and a miss must stay an honest 404 (a missing hashed bundle
/// file is a real deploy bug, never masked by index.html).
///
/// Shared test vectors live at
/// `elohim/sdk/fixtures/spa-route-discrimination.vectors.json` and are consumed
/// by both this crate and elohim-storage's fallback — the two-layer drift guard.
pub(crate) fn is_spa_route_subpath(sub: &str) -> bool {
    !sub.rsplit('/').next().unwrap_or(sub).contains('.')
}

/// Derive the storage sub-path for an EPR projection from the request path, the
/// projection's `url_path` prefix, and its `entry_file` (used on bare hits).
///
/// With `spa_fallback` true, extension-less deep ROUTES (per
/// [`is_spa_route_subpath`]) resolve to `entry_file` so the SPA bootstraps and
/// its client-side router handles the deep link. ASSET sub-paths always pass
/// through verbatim so honest 404s surface. See spec §12.2.
pub(crate) fn derive_app_subpath(
    request_path: &str,
    url_path: &str,
    entry_file: &str,
    spa_fallback: bool,
) -> String {
    let sub = if url_path == "/" {
        request_path.trim_start_matches('/')
    } else {
        request_path
            .strip_prefix(url_path)
            .unwrap_or(request_path)
            .trim_start_matches('/')
    };
    // Serve entry_file for the bare mount hit and for fallback-eligible deep
    // ROUTES; ASSET sub-paths (and routes with fallback off) pass through.
    if sub.is_empty() || (spa_fallback && is_spa_route_subpath(sub)) {
        entry_file.to_string()
    } else {
        sub.to_string()
    }
}

#[cfg(test)]
mod shakeout_tests {
    use super::*;

    // ── is_auth_owned_path — the /auth/portal un-shadow contract ──────────────
    #[test]
    fn shakeout_auth_owned_login_true() {
        assert!(is_auth_owned_path("/auth/login"));
    }
    #[test]
    fn shakeout_auth_owned_me_true() {
        assert!(is_auth_owned_path("/auth/me"));
    }
    #[test]
    fn shakeout_auth_owned_portal_host_true() {
        // The real endpoint is `/auth/portal-host` — distinct from `/auth/portal`.
        assert!(is_auth_owned_path("/auth/portal-host"));
    }
    #[test]
    fn shakeout_auth_owned_verify_start_true() {
        assert!(is_auth_owned_path("/auth/elohim-verify/start"));
    }
    #[test]
    fn shakeout_auth_portal_not_owned_false() {
        // THE bug: /auth/portal must fall through to the EPR router, not 404.
        assert!(!is_auth_owned_path("/auth/portal"));
    }
    #[test]
    fn shakeout_auth_unknown_not_owned_false() {
        assert!(!is_auth_owned_path("/auth/something-unseeded"));
    }
    #[test]
    fn shakeout_auth_owned_strips_query() {
        assert!(is_auth_owned_path("/auth/login?redirect=/lamad"));
    }

    // ── is_service_path — unowned /auth must not block the EPR router ─────────
    #[test]
    fn shakeout_service_path_owned_auth_true() {
        assert!(is_service_path("/auth/login"));
    }
    #[test]
    fn shakeout_service_path_unowned_auth_false() {
        // /auth/portal is NOT a service path → EPR router (~line 1365) fires.
        assert!(!is_service_path("/auth/portal"));
    }
    #[test]
    fn shakeout_service_path_still_guards_db() {
        // Regression guard: real service prefixes stay service paths.
        assert!(is_service_path("/db/rea_commitments"));
        assert!(is_service_path("/apps/lamad/index.html"));
    }
    #[test]
    fn shakeout_service_path_guards_metrics() {
        // /metrics MUST be a service path or the EPR router shadows it whenever a
        // root projection (url_path="/") is registered — the scrape would render
        // the SPA bundle instead of the exposition body.
        assert!(is_service_path("/metrics"));
    }
    /// T2.2 shadow-guard: the hosted-binding route MUST be a service path, or
    /// the EPR router (GET + !is_service_path) shadows it to the SPA bundle
    /// whenever a root projection is registered — the /auth/portal incident
    /// shape. A sibling doorway's JSON parse would then fail on an HTML body
    /// and every cross-doorway resolution would silently degrade to the 409.
    #[test]
    fn is_service_path_covers_hosted_binding() {
        assert!(is_service_path(
            "/api/v1/federation/hosted-binding/uhCAkAlice0001"
        ));
        // The sibling federation routes stay covered too.
        assert!(is_service_path("/api/v1/federation/doorways"));
        assert!(is_service_path("/api/v1/federation/coherence"));
    }

    #[test]
    fn shakeout_service_path_guards_chrome() {
        // /chrome/* (runtime chrome static assets — omni-element.js) MUST be a
        // service path so the EPR router doesn't shadow it with the SPA bundle
        // when a root projection is registered — same guard as /metrics.
        assert!(is_service_path("/chrome/omni-element.abc123.js"));
    }
    #[test]
    fn shakeout_service_path_guards_ssr_bundle_refresh() {
        // POST /admin/ssr-bundle/refresh (Track-4 T4-1) is covered as a service
        // path via the "/admin" prefix, so a registered root projection can
        // never shadow the admin surface. (It is POST-only, so the GET-only EPR
        // router would not shadow it regardless — this locks the coverage.)
        assert!(is_service_path("/admin/ssr-bundle/refresh"));
    }
    #[test]
    fn shakeout_service_path_identity_narrowed_to_did() {
        // The imagodei SPA pillar must fall through to the EPR router / SPA shell,
        // exactly like /community and /account. Only the DID service endpoints are
        // doorway-owned. Regression guard for the cold-load /identity → conductor
        // 404 shadow (the whole pillar was unreachable by direct URL).
        assert!(!is_service_path("/identity"));
        assert!(!is_service_path("/identity/login"));
        assert!(!is_service_path("/identity/profile"));
        assert!(is_service_path("/identity/did"));
        assert!(is_service_path("/identity/did.json"));
    }
    #[test]
    fn shakeout_service_path_guards_sync() {
        // /sync/* (Automerge doc-sync, proxied to storage via the route registry)
        // MUST be a service path or the EPR router (GET + !is_service_path)
        // shadows GET /sync to the SPA bundle when a root projection is
        // registered — the /auth/portal incident shape. FLIPPED 2026-06-27.
        assert!(is_service_path("/sync/v1/elohim/docs"));
        assert!(is_service_path(
            "/sync/v1/elohim/docs/node%3Aedit-prop-1/changes"
        ));
        // Bare root is reserved too (pins it against projection-mount collision).
        assert!(is_service_path("/sync"));
        assert!(is_reserved_url_path("/sync"));
    }
    #[test]
    fn shakeout_service_path_guards_p2p() {
        // /p2p/* (node-local P2P sync status, proxied to storage via the route
        // registry — storage build_manifest declares GET /p2p/status) MUST be a
        // service path or the EPR router (GET + !is_service_path) shadows it to
        // the SPA bundle — the /auth/portal incident shape (the lamad
        // SyncStatusService's JSON parse then fails on the HTML body).
        assert!(is_service_path("/p2p/status"));
        // Bare root is reserved too (pins it against projection-mount collision).
        assert!(is_service_path("/p2p"));
        assert!(is_reserved_url_path("/p2p"));
    }

    #[test]
    fn service_path_guards_universal_resolver() {
        // GET /1.0/identifiers/{did} (universal-resolver DID resolution) MUST be a
        // service path or the EPR router (GET + !is_service_path) shadows it to the
        // SPA bundle when a root projection is registered — the /auth/portal
        // incident shape. Two-gate partner of the explicit arm in handle_request.
        assert!(is_service_path(
            "/1.0/identifiers/did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
        ));
        assert!(is_service_path("/1.0/identifiers/did:elohim:uhCAkabcdef"));
        // The whole /1.0/ resolver namespace is reserved.
        assert!(is_service_path("/1.0/identifiers"));
        assert!(is_reserved_url_path("/1.0/identifiers"));
    }

    // ── derive_app_subpath — projection url_path → storage sub-path ───────────
    // All cases pass spa_fallback explicitly; the ROUTE/ASSET behavior is the
    // §12.2 contract (extension-less deep routes → entry_file when fallback on).
    #[test]
    fn shakeout_subpath_root_bare_uses_entry_file() {
        assert_eq!(
            derive_app_subpath("/", "/", "index.html", true),
            "index.html"
        );
    }
    #[test]
    fn shakeout_subpath_root_asset() {
        // main.js is an ASSET (dotted final segment) → passes through verbatim.
        assert_eq!(
            derive_app_subpath("/main.js", "/", "index.html", true),
            "main.js"
        );
    }
    #[test]
    fn shakeout_subpath_prefix_bare_uses_entry_file() {
        assert_eq!(
            derive_app_subpath("/lamad", "/lamad", "index.html", true),
            "index.html"
        );
    }
    #[test]
    fn shakeout_subpath_prefix_route_falls_back_to_entry_file() {
        // concept/x is a ROUTE (no dot in final segment); with spa_fallback on
        // it resolves to entry_file so the SPA bootstraps and routes client-side.
        assert_eq!(
            derive_app_subpath("/lamad/concept/x", "/lamad", "index.html", true),
            "index.html"
        );
    }
    #[test]
    fn shakeout_subpath_prefix_route_verbatim_when_fallback_off() {
        // Same ROUTE, but spa_fallback off → verbatim pass-through (no SPA serve).
        assert_eq!(
            derive_app_subpath("/lamad/concept/x", "/lamad", "index.html", false),
            "concept/x"
        );
    }
    #[test]
    fn shakeout_subpath_prefix_single_asset() {
        // main.js is an ASSET → verbatim even with spa_fallback on.
        assert_eq!(
            derive_app_subpath("/lamad/main.js", "/lamad", "index.html", true),
            "main.js"
        );
    }

    // ── is_spa_route_subpath — the shared ROUTE/ASSET rule, vector-driven ──────
    #[derive(serde::Deserialize)]
    struct RouteVector {
        #[serde(rename = "subPath")]
        sub_path: String,
        kind: String,
        #[allow(dead_code)]
        note: String,
    }
    #[derive(serde::Deserialize)]
    struct RouteVectorFile {
        vectors: Vec<RouteVector>,
    }

    #[test]
    fn shakeout_is_spa_route_agrees_with_shared_vectors() {
        // The ONE shared test-vector table (two-layer drift guard, §12.2). Both
        // this crate and elohim-storage's fallback consume it via include_str!.
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../elohim/sdk/fixtures/spa-route-discrimination.vectors.json"
        ));
        let file: RouteVectorFile =
            serde_json::from_str(raw).expect("shared spa-route vector fixture must parse");
        assert!(!file.vectors.is_empty(), "vector fixture must not be empty");
        for v in &file.vectors {
            let expected_route = match v.kind.as_str() {
                "route" => true,
                "asset" => false,
                other => panic!("unknown vector kind {:?} for {:?}", other, v.sub_path),
            };
            assert_eq!(
                is_spa_route_subpath(&v.sub_path),
                expected_route,
                "ROUTE/ASSET disagreement for {:?} (expected kind={})",
                v.sub_path,
                v.kind
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Portal-handoff routing contract (GAP-2)
//
// The steward portal-handoff reuses the EXISTING session-transfer pair —
// GET /auth/session-token (Bearer-authenticated single-use mint) and
// GET /auth/exchange-session (atomic consume → identity + portalHostUrl) —
// rather than minting a parallel mechanism. These paths MUST stay auth-owned,
// or they fall through to the EPR router and 404 (the exact bug commit
// 37c822d1c fixed for /auth/portal). The steward's storage redeems the code
// server-to-server via GET {doorway}/auth/exchange-session?session_token=…
//
// This module is intentionally SEPARATE from `shakeout_tests` above (the frozen
// oracle of the 2026-05-31 routing shakeout shift, which must not be edited).
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod handoff_routing_tests {
    use super::*;

    #[test]
    fn session_token_mint_is_auth_owned() {
        // GET /auth/session-token must reach the auth dispatch, not the EPR router.
        assert!(is_auth_owned_path("/auth/session-token"));
    }

    #[test]
    fn exchange_session_redeem_is_auth_owned() {
        // GET /auth/exchange-session must reach the auth dispatch, not the EPR
        // router — the steward portal-handoff's server-to-server redeem depends
        // on this route resolving from any caller.
        assert!(is_auth_owned_path("/auth/exchange-session"));
    }

    #[test]
    fn handoff_paths_strip_query() {
        // exchange-session is always called with the code in the query string.
        assert!(is_auth_owned_path("/auth/session-token?from=login"));
        assert!(is_auth_owned_path(
            "/auth/exchange-session?session_token=abc"
        ));
    }

    #[test]
    fn handoff_paths_are_service_paths() {
        // is_service_path delegates to is_auth_owned_path for /auth/*; both
        // paths must classify as service paths so the EPR router does not shadow
        // them when a root projection (url_path="/") is registered.
        assert!(is_service_path("/auth/session-token"));
        assert!(is_service_path("/auth/exchange-session"));
    }

    #[test]
    fn unrelated_handoff_lookalike_not_owned() {
        // Guard the un-shadow boundary: a non-registered /auth/* lookalike must
        // still fall through to the EPR router (stays false), exactly as
        // /auth/portal does.
        assert!(!is_auth_owned_path("/auth/session-token-extra"));
        assert!(!is_auth_owned_path("/auth/exchange-session/extra"));
    }
}

/// Decide whether a proxied storage body is an HTML *page* that must carry the
/// native runtime chrome, and if so splice the omnibar context island + loader
/// into it (before `</body>`, via the same `inject_element` mechanism the SSR
/// arm uses). Returns the (possibly transformed) bytes and whether injection
/// happened (the caller uses that to mirror the SSR arm's `no-store`).
///
/// Injects ONLY for a servable page: a 2xx status AND a `text/html`
/// content-type AND a valid-UTF-8 body. Assets (`.js`/`.css`/images) and
/// non-2xx bodies flow through this same proxy and pass through byte-identical.
/// Invalid-UTF-8 `text/html` is served unmodified (an un-injectable page must
/// never fail the response). Pure (no IO) so it is unit-testable without HTTP.
fn maybe_inject_chrome(
    status: u16,
    content_type: &str,
    body: Bytes,
    chrome_context_json: &str,
) -> (Bytes, bool) {
    let is_html_page = (200..300).contains(&status)
        && content_type
            .trim()
            .to_ascii_lowercase()
            .starts_with("text/html");
    if !is_html_page {
        return (body, false);
    }
    match std::str::from_utf8(&body) {
        Ok(html) => (
            Bytes::from(elohim_render::inject_element(html, chrome_context_json)),
            true,
        ),
        Err(e) => {
            tracing::debug!(
                error = %e,
                "EPR router: text/html body is not valid UTF-8 — serving unmodified (no chrome inject)"
            );
            (body, false)
        }
    }
}

/// Whole-request timeout for an EPR-router dispatch to the cached bundle
/// (`GET /apps/{epr_id}/{sub_path}`). A healthy dispatch completes in well
/// under 100ms — this is a browser-facing serve, not a background fetch, so
/// 30s was a blocking wall for a visitor whenever the configured storage peer
/// was unreachable (e.g. alpha-b's `storage_url` pointed at a
/// transport-blocked `adam`). Applied per-request over the shared
/// `storage_proxy_client` (client default 12s), keeping the browser-facing
/// budget tighter than the general proxy budget.
const EPR_DISPATCH_TIMEOUT_SECS: u64 = 10;

/// Fast-fail response for `dispatch_to_projected_epr` when either (a) the
/// per-upstream circuit breaker is already open for `storage_url`, or (b) a
/// dispatch's connect/timeout fails outright. Delegates to the shared
/// content-negotiated shed responder (`routes::catching_up::shed_response`):
/// browser navigations get the staged recovery page, everything else keeps
/// the legacy `{"status":"catching-up","retryAfter":N}` JSON. Pure (no IO):
/// the seam the unit tests exercise.
fn epr_dispatch_shed_response(
    wants_html: bool,
    retry_after_secs: u64,
    cause: routes::catching_up::ShedCause,
) -> Response<Full<Bytes>> {
    routes::catching_up::shed_response(wants_html, retry_after_secs, cause)
}

/// Dispatch a request to a projected EPR (B13).
///
/// MVP scope (§8.1 + §8.2):
/// - `reach != "commons"` → 401 (gated reach, deferred to later shift)
/// - `mode == StewardDirect` → 501 (deferred to later shift)
/// - `mode == Cached, reach == "commons"` → proxy to storage `/apps/{epr_id}/{sub_path}`
///
/// Sub-path computation:
/// - Strip the projection's `url_path` prefix from the request path.
/// - Fall back to `entry_file` when the remainder is empty (bare prefix hit).
///
/// Upstream circuit breaker (mirrors `storage_proxy::forward_to_storage`):
/// keyed by the same base `storage_url` (trimmed, no path suffix) that
/// `forward_to_storage` uses — so a peer already tripped via the
/// storage-proxy path shares breaker state with this EPR-bundle dispatch
/// path, and vice versa. An open breaker sheds immediately, before the
/// storage GET; a completed dispatch records its outcome (D6: only
/// connect/timeout/5xx counts as failure — an expected 404 must never open
/// the breaker).
async fn dispatch_to_projected_epr(
    state: &AppState,
    request_path: &str,
    projection: elohim_views::projection::EprProjectionView,
    chrome_context_json: &str,
    wants_html: bool,
) -> Response<Full<Bytes>> {
    use elohim_views::projection::ProjectionMode;

    // Reach gate — MVP: only anon-readable reaches (commons|public, the
    // substrate's anon rule) are served. Gated projections deferred (§8.2).
    if !anon_reach_readable(&projection.reach) {
        let body = serde_json::json!({
            "error": "reach-gated",
            "message": "This content requires authorization. Gated EPR serving is not yet implemented."
            // TODO(#6-2 gate-face / spec §5.1 "never a wall"): this MOUNT-arm 401 contradicts
            // the resolver-owns-the-gate invariant — gated mounts should fall to the shell
            // boundary (head-edge + inclusive path), not a hard 401. Tracked: gap #6-2 of
            // epr-route-claims-link-conformance-design + epr-routing-complementary-captures.md.
            // (Was: TODO(B-later) consult gate_hints + req auth for gated projections.)
        });
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(
                serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec()),
            )))
            .expect("infallible 401 response");
    }

    // Mode gate — MVP: only Cached is implemented. StewardDirect deferred (§8.2).
    if projection.mode != ProjectionMode::Cached {
        let body = serde_json::json!({
            "error": "steward-direct-not-implemented",
            "message": "Steward-direct projection mode is not yet implemented in this doorway."
            // TODO(B-later): open relay connection to steward_direct_endpoint and proxy bytes
        });
        return Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(
                serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec()),
            )))
            .expect("infallible 501 response");
    }

    // Storage URL is required to proxy to the bundle.
    let storage_url = match &state.args.storage_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"Storage URL not configured"}"#,
                )))
                .expect("infallible 503 response");
        }
    };

    // Derive the storage sub-path from the projection prefix and request path.
    let sub_path = derive_app_subpath(
        request_path,
        &projection.url_path,
        &projection.entry_file,
        projection.spa_fallback,
    );

    // Proxy to storage's /apps/{epr_id}/{sub_path} — the existing bundle-serving surface.
    // Storage's slug_index and AppFileCacheService handle caching; doorway proxies, not owns.
    //
    // Doorway is contract-authoritative for `spa_fallback` (§12.2). Storage's
    // safety-net fallback defaults ON (so tauri-direct deep links work with no
    // doorway in the loop). When this projection opts out, propagate the
    // decision via `?spaFallback=0` so storage's convention layer honours it and
    // the two layers stay consistent.
    let storage_apps_path = if projection.spa_fallback {
        format!("{}/apps/{}/{}", storage_url, projection.epr_id, sub_path)
    } else {
        format!(
            "{}/apps/{}/{}?spaFallback=0",
            storage_url, projection.epr_id, sub_path
        )
    };

    tracing::debug!(
        request_path = %request_path,
        epr_id = %projection.epr_id,
        storage_url = %storage_apps_path,
        "EPR router dispatching to cached bundle"
    );

    // Per-upstream breaker (Pillar 2 layer 4): consulted IMMEDIATELY before
    // send, exactly like `forward_to_storage`, and keyed by the same base
    // `storage_url` (not the full `/apps/{epr_id}/{sub_path}` bundle path) so
    // the two dispatch paths share one breaker per storage peer. Circuit-open
    // sheds WITHOUT calling storage — this is what makes a blocked peer (e.g.
    // adam on alpha-b) fast-fail here too, instead of riding the full
    // dispatch timeout on every visitor request.
    // The RAII guard resolves an outcome on every terminal path — including the
    // one that never runs, when this request future is dropped mid-send.
    let trial = match state.upstream_breakers.begin(&storage_url) {
        Some(t) => t,
        None => {
            tracing::warn!(
                target: "upstream_shed",
                counter = "doorway_upstream_breaker_open_total",
                storage_url = %storage_url,
                epr_id = %projection.epr_id,
                "EPR router: upstream circuit OPEN — shedding without calling storage (503 + Retry-After)"
            );
            crate::metrics::inc_breaker_open();
            return epr_dispatch_shed_response(
                wants_html,
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                routes::catching_up::upstream_cause(&state.upstream_breakers, &storage_url),
            );
        }
    };

    match state
        .storage_proxy_client
        .get(&storage_apps_path)
        .timeout(std::time::Duration::from_secs(EPR_DISPATCH_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let cache_control = resp
                .headers()
                .get(reqwest::header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            match resp.bytes().await {
                Ok(body_bytes) => {
                    // Record-once terminal outcome (D6, mirrors forward_to_storage):
                    // classify by status — 2xx/4xx (incl. an expected 404 blob/route
                    // miss) never opens the breaker; only 429/5xx counts as failure.
                    trial.record(
                        crate::routes::storage_proxy::ProxyOutcome::classify(status.as_u16())
                            != crate::routes::storage_proxy::ProxyOutcome::Failure,
                    );
                    // Native runtime chrome: an HTML page served via this proxy is
                    // an EPR-router HTML serve (the landing `/`, a pillar mount),
                    // and must carry the omnibar trust surface — the same island
                    // the SSR arm splices. Assets (.js/.css/images) flow through
                    // this same function and pass through byte-identical.
                    let (body_bytes, chrome_injected) = maybe_inject_chrome(
                        status.as_u16(),
                        &content_type,
                        body_bytes,
                        chrome_context_json,
                    );
                    let mut builder = Response::builder()
                        .status(status.as_u16())
                        .header("content-type", &content_type)
                        .header("x-epr-router", "dispatched");
                    // Cache-control parity with the SSR arm: an injected page
                    // carries the request-specific omnibar context (the
                    // `authenticated` flag), so it must not be shared-cached —
                    // mirror `ssr_html_response_with_observability`'s `no-store`.
                    // Untouched bytes (assets, non-HTML, non-2xx) keep storage's
                    // cache-control unchanged.
                    if chrome_injected {
                        builder = builder.header("cache-control", "no-store");
                    } else if let Some(cc) = cache_control {
                        builder = builder.header("cache-control", cc);
                    }
                    builder
                        .body(Full::new(body_bytes))
                        .expect("infallible EPR proxy response")
                }
                Err(e) => {
                    tracing::warn!(
                        epr_id = %projection.epr_id,
                        error = %e,
                        "EPR router: failed to read storage response body"
                    );
                    // A response arrived (connect succeeded) but the body stream
                    // failed mid-read — still a real upstream failure, so it must
                    // count toward the breaker even though this isn't the
                    // connect/timeout hang this fix targets.
                    trial.record(false);
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("content-type", "application/json")
                        .body(Full::new(Bytes::from(r#"{"error":"storage read failed"}"#)))
                        .expect("infallible 502 response")
                }
            }
        }
        Err(e) => {
            // Connect/timeout failure — the class this fix targets (a blocked
            // peer like alpha-b's adam). Record the breaker failure (D6: this
            // always counts) and shed fast with the same 503 + Retry-After
            // shape as the breaker-open branch above, instead of the previous
            // bare 502 (which cost nothing extra to return quickly, but gave
            // the caller no retry signal after riding the full dispatch
            // timeout).
            tracing::warn!(
                epr_id = %projection.epr_id,
                storage_url = %storage_apps_path,
                error = %error_with_source_chain(&e),
                "EPR router: storage request failed (connect/timeout) — recording breaker failure and shedding"
            );
            trial.record(false);
            epr_dispatch_shed_response(
                wants_html,
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                routes::catching_up::upstream_cause(&state.upstream_breakers, &storage_url),
            )
        }
    }
}

#[cfg(test)]
mod epr_dispatch_breaker_tests {
    use super::*;

    // `dispatch_to_projected_epr` itself is async, requires a live `AppState`
    // (storage_url, storage_proxy_client) and an actual upstream to exercise the
    // send()/bytes() branches — heavy scaffolding for little marginal
    // coverage over the already-tested `UpstreamBreakers::is_open`/`record`
    // (see `routes::upstream_health` unit tests: `opens_after_threshold_then_sheds`,
    // `success_keeps_closed`, `distinct_endpoints_isolated`) and
    // `ProxyOutcome::classify`'s D6 rule (`routes::storage_proxy` unit test
    // `proxy_outcome_classifies_failures`). What IS new and pure in this change —
    // and therefore what these tests cover — is the fast-fail response this
    // fix returns on both the breaker-open shed and the connect/timeout-error
    // paths, plus the tightened timeout constant.

    fn test_cause() -> routes::catching_up::ShedCause {
        routes::catching_up::ShedCause::Upstream {
            endpoint: "http://storage:8090".to_string(),
            circuit: "open".to_string(),
            error_streak: 3,
        }
    }

    #[tokio::test]
    async fn shed_response_is_503_with_retry_after_and_catching_up_body() {
        let resp = epr_dispatch_shed_response(false, 30, test_cause());
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok()),
            Some("30")
        );
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "catching-up");
        assert_eq!(json["retryAfter"], 30);
    }

    #[tokio::test]
    async fn shed_response_propagates_the_given_retry_after() {
        // Distinct from the default test above: proves the value isn't
        // hardcoded — a caller-supplied cooldown (e.g. an upstream Retry-After
        // in a future extension) round-trips into both the header and body.
        let resp = epr_dispatch_shed_response(false, 7, test_cause());
        assert_eq!(
            resp.headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok()),
            Some("7")
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["retryAfter"], 7);
    }

    #[test]
    fn dispatch_timeout_is_tightened_well_under_the_old_30s_wall() {
        // The regression this fix closes: a per-request .timeout() override
        // silently widened past the 10s browser-facing budget, back up to a
        // full 30s wall on every dispatch to an unreachable storage peer.
        assert_eq!(EPR_DISPATCH_TIMEOUT_SECS, 10);
        assert!(EPR_DISPATCH_TIMEOUT_SECS < 30);
    }

    fn shell_projection(spa_fallback: bool) -> elohim_views::projection::EprProjectionView {
        use elohim_views::projection::{EprProjectionView, ProjectionMode};
        EprProjectionView {
            commitment_id: "test".into(),
            epr_id: "elohim-host-landing".into(),
            doorway_id: "doorway:test".into(),
            url_path: "/".into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/".into(),
            entry_file: "index.html".into(),
            spa_fallback,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-06-06T00:00:00Z".into(),
            seeded_by: "test".into(),
        }
    }

    #[test]
    fn projected_shell_url_targets_the_apps_entry_file() {
        // The shell composed into is the app's index.html entry — the same
        // /apps/{epr}/{entry} surface the CSR fallback serves — NOT the request
        // sub-path. Base storage_url is trimmed of any trailing slash.
        let p = shell_projection(true);
        assert_eq!(
            projected_shell_url("http://storage:8090/", &p),
            "http://storage:8090/apps/elohim-host-landing/index.html"
        );
    }

    #[test]
    fn projected_shell_url_propagates_spa_fallback_opt_out() {
        // Mirrors dispatch_to_projected_epr: an opted-out projection carries
        // ?spaFallback=0 so storage's convention layer stays consistent.
        let p = shell_projection(false);
        assert_eq!(
            projected_shell_url("http://storage:8090", &p),
            "http://storage:8090/apps/elohim-host-landing/index.html?spaFallback=0"
        );
    }
}

/// The anon-readable reach set — the doorway's HALF of the substrate's anon
/// rule. MUST mirror storage's unauthenticated list filter
/// (`elohim-storage/src/http.rs` handle_db_content_list:
/// `reach == "commons" || reach == "public"`); reach hierarchy: public=6,
/// commons=7 most permissive. One definition per side; the wider 3-vocabulary
/// reach reconciliation is tracked in memory
/// `project_reach_enum_drift_reconciliation`.
pub(crate) fn anon_reach_readable(reach: &str) -> bool {
    matches!(reach, "commons" | "public")
}

/// The universal `/epr/{id}` address ALWAYS serves the shell (EPR Slice 1,
/// operator decision 2026-06-08: the universal-address claims-302 is DEMOTED).
/// Before this, a claimed, commons-reach target (e.g. a path) 302'd to its
/// pretty pillar mount (`/lamad/path/{id}`) — which meant a claimed type never
/// reached the lens-complete shell viewer; it bounced to the knowledge-only
/// pillar (the four-leg coupling violation). Now the universal address serves
/// the shell for BOTH the bare `/epr/{id}` and the `/epr/{id}/raw` inspector;
/// the shell renders the lens-complete viewer and offers its own "Open in
/// {pillar}" affordance, and its Angular routing distinguishes the two subviews.
/// The doorway therefore no longer consults head facts, content type, or the
/// claims table here — the only branch left is purely whether a ROOT projection
/// exists to serve: `Some(root)` serves the root projection's bundle (the
/// shell); `None` means no root mount is registered, so the caller falls back to
/// `/threshold`. Pure, table-data-only (no IO) so the contract is unit-testable
/// without an `AppState`. (The legacy `/lamad/resource/{id}` → `/epr/{id}`
/// alias-302 and the sitemap's per-claimed-id entries are a DIFFERENT dispatch
/// path and keep the claims table — only the universal-address 302 is removed.)
fn epr_universal_root(
    router: &crate::projection::EprRouter,
) -> Option<elohim_views::projection::EprProjectionView> {
    router.dispatch("/")
}

/// §12.1 universal EPR address: `/epr/{id}` (and `/epr/{id}/raw`) serve the ROOT
/// projection's bundle — the shell renders the lens-complete viewer. The
/// claims-302 to a pretty pillar mount is demoted (EPR Slice 1); the universal
/// address no longer redirects. No root projection registered → `/threshold`.
async fn dispatch_epr_universal(
    state: &AppState,
    original_path: &str,
    chrome_context_json: &str,
    wants_html: bool,
) -> Response<Full<Bytes>> {
    match epr_universal_root(&state.epr_router) {
        Some(root) => {
            tracing::debug!(path = %original_path,
                "universal /epr address — serving shell (root projection bundle)");
            dispatch_to_projected_epr(state, "/", root, chrome_context_json, wants_html).await
        }
        None => {
            tracing::debug!(path = %original_path,
                "universal /epr address — no root projection, falling back to /threshold");
            Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
    }
}

/// Render the doorway sitemap (spec §7.5) — a DERIVED projection of the routing
/// table that cannot drift from dispatch. One `<url>` per live mount plus one
/// per claimed commons id (minted via the SAME `mint_mount_location` dispatch
/// uses). Pure: no IO, table data only — the caller fetches commons ids.
fn render_sitemap(
    router: &crate::projection::EprRouter,
    base_url: &str,
    commons_ids_by_type: &[(String, Vec<String>)],
) -> String {
    let base = base_url.trim_end_matches('/');
    let mut out = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    out.push('\n');

    // One entry per live mount (the pretty roots).
    let mut mounts = router.mount_url_paths();
    mounts.sort();
    for mount in &mounts {
        let path = if mount == "/" { "" } else { mount.as_str() };
        out.push_str(&format!("  <url><loc>{base}{path}</loc></url>\n"));
    }

    // One entry per claimed commons id — minted through the same dispatch fn.
    for (content_type, ids) in commons_ids_by_type {
        for id in ids {
            if let Some(loc) = router.claimed_mount_location(content_type, id) {
                out.push_str(&format!("  <url><loc>{base}{loc}</loc></url>\n"));
            }
        }
    }

    out.push_str("</urlset>\n");
    out
}

/// Serve `/sitemap.xml` (spec §7.5). Generation-invalidated materialized
/// projection: serve the cache while it matches the router generation; on a
/// miss, fetch commons ids per claimed type, render, recache, serve. Storage
/// unavailable → render a mounts-only sitemap (fail open, log a warning).
async fn serve_sitemap(state: &AppState, base_url: &str) -> Response<Full<Bytes>> {
    let generation = state.epr_router.generation();

    // Fast path: cached XML still matches the current routing-table generation.
    {
        let cache = state.sitemap_cache.read().await;
        if let Some((cached_gen, xml)) = cache.as_ref() {
            if *cached_gen == generation {
                return sitemap_response(xml.clone(), generation);
            }
        }
    }

    // Materialize: fetch commons ids per claimed contentType (tolerant). A
    // failure for any type yields an empty list for that type (fail open).
    let mut commons_ids_by_type: Vec<(String, Vec<String>)> = Vec::new();
    let storage_url = state
        .args
        .storage_url
        .as_deref()
        .map(|u| u.trim_end_matches('/').to_string());
    for content_type in state.epr_router.claimed_content_types() {
        let ids = match &storage_url {
            Some(base) => fetch_commons_ids(state, base, &content_type).await,
            None => {
                tracing::warn!("sitemap: storage URL not configured — serving mounts-only sitemap");
                Vec::new()
            }
        };
        commons_ids_by_type.push((content_type, ids));
    }

    let xml = render_sitemap(&state.epr_router, base_url, &commons_ids_by_type);
    {
        let mut cache = state.sitemap_cache.write().await;
        *cache = Some((generation, xml.clone()));
    }
    sitemap_response(xml, generation)
}

/// Build the standard sitemap HTTP response (shared by hit + miss paths).
fn sitemap_response(xml: String, generation: u64) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/xml")
        .header("cache-control", "public, max-age=300")
        .header("ETag", format!("\"g{generation}\""))
        .body(Full::new(Bytes::from(xml)))
        .expect("infallible sitemap response")
}

/// Tolerant anon-readable-id fetch for one claimed contentType — one query per
/// reach in the anon-readable set ({commons, public}, see
/// [`anon_reach_readable`]), merged. Any failure → empty for that reach (the
/// sitemap omits those entries rather than 500ing — fail open).
async fn fetch_commons_ids(
    state: &AppState,
    storage_base: &str,
    content_type: &str,
) -> Vec<String> {
    let mut merged = Vec::new();
    for reach in ["commons", "public"] {
        merged.extend(fetch_ids_for_reach(state, storage_base, content_type, reach).await);
    }
    merged
}

/// Single-reach tolerant id fetch (see [`fetch_commons_ids`]).
async fn fetch_ids_for_reach(
    state: &AppState,
    storage_base: &str,
    content_type: &str,
    reach: &str,
) -> Vec<String> {
    let url =
        format!("{storage_base}/db/content?contentType={content_type}&reach={reach}&limit=500");
    let resp = match state
        .storage_proxy_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            tracing::warn!(content_type = %content_type, status = %r.status(),
                "sitemap: commons-id fetch non-success — omitting type");
            return Vec::new();
        }
        Err(e) => {
            tracing::warn!(content_type = %content_type, error = %e,
                "sitemap: commons-id fetch failed — omitting type");
            return Vec::new();
        }
    };
    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    // Tolerant parse: accept a bare array or an `{items:[...]}` envelope.
    let items = v
        .get("items")
        .and_then(|i| i.as_array())
        .or_else(|| v.as_array());
    match items {
        Some(arr) => arr
            .iter()
            .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(String::from))
            .collect(),
        None => Vec::new(),
    }
}

/// Which fallback strategy `serve_ssr_route` uses when SSR can't or shouldn't
/// render this request. The SSR body is identical across both dispatch sites;
/// only what a shed/failure degrades TO differs.
enum SsrFallback {
    /// Legacy registry dispatch (`classify_dispatch` → `SsrRoute` in the main
    /// match block): the pre-existing behavior — CSR shell for the gate/render
    /// sheds, `forward_to_storage` for spec-parse / renderer-absent — unchanged.
    Registry,
    /// EPR-router dispatch: EVERY shed/failure serves the projected bundle (which
    /// carries the same chrome island via `dispatch_to_projected_epr`), tagged
    /// `x-ssr-skipped:<reason>`. Degrades to exactly today's EPR serving behavior.
    /// Boxed — the view is ~472B and the other variant is empty (clippy
    /// large_enum_variant); the fallback is off the hot path so the indirection
    /// is free.
    ProjectedEpr(Box<elohim_views::projection::EprProjectionView>),
}

/// The point at which SSR fell back — drives `x-ssr-skipped` observability and
/// selects the legacy fallback shape.
enum SsrFallbackReason {
    AuthModeUnsupported,
    Overflow,
    SpecParseError,
    RendererAbsent,
    RenderError(String),
    /// The render-failure circuit breaker is OPEN — SSR was skipped BEFORE any
    /// render was attempted (no doomed-render tax during a systemic outage).
    ErrorBackoff,
    /// The render SUCCEEDED but activated NO route component — an empty
    /// `<router-outlet>` shell that can never hydrate (see
    /// `render_output_is_empty`). Shed to the fallback bundle exactly like a
    /// render error, so the client bundles the fallback carries can boot the app.
    EmptyRender,
    /// The loaded renderer serves a DIFFERENT app than this route's projection
    /// (`renderer_app` ≠ `projection.epr_id`). Rendering would produce the
    /// wrong app's markup, so SSR is skipped BEFORE any V8 work — the
    /// /lamad-dark failure mode, now named and render-free.
    RendererAppMismatch,
    // The Shell* + Compose variants below: a non-empty render succeeded, but
    // the bundle-carrying shell could not be composed (see
    // `compose_render_with_shell`) — a render doc carries NO client bundles on
    // its own, so serving it raw is a dead-end static page. Shed to the
    // fallback bundle (which DOES carry the client bundles) so the page is
    // always hydratable. Each variant names the seam that failed — the
    // predecessor single `shell-unavailable` tag hid a cross-app selector bug
    // and an upstream fetch stall behind one string for two weeks.
    /// No projection to resolve a shell URL from (legacy Registry dispatch).
    ShellNoProjection,
    /// The storage upstream's circuit breaker is open — shell fetch skipped.
    ShellBreakerOpen,
    /// The shell HTTP fetch failed (non-2xx, timeout, connect error).
    ShellFetchFailed,
    /// The splice itself refused — typed by `elohim_render::ComposeError`
    /// (render-root-missing / render-root-unclosed / shell-root-missing).
    Compose(elohim_render::ComposeError),
}

impl SsrFallbackReason {
    fn as_skip_str(&self) -> &'static str {
        match self {
            SsrFallbackReason::AuthModeUnsupported => "auth-mode-not-supported",
            SsrFallbackReason::Overflow => "overflow",
            SsrFallbackReason::SpecParseError => "spec-parse-error",
            SsrFallbackReason::RendererAbsent => "renderer-absent",
            SsrFallbackReason::RenderError(_) => "render-error",
            SsrFallbackReason::ErrorBackoff => "error-backoff",
            SsrFallbackReason::EmptyRender => "empty-render",
            SsrFallbackReason::RendererAppMismatch => "renderer-app-mismatch",
            SsrFallbackReason::ShellNoProjection => "shell-no-projection",
            SsrFallbackReason::ShellBreakerOpen => "shell-breaker-open",
            SsrFallbackReason::ShellFetchFailed => "shell-fetch-failed",
            SsrFallbackReason::Compose(e) => e.reason_str(),
        }
    }
}

/// Tag a projected-bundle fallback response with `x-ssr-skipped:<reason>` so an
/// EPR route that classified SSR-eligible but shed to the bundle is observable
/// (the SSR arm never ran; the bundle already carries the chrome island). Pure.
fn with_ssr_skipped_header(
    mut resp: Response<Full<Bytes>>,
    reason: &SsrFallbackReason,
) -> Response<Full<Bytes>> {
    resp.headers_mut().insert(
        "x-ssr-skipped",
        hyper::header::HeaderValue::from_static(reason.as_skip_str()),
    );
    resp
}

/// At the EPR-router dispatch: divert to the V8 SSR engine only when the route
/// classified SSR-eligible AND a renderer is loaded. A non-SSR disposition, or a
/// renderer-absent doorway, serves the projected bundle directly — the cheapest
/// path and today's exact behavior. Pure, so the decision is unit-testable
/// without a live storage or renderer.
fn epr_should_serve_ssr(dispo: &Disposition, renderer_present: bool) -> bool {
    matches!(dispo, Disposition::SsrRoute { .. }) && renderer_present
}

/// Detect a "successful but empty" Angular SSR render: the render returned HTML,
/// but no route component was activated, so the `<router-outlet>` has no element
/// sibling after it — only comment/whitespace nodes before `</app-root>`.
///
/// WHY an empty render MUST shed to the fallback bundle: the SSR document
/// template (`SSR_DOCUMENT`) carries NO client bundles by construction. An SSR
/// render is meant to be progressively enhanced by the client bundles that the
/// *fallback* SPA index ships. An empty shell (outlet with no activated
/// component) can therefore never hydrate — there is no server-rendered content
/// to enhance AND no client script to bootstrap the app — so serving or caching
/// it is a permanent blank page. Treating it exactly like a render error routes
/// the request to `SsrFallback::ProjectedEpr` (or the Registry CSR shell), which
/// DOES carry the client bundles and boots the app normally: the safe serve. A
/// HEALTHY render places the activated route component as an element sibling
/// immediately after the outlet (`</router-outlet><app-home …>…</app-home>`); an
/// empty one has only comment nodes.
///
/// Semantics: returns `true` iff the document contains at least one
/// `<router-outlet` AND no router-outlet in it is followed by an element sibling.
/// A document with NO `<router-outlet` returns `false` — we cannot judge a
/// non-router document (e.g. a raw error page) and must not gate it. Dependency-
/// free string scanning; Angular emits lowercase tag names.
fn render_output_is_empty(html: &str) -> bool {
    let mut found_outlet = false;
    let mut search_from = 0usize;
    while let Some(rel) = html[search_from..].find("<router-outlet") {
        found_outlet = true;
        let outlet_start = search_from + rel;
        // Advance past this outlet's close tag. Angular emits
        // `<router-outlet …></router-outlet>` (an empty element — the activated
        // component is a SIBLING, never a child). If a close tag is somehow
        // absent, treat the remainder of the document as "after the outlet".
        let after_close = match html[outlet_start..].find("</router-outlet>") {
            Some(close_rel) => outlet_start + close_rel + "</router-outlet>".len(),
            None => html.len(),
        };
        // Strip comment nodes (`<!-- … -->`, including the empty `<!---->`) and
        // whitespace repeatedly, to reach the next real sibling node.
        let mut rest = html[after_close..].trim_start();
        while let Some(stripped) = rest.strip_prefix("<!--") {
            match stripped.find("-->") {
                Some(end) => rest = stripped[end + "-->".len()..].trim_start(),
                // An unterminated comment eats the rest of the document — no
                // element sibling can follow.
                None => {
                    rest = "";
                    break;
                }
            }
        }
        // An element sibling opens with `<` immediately followed by an ASCII
        // letter (an opening tag). A closing tag (`</…`), a comment, or text does
        // not — those mean no activated component followed this outlet.
        let bytes = rest.as_bytes();
        let activated = bytes.first() == Some(&b'<')
            && matches!(bytes.get(1), Some(b) if b.is_ascii_alphabetic());
        if activated {
            // At least one outlet has an activated component → not an empty shell.
            return false;
        }
        search_from = after_close;
    }
    found_outlet
}

/// Join an error's `source()` chain into one line (`a: b: c`) so transport
/// failures name their terminal cause (connect refused vs timed out vs DNS)
/// instead of stopping at reqwest's opaque "error sending request" Display —
/// the truncation that kept the 2026-08-13 shell-fetch timeout class opaque.
fn error_with_source_chain(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut cur = e.source();
    while let Some(s) = cur {
        out.push_str(": ");
        out.push_str(&s.to_string());
        cur = s.source();
    }
    out
}

/// Produce the fallback response for `serve_ssr_route`. Consumes `req` (the
/// legacy `forward_to_storage` fallbacks need it) and `fallback` (owned so the
/// `ProjectedEpr` variant can move its `EprProjectionView` into the proxy).
async fn ssr_fallback_response(
    state: &AppState,
    req: Request<Incoming>,
    path: &str,
    endpoint: &str,
    chrome_context_json: &str,
    fallback: SsrFallback,
    reason: SsrFallbackReason,
) -> Response<Full<Bytes>> {
    // Negotiated before `req` is consumed by the proxy/dispatch arms below.
    let wants_html = routes::catching_up::accepts_html(req.headers());
    match fallback {
        SsrFallback::Registry => match reason {
            SsrFallbackReason::AuthModeUnsupported
            | SsrFallbackReason::Overflow
            | SsrFallbackReason::ErrorBackoff
            | SsrFallbackReason::EmptyRender
            | SsrFallbackReason::RendererAppMismatch
            | SsrFallbackReason::ShellNoProjection
            | SsrFallbackReason::ShellBreakerOpen
            | SsrFallbackReason::ShellFetchFailed
            | SsrFallbackReason::Compose(_) => ssr_spa_shell_fallback_with_skip_reason(
                Some(reason.as_skip_str()),
                Some(chrome_context_json),
            ),
            SsrFallbackReason::RenderError(err) => {
                ssr_spa_shell_fallback_with_error(&err, Some(chrome_context_json))
            }
            SsrFallbackReason::SpecParseError | SsrFallbackReason::RendererAbsent => {
                let agent_cid_owned = resolve_agent_cid_from_request(state, &req);
                let ctx = routes::ForwardCtx {
                    agent_cid: agent_cid_owned.as_deref(),
                };
                routes::forward_to_storage(
                    req,
                    endpoint,
                    path,
                    &state.storage_proxy_client,
                    &state.upstream_breakers,
                    ctx,
                )
                .await
            }
        },
        SsrFallback::ProjectedEpr(projection) => {
            // EVERY fallback serves the projected bundle; the chrome island is
            // spliced INSIDE dispatch_to_projected_epr (never double-spliced —
            // the SSR-serve inject path is a disjoint branch that never runs on
            // any request that reaches here). Tag for observability.
            let resp = dispatch_to_projected_epr(
                state,
                path,
                *projection,
                chrome_context_json,
                wants_html,
            )
            .await;
            with_ssr_skipped_header(resp, &reason)
        }
    }
}

/// The projected app's shell document URL — the browser build's entry file
/// (index.html), which carries the client bundle `<script>`s, `<title>`,
/// `<base>`, and styles. This is the SAME storage `/apps/{epr}/{entry}` surface
/// `dispatch_to_projected_epr` serves on a CSR fallback; a successful SSR render
/// is composed INTO it (see `compose_render_with_shell`) so it can hydrate.
/// Mirrors `dispatch_to_projected_epr`'s `spaFallback=0` propagation.
fn projected_shell_url(
    storage_url: &str,
    projection: &elohim_views::projection::EprProjectionView,
) -> String {
    let base = storage_url.trim_end_matches('/');
    if projection.spa_fallback {
        format!(
            "{}/apps/{}/{}",
            base, projection.epr_id, projection.entry_file
        )
    } else {
        format!(
            "{}/apps/{}/{}?spaFallback=0",
            base, projection.epr_id, projection.entry_file
        )
    }
}

/// Compose a successful (non-empty) SSR render into the projected app's
/// bundle-carrying shell so the served document can hydrate. The V8 render is
/// produced against a minimal document template that carries NO client bundles;
/// the shell (the browser build's index.html — already the CSR-fallback source)
/// carries the `main-<hash>.js` scripts + `<title>` + `<base>` but an empty
/// `<app-root>` placeholder. On success returns the composed HTML. Returns `None`
/// when the shell can't be fetched or composed — the caller then sheds to the
/// bundle fallback (guaranteed hydratable), never serving a bundle-less render.
///
/// Only the `SsrFallback::ProjectedEpr` dispatch (the EPR router — what serves
/// `/`) can resolve a shell URL; the legacy `Registry` path has no projection and
/// returns `None` (its CSR-shell fallback already carries bundles). Honors the
/// same per-upstream breaker + tight timeout the dispatch path uses, so a blocked
/// peer fast-fails here instead of riding the fetch wall on the render hot path.
async fn compose_render_with_shell(
    state: &AppState,
    fallback: &SsrFallback,
    rendered_html: &str,
) -> Result<String, SsrFallbackReason> {
    let SsrFallback::ProjectedEpr(projection) = fallback else {
        return Err(SsrFallbackReason::ShellNoProjection);
    };
    let storage_url = state
        .args
        .storage_url
        .as_deref()
        .ok_or(SsrFallbackReason::ShellNoProjection)?;
    // Endpoint key shared with the gate below — `begin()` consumes a half-open
    // trial when it admits one, and whoever consumes a trial MUST resolve its
    // outcome on every terminal path or the breaker parks in HalfOpen forever
    // (no further trial is ever admitted). The returned guard resolves it even
    // when this future is dropped mid-fetch. The `None` arm records nothing —
    // the gate shed the request; no attempt was made.
    let endpoint = storage_url.trim_end_matches('/');
    let Some(trial) = state.upstream_breakers.begin(endpoint) else {
        return Err(SsrFallbackReason::ShellBreakerOpen);
    };
    let shell_url = projected_shell_url(storage_url, projection);
    let fetch_started = std::time::Instant::now();
    let shell_html = match state
        .storage_proxy_client
        .get(&shell_url)
        .timeout(std::time::Duration::from_secs(EPR_DISPATCH_TIMEOUT_SECS))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(text) => {
                trial.record(true);
                text
            }
            Err(e) => {
                tracing::warn!(
                    target: "doorway::ssr",
                    shell_url = %shell_url,
                    elapsed_ms = fetch_started.elapsed().as_millis() as u64,
                    error = %e,
                    "SSR shell fetch: body read failed"
                );
                trial.record(false);
                return Err(SsrFallbackReason::ShellFetchFailed);
            }
        },
        Ok(resp) => {
            tracing::warn!(
                target: "doorway::ssr",
                shell_url = %shell_url,
                status = resp.status().as_u16(),
                elapsed_ms = fetch_started.elapsed().as_millis() as u64,
                "SSR shell fetch: non-success status"
            );
            trial.record(false);
            return Err(SsrFallbackReason::ShellFetchFailed);
        }
        Err(e) => {
            // A stalled upstream rides the full EPR_DISPATCH_TIMEOUT here — the
            // elapsed_ms field is what distinguishes a 10s timeout (peer stall)
            // from an instant connect refusal in the shed accounting.
            tracing::warn!(
                target: "doorway::ssr",
                shell_url = %shell_url,
                elapsed_ms = fetch_started.elapsed().as_millis() as u64,
                error = %error_with_source_chain(&e),
                "SSR shell fetch failed"
            );
            trial.record(false);
            return Err(SsrFallbackReason::ShellFetchFailed);
        }
    };
    elohim_render::compose_ssr_with_shell(rendered_html, &shell_html)
        .map_err(SsrFallbackReason::Compose)
}

/// Serve a `render:"angular-ssr"` route through the V8 SSR engine, gated by the
/// peer `RenderCapabilityProfile` (auth-mode claim → render semaphore → render
/// cache → per-request fetcher+credential → render trace/stats → chrome splice).
/// Every shed/failure degrades via `fallback`. Shared by BOTH dispatch sites so
/// the SSR body is never duplicated: the legacy registry arm passes
/// `SsrFallback::Registry` (CSR shell / storage proxy), the EPR-router block
/// passes `SsrFallback::ProjectedEpr` (the projected bundle, chrome-carrying).
///
/// OP-GATE INVARIANT: this helper's `forward_to_storage` fallback (Registry
/// variant, spec-parse / renderer-absent) carries NO delegates-compute op-gate
/// and NO write-method guard — the gate lives only in the StorageProxy arm. Both
/// call sites are GET-only: the main-match `SsrRoute` arm classifies a gated
/// write (`POST /db/content*`) as StorageProxy, never SsrRoute (no
/// server-controlled render_spec — pinned by
/// `gated_writes_classify_as_storage_proxy_not_ssr`), and the EPR-router block
/// is guarded by `method == Method::GET`. A gated write can never reach here. If
/// a manifest ever attaches a render_spec to `/db/content*`, hoist the gate into
/// `forward_to_storage` rather than relying on this invariant.
async fn serve_ssr_route(
    state: &AppState,
    req: Request<Incoming>,
    path: &str,
    spec: String,
    endpoint: String,
    observation_id: Option<&str>,
    fallback: SsrFallback,
) -> Response<Full<Bytes>> {
    // Native runtime chrome: build the omnibar element's inline context island
    // ONCE (authoritative slug + authenticated flag from this request). Both the
    // SSR serve and every fallback splice this identical island.
    let chrome_context_json = build_chrome_context_json(path, &req);

    // App-match gate (BEFORE any render/breaker/semaphore work): a renderer
    // can only produce ITS OWN app's markup. A projected route belonging to an
    // app no loaded renderer serves must not be rendered at all — rendering
    // the wrong app burns a V8 pass whose output compose then rightly refuses
    // (the /lamad-dark shape: every lamad request paid a landing-app render
    // just to shed to CSR). Skip render-free, named. Gate and renderer lookup
    // share RendererRegistry::select, so they cannot drift apart.
    if let SsrFallback::ProjectedEpr(projection) = &fallback {
        if state.renderer_registry.mismatch(&projection.epr_id) {
            tracing::info!(
                target: "doorway::ssr",
                path = %path,
                loaded_apps = %state.renderer_registry.app_names(),
                projected_app = %projection.epr_id,
                "no loaded renderer serves this app — SSR skipped without rendering"
            );
            return ssr_fallback_response(
                state,
                req,
                path,
                &endpoint,
                &chrome_context_json,
                fallback,
                SsrFallbackReason::RendererAppMismatch,
            )
            .await;
        }
    }

    // Render-failure circuit breaker (FIRST gate, cheapest + most protective):
    // after FAILURE_THRESHOLD consecutive render errors the breaker is OPEN, so
    // we skip the V8 diversion entirely and serve the fallback tagged
    // x-ssr-skipped: error-backoff — instead of each request burning a doomed
    // render first (the un-cached-failure TTFB balloon during a systemic outage,
    // e.g. a server bundle the runtime cannot load). A single successful render
    // closes it. See `crate::render::breaker`.
    if state.ssr_render_breaker.is_open() {
        return ssr_fallback_response(
            state,
            req,
            path,
            &endpoint,
            &chrome_context_json,
            fallback,
            SsrFallbackReason::ErrorBackoff,
        )
        .await;
    }

    // Auth-mode enforcement: if the doorway publishes a render_capability claim
    // and the request's posture isn't in the claim's auth_modes, fall back with
    // x-ssr-skipped: auth-mode-not-supported. Closes the audit's anonymous-
    // render-of-authenticated-content failure mode at the dispatch boundary.
    if let Some(claim) = state.render_capability.as_ref() {
        let posture = determine_auth_posture(&req);
        if !claim.auth_modes.iter().any(|m| m == posture.as_claim_str()) {
            tracing::info!(
                target: "doorway::ssr",
                path = %path,
                posture = %posture.as_claim_str(),
                claim_modes = ?claim.auth_modes,
                "auth mode not in claim — SSR fallback"
            );
            return ssr_fallback_response(
                state,
                req,
                path,
                &endpoint,
                &chrome_context_json,
                fallback,
                SsrFallbackReason::AuthModeUnsupported,
            )
            .await;
        }
    }

    // Concurrency limiter: bound simultaneous V8 renders by
    // render_capability.max_concurrent_renders (or the conservative default the
    // semaphore was sized to when no capability was derived — see
    // doorway::render::ssr_semaphore_permits). Overflow sheds to the fallback
    // with x-ssr-skipped: overflow — no queueing (a fallback beats waiting).
    let _render_permit = match state.render_semaphore.as_ref() {
        Some(sem) => match Arc::clone(sem).try_acquire_owned() {
            Ok(p) => Some(p),
            Err(_) => {
                tracing::info!(
                    target: "doorway::ssr",
                    path = %path,
                    available = sem.available_permits(),
                    "render semaphore at limit — SSR fallback"
                );
                // SSR saturation is otherwise INVISIBLE to status-code metrics
                // (the shed still serves HTTP 200). Emit a distinct, greppable,
                // aggregation-countable signal on its own `ssr_busy` target.
                tracing::warn!(
                    target: "ssr_busy",
                    counter = "ssr_render_busy_total",
                    path = %path,
                    available = sem.available_permits(),
                    "SSR render queue saturated — shedding to fallback (HTTP 200)"
                );
                return ssr_fallback_response(
                    state,
                    req,
                    path,
                    &endpoint,
                    &chrome_context_json,
                    fallback,
                    SsrFallbackReason::Overflow,
                )
                .await;
            }
        },
        // No semaphore → no limiter; renderer absence is handled below.
        None => None,
    };

    // Select the renderer FOR THIS ROUTE'S APP (the mismatch gate above
    // guarantees a projected app reaching here has one; a legacy Registry
    // route with no projection gets the default renderer).
    let selected_renderer = state.renderer_registry.select(match &fallback {
        SsrFallback::ProjectedEpr(p) => Some(p.epr_id.as_str()),
        SsrFallback::Registry => None,
    });
    if let Some(renderer) = selected_renderer.as_ref() {
        let render_spec = match elohim_render::RenderSpec::parse(&spec) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    target: "doorway::ssr",
                    path = %path,
                    spec = %spec,
                    error = %e,
                    "Unknown render spec — SSR fallback"
                );
                return ssr_fallback_response(
                    state,
                    req,
                    path,
                    &endpoint,
                    &chrome_context_json,
                    fallback,
                    SsrFallbackReason::SpecParseError,
                )
                .await;
            }
        };

        let url = format!(
            "{}{}",
            path,
            req.uri()
                .query()
                .map(|q| format!("?{q}"))
                .unwrap_or_default()
        );

        // Build the originating user's credential for the V8 fetch shim
        // (anonymous → None). Forwards the session header to outbound storage
        // fetches so reach-aware content renders for logged-in users instead of
        // falling back to public — the SSR-audit anonymous-render failure mode.
        //
        // Constructed BEFORE the cache lookup because the fetcher's declared
        // trust scope decides whether this request may touch the shared render
        // cache at all (see below).
        let user_credential = build_ssr_user_credential(&req);
        let fetcher: Arc<dyn elohim_render::DataFetcher> = Arc::new(
            crate::ssr::ResolverFetcher::new(
                Arc::clone(&state.storage_proxy_client),
                endpoint.clone(),
            )
            .maybe_with_user_credential(user_credential),
        );
        // The trust scope of THIS request's fetcher. `Principal` (a credentialed
        // render) is excluded from the render cache in BOTH directions: the
        // cache key carries no principal, so a credentialed render stored under
        // it would be served to every other requester for the TTL, and a
        // credentialed requester served from it would get someone else's
        // reach-gated view. See `crate::ssr::render_is_cache_shareable`.
        let render_trust = fetcher.trust_scope();

        // MVP cache key: (url, spec_version) — deliberately principal-free; the
        // trust gate above, not the key, is what scopes this cache.
        // `fetched_inputs` is captured in the audit trail (RenderOutput) but not
        // in the lookup key. Hash-aware invalidation lands with a DHT signal
        // subscriber driving evictions (Task 14+).
        let cache_key = crate::ssr::render_cache_key(&url, &[], "v1");
        if let Some(cached_html) =
            crate::ssr::cached_render_for_trust(&state.cache, &cache_key, render_trust)
        {
            tracing::debug!(target: "doorway::ssr", path = %path, "SSR cache HIT");
            // The cache holds the UN-composed render (the shell's bundle hashes
            // vary per deploy and must never be baked into the render-TTL entry).
            // Compose it into the bundle-carrying shell at serve time; on failure
            // shed to the hydratable bundle fallback rather than serve a
            // bundle-less document.
            return match compose_render_with_shell(state, &fallback, &cached_html).await {
                Ok(composed) => ssr_html_response_with_observability(
                    composed,
                    "HIT",
                    state.render_capability.as_ref(),
                    None,
                    Some(&chrome_context_json),
                ),
                Err(reason) => {
                    tracing::warn!(
                        target: "doorway::ssr",
                        path = %path,
                        reason = reason.as_skip_str(),
                        "SSR shell composition unavailable (cache HIT) — serving hydratable bundle fallback"
                    );
                    ssr_fallback_response(
                        state,
                        req,
                        path,
                        &endpoint,
                        &chrome_context_json,
                        fallback,
                        reason,
                    )
                    .await
                }
            };
        }

        // Test-only stall fault injection (env-gated, off by default), applied to
        // the per-request fetcher (renders run against ctx.data_fetcher). Applied
        // after the cache lookup so the fault-injection warning marks an actual
        // render, not a cache hit. The decorator delegates `trust_scope`, so
        // `render_trust` computed above still describes this fetcher.
        let fetcher = crate::ssr::maybe_inject_stall_fault(fetcher);
        // 60s wall: cold-start parses a ~51MB bundle (171 .mjs files) + walks
        // Angular bootstrap (which fetches); warm renders settle to tens of ms
        // once deno_core's module loader caches imports + V8 JIT settles.
        let ctx = elohim_render::RenderContext {
            spec: render_spec,
            url: url.clone(),
            data_fetcher: fetcher,
            limits: elohim_render::RenderLimits {
                wall_time_ms: 60_000,
                ..Default::default()
            },
        };
        return match renderer.render(ctx).await {
            Ok(mut out) => {
                // Stamp the correlation token onto the trace so the header + Loki
                // line join browser ↔ doorway ↔ peer.
                if let Some(obs) = observation_id {
                    out.trace.trace_id = obs.to_string();
                }
                // The render-trace signal: terminal classification + per-peer
                // latency. Shipped to Loki (info) and the feed-forward seam for
                // compute-commitment tuning. Emitted BEFORE the empty-shell check
                // so observability sees every render, empty or not.
                tracing::info!(
                    target: "doorway::ssr::trace",
                    path = %path,
                    terminal = out.trace.terminal.as_str(),
                    fetches = out.trace.fetches.len(),
                    wall_ms = out.trace.wall_ms,
                    observation_id = observation_id.unwrap_or(""),
                    "SSR render trace"
                );
                state.render_trace_stats.record(&out.trace);

                // A render can "succeed" yet activate NO route component (a caught
                // bootstrap error inside the app) — an empty <router-outlet> shell.
                // The SSR document template carries no client bundles, so that
                // shell can never hydrate: serving or caching it is a permanent
                // blank page. Treat it exactly like a render error — record a
                // breaker failure, do NOT record success, do NOT cache, and shed to
                // the fallback bundle (which DOES carry the client bundles). See
                // `render_output_is_empty`.
                if render_output_is_empty(&out.html) {
                    // Record the failure; a `true` return is the trip transition —
                    // emit ONE warn when the breaker OPENS (mirrors the Err arm).
                    if state.ssr_render_breaker.record_failure() {
                        tracing::warn!(
                            target: "doorway::ssr",
                            path = %path,
                            threshold = crate::render::breaker::FAILURE_THRESHOLD,
                            cooldown_ms = crate::render::breaker::COOLDOWN_MS,
                            "SSR render breaker OPEN — skipping SSR for the cooldown window \
                             (subsequent skips are silent)"
                        );
                    }
                    tracing::warn!(
                        target: "doorway::ssr",
                        path = %path,
                        "SSR render produced an empty app shell (no activated route component) — SSR fallback"
                    );
                    return ssr_fallback_response(
                        state,
                        req,
                        path,
                        &endpoint,
                        &chrome_context_json,
                        fallback,
                        SsrFallbackReason::EmptyRender,
                    )
                    .await;
                }

                // A successful, non-empty render closes the failure breaker
                // (resets the consecutive-error count).
                state.ssr_render_breaker.record_success();
                let trace = out.trace;
                let html = out.html;
                // Cache the UN-injected, UN-composed render; the chrome island
                // (slug/auth) AND the bundle-carrying shell (per-deploy bundle
                // hashes) are both request/deploy-specific and spliced at serve
                // time on both MISS and a later HIT — never baked into the entry.
                // Trust-scoped: a credentialed (`Principal`) render is NOT
                // written — the key carries no principal, so caching it would
                // serve one user's reach-gated view to everyone for the TTL.
                crate::ssr::cache_render_for_trust(
                    &state.cache,
                    &cache_key,
                    html.clone(),
                    std::time::Duration::from_secs(5 * 60),
                    render_trust,
                );
                // Compose the render into the bundle-carrying shell so it can
                // hydrate; on failure shed to the hydratable bundle fallback.
                match compose_render_with_shell(state, &fallback, &html).await {
                    Ok(composed) => ssr_html_response_with_observability(
                        composed,
                        "MISS",
                        state.render_capability.as_ref(),
                        Some(&trace),
                        Some(&chrome_context_json),
                    ),
                    Err(reason) => {
                        tracing::warn!(
                            target: "doorway::ssr",
                            path = %path,
                            reason = reason.as_skip_str(),
                            "SSR shell composition unavailable — serving hydratable bundle fallback"
                        );
                        ssr_fallback_response(
                            state,
                            req,
                            path,
                            &endpoint,
                            &chrome_context_json,
                            fallback,
                            reason,
                        )
                        .await
                    }
                }
            }
            Err(e) => {
                let err_str = format!("{e}");
                // Record the failure; a `true` return is the trip transition —
                // emit ONE warn when the breaker OPENS, not one per subsequent
                // skipped request (those are silent; ongoing failures still show
                // via the per-render warn below on each half-open trial).
                if state.ssr_render_breaker.record_failure() {
                    tracing::warn!(
                        target: "doorway::ssr",
                        path = %path,
                        threshold = crate::render::breaker::FAILURE_THRESHOLD,
                        cooldown_ms = crate::render::breaker::COOLDOWN_MS,
                        error = %err_str,
                        "SSR render breaker OPEN — skipping SSR for the cooldown window \
                         (subsequent skips are silent)"
                    );
                }
                tracing::warn!(
                    target: "doorway::ssr",
                    path = %path,
                    error = %err_str,
                    "SSR render error — SSR fallback"
                );
                ssr_fallback_response(
                    state,
                    req,
                    path,
                    &endpoint,
                    &chrome_context_json,
                    fallback,
                    SsrFallbackReason::RenderError(err_str),
                )
                .await
            }
        };
    }

    // Renderer absent (SSR_BUNDLE_PATH not set) — fall back so the Angular CSR
    // bundle can hydrate client-side (Registry: storage proxy; the EPR site
    // never reaches here — it short-circuits renderer-absent to the bundle).
    tracing::debug!(
        target: "doorway::ssr",
        path = %path,
        "SSR-eligible route but no renderer — SSR fallback"
    );
    ssr_fallback_response(
        state,
        req,
        path,
        &endpoint,
        &chrome_context_json,
        fallback,
        SsrFallbackReason::RendererAbsent,
    )
    .await
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

    // ── Pillar 2 / layer 3: membrane rate-limit policy ────────────────────────
    // Placed AFTER the wisdom gate and BEFORE the admission semaphore. Health,
    // /metrics, and WebSocket upgrades are already short-circuited above or
    // handled by `is_static_asset` inside `apply_membrane`. The membrane only
    // fires when the caller SHOULD have reached routing — it doesn't replace the
    // semaphore but it sheds bad actors before they consume a slot.
    {
        let is_ws = hyper_tungstenite::is_upgrade_request(&req);
        if !admission_exempt(&path, is_ws) {
            if let Some(membrane_resp) = apply_membrane(&state, &req, &addr, &path).await {
                return Ok(membrane_resp);
            }
        }
    }

    // ── Pillar 2 / layer 2: global inbound admission ──────────────────────────
    // Placed AFTER the wisdom gate and BEFORE routing. Liveness, /version, and
    // WebSocket upgrades are exempt (admission_exempt) so /health stays
    // answerable while we shed. SHED (try_acquire_owned), never QUEUE — an
    // unbounded queue is just a slower wedge. The permit is bound at function
    // scope (`_admit`, a named binding, NOT bare `_`) so it is held across the
    // downstream forward_to_storage await for the whole request.
    let is_upgrade = hyper_tungstenite::is_upgrade_request(&req);
    let _admit = if admission_exempt(&path, is_upgrade) {
        None
    } else {
        // Reads (GET/HEAD) draw from a separate, larger pool than writes so a
        // write burst (proxied seed storm) can't starve cheap projection reads —
        // the `/epr/*` read-path shed in genesis #1272 E2E.
        let is_read = method == hyper::Method::GET || method == hyper::Method::HEAD;
        let sem = if is_read {
            &state.read_semaphore
        } else {
            &state.inbound_semaphore
        };
        match Arc::clone(sem).try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                // Admission saturation is otherwise invisible to status-code
                // metrics on the happy path. Emit a distinct, greppable,
                // aggregation-countable signal (Plan D seam).
                tracing::warn!(
                    target: "admission_busy",
                    counter = "doorway_admission_shed_total",
                    path = %path,
                    pool = if is_read { "read" } else { "write" },
                    available = sem.available_permits(),
                    "inbound admission at ceiling — shedding (503 + Retry-After)"
                );
                crate::metrics::inc_admission_shed();
                return Ok(to_boxed(catching_up_response(
                    routes::catching_up::accepts_html(req.headers()),
                    DOORWAY_ADMISSION_RETRY_AFTER_SECS,
                )));
            }
        }
    };

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

    // Handle auth routes (/auth/*) - only paths the auth layer owns.
    // Unowned /auth/* paths (e.g. /auth/portal) fall through to the EPR router.
    if is_auth_owned_path(&path) {
        if let Some(response) = routes::handle_auth_request(req, Arc::clone(&state)).await {
            return Ok(response);
        }
        return Ok(to_boxed(not_found_response(&path)));
    }

    // ── B13: EPR router — consult BEFORE the main match block ─────────────────
    //
    // Only fires for GET requests on non-service paths. Service paths (health,
    // admin, api/v1/*, db/*, apps/*, etc.) are listed in `is_service_path` and
    // must reach their own explicit match arms — even when a root projection
    // (url_path = "/") is registered.
    //
    // WebSocket upgrades are also excluded: the EPR router only serves static
    // SPA/asset bundles, never WebSocket streams.
    if method == Method::GET
        && !hyper_tungstenite::is_upgrade_request(&req)
        && !is_service_path(&path)
    {
        // Slice 3 alias law (spec §4): a notarized alias promise 302s BEFORE
        // mount dispatch — template aliases may live under a live mount
        // (e.g. /lamad/resource/{id}).
        if let Some(location) = state.epr_router.resolve_alias(&path) {
            tracing::debug!(path = %path, location = %location,
                "alias promise matched — 302 (redirects_from / redirectTemplates)");
            return Ok(to_boxed(
                Response::builder()
                    .status(StatusCode::FOUND)
                    .header("Location", location)
                    .header("cache-control", "public, max-age=300")
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            ));
        }
        if let Some(projection) = state.epr_router.dispatch(&path) {
            tracing::debug!(
                path = %path,
                epr_id = %projection.epr_id,
                url_path = %projection.url_path,
                "EPR router matched — dispatching to projected bundle"
            );
            // The manifest `render` field is the agnostic SSR contract: a
            // projection whose path is ALSO declared render:"angular-ssr" must
            // serve through the V8 SSR engine (peer-capability-gated), not the
            // static projected bundle — the EPR router would otherwise shadow it
            // (Loki-verified: zero SSR renders reached the main-match arm). Also
            // consult the registry; ONLY the SsrRoute disposition diverts. GET-
            // only (the enclosing `method == Method::GET` guard), so a gated write
            // can never reach the SSR helper — the op-gate invariant holds.
            let dispo = classify_dispatch(&state.route_registry, &method, &path).await;
            if epr_should_serve_ssr(&dispo, state.renderer_registry.any_loaded()) {
                if let Disposition::SsrRoute { spec, endpoint } = dispo {
                    // EVERY SSR shed/failure degrades to the projected bundle
                    // (chrome-carrying) — i.e. exactly today's EPR serving.
                    return Ok(to_boxed(
                        serve_ssr_route(
                            &state,
                            req,
                            &path,
                            spec,
                            endpoint,
                            observation_id.as_deref(),
                            SsrFallback::ProjectedEpr(Box::new(projection)),
                        )
                        .await,
                    ));
                }
                // Unreachable: epr_should_serve_ssr guaranteed SsrRoute.
            }
            // Any other disposition (StorageProxy / RegistryUnhandled / NotFound)
            // or a renderer-absent doorway: serve the projected bundle as today —
            // zero behavioral change for assets, deep links, SPA fallback, alias-
            // 302s, and the catch-all. Build the omnibar context once so the HTML
            // serve carries the trust surface.
            let chrome_context_json = build_chrome_context_json(&path, &req);
            let wants_html = routes::catching_up::accepts_html(req.headers());
            return Ok(to_boxed(
                dispatch_to_projected_epr(
                    &state,
                    &path,
                    projection,
                    &chrome_context_json,
                    wants_html,
                )
                .await,
            ));
        }
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

        // Prometheus exposition — doorway-local Operational metrics (the
        // complement surface; storage owns the per-node memory/corpus surface).
        // On the MAIN listener, admission-exempt, no auth (in-cluster scrape).
        // Needs doorway-specific logic (renders the in-process registry, not a
        // storage proxy) → an explicit arm above the registry fallback is correct.
        (Method::GET, "/metrics") => to_boxed(routes::handle_metrics()),

        // Runtime chrome static assets — the content-addressed `omni-element.js`
        // (the self-contained, runtime-served EPR omnibar client element).
        // Doorway-specific runtime chrome (same legitimate class as
        // bootstrap/signal/metrics — a surface the doorway OWNS and paints, NOT
        // a per-domain proxy of substrate truth). There is no storage endpoint
        // behind it and every doorway serves identical content-addressed bytes
        // from the linked elohim-render crate, so it must NOT flow through the
        // manifest registry. Explicit, documented exception ABOVE the wildcard
        // registry arm (see doorway/CLAUDE.md "No Per-Domain Proxy Files" and
        // server/CLAUDE.md). Content-addressed ⇒ immutable cache; unknown
        // /chrome/* path ⇒ 404.
        (Method::GET, p) if p.starts_with("/chrome/") => {
            to_boxed(routes::handle_chrome_asset(p))
        }

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

        // Universal-resolver DID resolution (W3C, universal-resolver-compatible).
        // did:key resolves locally/offline; did:elohim forwards to storage's
        // /db/identity/did/{did}; did:web and other methods → methodNotSupported.
        // DID bridge design §3.5. TWO-GATE: this arm MUST have a matching entry in
        // `is_service_path` (the "/1.0/" prefix) or the EPR router shadows it to
        // the SPA bundle (the /auth/portal shadow trap).
        (Method::GET, p) if p == "/1.0/identifiers" || p.starts_with("/1.0/identifiers/") => {
            let did_param = p.strip_prefix("/1.0/identifiers/").unwrap_or("");
            let accept = req
                .headers()
                .get("accept")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            to_boxed(
                routes::handle_universal_resolver(Arc::clone(&state), did_param, accept).await,
            )
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

        // Cross-edge EPR-head coherence — this edge's self-fingerprint (CIDv1
        // dag-cbor digest). Detect-only; F-DEPLOY/F-EDGE consume the route.
        (Method::GET, "/api/v1/federation/coherence") => to_boxed(
            routes::coherence::handle_federation_coherence(Arc::clone(&state)).await,
        ),

        // Hosted-at binding resolution (T2.2). Doorway-specific logic (single
        // storage read + honest 404/502 split), so it earns an explicit arm
        // rather than a registry declaration. `/api/` is already covered by
        // `is_service_path`, which keeps the EPR router from shadowing this to
        // the SPA bundle — the /auth/portal incident shape. Guarded by
        // `is_service_path_covers_hosted_binding`.
        (Method::GET, p) if p.starts_with("/api/v1/federation/hosted-binding/") => {
            let agent = p.trim_start_matches("/api/v1/federation/hosted-binding/");
            to_boxed(routes::handle_federation_hosted_binding(Arc::clone(&state), agent).await)
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
                // Post-B14: ROOT_APP_SLUG is gone. The EPR router (consulted earlier in
                // handle_request) serves "/" when a projection exists for url_path="/".
                // We only reach this arm when NO root projection is registered. On a
                // doorway meant to project a root landing (e.g. elohim.host), an empty
                // router means the storage/conductor backend is unreachable (the
                // connect-refused outage) — the EprRouter could not be populated.
                //
                // Serve an HONEST LAYERED degraded page — NOT a 302 to the operator
                // dashboard (leaks ops to public visitors) and NOT a blunt 503. Report
                // what THIS doorway can see at its own layer (federation peers +
                // projection cache) and be explicit that the substrate dataplane
                // (Holochain DHT / libp2p / iroh, via the conductor) is NOT visible —
                // so humans/content/peers are unknown, NOT zero. (See `root_unavailable_html`.)
                let routes_loaded = state.epr_router.len();
                let generation = state.epr_router.generation();
                let federation_peers =
                    crate::services::federation::get_peer_urls(&state.peer_url_list)
                        .await
                        .len();
                to_boxed(root_unavailable_response(
                    routes_loaded,
                    generation,
                    federation_peers,
                ))
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

        // Render capability profile — SSR bundle inventory + renderer kinds derived at startup.
        // No auth required: storage peers pull this to discover SSR availability.
        (Method::GET, "/admin/capability") => {
            to_boxed(routes::handle_admin_capability(Arc::clone(&state)).await)
        }

        // Render-trace feed-forward aggregate (terminal classes + latency)
        (Method::GET, "/admin/render-stats") => {
            to_boxed(routes::handle_admin_render_stats(Arc::clone(&state)).await)
        }

        // Unified self-healing read model — Cat C node-local Operational state
        // (genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md §6). Composed fresh
        // per request from in-process snapshots + on-demand projector fetch.
        // Legitimate doorway-local aggregate (NOT a per-domain proxy): fails the
        // swap test by design — a node serves its OWN runtime state, same class
        // as /admin/capability + /admin/render-stats. No auth (operator-only is
        // an ingress property). PENDING sibling keys (autoPreset/admission/
        // upstreams) are null/empty with FOLLOW-ON wire-up seams.
        (Method::GET, "/admin/self-healing") => {
            to_boxed(routes::handle_self_healing(Arc::clone(&state)).await)
        }

        // Conductor pool visibility (available on ALL instances)
        (Method::GET, "/admin/conductors") => {
            to_boxed(routes::handle_list_conductors(&req, Arc::clone(&state)).await)
        }

        // Route registry visibility — which routes are live and from which source
        (Method::GET, "/admin/routes") => {
            to_boxed(routes::handle_route_registry(Arc::clone(&state)).await)
        }

        // Route registry actuation twin — re-fetch every configured storage
        // peer's /manifest and recompile its routes in place. Closes the
        // boot-only registration gap: a storage deploy that ADDS a route no
        // longer needs a doorway restart to become reachable through it.
        (Method::POST, "/admin/steward-peers/refresh") => {
            to_boxed(routes::handle_steward_peers_refresh(Arc::clone(&state)).await)
        }

        // SSR bundle-head reconcile actuation (Track-4 T4-1): run one pass that
        // re-resolves each served slug's declared serverBlobHash and hot-swaps
        // the isolate on a mismatch — converge-to-declared-head without a
        // doorway restart. Actuation twin of the background reconcile tick;
        // same operator-seat class as /admin/steward-peers/refresh. Covered by
        // is_service_path via the "/admin" prefix (POST, so the GET-only EPR
        // router never shadows it regardless).
        (Method::POST, "/admin/ssr-bundle/refresh") => {
            to_boxed(routes::handle_ssr_bundle_refresh(Arc::clone(&state)).await)
        }

        // Kitsune2 bootstrap coherence read-model (Cat-C node-local Operational
        // state): spaces/agents held + backend (mem|mongo). With the shared
        // mongo backend live, this is the surface that shows the genesis pair
        // converged on ONE bootstrap table. Same class as /admin/self-healing.
        (Method::GET, "/admin/bootstrap-coherence") => to_boxed(
            routes::bootstrap_coherence::handle_bootstrap_coherence(Arc::clone(&state)).await,
        ),

        // Conductor agents listing
        (Method::GET, p) if p.starts_with("/admin/conductors/") && p.ends_with("/agents") => {
            let conductor_id = p
                .strip_prefix("/admin/conductors/")
                .and_then(|s| s.strip_suffix("/agents"))
                .unwrap_or("");
            to_boxed(routes::handle_conductor_agents(&req, Arc::clone(&state), conductor_id).await)
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
            to_boxed(routes::handle_agent_conductor(&req, Arc::clone(&state), agent_key).await)
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
            match routes::seed::require_seed_authority(&state, &req) {
                Ok(()) => to_boxed(routes::admin_cache::cache_disable(Arc::clone(&state)).await),
                Err(resp) => to_boxed(resp),
            }
        }
        (Method::POST, "/admin/cache/enable") => {
            match routes::seed::require_seed_authority(&state, &req) {
                Ok(()) => to_boxed(routes::admin_cache::cache_enable(Arc::clone(&state)).await),
                Err(resp) => to_boxed(resp),
            }
        }
        (Method::POST, p) if p.starts_with("/admin/cache/clear/") => {
            match routes::seed::require_seed_authority(&state, &req) {
                Ok(()) => {
                    let slug = p.strip_prefix("/admin/cache/clear/").unwrap_or("");
                    to_boxed(routes::admin_cache::cache_clear_slug(Arc::clone(&state), slug).await)
                }
                Err(resp) => to_boxed(resp),
            }
        }
        (Method::POST, "/admin/cache/warm") => {
            match routes::seed::require_seed_authority(&state, &req) {
                Ok(()) => to_boxed(routes::admin_cache::cache_warm(Arc::clone(&state)).await),
                Err(resp) => to_boxed(resp),
            }
        }

        // ====================================================================
        // DEV/FIXTURE ONLY — doorway-local portal-host health override.
        // Hard-gated behind dev_mode (403 FIXTURE_ONLY otherwise). Lets a2o
        // fixtures assert a portal host reachable/unreachable without standing
        // up a real server at a non-resolving .example origin. Doorway-local
        // OPERATIONAL state — never touches the notarized portal_hosts entry.
        // ====================================================================
        (Method::PUT, "/admin/dev/portal-health") => {
            to_boxed(routes::admin_dev::handle_set_portal_health(req, Arc::clone(&state)).await)
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

        // Kitsune2 bootstrap protocol (HC 0.6 conductors):
        //   PUT /bootstrap/{spaceB64Url}/{agentB64Url}  — publish agent info
        //   GET /bootstrap/{spaceB64Url}                — list agent infos
        // Before 2026-06-12 these fell through the route registry to 404 and
        // conductor-plane peer discovery never worked (each conductor was its
        // own DHT island — the custody-convergence root cause).
        (Method::PUT, p) if p.starts_with("/bootstrap/") => {
            handle_k2_bootstrap_put(state, req, &path).await
        }
        (Method::GET, p) if p.starts_with("/bootstrap/") => {
            handle_k2_bootstrap_get(state, &path).await
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

        // /db/* routes — handled by the dynamic route registry below.
        //
        // Removed 2026-05-25 as part of Pattern Z anti-pattern cleanup
        // (genesis/docs/superpowers/specs/2026-05-23-doorway-access-tier-patterns.md).
        // The old hardcoded short-circuit (handle_db_request in routes/db.rs)
        // was the 14th instance of the per-domain proxy anti-pattern that
        // doorway/CLAUDE.md says was deleted: "We deleted 13 identical
        // proxy files that violated this rule. They must never come back."
        // Storage's build_manifest() declares 28+ /db/* routes
        // (elohim/elohim-storage/src/http.rs ~L9248+); the route registry
        // compiles them as StorageProxy entries and forward_to_storage
        // (storage_proxy.rs) handles all methods (GET/POST/PUT/PATCH/
        // DELETE/HEAD) with the same CORP cross-origin header parity.
        // Net change: -1 hand-maintained method list, +1 manifest-driven
        // route table.

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

        // Sitemap (spec §7.5): a derived projection of the routing table —
        // mounts plus claimed commons entries. Event-invalidated by the EPR
        // router generation; it cannot drift from dispatch.
        (Method::GET, "/sitemap.xml") => {
            // Derive the public origin from the Host header (the canonical
            // doorway-origin pattern used by the streaming/SEO arms — no
            // public_url/external_url field exists on Args).
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
            to_boxed(serve_sitemap(&state, &base_url).await)
        }

        // Universal EPR address (§12.1): /epr/{id} → root bundle (shell epr/:id route).
        (Method::GET, p) if p == "/epr" || p.starts_with("/epr/") => {
            let chrome_context_json = build_chrome_context_json(p, &req);
            let wants_html = routes::catching_up::accepts_html(req.headers());
            return Ok(to_boxed(
                dispatch_epr_universal(&state, p, &chrome_context_json, wants_html).await,
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

        // pkarr resolver endpoint — cutover gate #10 (n0 mitigation step 2).
        // Doorway-specific because the bytes terminate inside doorway's LRU
        // cache; not a storage-proxy concern.
        (m, p) if p.starts_with("/pkarr/") => {
            let key = &p["/pkarr/".len()..];
            let svc = match state.pkarr_resolver.as_ref() {
                Some(s) => Arc::clone(s),
                None => {
                    return Ok(to_boxed(
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header("content-type", "text/plain; charset=utf-8")
                            .body(Full::new(Bytes::from(
                                "pkarr resolver not enabled on this doorway",
                            )))
                            .expect("response builds"),
                    ));
                }
            };
            match m {
                Method::GET => {
                    let ims = req
                        .headers()
                        .get(hyper::header::IF_MODIFIED_SINCE)
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    return Ok(to_boxed(
                        routes::pkarr_resolver::handle_get_pkarr(svc, key, ims.as_deref()).await,
                    ));
                }
                Method::PUT => {
                    let body_bytes = match req.collect().await {
                        Ok(collected) => collected.to_bytes(),
                        Err(e) => {
                            warn!("pkarr PUT body read error: {}", e);
                            return Ok(to_boxed(bad_request_response("Failed to read body")));
                        }
                    };
                    // Enforce size cap before passing to handler (handler double-checks).
                    if body_bytes.len() as u64
                        > pkarr::SignedPacket::MAX_BYTES + 1
                    {
                        return Ok(to_boxed(
                            Response::builder()
                                .status(StatusCode::PAYLOAD_TOO_LARGE)
                                .body(Full::new(Bytes::new()))
                                .expect("response builds"),
                        ));
                    }
                    return Ok(to_boxed(
                        routes::pkarr_resolver::handle_put_pkarr(
                            svc,
                            key,
                            body_bytes,
                            None,
                        )
                        .await,
                    ));
                }
                _ => {
                    return Ok(to_boxed(
                        Response::builder()
                            .status(StatusCode::METHOD_NOT_ALLOWED)
                            .body(Full::new(Bytes::new()))
                            .expect("response builds"),
                    ));
                }
            }
        }

        // ====================================================================
        // Dynamic Route Registry + Manifest-driven SSR + SPA fallback.
        //
        // Task 13: manifest-driven SSR dispatch replaces the temporary
        // /render-test/* hardcoded arm. Any route declared in elohim-storage's
        // manifest with `render = "angular-ssr"` is dispatched through the
        // in-process Renderer when SSR_BUNDLE_PATH is configured at startup.
        // Routes without a render spec, and all other registry-matched routes,
        // fall through to the existing StorageProxy forwarder.
        //
        // Dispatch priority:
        //   1. Registry match + render_spec set + renderer available → SSR render
        //   2. Registry match + render_spec set + no renderer → StorageProxy fallback
        //   3. Registry match + no render_spec → StorageProxy
        //   4. Registry miss → 404
        //      (Root-path dispatch is handled by the EPR router above this match block;
        //       ROOT_APP_SLUG is gone — the projection whose url_path="/" is the root.)
        // ====================================================================
        (_, p) => {
            let dispo = classify_dispatch(
                &state.route_registry,
                req.method(),
                p,
            )
            .await;

            match dispo {
                Disposition::SsrRoute { spec, endpoint } => {
                    // Manifest-driven SSR dispatch (Task 13). The whole SSR body —
                    // auth-mode gate, render semaphore, cache, per-request
                    // fetcher+credential, render trace/stats, chrome splice, and
                    // every fallback — lives in `serve_ssr_route` and is shared
                    // with the EPR-router dispatch site (no duplication). This arm
                    // passes SsrFallback::Registry: sheds go to the CSR shell,
                    // spec-parse / renderer-absent forward to storage — the
                    // pre-existing behavior, unchanged.
                    //
                    // OP-GATE INVARIANT (full statement on serve_ssr_route): a
                    // gated write (`POST /db/content*`) never classifies as
                    // SsrRoute (no server-controlled render_spec), so the helper's
                    // un-gated forward_to_storage fallback can never serve one.
                    // Pinned by `gated_writes_classify_as_storage_proxy_not_ssr`.
                    // Do NOT add the gate here; keep gated writes out.
                    return Ok(to_boxed(
                        serve_ssr_route(
                            &state,
                            req,
                            p,
                            spec,
                            endpoint,
                            observation_id.as_deref(),
                            SsrFallback::Registry,
                        )
                        .await,
                    ));
                }

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
                                &state.storage_proxy_client,
                                &state.upstream_breakers,
                                ctx,
                            )
                            .await,
                        ));
                    }

                    // ── delegates-compute op-gate ─────────────────────────────
                    // Gate POST /db/content and POST /db/content/bulk before
                    // forwarding.  Off → no storage call (exact passthrough).
                    // Observe → log verdict, always forward.
                    // Enforce → allowed → forward; denied|error → 403 fail-closed.
                    {
                        let gate_mode = state.args.op_gate_mode.clone();
                        let is_gated_write = *req.method() == Method::POST
                            && (p == "/db/content" || p == "/db/content/bulk");
                        if gate_mode != OpGateMode::Off && is_gated_write {
                            // performer = doorway-verified human_id (never the client
                            // X-Agent-Cid header, which is trivially spoofable — C8).
                            // The authorize call runs ONLY when a performer exists; the
                            // no-credential case branches by mode (D2: observe is a
                            // non-blocking shadow stage — log the would-deny but never
                            // block; only enforce fails closed).
                            match agent_cid_owned.as_ref() {
                                Some(performer) => {
                                    let performer = performer.clone();
                                    // Forward the user's own Bearer (D6: storage logs requester identity).
                                    let bearer = extract_bearer_from_req(&req);
                                    let verdict = call_authorize_operation(
                                        &state.storage_proxy_client,
                                        bearer.as_deref(),
                                        &performer,
                                        "orchestrate-node",
                                        &endpoint,
                                    )
                                    .await;
                                    let allowed_result: Result<bool, String> = match &verdict {
                                        Ok(v) => Ok(v.allowed),
                                        Err(e) => Err(e.clone()),
                                    };
                                    match compute_op_gate_decision(
                                        &gate_mode,
                                        req.method(),
                                        p,
                                        &allowed_result,
                                    ) {
                                        OpGateDecision::Allow => { /* fall through to forward */ }
                                        OpGateDecision::AllowWithWarn => {
                                            let summary = match &verdict {
                                                Ok(v) if v.allowed => {
                                                    format!("would-allow: {}", v.reason)
                                                }
                                                Ok(v) => format!("would-deny: {}", v.reason),
                                                Err(e) => format!("would-deny (error): {e}"),
                                            };
                                            warn!(
                                                path = %p,
                                                %performer,
                                                %summary,
                                                "op-gate OBSERVE"
                                            );
                                        }
                                        OpGateDecision::Deny403 => {
                                            match &verdict {
                                                Ok(v) => warn!(
                                                    path = %p,
                                                    %performer,
                                                    reason = %v.reason,
                                                    "op-gate ENFORCE deny"
                                                ),
                                                Err(e) => warn!(
                                                    path = %p,
                                                    %performer,
                                                    error = %e,
                                                    "op-gate ENFORCE deny (infra error)"
                                                ),
                                            }
                                            return Ok(to_boxed(make_op_gate_forbidden()));
                                        }
                                    }
                                }
                                None => {
                                    // No verified credential ⇒ no performer to authorize.
                                    // Observe must NOT block (D2: log the would-deny and
                                    // forward); only Enforce fails closed with a 403.
                                    match no_credential_decision(&gate_mode) {
                                        NoCredentialDecision::Forward => {
                                            warn!(
                                                path = %p,
                                                "op-gate OBSERVE: no verified credential; \
                                                 would-deny (forwarding)"
                                            );
                                            // fall through to forward
                                        }
                                        NoCredentialDecision::Deny => {
                                            warn!(
                                                path = %p,
                                                "op-gate: no verified credential; denying (fail-closed)"
                                            );
                                            return Ok(to_boxed(make_op_gate_forbidden()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // ── end op-gate ───────────────────────────────────────────

                    return Ok(to_boxed(
                        routes::forward_to_storage(
                        req,
                        &endpoint,
                        p,
                        &state.storage_proxy_client,
                        &state.upstream_breakers,
                        ctx,
                    )
                    .await,
                    ));
                }
                Disposition::RegistryUnhandled => {
                    debug!(path = %p, "Registry matched but target type not yet handled");
                    to_boxed(not_found_response(p))
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

/// Handle a kitsune2 bootstrap PUT: `PUT /bootstrap/{spaceB64Url}/{agentB64Url}`.
///
/// Body is the JSON-encoded signed agent info; validation (path/body match,
/// timing windows, ed25519 signature) lives in `bootstrap::k2`.
async fn handle_k2_bootstrap_put(
    state: Arc<AppState>,
    req: Request<Incoming>,
    path: &str,
) -> Response<BoxBody> {
    let Some(store) = state.bootstrap.as_ref().map(Arc::clone) else {
        return to_boxed(service_unavailable_response(
            "Bootstrap service not enabled",
        ));
    };

    let rest = path.strip_prefix("/bootstrap/").unwrap_or("");
    let segments: Vec<&str> = rest.split('/').collect();
    if segments.len() != 2 || segments.iter().any(|s| s.is_empty()) {
        return to_boxed(bad_request_response(
            "Expected PUT /bootstrap/{space}/{agent}",
        ));
    }
    let (space_b64, agent_b64) = (segments[0].to_string(), segments[1].to_string());

    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("k2 bootstrap PUT body error: {}", e);
            return to_boxed(bad_request_response("Failed to read request body"));
        }
    };

    match store
        .k2()
        .put_at(
            &space_b64,
            &agent_b64,
            &body,
            crate::bootstrap::k2::current_time_micros(),
        )
        .await
    {
        Ok(outcome) => {
            debug!(
                "k2 bootstrap PUT {:?} space={} agent={}",
                outcome, space_b64, agent_b64
            );
            to_boxed(
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from("{}")))
                    .unwrap(),
            )
        }
        Err(e) => {
            warn!(
                "k2 bootstrap PUT rejected space={} agent={}: {}",
                space_b64, agent_b64, e
            );
            to_boxed(bad_request_response(&e))
        }
    }
}

/// Handle a kitsune2 bootstrap GET: `GET /bootstrap/{spaceB64Url}` — returns
/// the JSON array of signed agent infos currently held for the space.
async fn handle_k2_bootstrap_get(state: Arc<AppState>, path: &str) -> Response<BoxBody> {
    let Some(store) = state.bootstrap.as_ref().map(Arc::clone) else {
        return to_boxed(service_unavailable_response(
            "Bootstrap service not enabled",
        ));
    };

    let rest = path.strip_prefix("/bootstrap/").unwrap_or("");
    if rest.is_empty() || rest.contains('/') {
        return to_boxed(bad_request_response("Expected GET /bootstrap/{space}"));
    }

    let body = store
        .k2()
        .get_at(rest, crate::bootstrap::k2::current_time_micros())
        .await;
    to_boxed(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap(),
    )
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

/// Service unavailable response
fn service_unavailable_response(message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "Service Unavailable",
        "message": message
    });

    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
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

/// True ONLY for paths that must NEVER be shed by the inbound admission gate:
/// the liveness/health family, /version, and ANY WebSocket upgrade. Exempt paths
/// take NO permit (`_admit = None`) — upgrades (signal, /debug/stream) are
/// long-lived relays that must not occupy a bounded admission slot for their
/// whole socket lifetime, and liveness must answer even at zero permits.
/// Everything else — proxy, api, blob, apps, SPA/EPR — is gated. Deliberately
/// NARROW: gating on `!is_service_path` would exempt /api,/db,/blob,/apps and
/// gut the gate.
fn admission_exempt(path: &str, is_upgrade: bool) -> bool {
    if is_upgrade {
        return true;
    }
    matches!(
        path,
        "/health"
            | "/healthz"
            | "/health/startup"
            | "/ready"
            | "/readyz"
            | "/version"
            // A scrape endpoint that gets 503'd under admission pressure is dead
            // exactly when you need it — never shed /metrics.
            | "/metrics"
    )
}

/// Propagated-backpressure shed response for the inbound admission gate:
/// 503 + Retry-After, content-negotiated (browser navigations get the staged
/// recovery page; everything else the structured
/// `{status:"catching-up", retryAfter:N}` JSON). Never a bare drop/hang/502.
fn catching_up_response(wants_html: bool, retry_after_secs: u64) -> Response<Full<Bytes>> {
    routes::catching_up::shed_response(
        wants_html,
        retry_after_secs,
        routes::catching_up::ShedCause::Admission,
    )
}

// ─── Membrane policy stage ────────────────────────────────────────────────────

/// Apply the membrane rate-limit policy for one inbound request.
///
/// Returns `Some(response)` when the request must be short-circuited (Challenge
/// or Deny); `None` when the caller should continue dispatching (Allow or Shape —
/// Shape applies a brief sleep then returns None).
///
/// The Mutex lock is held ONLY for the synchronous `assess` call, then dropped
/// before any `.await`.  This satisfies two requirements:
///   1. `std::sync::MutexGuard` cannot cross an await boundary (won't compile).
///   2. Fail-open on poison: `into_inner()` recovers the guard rather than
///      panicking the entire request handler.
async fn apply_membrane(
    state: &AppState,
    req: &Request<Incoming>,
    addr: &SocketAddr,
    path: &str,
) -> Option<Response<BoxBody>> {
    use crate::server::membrane::{client_ip_from_xff, derive_source, is_static_asset, EdgeClock};
    use elohim_peer_fabric::guard::{assess, Clock, Verdict};

    // Static assets are high-frequency browser traffic — exempt from scoring.
    if is_static_asset(path) {
        return None;
    }

    let xff = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok());
    let client_ip = client_ip_from_xff(xff, addr, state.trusted_proxy_hops);
    let human_id = resolve_agent_cid_from_request(state, req);
    let source = derive_source(human_id.as_deref(), &client_ip);

    // ── Assess under lock; drop lock before any await ─────────────────────
    let verdict = {
        let mut guard = match state.membrane_guard.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(), // fail-open on poison
        };
        let clk = EdgeClock;
        let now = clk.now_secs();
        guard.maybe_sweep(now);
        assess(&mut *guard, &clk, &state.membrane_cfg, &source)
    };

    match verdict {
        Verdict::Allow => {
            crate::metrics::inc_membrane_verdict("allow");
            None
        }
        Verdict::Shape { delay_ms } => {
            crate::metrics::inc_membrane_verdict("shape");
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            None
        }
        Verdict::Challenge => {
            crate::metrics::inc_membrane_verdict("challenge");
            let retry_after = state.membrane_cfg.ban_secs;
            Some(to_boxed(
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Content-Type", "application/json")
                    .header("Retry-After", retry_after.to_string())
                    .header("x-membrane", "challenge")
                    .body(Full::new(Bytes::from(
                        serde_json::json!({
                            "error": "Too Many Requests",
                            "retryAfter": retry_after,
                        })
                        .to_string(),
                    )))
                    .unwrap(),
            ))
        }
        Verdict::Deny => {
            crate::metrics::inc_membrane_verdict("deny");
            Some(to_boxed(
                Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header("Content-Type", "application/json")
                    .header("x-membrane", "deny")
                    .body(Full::new(Bytes::from(
                        serde_json::json!({ "error": "Forbidden" }).to_string(),
                    )))
                    .unwrap(),
            ))
        }
    }
}

// ─── SSR helpers ──────────────────────────────────────────────────────────────

/// SSR success response with the observability header contract:
/// - x-render-cache: MISS / HIT (existing)
/// - x-ssr-rendered: 1 (always on success)
/// - x-ssr-renderer: e.g. "angular-ssr" (when capability is present)
/// - x-ssr-bundle-version: e.g. "lamad-app@1.0.3" (when capability is present)
///
/// `capability` is the doorway's published RenderCapabilityProfile (None
/// when the doorway hasn't derived one — typical for ssr-without-claim
/// deployments). When None, only x-render-cache and x-ssr-rendered are
/// emitted; bundle/renderer headers are omitted rather than guessed.
fn ssr_html_response_with_observability(
    html: String,
    cache_status: &str,
    capability: Option<&crate::render::types::RenderCapabilityProfile>,
    trace: Option<&elohim_render::RenderTrace>,
    chrome_context_json: Option<&str>,
) -> Response<Full<Bytes>> {
    // Native runtime chrome: splice the omnibar element loader + an inline JSON
    // context island into the finalized SSR HTML, immediately before </body>.
    // The element self-mounts client-side, identically to the Tauri/CSR paths.
    // `inject_element` escapes the JSON so it cannot break out of the island.
    let html = match chrome_context_json {
        Some(ctx) => elohim_render::inject_element(&html, ctx),
        None => html,
    };
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .header("cache-control", "no-store")
        .header("x-render-cache", cache_status)
        .header("x-ssr-rendered", "1");
    // Render-trace observability (fresh renders only — a cache HIT has no live
    // trace). `x-ssr-terminal` is the rendered-empty-vs-stalled cut; `x-ssr-wall-ms`
    // is the per-peer latency signal that feeds compute-commitment tuning.
    if let Some(trace) = trace {
        builder = builder
            .header("x-ssr-terminal", trace.terminal.as_str())
            .header("x-ssr-fetches", trace.fetches.len().to_string())
            .header("x-ssr-wall-ms", trace.wall_ms.to_string());
        if !trace.trace_id.is_empty() {
            if let Ok(v) = hyper::header::HeaderValue::from_str(&trace.trace_id) {
                builder = builder.header("x-ssr-trace-id", v);
            }
        }
    }
    if let Some(claim) = capability {
        if let Some(bundle) = claim.bundles.first() {
            builder = builder.header(
                "x-ssr-bundle-version",
                format!("{}@{}", bundle.name, bundle.version),
            );
            // The renderer's serialized wire-shape is the kebab-case enum
            // value, the same string that appears in storage's manifest.
            let renderer_str = match bundle.renderer {
                crate::render::types::RendererKind::AngularSsr => "angular-ssr",
                crate::render::types::RendererKind::ReactRsc => "react-rsc",
                crate::render::types::RendererKind::VueSsr => "vue-ssr",
                crate::render::types::RendererKind::SvelteSsr => "svelte-ssr",
                crate::render::types::RendererKind::LitSsr => "lit-ssr",
                crate::render::types::RendererKind::StaticHtml => "static-html",
            };
            builder = builder.header("x-ssr-renderer", renderer_str);
        }
    }
    builder.body(Full::new(Bytes::from(html))).unwrap()
}

/// SPA shell fallback that surfaces the render error in an `x-ssr-error`
/// header for ops/debug visibility. Empty `err` omits the header.
///
/// Header values must be ASCII-printable; we sanitize CR/LF/non-ASCII to `?`
/// so a panic message containing newlines doesn't trip hyper's validation.
fn ssr_spa_shell_fallback_with_error(
    err: &str,
    chrome_context_json: Option<&str>,
) -> Response<Full<Bytes>> {
    let mut resp = ssr_spa_shell_fallback_with_skip_reason(None, chrome_context_json);
    if !err.is_empty() {
        let sanitized: String = err
            .chars()
            .map(|c| {
                if (0x20..=0x7e).contains(&(c as u32)) {
                    c
                } else {
                    '?'
                }
            })
            .take(512)
            .collect();
        if let Ok(value) = hyper::header::HeaderValue::from_str(&sanitized) {
            resp.headers_mut().insert("x-ssr-error", value);
        }
    }
    resp
}

/// SPA shell fallback that records WHY SSR was skipped. Returned when:
/// - SSR-eligible route but the doorway's auth_modes don't include the
///   request's posture (`auth-mode-not-supported`)
/// - Concurrency limit reached (`overflow`)
/// - Renderer absent for a route that requires SSR (`bundle-not-loaded`)
///
/// `skip_reason: None` is the generic fallback (no `x-ssr-skipped` header,
/// just the SPA shell + `x-ssr-rendered: 0`).
fn ssr_spa_shell_fallback_with_skip_reason(
    skip_reason: Option<&str>,
    chrome_context_json: Option<&str>,
) -> Response<Full<Bytes>> {
    const SHELL: &str = concat!(
        "<!doctype html><html><body>",
        "<app-root></app-root>",
        "<script>console.log('SSR fallback \u{2014} CSR will hydrate')</script>",
        "</body></html>",
    );
    // Native runtime chrome: the CSR fallback is still an HTML serve, so it must
    // carry the omnibar trust surface too. The element self-mounts client-side
    // before Angular hydrates, identically to the SSR-success path. Already
    // `no-store`, so no cache-control divergence for the injected island.
    let body: Bytes = match chrome_context_json {
        Some(ctx) => Bytes::from(elohim_render::inject_element(SHELL, ctx)),
        None => Bytes::from_static(SHELL.as_bytes()),
    };
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .header("x-ssr-rendered", "0");
    if let Some(reason) = skip_reason {
        builder = builder.header("x-ssr-skipped", reason);
    }
    builder.body(Full::new(body)).unwrap()
}

// ─── Gate layer tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod ssr_session_tests {
    use super::*;
    use hyper::header::{HeaderValue, AUTHORIZATION, COOKIE};

    fn req_with_headers(headers: &[(&str, &str)]) -> Request<Bytes> {
        let mut builder = Request::builder().uri("/lamad/concept/abc");
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        builder.body(Bytes::new()).unwrap()
    }

    #[test]
    fn anonymous_request_returns_none() {
        let req = req_with_headers(&[]);
        assert!(build_ssr_user_credential(&req).is_none());
    }

    #[test]
    fn authorization_bearer_is_forwarded() {
        let req = req_with_headers(&[("authorization", "Bearer token-xyz")]);
        let cred = build_ssr_user_credential(&req).expect("some");
        assert_eq!(cred.header_name, "Authorization");
        assert_eq!(cred.header_value, "Bearer token-xyz");
    }

    #[test]
    fn doorway_session_cookie_is_forwarded() {
        let req = req_with_headers(&[("cookie", "doorway_session=abc; theme=dark")]);
        let cred = build_ssr_user_credential(&req).expect("some");
        assert_eq!(cred.header_name, "Cookie");
        assert!(cred.header_value.contains("doorway_session=abc"));
    }

    #[test]
    fn steward_attestation_cookie_is_forwarded() {
        let req = req_with_headers(&[("cookie", "steward_attestation=xyz; locale=en")]);
        let cred = build_ssr_user_credential(&req).expect("some");
        assert_eq!(cred.header_name, "Cookie");
        assert!(cred.header_value.contains("steward_attestation=xyz"));
    }

    #[test]
    fn unknown_cookies_are_not_forwarded() {
        let req = req_with_headers(&[("cookie", "theme=dark; locale=en")]);
        assert!(build_ssr_user_credential(&req).is_none());
    }

    #[test]
    fn authorization_takes_precedence_over_cookie() {
        let req = req_with_headers(&[
            ("authorization", "Bearer t"),
            ("cookie", "doorway_session=abc"),
        ]);
        let cred = build_ssr_user_credential(&req).expect("some");
        assert_eq!(cred.header_name, "Authorization");
    }

    #[test]
    fn non_utf8_authorization_falls_through() {
        // Bytes that don't form valid UTF-8 — to_str() returns Err
        let mut req = req_with_headers(&[]);
        req.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(&[0xFF, 0xFE, 0xFD]).unwrap(),
        );
        assert!(build_ssr_user_credential(&req).is_none());
    }

    #[test]
    fn non_utf8_cookie_falls_through() {
        let mut req = req_with_headers(&[]);
        req.headers_mut()
            .insert(COOKIE, HeaderValue::from_bytes(&[0xFF, 0xFE]).unwrap());
        assert!(build_ssr_user_credential(&req).is_none());
    }

    // ── native chrome context island (the doorway inject path) ──────────────

    fn req_for(path: &str, headers: &[(&str, &str)]) -> Request<Bytes> {
        let mut builder = Request::builder().uri(path);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        builder.body(Bytes::new()).unwrap()
    }

    #[test]
    fn chrome_context_slug_is_last_path_segment() {
        let req = req_for("/lamad/concept/abc", &[]);
        let ctx = build_chrome_context_json("/lamad/concept/abc", &req);
        let v: serde_json::Value = serde_json::from_str(&ctx).unwrap();
        assert_eq!(v["slug"], "abc");
        // Anonymous request → not authenticated.
        assert_eq!(v["authenticated"], false);
    }

    #[test]
    fn chrome_context_root_resolves_to_landing_slug() {
        let req = req_for("/", &[]);
        let ctx = build_chrome_context_json("/", &req);
        let v: serde_json::Value = serde_json::from_str(&ctx).unwrap();
        assert_eq!(v["slug"], "elohim-host-landing");
    }

    #[test]
    fn chrome_context_authenticated_when_credential_present() {
        let req = req_for("/lamad/concept/x", &[("authorization", "Bearer t")]);
        let ctx = build_chrome_context_json("/lamad/concept/x", &req);
        let v: serde_json::Value = serde_json::from_str(&ctx).unwrap();
        assert_eq!(v["authenticated"], true);
    }

    #[test]
    fn chrome_context_trailing_slash_ignored() {
        let req = req_for("/resource/cid123/", &[]);
        let ctx = build_chrome_context_json("/resource/cid123/", &req);
        let v: serde_json::Value = serde_json::from_str(&ctx).unwrap();
        assert_eq!(v["slug"], "cid123");
    }

    #[test]
    fn ssr_response_injects_the_omnibar_element() {
        // The finalized SSR response splices the element loader + context island
        // before </body>. We exercise the real response builder with a context.
        let ctx = r#"{"slug":"abc","authenticated":false}"#;
        let resp = ssr_html_response_with_observability(
            "<html><body><app-root></app-root></body></html>".to_string(),
            "HIT",
            None,
            None,
            Some(ctx),
        );
        let body = futures::executor::block_on(async {
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            html.contains("id=\"elohim-omni-context\""),
            "island present: {html}"
        );
        assert!(html.contains("omni-element."), "loader present: {html}");
        // Injected before </body>, document intact.
        let island_at = html.find("elohim-omni-context").unwrap();
        let body_close = html.rfind("</body>").unwrap();
        assert!(island_at < body_close);
        assert!(html.contains("<app-root></app-root>"));
    }

    #[test]
    fn ssr_response_without_context_is_unchanged() {
        // No chrome context → no inject (the non-SSR-chrome path stays clean).
        let resp = ssr_html_response_with_observability(
            "<html><body>plain</body></html>".to_string(),
            "MISS",
            None,
            None,
            None,
        );
        let body = futures::executor::block_on(async {
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            !html.contains("elohim-omni-context"),
            "no island injected: {html}"
        );
        assert_eq!(html, "<html><body>plain</body></html>");
    }

    // ── maybe_inject_chrome (the EPR-router HTML serve path) ─────────────────

    #[test]
    fn epr_html_page_gets_exactly_one_island_and_loader() {
        // An HTML 200 proxied through the EPR router (`dispatch_to_projected_epr`)
        // carries the omnibar chrome — exactly one island + one loader, before
        // </body>, in order.
        let ctx = r#"{"slug":"elohim-host-landing","authenticated":false}"#;
        let (out, injected) = maybe_inject_chrome(
            200,
            "text/html; charset=utf-8",
            Bytes::from_static(b"<!doctype html><html><body><app-root></app-root></body></html>"),
            ctx,
        );
        assert!(injected, "an HTML 200 must inject chrome");
        let html = String::from_utf8(out.to_vec()).unwrap();
        assert_eq!(
            html.matches("id=\"elohim-omni-context\"").count(),
            1,
            "exactly one context island: {html}"
        );
        assert_eq!(
            html.matches("omni-element.").count(),
            1,
            "exactly one element loader: {html}"
        );
        let island_at = html.find("elohim-omni-context").unwrap();
        let loader_at = html.find("omni-element.").unwrap();
        let body_close = html.rfind("</body>").unwrap();
        assert!(island_at < body_close, "island precedes </body>");
        assert!(loader_at < body_close, "loader precedes </body>");
        assert!(island_at < loader_at, "island precedes loader");
        assert!(html.contains("<app-root></app-root>"), "document survives");
    }

    #[test]
    fn epr_asset_body_is_byte_identical() {
        // A JS/CSS asset (non-HTML content-type) flows through the same proxy —
        // the bytes must be untouched (a mangled asset breaks the SPA).
        let js = b"export const x = 1; // </body> lookalike, must not trip inject";
        let (out, injected) = maybe_inject_chrome(
            200,
            "application/javascript",
            Bytes::from_static(js),
            r#"{"slug":"x"}"#,
        );
        assert!(!injected, "non-HTML must not inject");
        assert_eq!(out.as_ref(), js.as_ref(), "asset bytes byte-identical");
    }

    #[test]
    fn epr_non_2xx_html_is_untouched() {
        // A non-2xx HTML body (e.g. storage 404/503 error page) is not a servable
        // page — pass it through unmodified.
        let err_html = Bytes::from_static(b"<html><body>not found</body></html>");
        let (out, injected) =
            maybe_inject_chrome(404, "text/html; charset=utf-8", err_html.clone(), "{}");
        assert!(!injected, "non-2xx must not inject");
        assert_eq!(out, err_html, "error-page bytes untouched");
    }

    #[test]
    fn epr_invalid_utf8_html_is_untouched() {
        // Invalid-UTF-8 `text/html` must be served unmodified — never fail the
        // response over an un-injectable page.
        let bad = Bytes::from_static(&[0xff, 0xfe, b'<', b'b', b'o', b'd', b'y', b'>']);
        let (out, injected) = maybe_inject_chrome(200, "text/html", bad.clone(), "{}");
        assert!(!injected, "invalid UTF-8 must not inject");
        assert_eq!(out, bad, "invalid-UTF-8 bytes untouched");
    }

    // ── CSR fallback shells now carry the chrome island ──────────────────────

    #[test]
    fn fallback_shell_injects_chrome_when_context_present() {
        // The SSR error/shed/auth-mode CSR fallback is still an HTML serve — with
        // a context it carries exactly one island + loader before </body>.
        let ctx = r#"{"slug":"abc","authenticated":false}"#;
        let resp = ssr_spa_shell_fallback_with_skip_reason(Some("overflow"), Some(ctx));
        let body = futures::executor::block_on(async {
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(
            html.matches("id=\"elohim-omni-context\"").count(),
            1,
            "{html}"
        );
        assert_eq!(html.matches("omni-element.").count(), 1, "{html}");
        let island_at = html.find("elohim-omni-context").unwrap();
        let body_close = html.rfind("</body>").unwrap();
        assert!(island_at < body_close, "island precedes </body>: {html}");
    }

    #[test]
    fn fallback_shell_is_clean_without_context() {
        // No context → the shell is unchanged (defensive: the None path stays the
        // original bytes).
        let resp = ssr_spa_shell_fallback_with_skip_reason(None, None);
        let body = futures::executor::block_on(async {
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(!html.contains("elohim-omni-context"), "no island: {html}");
        assert!(
            html.contains("<app-root></app-root>"),
            "shell intact: {html}"
        );
    }

    #[test]
    fn fallback_error_shell_injects_and_carries_error_header() {
        // The error fallback both surfaces `x-ssr-error` AND carries the island.
        let ctx = r#"{"slug":"z","authenticated":true}"#;
        let resp = ssr_spa_shell_fallback_with_error("boom", Some(ctx));
        assert_eq!(
            resp.headers().get("x-ssr-error").unwrap(),
            "boom",
            "error header preserved"
        );
        let body = futures::executor::block_on(async {
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            html.contains("id=\"elohim-omni-context\""),
            "island present: {html}"
        );
    }

    // ── EPR→SSR fallback (the projected-bundle path) ─────────────────────────

    #[test]
    fn ssr_fallback_reason_skip_strings() {
        // The x-ssr-skipped vocabulary the EPR fallback tags responses with.
        assert_eq!(
            SsrFallbackReason::AuthModeUnsupported.as_skip_str(),
            "auth-mode-not-supported"
        );
        assert_eq!(SsrFallbackReason::Overflow.as_skip_str(), "overflow");
        assert_eq!(
            SsrFallbackReason::SpecParseError.as_skip_str(),
            "spec-parse-error"
        );
        assert_eq!(
            SsrFallbackReason::RendererAbsent.as_skip_str(),
            "renderer-absent"
        );
        assert_eq!(
            SsrFallbackReason::RenderError("boom".to_string()).as_skip_str(),
            "render-error"
        );
        assert_eq!(
            SsrFallbackReason::ErrorBackoff.as_skip_str(),
            "error-backoff"
        );
        // The granular shell-compose vocabulary: the predecessor single
        // "shell-unavailable" hid a cross-app selector bug (every /lamad
        // request burned a wrong-app V8 render) and an upstream fetch stall
        // behind one opaque string. Each seam now names itself.
        assert_eq!(
            SsrFallbackReason::RendererAppMismatch.as_skip_str(),
            "renderer-app-mismatch"
        );
        assert_eq!(
            SsrFallbackReason::ShellNoProjection.as_skip_str(),
            "shell-no-projection"
        );
        assert_eq!(
            SsrFallbackReason::ShellBreakerOpen.as_skip_str(),
            "shell-breaker-open"
        );
        assert_eq!(
            SsrFallbackReason::ShellFetchFailed.as_skip_str(),
            "shell-fetch-failed"
        );
        assert_eq!(
            SsrFallbackReason::Compose(elohim_render::ComposeError::ShellRootMissing {
                tag: "lamad-root".into()
            })
            .as_skip_str(),
            "shell-root-missing"
        );
        assert_eq!(
            SsrFallbackReason::Compose(elohim_render::ComposeError::RenderedRootNotFound)
                .as_skip_str(),
            "render-root-missing"
        );
    }

    // The renderer-app gate decisions (exact-match renders, unserved app is a
    // mismatch shed, unknown-app image-baked bundle keeps legacy allow) are
    // pinned in crate::render::registry tests — the gate and the renderer
    // lookup share RendererRegistry::select, so those tests cover this
    // dispatch's decision surface too.

    #[test]
    fn epr_ssr_fallback_carries_skip_header_and_single_island() {
        // The EPR→SSR fallback serves the projected bundle: the chrome island is
        // spliced INSIDE dispatch_to_projected_epr (via maybe_inject_chrome), and
        // the response is tagged x-ssr-skipped:<reason>. Exercise the pure
        // composition without a live storage/renderer: inject once (as the proxy
        // does), then tag — the island stays exactly-one and the header lands.
        let ctx = r#"{"slug":"elohim-host-landing","authenticated":false}"#;
        let (body, injected) = maybe_inject_chrome(
            200,
            "text/html; charset=utf-8",
            Bytes::from_static(b"<!doctype html><html><body><app-root></app-root></body></html>"),
            ctx,
        );
        assert!(injected, "the proxied HTML page must inject chrome");
        let resp = Response::builder()
            .status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body(Full::new(body))
            .unwrap();
        let tagged = with_ssr_skipped_header(resp, &SsrFallbackReason::Overflow);
        assert_eq!(
            tagged.headers().get("x-ssr-skipped").unwrap(),
            "overflow",
            "fallback carries the skip reason"
        );
        let out = futures::executor::block_on(async {
            tagged.into_body().collect().await.unwrap().to_bytes()
        });
        let html = String::from_utf8(out.to_vec()).unwrap();
        assert_eq!(
            html.matches("id=\"elohim-omni-context\"").count(),
            1,
            "exactly one context island survives the fallback: {html}"
        );
        assert_eq!(
            html.matches("omni-element.").count(),
            1,
            "exactly one element loader: {html}"
        );
    }
}

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

    use super::{classify_dispatch, epr_should_serve_ssr, render_output_is_empty, Disposition};
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
            render_spec: None,
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
        // to a SPA bootstrap path.
        let registry = registry_with_storage_route(HttpMethod::Get, "/blob/:hash").await;
        let dispo = classify_dispatch(&registry, &Method::GET, "/blob/sha256-abcdef123456").await;
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
        let dispo = classify_dispatch(&registry, &Method::GET, "/future/some-id").await;
        assert!(
            matches!(dispo, Disposition::StorageProxy { .. }),
            "Any registry-compiled path must route through the registry"
        );
    }

    #[tokio::test]
    async fn unregistered_get_returns_not_found() {
        // Root-path / SPA dispatch is handled by the EPR router (B11–B13) before
        // classify_dispatch is reached. Unregistered paths that slip through here
        // must return NotFound — there is no ROOT_APP_SLUG slug fallback.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(&registry, &Method::GET, "/learn/some-path-id").await;
        assert_eq!(
            dispo,
            Disposition::NotFound,
            "Unregistered GET with no registry match must return NotFound (EPR router handles root)"
        );
    }

    #[tokio::test]
    async fn unregistered_post_returns_not_found() {
        // API misses must 404 (not serve HTML). Without this, an unknown
        // POST /api/v1/foo would render the SPA bootstrap to a JSON client.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(&registry, &Method::POST, "/api/v1/no-such-route").await;
        assert_eq!(
            dispo,
            Disposition::NotFound,
            "Unregistered non-GET must 404, never SPA"
        );
    }

    /// Helper: inject a `StorageProxy` route with a render_spec into a fresh registry.
    async fn registry_with_ssr_route(method: HttpMethod, path: &str, spec: &str) -> RouteRegistry {
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
            render_spec: Some(spec.to_string()),
        };
        let mut compiled = registry.compiled_routes.write().await;
        compiled.push(route);
        drop(compiled);
        registry
    }

    #[tokio::test]
    async fn classify_dispatch_returns_ssr_route_when_render_spec_set() {
        // Regression contract: routes with render_spec must classify as SsrRoute,
        // not StorageProxy, so the caller can dispatch through the in-process renderer.
        let registry =
            registry_with_ssr_route(HttpMethod::Get, "/lamad/concept/:id", "angular-ssr").await;
        let dispo = classify_dispatch(&registry, &Method::GET, "/lamad/concept/abc").await;
        match dispo {
            Disposition::SsrRoute { spec, .. } => {
                assert_eq!(
                    spec, "angular-ssr",
                    "render_spec must propagate to SsrRoute"
                )
            }
            other => panic!("expected SsrRoute, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gated_writes_classify_as_storage_proxy_not_ssr() {
        // Op-gate placement contract: the delegates-compute op-gate lives ONLY in
        // the StorageProxy disposition arm. The SsrRoute arm has its own
        // forward_to_storage fallbacks with NO gate and NO method guard, so a
        // gated write must NEVER classify as SsrRoute. Today `POST /db/content*`
        // carries no server-controlled render_spec, so it classifies as
        // StorageProxy — this test fails the day a manifest change adds a render
        // field to a content-write route (the latent un-gating regression).
        for path in ["/db/content", "/db/content/bulk"] {
            let registry = registry_with_storage_route(HttpMethod::Post, path).await;
            let dispo = classify_dispatch(&registry, &Method::POST, path).await;
            assert!(
                matches!(dispo, Disposition::StorageProxy { .. }),
                "POST {path} must classify as StorageProxy (gated arm), never SsrRoute; got {dispo:?}"
            );
        }
    }

    #[tokio::test]
    async fn landing_routes_that_epr_router_also_serves_still_classify_ssr() {
        // EPR-router SSR divert (rollout step 5): the four landing routes the EPR
        // router mounts (render:"angular-ssr") must still classify as SsrRoute so
        // the EPR-dispatch block can route them through the V8 SSR engine.
        // classify_dispatch consults ONLY the registry — the EPR match is
        // orthogonal — so a route BOTH mounted and rendered still yields SsrRoute
        // + the render spec (the pin for the divert's classify contract).
        let cases = [
            ("/lamad/concept/:id", "/lamad/concept/abc"),
            ("/lamad/path/:slug", "/lamad/path/intro"),
            ("/lamad/path/:slug/step/:n", "/lamad/path/intro/step/2"),
        ];
        for (registered, concrete) in cases {
            let registry =
                registry_with_ssr_route(HttpMethod::Get, registered, "angular-ssr").await;
            let dispo = classify_dispatch(&registry, &Method::GET, concrete).await;
            assert!(
                matches!(&dispo, Disposition::SsrRoute { spec, .. } if spec == "angular-ssr"),
                "{registered} must classify as SsrRoute for the EPR SSR divert; got {dispo:?}"
            );
        }
    }

    #[test]
    fn epr_diverts_to_ssr_only_when_ssr_route_and_renderer_present() {
        // The EPR-dispatch decision (pure, no HTTP): divert to the V8 SSR engine
        // iff the disposition is SsrRoute AND a renderer is loaded. Every other
        // combination serves the projected bundle (today's exact behavior).
        let ssr = Disposition::SsrRoute {
            spec: "angular-ssr".to_string(),
            endpoint: "http://storage:8090".to_string(),
        };
        let proxy = Disposition::StorageProxy {
            endpoint: "http://storage:8090".to_string(),
        };
        // SsrRoute + renderer → divert.
        assert!(epr_should_serve_ssr(&ssr, true));
        // SsrRoute but NO renderer → projected bundle (cheapest path, today's behavior).
        assert!(!epr_should_serve_ssr(&ssr, false));
        // Non-SSR dispositions never divert, renderer present or not.
        assert!(!epr_should_serve_ssr(&proxy, true));
        assert!(!epr_should_serve_ssr(&Disposition::RegistryUnhandled, true));
        assert!(!epr_should_serve_ssr(&Disposition::NotFound, true));
    }

    #[test]
    fn render_output_is_empty_flags_production_empty_shell() {
        // The exact structure of the 2026-07-04 blank landing page: a render that
        // "succeeded" but activated no route component — the <router-outlet> is
        // followed only by comment nodes before </app-root>.
        let empty = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head>\
            <body><!--nghm--><app-root ng-version=\"19.2.19\" ngh=\"0\">\
            <a href=\"#main-content\" class=\"skip-link\">Skip to main content</a>\
            <!----><router-outlet _ngcontent-ng-c2521897509=\"\"></router-outlet><!----></app-root>\
            <script id=\"ng-state\" type=\"application/json\">{}</script></body></html>";
        assert!(
            render_output_is_empty(empty),
            "an outlet with no element sibling (only comment nodes) must read as empty"
        );
    }

    #[test]
    fn render_output_is_empty_false_for_activated_route() {
        // A healthy Angular SSR render places the activated route component as an
        // element sibling immediately after the outlet.
        let healthy =
            "<app-root><router-outlet></router-outlet><app-home>content</app-home></app-root>";
        assert!(
            !render_output_is_empty(healthy),
            "an outlet followed by an element sibling is an activated (healthy) render"
        );
    }

    #[test]
    fn render_output_is_empty_false_when_no_router_outlet() {
        // A document with no <router-outlet cannot be judged — never gate it (a
        // raw error page or non-Angular response must serve as-is).
        let no_outlet = "<!DOCTYPE html><html><body><h1>Some other page</h1></body></html>";
        assert!(
            !render_output_is_empty(no_outlet),
            "a document with no router-outlet must not be gated"
        );
    }

    #[test]
    fn render_output_is_empty_true_for_only_comments_and_whitespace() {
        // Comment nodes (including the empty <!---->) and whitespace after the
        // outlet are stripped: they are not an activated component.
        let only_comments =
            "<app-root><router-outlet></router-outlet>\n  <!----> <!-- ng --> \n</app-root>";
        assert!(
            render_output_is_empty(only_comments),
            "an outlet followed only by comments/whitespace must read as empty"
        );
    }

    #[test]
    fn render_output_is_empty_false_for_outlet_with_attributes() {
        // The real outlet carries _ngcontent attributes; the sibling element
        // carries them too. Attribute forms must not defeat sibling detection.
        let activated = "<router-outlet _ngcontent-ng-c123=\"\"></router-outlet>\
            <elohim-landing _ngcontent-ng-c123=\"\">x</elohim-landing>";
        assert!(
            !render_output_is_empty(activated),
            "attribute-bearing outlet + element sibling is an activated render"
        );
    }
}

#[cfg(test)]
mod epr_universal_tests {
    use super::*;

    // §12.1: /epr is reserved — a service path, never a projection mount.
    #[test]
    fn epr_prefix_is_a_service_path() {
        assert!(is_service_path("/epr"));
        assert!(is_service_path("/epr/manifesto-foundations"));
        assert!(is_service_path("/epr/foundations-christian-technology"));
    }

    #[test]
    fn epr_head_remains_a_service_path() {
        // /epr-head/ does NOT start with "/epr/" — the two prefixes are disjoint.
        assert!(is_service_path("/epr-head/manifesto"));
    }

    #[test]
    fn epr_reservation_rejects_projection_mounts() {
        assert!(is_reserved_url_path("/epr"));
        assert!(is_reserved_url_path("/epr/anything"));
        assert!(is_reserved_url_path("/db"));
        assert!(is_reserved_url_path("/api/v1"));
        // The root mount and the portal mount stay legal.
        assert!(!is_reserved_url_path("/"));
        assert!(!is_reserved_url_path("/auth/portal"));
        assert!(!is_reserved_url_path("/lamad"));
    }
}

#[cfg(test)]
mod epr_claims_dispatch_tests {
    use super::*;

    // EPR Slice 1 — the universal `/epr/{id}` claims-302 is DEMOTED (operator
    // decision 2026-06-08). The universal address now serves the shell
    // UNCONDITIONALLY (bare `/epr/{id}` and `/epr/{id}/raw` alike) — a claimed,
    // commons-reach type that USED to 302 to its pretty pillar mount
    // (`/lamad/path/{id}`) now reaches the lens-complete shell viewer instead of
    // bouncing to the knowledge-only pillar. The doorway no longer consults head
    // facts, content type, or the claims table on the universal-address path;
    // the only branch left is "root projection present → serve shell; absent →
    // /threshold". These tests INVERT the Slice-0 tests (which asserted
    // RedirectToMount for claimed commons/public types).

    /// A commons-reach `/lamad` mount that CLAIMS the `path` content type — i.e.
    /// exactly the projection that minted the `/lamad/path/{id}` 302 the old
    /// universal-address behavior produced. Used to prove the demotion: even with
    /// this claim live, the universal address serves the shell, never the mount.
    fn router_with_path_claiming_root() -> crate::projection::EprRouter {
        use elohim_views::projection::{
            EprProjectionView, ProjectionMode, RouteClaimGrant, RouteClaimTemplate,
        };
        let router = crate::projection::EprRouter::new();
        let mut lamad = EprProjectionView {
            commitment_id: "test-lamad".into(),
            epr_id: "lamad".into(),
            doorway_id: "doorway:test".into(),
            // Root mount: the bundle the shell is served from for /epr/{id}.
            url_path: "/".into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-06-06T00:00:00Z".into(),
            seeded_by: "test".into(),
        };
        // Claim the `path` content type — the would-302 condition of Slice 0.
        lamad.route_claims = Some(RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: Default::default(),
            }],
        });
        router.replace_all(vec![lamad]);
        router
    }

    #[test]
    fn universal_address_serves_shell_for_claimed_commons_type() {
        // INVERTS classify_redirects_claimed_commons: a claimed `path` (commons)
        // no longer 302s to /lamad/path/{id}; the universal address resolves to
        // the ROOT projection (the shell). The claim is still in the table (the
        // sitemap uses it) — it just no longer drives a universal-address 302.
        let router = router_with_path_claiming_root();
        // Sanity: the claim IS live (so this is genuinely the would-302 case).
        // The claim mounts under the claiming projection's url_path (the root
        // mount `/` here, so the minted location is `/path/abc`). The point is
        // only that a claimed-mount location EXISTS — the demoted universal
        // address no longer turns it into a 302; the sitemap still uses it.
        assert_eq!(
            router.claimed_mount_location("path", "abc"),
            Some("/path/abc".to_string()),
            "claim must remain in the table (sitemap depends on it)"
        );
        let root = epr_universal_root(&router);
        assert!(
            root.is_some(),
            "universal /epr/{{id}} for a claimed commons type now serves the shell (root projection), not a mount 302"
        );
        assert_eq!(root.unwrap().url_path, "/");
    }

    #[test]
    fn universal_address_serves_shell_regardless_of_subview() {
        // The doorway no longer distinguishes the subview: bare /epr/{id} and
        // /epr/{id}/raw both serve the shell (the shell's Angular routing does the
        // distinguishing). One outcome for the universal address: serve the root.
        let router = router_with_path_claiming_root();
        // The resolver is purely root-driven — the same root regardless of which
        // /epr/* path the request carried (bare, /raw, or any subview).
        assert!(epr_universal_root(&router).is_some());
    }

    #[test]
    fn universal_address_falls_back_to_threshold_without_root() {
        // Unchanged behavior: no ROOT projection registered → None, which the
        // dispatcher turns into a 302 to /threshold (the operator dashboard).
        let router = crate::projection::EprRouter::new();
        assert!(
            epr_universal_root(&router).is_none(),
            "no root projection → fall back to /threshold"
        );
    }

    #[test]
    fn sitemap_xml_renders_mounts_and_claimed_entries() {
        use elohim_views::projection::{
            EprProjectionView, ProjectionMode, RouteClaimGrant, RouteClaimTemplate,
        };
        let router = crate::projection::epr_router::EprRouter::new();
        let mut lamad = EprProjectionView {
            commitment_id: "test-lamad".into(),
            epr_id: "lamad".into(),
            doorway_id: "doorway:test".into(),
            url_path: "/lamad".into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/lamad/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-06-06T00:00:00Z".into(),
            seeded_by: "test".into(),
        };
        // give it the path claim as in Task 8's claim_binding test
        lamad.route_claims = Some(RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: Default::default(),
            }],
        });
        router.replace_all(vec![lamad]);
        let xml = render_sitemap(
            &router,
            "https://alpha.elohim.host",
            &[(
                "path".to_string(),
                vec!["foundations-christian-technology".to_string()],
            )],
        );
        assert!(xml.contains("<loc>https://alpha.elohim.host/lamad</loc>"));
        assert!(xml.contains(
            "<loc>https://alpha.elohim.host/lamad/path/foundations-christian-technology</loc>"
        ));
        assert!(xml.starts_with("<?xml"));
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn liveness_paths_are_exempt() {
        for p in [
            "/health",
            "/healthz",
            "/health/startup",
            "/ready",
            "/readyz",
            "/version",
        ] {
            assert!(admission_exempt(p, false), "{p} must be admission-exempt");
        }
    }

    #[test]
    fn upgrades_are_exempt() {
        assert!(
            admission_exempt("/some-signal-pubkey", true),
            "WS upgrades hold the socket; never gate"
        );
        assert!(admission_exempt("/debug/stream", true));
    }

    #[test]
    fn metrics_is_admission_exempt() {
        // A scrape endpoint that gets 503'd under admission pressure is dead
        // exactly when you need it.
        assert!(
            admission_exempt("/metrics", false),
            "/metrics must be exempt"
        );
    }

    #[test]
    fn flood_surface_is_gated() {
        // The whole point: proxy/api/blob/apps traffic is NOT exempt.
        for p in [
            "/api/v1/cache/x",
            "/db/content/y",
            "/blob/sha256-z",
            "/apps/foo",
            "/lamad",
        ] {
            assert!(
                !admission_exempt(p, false),
                "{p} must be gated (not exempt)"
            );
        }
    }

    #[test]
    fn catching_up_is_503_with_retry_after_and_body() {
        let resp = catching_up_response(false, 2);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get("Retry-After").unwrap(), "2");
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn catching_up_html_variant_for_browser_navigations() {
        let resp = catching_up_response(true, 2);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get("Retry-After").unwrap(), "2");
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));
    }

    #[test]
    fn zero_permit_semaphore_sheds_nonexempt() {
        // Models the gate: a zero-permit semaphore fails try_acquire_owned ->
        // the gate returns catching_up; an exempt path is never consulted.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        assert!(
            std::sync::Arc::clone(&sem).try_acquire_owned().is_err(),
            "0 permits => shed"
        );
    }

    #[test]
    fn appstate_has_inbound_admission_fields() {
        // Assert the semaphore is usable + a fresh breaker is closed.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT));
        assert!(
            sem.available_permits() >= MIN_MAX_INFLIGHT,
            "ceiling floored"
        );
        let breakers = std::sync::Arc::new(crate::routes::UpstreamBreakers::default());
        assert!(!breakers.is_open("http://x:8090"), "fresh breaker closed");
    }

    #[test]
    fn health_never_shed_even_at_zero_permits() {
        // Compose the gate decision exactly as handle_request does: exempt paths
        // bypass the semaphore entirely, so a 0-permit ceiling cannot shed them.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        for p in [
            "/health",
            "/healthz",
            "/health/startup",
            "/ready",
            "/readyz",
            "/version",
        ] {
            let exempt = admission_exempt(p, false);
            assert!(exempt, "{p} must be exempt");
            let would_shed = !exempt && std::sync::Arc::clone(&sem).try_acquire_owned().is_err();
            assert!(!would_shed, "{p} must NOT shed at 0 permits");
        }
        // Control: a gated path DOES shed at 0 permits.
        let gated = "/api/v1/cache/x";
        let would_shed = !admission_exempt(gated, false)
            && std::sync::Arc::clone(&sem).try_acquire_owned().is_err();
        assert!(would_shed, "gated path must shed at 0 permits");
    }
}

#[cfg(test)]
mod op_gate_tests {
    //! Unit tests for the delegates-compute op-gate decision matrix.
    //!
    //! `compute_op_gate_decision` is a pure function over (mode, method, path, verdict);
    //! these tests pin the full mode × path × verdict matrix without any I/O.
    //!
    //! Self-review invariant: the gated path set MUST match the routes that call
    //! `distribute_shards` in elohim-storage (`/db/content` and `/db/content/bulk`).

    use super::{
        compute_op_gate_decision, no_credential_decision, NoCredentialDecision, OpGateDecision,
    };
    use crate::config::OpGateMode;
    use hyper::Method;

    // ── enforce mode ──────────────────────────────────────────────────────────

    #[test]
    fn enforce_allowed_yields_allow() {
        let verdict = Ok(true);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::POST, "/db/content", &verdict),
            OpGateDecision::Allow,
            "enforce + allowed:true must forward"
        );
    }

    #[test]
    fn enforce_denied_yields_deny403() {
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::POST, "/db/content", &verdict),
            OpGateDecision::Deny403,
            "enforce + allowed:false must 403"
        );
    }

    #[test]
    fn enforce_error_yields_deny403_fail_closed() {
        // Infra/transport error in enforce mode → fail-closed (deny).
        let verdict: Result<bool, String> = Err("storage unreachable".into());
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::POST, "/db/content", &verdict),
            OpGateDecision::Deny403,
            "enforce + infra error must deny (fail-closed)"
        );
    }

    // ── observe mode ──────────────────────────────────────────────────────────

    #[test]
    fn observe_denied_yields_allow_with_warn() {
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Observe, &Method::POST, "/db/content", &verdict),
            OpGateDecision::AllowWithWarn,
            "observe + denied must log and forward"
        );
    }

    #[test]
    fn observe_error_yields_allow_with_warn() {
        let verdict: Result<bool, String> = Err("storage unreachable".into());
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Observe, &Method::POST, "/db/content", &verdict),
            OpGateDecision::AllowWithWarn,
            "observe mode always forwards (even on infra error)"
        );
    }

    // ── off mode ──────────────────────────────────────────────────────────────

    #[test]
    fn off_yields_allow_for_any_verdict() {
        // off must short-circuit before any storage call; these tests assert the
        // decision is Allow regardless of what the verdict would be. The outer
        // dispatch code is structured so the verdict is never computed when Off.
        for verdict in [Ok(true), Ok(false), Err("irrelevant".to_string())] {
            assert_eq!(
                compute_op_gate_decision(&OpGateMode::Off, &Method::POST, "/db/content", &verdict),
                OpGateDecision::Allow,
                "off mode must always allow (no storage call)"
            );
        }
    }

    // ── path gating ───────────────────────────────────────────────────────────

    #[test]
    fn content_path_is_gated_in_enforce_denied() {
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::POST, "/db/content", &verdict),
            OpGateDecision::Deny403,
            "/db/content must be gated"
        );
    }

    #[test]
    fn bulk_path_is_gated_identically_to_content() {
        // C5: /db/content/bulk spawns distribute_shards — gate must match /db/content.
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(
                &OpGateMode::Enforce,
                &Method::POST,
                "/db/content/bulk",
                &verdict
            ),
            OpGateDecision::Deny403,
            "/db/content/bulk must be gated identically to /db/content"
        );
    }

    #[test]
    fn non_matching_path_not_gated() {
        // Paths outside the gated set must never be gated, regardless of mode.
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::POST, "/db/other", &verdict),
            OpGateDecision::Allow,
            "/db/other must never be gated"
        );
    }

    #[test]
    fn get_method_not_gated_even_on_content_path() {
        // GET requests are read-only — never gated, even on the content paths.
        let verdict = Ok(false);
        assert_eq!(
            compute_op_gate_decision(&OpGateMode::Enforce, &Method::GET, "/db/content", &verdict),
            OpGateDecision::Allow,
            "GET /db/content must never be gated"
        );
    }

    // ── config: CHE_FACING boot-refusals ─────────────────────────────────────

    #[test]
    fn che_facing_requires_enforce_mode() {
        use crate::config::Args;
        use clap::Parser;
        // Simulate CHE_FACING=1 with the wrong mode — validate() must reject.
        let args = Args::try_parse_from([
            "doorway",
            "--che-facing",
            "--delegates-compute-op-gate",
            "off",
            "--jwt-secret",
            "a-secret-that-is-at-least-32-chars-long",
        ])
        .expect("parse must succeed");
        let err = args
            .validate()
            .expect_err("che_facing+off must be rejected");
        assert!(
            err.contains("enforce"),
            "error must mention enforce; got: {err}"
        );
    }

    #[test]
    fn che_facing_requires_jwt_secret_32_chars() {
        use crate::config::Args;
        use clap::Parser;
        let args = Args::try_parse_from([
            "doorway",
            "--che-facing",
            "--delegates-compute-op-gate",
            "enforce",
            "--jwt-secret",
            "short",
        ])
        .expect("parse must succeed");
        let err = args
            .validate()
            .expect_err("short jwt_secret must be rejected");
        assert!(
            err.contains("32"),
            "error must mention 32 chars; got: {err}"
        );
    }

    #[test]
    fn che_facing_rejects_dev_mode() {
        use crate::config::Args;
        use clap::Parser;
        let args = Args::try_parse_from([
            "doorway",
            "--che-facing",
            "--dev-mode",
            "--delegates-compute-op-gate",
            "enforce",
            "--jwt-secret",
            "a-secret-that-is-at-least-32-chars-long",
        ])
        .expect("parse must succeed");
        let err = args
            .validate()
            .expect_err("che_facing+dev_mode must be rejected");
        assert!(
            err.contains("DEV_MODE"),
            "error must mention DEV_MODE; got: {err}"
        );
    }

    #[test]
    fn che_facing_passes_with_correct_config() {
        use crate::config::Args;
        use clap::Parser;
        let args = Args::try_parse_from([
            "doorway",
            "--che-facing",
            "--delegates-compute-op-gate",
            "enforce",
            "--jwt-secret",
            "a-secret-that-is-at-least-32-chars-long",
        ])
        .expect("parse must succeed");
        assert!(
            args.validate().is_ok(),
            "CHE_FACING with enforce+jwt_secret(32+) must pass"
        );
    }

    // ── no-credential branch (mode-dependent) ─────────────────────────────────

    #[test]
    fn no_credential_observe_forwards_not_blocks() {
        // D2: observe is a non-blocking shadow stage. A no-credential write must
        // forward (log the would-deny), never 403 — that's enforce's job.
        assert_eq!(
            no_credential_decision(&OpGateMode::Observe),
            NoCredentialDecision::Forward,
            "observe + no credential must forward (never block)"
        );
    }

    #[test]
    fn no_credential_enforce_denies() {
        // Enforce fails closed: no verified performer ⇒ 403.
        assert_eq!(
            no_credential_decision(&OpGateMode::Enforce),
            NoCredentialDecision::Deny,
            "enforce + no credential must deny (fail-closed)"
        );
    }

    #[test]
    fn no_credential_off_forwards() {
        // Off never reaches the gate, but the helper must be total: treat as Forward.
        assert_eq!(
            no_credential_decision(&OpGateMode::Off),
            NoCredentialDecision::Forward,
            "off maps to forward for totality (gate never fires in Off)"
        );
    }
}
