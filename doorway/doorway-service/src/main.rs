//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8

use clap::Parser;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use doorway::{
    conductor::{
        max_agents_per_conductor_from_env, ConductorInfo, ConductorPoolMap, ConductorRegistry,
        ConductorRouter, TypedAdminClient, MAX_AGENTS_PER_CONDUCTOR_ENV,
    },
    config::Args,
    db::MongoClient,
    nats::NatsClient,
    orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorState},
    projection::{
        spawn_engine_task, spawn_subscriber, EngineConfig, EprRouter, FallbackOutcome,
        ProjectionEngine, ProjectionSignal, SubscriberConfig,
    },
    server,
    services::{
        self, register_local_storage, spawn_discovery_task_with_signal, DiscoveryConfig,
        StorageRegistrationConfig,
    },
    worker::{PoolConfig, WorkerPool},
};
// DEFAULT_MAX_INFLIGHT / MIN_MAX_INFLIGHT have their single home in the lib
// (server::http) because the AppState ctors reference them and a lib cannot
// import a const from this bin. We `use` them here for inbound_max().
use doorway::server::http::{DEFAULT_MAX_INFLIGHT, DEFAULT_MAX_INFLIGHT_READ, MIN_MAX_INFLIGHT};

/// Number of tokio worker threads to run, regardless of the cgroup CPU limit.
///
/// LIVE-INCIDENT INVARIANT (2026-06-13 doorway freeze): the pod has
/// `resources.limits.cpu: 1`, so the default `#[tokio::main]` (worker_threads =
/// available parallelism) spun up a SINGLE worker thread. The thread dump showed
/// exactly one `tokio-runtime-w` thread, and one synchronously-blocked await on
/// it froze the entire gateway — `/health` included — letting kubelet's
/// restart-on-hang never fire.
///
/// A worker thread blocked in `futex_wait` consumes NO CPU, so running several
/// worker threads breaks the single-blocked-await wedge even at `cpu: 1`. This
/// is the direct antidote to the freeze: raising `limits.cpu` only buys
/// CPU-bound render throughput; it is NOT required to un-freeze the runtime.
///
/// We set the count EXPLICITLY (never CPU-derived) so the cgroup limit can never
/// silently collapse us back to one worker. Env-overridable via
/// `DOORWAY_WORKER_THREADS` (0/garbage → default).
const DEFAULT_WORKER_THREADS: usize = 4;

fn worker_threads() -> usize {
    std::env::var("DOORWAY_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_WORKER_THREADS)
}

/// Inbound admission ceiling (Pillar 2): max concurrent in-flight requests
/// before doorway sheds 503+Retry-After. Floored at `MIN_MAX_INFLIGHT` so a
/// fat-fingered tiny/zero value can never deadlock the gate.
fn inbound_max() -> usize {
    // FOLLOW-ON: when elohim_compute::limits::derive() lands (Auto-config plan),
    // source the default from the host-derived ceiling instead of the constant.
    std::env::var("DOORWAY_MAX_INFLIGHT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_INFLIGHT)
        .max(MIN_MAX_INFLIGHT)
}

/// Read-admission ceiling (Pillar 2, read pool): max concurrent in-flight
/// GET/HEAD requests before the doorway sheds. Separate from `inbound_max()`
/// (writes) so a write burst can't starve reads. Same floor for anti-deadlock.
fn inbound_max_read() -> usize {
    std::env::var("DOORWAY_MAX_INFLIGHT_READ")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_INFLIGHT_READ)
        .max(MIN_MAX_INFLIGHT)
}

fn main() -> anyhow::Result<()> {
    // Load .env BEFORE reading any config env var (incl. DOORWAY_WORKER_THREADS
    // below) so a .env-provided value is honored, not silently ignored. In the
    // live deployment these are real container env vars (seen regardless), but a
    // .env-driven local run must agree.
    let _ = dotenvy::dotenv();

    // Build the multi-threaded runtime with an EXPLICIT worker count so a
    // cpu-limited cgroup cannot collapse us to a single worker (the freeze).
    // `enable_all` wires the I/O + time drivers the gateway depends on.
    let workers = worker_threads();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .thread_name("doorway-tokio-w")
        .build()
        .expect("failed to build tokio runtime");
    runtime.block_on(async_main(workers))
}

async fn async_main(worker_threads: usize) -> anyhow::Result<()> {
    // Parse command line arguments (.env already loaded in main()).
    let args = Args::parse();

    // Initialize tracing/logging
    let log_level = args.log_level.clone();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("doorway={log_level},info").into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Structured commit-bearing startup line — the Jenkins build <-> Loki log join key.
    // ci-investigator filters a pod's logs, reads `commit`, and joins to the build via getBuildScm.
    let build_info = elohim_compute::BuildInfo::new("elohim-doorway");
    info!(
        version = %build_info.version,
        commit = %build_info.commit,
        build_time = %build_info.build_time,
        "doorway starting"
    );

    // Validate configuration
    if let Err(e) = args.validate() {
        error!("Configuration error: {}", e);
        std::process::exit(1);
    }

    // Print startup banner
    info!("======================================");
    info!("  Doorway - Elohim Holochain Gateway");
    info!("  \"Knock and it shall be opened\"");
    info!("======================================");
    info!("Node ID: {}", args.node_id);
    info!("Listen: {}", args.listen);
    info!(
        "Mode: {}",
        if args.dev_mode {
            "DEVELOPMENT"
        } else {
            "PRODUCTION"
        }
    );
    info!("Projection writer: {}", args.projection_writer);
    let conductor_urls = args.conductor_url_list();
    let _startup_app_url = derive_app_url(&args.conductor_url, args.app_port_min);
    info!(
        "Conductor admin: {} (discovery, list_apps)",
        args.admin_url()
    );
    info!("Conductor pool: {} conductor(s)", conductor_urls.len());
    for (i, url) in conductor_urls.iter().enumerate() {
        info!("  conductor-{}: {}", i, url);
    }
    info!("App ports: {}-{}", args.app_port_min, args.app_port_max);
    info!("NATS: {}", args.nats.nats_url);
    info!("MongoDB: {}", args.mongodb_uri);
    info!("Workers: {}", args.worker_count);
    // Tokio runtime worker threads — set EXPLICITLY (not CPU-derived) so a
    // cpu-limited cgroup cannot collapse the runtime to a single worker (the
    // 2026-06-13 freeze). See worker_threads()/DEFAULT_WORKER_THREADS.
    info!("Tokio worker threads: {} (explicit)", worker_threads);
    info!("======================================");

    // Connect to MongoDB (optional in dev mode)
    let mongo = match MongoClient::new(&args.mongodb_uri, &args.mongodb_db).await {
        Ok(client) => {
            info!("MongoDB connected successfully");
            Some(client)
        }
        Err(e) => {
            if args.dev_mode {
                warn!(
                    "MongoDB connection failed (dev mode, continuing without): {}",
                    e
                );
                None
            } else {
                error!("MongoDB connection failed: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Fail-loud, not fail-open, for the bootstrap store specifically: when
    // BOOTSTRAP_MONGODB_DB declares a SHARED bootstrap table but mongo is
    // unavailable, `dev_mode`'s general "continue without mongo" leniency
    // above would otherwise let `BootstrapStore::new` silently pick the
    // per-pod mem backend — exactly the doorway-A incident (2026-08-07):
    // A read backend:"mem" while B stayed on "mongo", so the two doorways
    // could never converge on one bootstrap table and nobody noticed for
    // hours. `mem` is fine when `mem` is the CONFIGURED backend (no
    // BOOTSTRAP_MONGODB_DB — local dev); this only trips when the deployment
    // explicitly wanted the shared table and can't reach it.
    let bootstrap_mongo_db_configured = std::env::var("BOOTSTRAP_MONGODB_DB").is_ok();
    if doorway::bootstrap::mongo_bootstrap_misconfigured(
        args.bootstrap_enabled,
        mongo.is_some(),
        bootstrap_mongo_db_configured,
    ) {
        error!(
            "FATAL: BOOTSTRAP_MONGODB_DB is set (shared kitsune2 bootstrap table \
             configured) but MongoDB is unavailable — refusing to silently fall back \
             to the per-pod mem bootstrap store. This is the doorway-A mem-fallback \
             class (genesis/data/timeline/backlog/doorway-a-bootstrap-mem-fallback.md): \
             a silent downgrade here breaks the shared-bootstrap invariant between \
             doorways and hides a real infra fault. Fix MongoDB connectivity and \
             redeploy."
        );
        std::process::exit(1);
    }

    // Connect to NATS (optional in dev mode)
    let nats = match NatsClient::new(&args.nats, &format!("doorway-{}", args.node_id)).await {
        Ok(client) => {
            info!("NATS connected successfully");
            Some(client)
        }
        Err(e) => {
            if args.dev_mode {
                warn!(
                    "NATS connection failed (dev mode, continuing without): {}",
                    e
                );
                None
            } else {
                error!("NATS connection failed: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Create TWO worker pools for scalable request routing:
    // 1. APP pool: connects to app interface (4445) for zome calls
    // 2. ADMIN pool: connects to admin interface (4444) for admin commands
    //
    // The browser app needs admin commands (generate_agent_pub_key, list_apps, etc.)
    // which MUST go to the admin interface, not the app interface.
    //
    // The Holochain 0.6 app interface requires per-connection authentication: the client
    // sends an `authenticate` message with a token issued via the admin interface's
    // `issue_app_auth_token`. Without it the conductor closes every WebSocket immediately,
    // so the app pool MUST have a token. The admin pool needs no token.

    // APP pool - for zome calls
    let worker_app_url = derive_app_url(&args.conductor_url, args.app_port_min);
    let app_auth_token =
        mint_app_auth_token(args.admin_url(), &args.installed_app_id, args.dev_mode).await;
    let app_pool = match WorkerPool::new(PoolConfig {
        worker_count: args.worker_count,
        conductor_url: worker_app_url.clone(),
        request_timeout_ms: args.request_timeout_ms,
        max_queue_size: 1000,
        auth_token: app_auth_token.clone(),
        token_minter: Some(make_token_minter(
            args.admin_url().to_string(),
            args.installed_app_id.clone(),
            args.dev_mode,
        )),
    })
    .await
    {
        Ok(p) => {
            info!(
                "App worker pool started with {} workers (app interface: {})",
                args.worker_count, worker_app_url
            );
            Some(Arc::new(p))
        }
        Err(e) => {
            if args.dev_mode {
                warn!(
                    "App worker pool failed to start (dev mode, using direct proxy): {}",
                    e
                );
                None
            } else {
                error!("App worker pool failed to start: {}", e);
                std::process::exit(1);
            }
        }
    };

    // ADMIN pool - for admin commands (generate_agent_pub_key, list_apps, etc.)
    // Admin interface does not use app authentication, so auth_token stays None.
    let admin_url = args.admin_url().to_string();
    let admin_pool = match WorkerPool::new(PoolConfig {
        worker_count: args.worker_count,
        conductor_url: admin_url.clone(),
        request_timeout_ms: args.request_timeout_ms,
        max_queue_size: 1000,
        auth_token: None,
        token_minter: None,
    })
    .await
    {
        Ok(p) => {
            info!(
                "Admin worker pool started with {} workers (admin interface: {})",
                args.worker_count, admin_url
            );
            Some(Arc::new(p))
        }
        Err(e) => {
            if args.dev_mode {
                warn!(
                    "Admin worker pool failed to start (dev mode, using direct proxy): {}",
                    e
                );
                None
            } else {
                error!("Admin worker pool failed to start: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Create orchestrator state (before AppState so it can be shared)
    let orchestrator_state = if args.orchestrator_enabled {
        let config = OrchestratorConfig {
            mdns_service_type: "elohim-node".to_string(),
            admin_port: args.orchestrator_admin_port,
            nats_url: args.nats.nats_url.clone(),
            heartbeat_interval_secs: 30,
            failure_threshold: 3,
            auto_assign_custodians: true,
            region: args.region.clone().unwrap_or_else(|| "default".to_string()),
        };
        Some(Arc::new(OrchestratorState::new(config)))
    } else {
        None
    };

    // Create application state
    let mut state = if let Some(p) = app_pool {
        server::AppState::with_pool(args.clone(), mongo, nats, p, admin_pool)
    } else {
        server::AppState::with_services(args.clone(), mongo, nats)
    };
    state.orchestrator = orchestrator_state.clone();

    // Upgrade projection store from memory-only to MongoDB-backed
    if let Some(mongo) = state.mongo.clone() {
        match state.init_projection(&mongo).await {
            Ok(()) => info!("Projection store initialized with MongoDB"),
            Err(e) => {
                if args.dev_mode {
                    warn!(
                        "MongoDB projection init failed (dev mode, using memory-only): {}",
                        e
                    );
                } else {
                    error!("MongoDB projection init failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    // Create single-connection ImportClient for import operations
    // Uses ONE connection to app interface to avoid overwhelming conductor during batch imports
    let import_app_url = derive_app_url(&args.conductor_url, args.app_port_min);
    let import_client = services::ImportClient::new(services::ImportClientConfig {
        app_url: import_app_url.clone(),
        // Reuse the app pool's token (same conductor + app); the minter heals
        // a stale token after a conductor restart.
        auth_token: app_auth_token.clone(),
        token_minter: Some(make_token_minter(
            args.admin_url().to_string(),
            args.installed_app_id.clone(),
            args.dev_mode,
        )),
        ..Default::default()
    });
    state.import_client = Some(Arc::new(import_client));
    info!(
        "ImportClient created (single connection to app interface: {})",
        import_app_url
    );

    // Initialize Conductor Registry — available on ALL instances (writer + reader)
    // Tracks which conductor hosts which agent for future per-request routing
    let registry_collection = state.mongo.as_ref().map(|m| {
        m.inner()
            .database(m.db_name())
            .collection::<bson::Document>("conductor_registry")
    });
    let registry = ConductorRegistry::new(registry_collection).await;

    // Hosted-agent ceiling, read ONCE at startup and threaded into BOTH capacity
    // gates below (registry `capacity_max` = the provisioning gate; ConductorPoolMap
    // = the request-routing/overflow gate). Kitsune2 budgets gossip per SPACE, so
    // arc-convergence cost scales with the hosted-agent count while the convergence
    // budget does not — past ~30 agents on one conductor, storage-arc convergence
    // stalls and the projection never reports caught_up. This is the operator's
    // ceiling on that; see genesis/data/timeline/backlog/
    // self-heal-adam-projection-catchup-exhaustion-full-arc.md ("The real ceiling").
    let max_agents_per_conductor = max_agents_per_conductor_from_env();
    info!(
        env = MAX_AGENTS_PER_CONDUCTOR_ENV,
        max_agents_per_conductor, "Hosted-agent ceiling per conductor"
    );

    // Register all conductors from config
    for (i, url) in conductor_urls.iter().enumerate() {
        let conductor_id = format!("conductor-{i}");
        // Derive admin URL: same host, port - 1 (socat convention: 8444=admin, 8445=app)
        let admin_url = derive_admin_url_from_app(url);
        registry.register_conductor(ConductorInfo {
            conductor_id,
            conductor_url: url.clone(),
            admin_url,
            capacity_used: 0,
            capacity_max: max_agents_per_conductor,
        });
    }

    // Seed capacity_used from the PERSISTED agent mappings (loaded from Mongo by
    // ConductorRegistry::new above). Must run AFTER the register_conductor loop —
    // which hard-sets capacity_used: 0 — and BEFORE discover_existing_agents,
    // whose register_agent calls increment on top of this seed.
    //
    // Without this, capacity_used only ever counts agents registered during the
    // CURRENT process lifetime, so the cap above would mean "N new registrations
    // per boot" rather than a population ceiling — on a doorway that restarts
    // several times a day that is effectively no cap at all. Counts DISTINCT
    // app_ids, not agent keys: discover_existing_agents registers each agent
    // under two base64 encodings, so raw key counts double.
    for (conductor_id, seeded) in registry.seed_capacity_from_agents() {
        info!(
            conductor = %conductor_id,
            capacity_used = seeded,
            capacity_max = max_agents_per_conductor,
            at_capacity = seeded >= max_agents_per_conductor,
            "Seeded conductor capacity from persisted agent mappings"
        );
    }

    // Discover existing agents on each conductor (populate registry for affinity routing)
    // Without this, agents provisioned before the registry existed would have no
    // conductor affinity → requests load-balance via ClusterIP → CellMissing on wrong conductor
    if conductor_urls.len() > 1 {
        discover_existing_agents(&registry, &conductor_urls).await;
    }

    let registry = Arc::new(registry);
    state.conductor_registry = Some(Arc::clone(&registry));
    info!(
        "Conductor registry initialized: {} conductor(s), {} agent mapping(s)",
        registry.conductor_count(),
        registry.agent_count()
    );

    // Create per-conductor WorkerPools for multi-conductor routing
    // Each conductor in CONDUCTOR_URLS gets its own pool of workers
    // Requires a default pool (always exists in production; absent only in dev mode without conductor)
    if let Some(ref default_pool) = state.pool {
        // Same ceiling as the registry above (DOORWAY_MAX_AGENTS_PER_CONDUCTOR),
        // so the routing/overflow gate and the provisioning gate agree.
        let pool_map =
            ConductorPoolMap::with_capacity(Arc::clone(default_pool), max_agents_per_conductor);

        let mut pools_created = 0usize;
        for (i, url) in conductor_urls.iter().enumerate() {
            let conductor_id = format!("conductor-{i}");
            // Use URL as-is from CONDUCTOR_URLS — it already contains the correct port.
            // derive_app_url would replace the port with app_port_min (4445), which breaks
            // headless k8s services where the socat proxy listens on a different port (e.g. 8445).
            let app_url = url.clone();
            let admin_url_for_pool = derive_admin_url_from_app(&app_url);
            // Do NOT mint the initial token synchronously here. The per-conductor
            // pool loop iterates the FULL alpha conductor list (CONDUCTOR_URLS =
            // every env human), and a synchronous mint_app_auth_token retries up
            // to 5× with exponential backoff (~12.5s) per call when a conductor's
            // DNS is briefly unresolvable. Serialized across N conductors that is
            // up to N×~12.5s of startup blocking BEFORE the HTTP listener binds
            // :8080 (server::run, far below) — on a slow/flaky node (intel-nuc,
            // 2026-06-14) that blew the startupProbe budget → `/health` connection
            // refused → SIGKILL → crash-loop, even with the warm_stream firehose
            // cured. The pool's token_minter (below) + the background connection
            // loop already mint on the first unstable (unauthenticated) app-iface
            // session and re-mint on conductor restart, so the upfront mint is
            // redundant — drop it and let auth heal in the background. The default
            // pool (state.pool) serves every request until each per-conductor pool
            // authenticates; per-conductor routing is an affinity optimization,
            // never a request precondition ("conductor will use default" below).
            // See genesis/data/timeline/backlog/self-heal-doorway-startup-conductor-mint-serialization.md
            match WorkerPool::new(PoolConfig {
                worker_count: 2, // Per-conductor pools are smaller than the main pool
                conductor_url: app_url.clone(),
                request_timeout_ms: args.request_timeout_ms,
                max_queue_size: 500,
                auth_token: None,
                token_minter: Some(make_token_minter(
                    admin_url_for_pool.clone(),
                    args.installed_app_id.clone(),
                    args.dev_mode,
                )),
            })
            .await
            {
                Ok(pool) => {
                    pool_map.add_pool(&conductor_id, Arc::new(pool));
                    pools_created += 1;
                    info!(
                        conductor = %conductor_id,
                        url = %app_url,
                        "Per-conductor pool created (2 workers)"
                    );
                }
                Err(e) => {
                    warn!(
                        conductor = %conductor_id,
                        url = %app_url,
                        error = %e,
                        "Failed to create per-conductor pool, conductor will use default"
                    );
                }
            }
        }

        let pool_map = Arc::new(pool_map);
        let router = ConductorRouter::new(Arc::clone(&registry), pool_map);
        state.conductor_router = Some(Arc::new(router));
        info!(
            "Conductor router initialized: {}/{} per-conductor pools created",
            pools_created,
            conductor_urls.len()
        );
    }

    // Generate node Ed25519 signing key for federation
    // This key is used in the DID document and JWKS endpoint
    {
        let (signing_key, verifying_key) = doorway::custodial_keys::crypto::generate_keypair();
        state.node_verifying_key = Some(verifying_key);
        // T2.2: retain the private half so `federation_jwt_validator()` can mint
        // and self-verify EdDSA tokens. Minting stays HS256 unless the operator
        // sets DOORWAY_JWT_SIGN_ALG=eddsa; see the AppState field doc for the
        // boot-rotation caveat that keeps hs256 the default.
        state.node_signing_key = Some(signing_key);
        info!("Node signing key generated for federation");
    }

    // Create ZomeCaller for federation + service registration.
    //
    // SPOF removal (2026-08-06): the caller keeps its pinned primary — the
    // doorway's co-located premise conductor — but now carries the already-declared
    // conductor pool (CONDUCTOR_URLS) as an ordered fallback list. When the primary
    // cannot answer (adam under sqlite write-guard pressure took
    // GET /api/v1/federation/doorways down for 5.5h on doorway-B), agent-agnostic
    // DHT reads route to another peer instead of dying. Identity-binding writes
    // stay pinned — see the ZomeCaller module header. No new config: a
    // single-conductor deployment dedupes to zero fallbacks and behaves exactly
    // as before.
    {
        let admin_url = args.admin_url().to_string();
        let app_url = derive_app_url(&args.conductor_url, args.app_port_min);
        let zome_caller = services::ZomeCaller::with_fallbacks(
            &admin_url,
            &app_url,
            &args.installed_app_id,
            &conductor_urls,
        );
        info!(
            "ZomeCaller created for federation (admin: {}, app: {}, fallback conductors: {})",
            admin_url,
            app_url,
            zome_caller.fallback_count()
        );
        state.zome_caller = Some(Arc::new(zome_caller));
    }

    // Set up P2P status polling from elohim-storage (if STORAGE_URL configured)
    if let Some(ref storage_url) = state.args.storage_url {
        let p2p_health = state.p2p_health.clone();
        let url = format!("{}/p2p/status", storage_url.trim_end_matches('/'));
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(status) = resp.json::<serde_json::Value>().await {
                            let recon = &status["projectionReconcile"];
                            let health = doorway::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: status["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: status["peerId"].as_str().map(|s| s.to_string()),
                                caught_up: recon["caughtUp"].as_bool(),
                                converged: recon["converged"].as_bool(),
                                divergent_anchor: recon["divergentAnchor"]
                                    .as_u64()
                                    .map(|n| n as usize),
                                ..Default::default()
                            };
                            *p2p_health.write().await =
                                Some(doorway::routes::health::P2PHealthSnapshot::new(health));
                        }
                    }
                    _ => {
                        // Storage not reachable or P2P not enabled — clear cached status
                        *p2p_health.write().await = None;
                    }
                }
            }
        });
        info!("P2P status polling enabled (every 30s from elohim-storage)");
    }

    // Register all steward storage peers in the route registry.
    // Fetches GET {url}/manifest for each and compiles the returned routes.
    // A peer that is not yet up at boot is retried in the background until it
    // registers — doorway and storage restart independently (a pod restart, a
    // deploy, the local hc-mesh), and the previous boot-once behavior left the
    // registry EMPTY until an operator restarted doorway (the
    // restart-doorway-epr.sh crutch; reproduced live on the 2026-08-16 local
    // mesh: "registered:0 failed:3", totalRoutes 0, every /db/* 404).
    // Registration is idempotent (install replaces that peer's routes), so the
    // retry can never duplicate.
    let mut peer_urls: Vec<String> = Vec::new();
    if let Some(ref storage_url) = state.args.storage_url {
        peer_urls.push(storage_url.clone());
    }
    for url in &state.args.storage_urls {
        let trimmed = url.trim().to_string();
        if !trimmed.is_empty() && !peer_urls.contains(&trimmed) {
            peer_urls.push(trimmed);
        }
    }

    let mut registered = 0usize;
    let mut pending_peers: Vec<String> = Vec::new();
    for storage_url in &peer_urls {
        match state
            .route_registry
            .register_steward_peer(storage_url)
            .await
        {
            Ok(count) => {
                tracing::info!(
                    routes = count,
                    storage_url = %storage_url,
                    "Steward peer registered"
                );
                registered += 1;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    storage_url = %storage_url,
                    "Failed to register steward peer — will retry in background until it registers"
                );
                pending_peers.push(storage_url.clone());
            }
        }
    }
    if !peer_urls.is_empty() {
        tracing::info!(
            registered,
            pending = pending_peers.len(),
            "Steward peer registration complete"
        );
    }
    if !pending_peers.is_empty() {
        let registry = state.route_registry.clone();
        tokio::spawn(async move {
            const RETRY_SECS: u64 = 15;
            let mut remaining = pending_peers;
            while !remaining.is_empty() {
                tokio::time::sleep(std::time::Duration::from_secs(RETRY_SECS)).await;
                let mut still_pending = Vec::new();
                for storage_url in remaining {
                    match registry.register_steward_peer(&storage_url).await {
                        Ok(count) => {
                            tracing::info!(
                                routes = count,
                                storage_url = %storage_url,
                                "Steward peer registered (late — background retry)"
                            );
                        }
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                storage_url = %storage_url,
                                "Steward peer still unreachable — will keep retrying"
                            );
                            still_pending.push(storage_url);
                        }
                    }
                }
                remaining = still_pending;
            }
            tracing::info!("All steward peers registered — background retry task done");
        });
    }

    // Derive render-capability profile from SSR bundles directory (Task 15).
    //
    // Reads `SSR_BUNDLES_DIR` (directory of pre-built SSR bundles) and fetches
    // the storage manifest from `STORAGE_URL` to determine supported renderers.
    // The result is cached on AppState and served at GET /admin/capability.
    //
    // Non-fatal: if the env var is unset or the deriver fails, render_capability
    // stays None and the endpoint returns JSON null. The existing SSR renderer
    // (`renderer` field, driven by `SSR_BUNDLE_PATH`) is unaffected.
    {
        let render_capability = if let Ok(bundles_dir) = std::env::var("SSR_BUNDLES_DIR") {
            let manifest_url = std::env::var("STORAGE_URL")
                .map(|u| format!("{}/admin/manifest", u.trim_end_matches('/')))
                .unwrap_or_else(|_| "http://localhost:8090/admin/manifest".into());
            let override_path = std::env::var("DOORWAY_RENDER_OVERRIDE")
                .ok()
                .map(std::path::PathBuf::from);
            // Subscribe to elohim-storage's per-node compute view (operator-as-pod
            // model). When set, the deriver picks max_concurrent_renders =
            // min(cpu_total_cores, ceiling_max_cores, allocation_cpu_cores) instead
            // of the static fallback. Falls back to STORAGE_URL/api/v1/compute/dashboard
            // when STORAGE_COMPUTE_URL is unset.
            let compute_url = std::env::var("STORAGE_COMPUTE_URL").ok().or_else(|| {
                std::env::var("STORAGE_URL")
                    .ok()
                    .map(|u| format!("{}/api/v1/compute/dashboard", u.trim_end_matches('/')))
            });
            let compute_budget = match compute_url.as_deref() {
                Some(url) => doorway::render::fetch_compute_budget(url).await,
                None => None,
            };
            if let Some(b) = &compute_budget {
                tracing::info!(
                    cpu_total_cores = b.cpu_total_cores,
                    ceiling_max_cores = b.ceiling_max_cores,
                    allocation_cpu_cores = b.allocation_cpu_cores,
                    derived_default = ?b.min_cpu_budget(),
                    "compute budget subscribed for SSR concurrency derivation"
                );
            }
            match doorway::render::derive_capability(
                std::path::Path::new(&bundles_dir),
                &manifest_url,
                override_path.as_deref(),
                compute_budget.as_ref(),
            )
            .await
            {
                Ok(c) => {
                    tracing::info!(
                        capability = ?c,
                        "render capability derived"
                    );
                    c
                }
                Err(e) => {
                    tracing::error!(error = %e, "deriver failed — render_capability=null");
                    None
                }
            }
        } else {
            tracing::debug!("SSR_BUNDLES_DIR unset — render_capability will be None");
            None
        };
        // Concurrency semaphore. A derived capability sizes it to
        // max_concurrent_renders (unchanged). Capability absent but a renderer IS
        // loaded → a conservative default (the live alpha state: SSR_BUNDLE_PATH
        // set, capability URL unset), so the SSR landing hot path can't spawn
        // unbounded V8 isolates on a cpu:1 pod (each cold render holds a 60s
        // wall). Overflow sheds to the projected-bundle fallback — today's
        // serving behavior — so the bound degrades safely. No renderer → None
        // (nothing to render, no limiter). See doorway::render::ssr_semaphore_permits.
        state.render_semaphore = doorway::render::ssr_semaphore_permits(
            render_capability.as_ref(),
            state.renderer_registry.any_loaded(),
        )
        .map(|permits| std::sync::Arc::new(tokio::sync::Semaphore::new(permits)));
        state.render_capability = render_capability;
    }

    // Wire pkarr resolver service (cutover gate #10).
    // Opt-in via DOORWAY_PKARR_RESOLVER_ENABLED=true.
    if args.pkarr_resolver_enabled {
        use doorway::services::pkarr_resolver::{PkarrResolverConfig, PkarrResolverService};
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: args.pkarr_cache_capacity,
            persist_dir: args.pkarr_cache_dir.clone(),
        };
        state.pkarr_resolver = Some(Arc::new(PkarrResolverService::new(cfg)));
        info!("pkarr resolver enabled at /pkarr/{{key}} (gate #10)");
    }

    // Create WarmupState before Arc::new(state) so it can be stored in AppState
    // and also passed to spawn_stream_task later.
    if args.projection_writer && !peer_urls.is_empty() && state.projection.is_some() {
        state.warmup_state = Some(Arc::new(
            doorway::projection::warm_stream::WarmupState::new(),
        ));
    }

    // Pillar 2: size the inbound admission ceiling from DOORWAY_MAX_INFLIGHT
    // (the ctor set a default; this overrides with the env-derived value).
    // storage_proxy_client + upstream_breakers are already ctor-set to identical
    // defaults — not reassigned here.
    state.inbound_semaphore = Arc::new(tokio::sync::Semaphore::new(inbound_max()));
    // Read pool: separate ceiling so a write burst can't starve reads (genesis
    // #1272 /epr/* shed). Ctor set a default; override with the env-derived value.
    state.read_semaphore = Arc::new(tokio::sync::Semaphore::new(inbound_max_read()));
    // Expose the ceiling on /metrics + as the AdmissionView maxInflight source.
    doorway::metrics::set_inbound_max(inbound_max());
    info!(
        max_inflight = inbound_max(),
        max_inflight_read = inbound_max_read(),
        "inbound admission ceiling set"
    );

    // What this peer knows about ITSELF decides how much of the Chain-R hop
    // chain it can afford to record. A peer must not be TOLD it can afford
    // full instrumentation — a watch or a shared 1-2 core pod has to decline,
    // and only the peer can know that. `available_parallelism` is the probe
    // this process can actually make about its own host; 0 (unknowable) is
    // honest absence and `derive_hop_tier` resolves it DOWN, never up.
    //
    // Same shape as elohim-storage's `conductor_admission::default_capacity`:
    // env is the OVERRIDE (`DOORWAY_HOP_METRICS`), the default is derived.
    let hop_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(0);
    let hop_tier = doorway::metrics::derive_hop_tier(hop_cores);
    doorway::metrics::set_hop_tier(hop_tier);
    info!(
        cores = hop_cores,
        tier = doorway::metrics::hop_tier_label(),
        "chain-R hop metrics tier derived from this peer's own capacity"
    );

    // Create discovery readiness channel before Arc::new(state)
    let (discovery_tx, discovery_rx) = tokio::sync::watch::channel(false);
    state.discovery_ready = discovery_rx;

    let state = Arc::new(state);

    // B12: Load active EPR projections from storage at boot.
    //
    // Seeds the EprRouter so pillar URLs (e.g. /lamad) are routable from the
    // first request. Wrapped in match — never fails boot if storage isn't up yet.
    // SSE events from Phase A's events.rs will repopulate the router as soon as
    // storage becomes reachable.
    {
        let epr_http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let node_id_str = state.args.node_id.to_string();
        let doorway_id = match state.args.doorway_id.as_deref() {
            Some(id) => id,
            None => {
                warn!(
                    node_id = %node_id_str,
                    "DOORWAY_ID env var is not set — falling back to node UUID. The EPR \
                     router filters projections by doorway_id (in_scope_of LIKE \
                     'doorway:{{id}}|'), so it will match ZERO seeded rows and '/' will \
                     fall through to /threshold while '/lamad' returns 404. Set DOORWAY_ID \
                     in the deployment manifest (e.g. DOORWAY_ID=alpha-elohim-host)."
                );
                node_id_str.as_str()
            }
        };

        // Registration-driven pool: preserve the configured primary first,
        // then try signed peer gateway endpoints discovered from the
        // infrastructure DHT before the remaining configured fallbacks.
        // The configured pool remains the bootstrap/fallback floor when
        // discovery is empty or the conductor is not ready.
        let configured_epr_urls = epr_storage_pool(&state.args);
        let federation_config = services::FederationConfig::from_args(&state.args);
        let epr_pool_urls = resolve_epr_storage_pool(
            federation_config.as_ref(),
            state.zome_caller.as_deref(),
            &configured_epr_urls,
        )
        .await;
        if epr_pool_urls.is_empty() {
            info!(
                "No signed doorway endpoints or STORAGE_URL/STORAGE_URLS configured — EPR router starts empty"
            );
        } else {
            apply_epr_fallback_outcome(
                doorway::projection::fetch_projections_with_fallback(
                    &epr_pool_urls,
                    doorway_id,
                    &epr_http,
                )
                .await,
                doorway_id,
                &state.epr_router,
                "boot",
            );
        }
    }

    // Warm-boot shell cache hydration (Task 3.4).
    //
    // The SSR hot cache is in-memory and dies with the pod; the shell it needs
    // used to be re-fetched from a still-catching-up upstream on every `/`
    // request, which is a doomed 10s stall on the browser hot path. The
    // Mongo-backed app_file_cache OUTLIVES the pod and already holds the last
    // reconciled bundle, so hydrate each mounted projection's entry file from it
    // NOW — before the upstream has answered anything — and `/` is servable from
    // the first request. Best-effort: a cold archive hydrates 0 and the lazy
    // path still converges.
    {
        let targets: Vec<(String, String)> = state
            .epr_router
            .mount_url_paths()
            .into_iter()
            .filter_map(|p| state.epr_router.dispatch(&p))
            .map(|projection| (projection.epr_id, projection.entry_file))
            .collect();
        if !targets.is_empty() {
            let hydrated = state.warm_shell.hydrate(&targets).await;
            info!(
                mounts = targets.len(),
                hydrated, "Warm-boot shell cache hydration complete"
            );
        }
    }

    // Shell pre-warm (best-effort, off the request path). `hydrate` above only
    // converges what the doorway's OWN Mongo archive already holds; a genuinely
    // cold archive (fresh deploy, fresh mesh) still needs one upstream fetch per
    // mount before `warm_shell` is warm. MEASURED (local mesh regen,
    // 2026-08-22): `GET /` 503'd at 11.1s on the FIRST request with
    // servedBundleHeads populated and the breaker closed — the render engine
    // was warm, the shell store was not, and nobody had paid that cost before a
    // real browser did. Spawned (not awaited) so a slow first fetch never
    // delays the HTTP listener binding; see `prewarm_projected_shells` for why
    // repeat calls (this one and the periodic-refresh one below) are cheap.
    tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            doorway::server::http::prewarm_projected_shells(&state).await;
        }
    });

    // Periodic EPR-router self-heal refresh (operator-free recovery).
    //
    // The B12 boot-fetch above is a one-shot with a 10s timeout. If storage is
    // crashlooping at boot it times out → the router starts empty → '/lamad'
    // 404s and '/' falls through to /threshold, and it stays that way until a
    // doorway restart. This task re-fetches every DOORWAY_EPR_REFRESH_SECS
    // (default 30) and atomically replaces the routing table on success, so the
    // router self-populates once storage recovers — no kubectl restart needed.
    // On failure it logs at debug and preserves the last-good table (never
    // clears the router on transient storage unavailability).
    let refresh_configured_urls = epr_storage_pool(&state.args);
    let refresh_federation_config = services::FederationConfig::from_args(&state.args);
    let refresh_zome_caller = state.zome_caller.clone();
    if !refresh_configured_urls.is_empty()
        || (refresh_federation_config.is_some() && refresh_zome_caller.is_some())
    {
        let refresh_secs = std::env::var("DOORWAY_EPR_REFRESH_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let refresh_epr_router = Arc::clone(&state.epr_router);
        let refresh_state = Arc::clone(&state);
        let refresh_node_id = state.args.node_id.to_string();
        let refresh_doorway_id = state.args.doorway_id.clone().unwrap_or(refresh_node_id);
        let configured_pool_size = refresh_configured_urls.len();
        let registration_discovery = refresh_federation_config.is_some();
        tokio::spawn(async move {
            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(refresh_secs));
            // Skip the immediate first tick — the boot-fetch already ran.
            interval.tick().await;
            loop {
                interval.tick().await;
                // Re-resolve every tick so key-authenticated registration
                // updates change the candidate set without a doorway restart.
                let refresh_pool_urls = resolve_epr_storage_pool(
                    refresh_federation_config.as_ref(),
                    refresh_zome_caller.as_deref(),
                    &refresh_configured_urls,
                )
                .await;
                let outcome = doorway::projection::fetch_projections_with_fallback(
                    &refresh_pool_urls,
                    &refresh_doorway_id,
                    &http,
                )
                .await;
                apply_epr_fallback_outcome(
                    outcome,
                    &refresh_doorway_id,
                    &refresh_epr_router,
                    "periodic refresh",
                );
                // A mount that just appeared here (storage was still catching
                // up at boot) never got the boot-time warm_shell.hydrate() or
                // prewarm_projected_shells() pass — give it the same
                // pre-warming a boot-time mount gets, off the request path.
                // Awaited inline: this loop already runs on its own
                // `refresh_secs` cadence, not the request hot path.
                doorway::server::http::prewarm_projected_shells(&refresh_state).await;
            }
        });
        info!(
            interval_secs = refresh_secs,
            configured_pool_size,
            registration_discovery,
            "EPR router periodic self-heal refresh task started"
        );
    }

    // Start zome capability discovery (import configs, cache rules)
    // This populates zome_configs and import_config_store for route matching
    // Only needed on writer instances (readers serve from shared MongoDB)
    if args.projection_writer {
        if let Some(ref import_config_store) = state.import_config_store {
            let admin_url = args.admin_url().to_string();
            let discovery_config = DiscoveryConfig {
                admin_url: admin_url.clone(),
                installed_app_id: args.installed_app_id.clone(),
                zome_name: "content_store".to_string(), // TODO: make configurable
                ..DiscoveryConfig::default()
            };

            let _discovery_handle = spawn_discovery_task_with_signal(
                discovery_config,
                Arc::clone(&state.zome_configs),
                Arc::clone(import_config_store),
                discovery_tx,
            );
            info!(
                "Zome capability discovery started (admin: {}, import routes will be available after discovery completes)",
                admin_url
            );
        } else {
            warn!("Import config store not initialized, skipping zome discovery");
        }
    } else {
        info!("Zome discovery skipped (read replica mode)");
    }

    // Start Projection Engine (if projection store is available)
    //
    // Gating logic (projection_writer flag):
    //   projection_writer=true  → starts signal subscriber (populates MongoDB from DHT signals)
    //   projection_writer=false → reads from shared MongoDB, no subscriber (read replica mode)
    //
    // In dev mode, the signal subscriber is skipped by default: many dev-mode
    // contexts run with no conductor at all and would spin reconnect noise.
    // Auth is NOT the blocker (the subscriber self-authenticates via
    // TypedAppClient), so --dev-signal-subscriber opts back in when the dev
    // stack fronts real conductors (e.g. hc-mesh.sh).
    let _projection_handle = if let Some(ref projection_store) = state.projection {
        if !args.projection_writer || (args.dev_mode && !args.dev_signal_subscriber) {
            if !args.projection_writer {
                info!("Projection reader: using shared MongoDB (PROJECTION_WRITER=false)");
            } else {
                info!("Projection engine started (dev mode: signal subscriber disabled; opt in with --dev-signal-subscriber)");
            }

            // Create engine without signals (it will still work for manual queries)
            let engine = Arc::new(ProjectionEngine::new(
                projection_store.clone(),
                EngineConfig::default(),
            ));

            // Start engine without signal subscription
            let (signal_tx, _) = tokio::sync::broadcast::channel(1);
            let signal_rx = signal_tx.subscribe();
            let engine_handle = spawn_engine_task(engine, signal_rx);

            Some((tokio::spawn(async {}), engine_handle))
        } else {
            // Production mode + projection_writer=true: start multi-peer signal subscribers
            // One subscriber per conductor, all feeding into a shared signal channel.
            info!(
                "Starting projection engine with {} signal subscriber(s)",
                conductor_urls.len()
            );

            // Shared channel: all subscribers forward signals here → engine
            let (all_signals_tx, all_signals_rx) =
                tokio::sync::broadcast::channel::<ProjectionSignal>(2000);

            let peer_health = Arc::clone(&state.peer_health);

            for (i, conductor_app_url) in conductor_urls.iter().enumerate() {
                let admin_url = derive_admin_url_from_app(conductor_app_url);
                let conductor_id = format!("conductor-{i}");

                let subscriber_config = SubscriberConfig {
                    admin_url: admin_url.clone(),
                    app_url: conductor_app_url.clone(),
                    installed_app_id: args.installed_app_id.clone(),
                    storage_url: peer_urls.get(i).cloned(),
                    projection_store: state.projection.as_ref().map(Arc::clone),
                    warmup_state: state.warmup_state.clone(),
                    on_reconnect: Some({
                        // Make reconnect churn visible to operators (/status):
                        // each subscriber reconnect attempt marks the peer
                        // Degraded and bumps reconnect_attempts.
                        let peer_health = Arc::clone(&peer_health);
                        let conductor_id = conductor_id.clone();
                        Arc::new(move || peer_health.record_reconnect(&conductor_id))
                    }),
                    ..SubscriberConfig::default()
                };

                let (subscriber, _sub_handle) = spawn_subscriber(subscriber_config);

                peer_health.register(&conductor_id, conductor_app_url);

                // Forward this subscriber's signals to the shared engine channel
                let mut sub_rx = subscriber.subscribe();
                let fwd_tx = all_signals_tx.clone();
                let peer_health_clone = Arc::clone(&peer_health);
                let conductor_id_clone = conductor_id.clone();
                tokio::spawn(async move {
                    peer_health_clone.update_health(
                        &conductor_id_clone,
                        elohim_compute::ServiceHealth::Healthy,
                        "connected",
                    );
                    loop {
                        match sub_rx.recv().await {
                            Ok(signal) => {
                                peer_health_clone.record_signal(&conductor_id_clone);
                                if fwd_tx.send(signal).is_err() {
                                    break; // engine dropped
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                // Reconnect attempts are recorded via the
                                // subscriber's on_reconnect hook (see
                                // SubscriberConfig above).
                                peer_health_clone.update_health(
                                    &conductor_id_clone,
                                    elohim_compute::ServiceHealth::Offline,
                                    "channel closed",
                                );
                                break;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                peer_health_clone.update_health(
                                    &conductor_id_clone,
                                    elohim_compute::ServiceHealth::Degraded,
                                    &format!("lagged {n} signals"),
                                );
                                warn!(
                                    conductor = %conductor_id_clone,
                                    lagged = n,
                                    "Signal forwarder lagged"
                                );
                            }
                        }
                    }
                });

                info!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    app_url = %conductor_app_url,
                    "Signal subscriber spawned"
                );
            }

            // Drop the extra sender so channel closes when all forwarders complete
            drop(all_signals_tx);

            // Create and start projection engine with aggregated signals
            let engine = Arc::new(ProjectionEngine::new(
                projection_store.clone(),
                EngineConfig::default(),
            ));
            let engine_handle = spawn_engine_task(engine, all_signals_rx);

            info!(
                "Projection engine started (writer mode, {} subscriber(s))",
                conductor_urls.len()
            );
            Some((tokio::spawn(async {}), engine_handle))
        }
    } else {
        warn!("Projection engine not started (no projection store)");
        None
    };

    // Start app file cache invalidation hook — watches projection store updates
    // for html5-app content changes and invalidates cached files + blob hash index.
    if let (Some(ref projection_store), Some(ref app_cache)) =
        (&state.projection, &state.app_file_cache)
    {
        let update_rx = projection_store.subscribe();
        let _invalidation_handle =
            doorway::cache::spawn_app_cache_invalidation_task(Arc::clone(app_cache), update_rx);
        info!("App file cache invalidation hook started");
    }

    // Pre-warm the hot cache from MongoDB BEFORE scheduling the warm_stream
    // replay so the cold-start firehose hits the idempotency guard and is
    // skipped (the 2026-06-14 crashloop cure: a fresh pod's empty cache made
    // warm_stream re-upsert the whole corpus → CPU burst → /health starved →
    // liveness SIGKILL). Awaited (bounded 30s internally) so it completes
    // before the warm_stream task's 10s delay elapses. Degrade-open.
    if args.projection_writer {
        if let Some(ref projection_store) = state.projection {
            match projection_store.prewarm_hot_cache_from_mongo().await {
                Ok(n) => info!(
                    prewarmed = n,
                    "Hot cache pre-warm complete (cold-start firehose guard)"
                ),
                Err(e) => {
                    warn!("Hot cache pre-warm failed (degrade-open — cold-start may firehose): {e}")
                }
            }
        }
    }

    // Warm projection cache from existing storage content (background, delayed)
    // Signals only deliver FUTURE writes — existing content needs explicit fetch.
    if args.projection_writer && !peer_urls.is_empty() {
        if let Some(ref projection_store) = state.projection {
            let _warm_handle = doorway::projection::warm_stream::spawn_stream_task(
                Arc::clone(projection_store),
                peer_urls.clone(),
                10, // 10s delay — let MongoDB + storage settle
                state.warmup_state.clone(),
            );
            info!(
                peers = peer_urls.len(),
                "Projection cache warm-up scheduled (10s delay)"
            );
        }
    }

    // Pattern Z.B.2 bridge + B15 projection refresh: subscribe to storage's
    // /api/v1/events SSE so doorway accepts content updates and projection
    // registration/revocation as they happen.
    //
    //  content.{created,updated,deleted}  → evict AppFileCache slug entry
    //  projection.{registered,revoked}    → re-fetch all projections and call
    //                                       EprRouter::replace_all (B15)
    //
    // Pairs with warm_stream above: warm_stream is the cold-start snapshot,
    // this subscriber is the live tail. Doorway is a projection of substrate
    // truth — without this subscriber, PATCHes and commitment changes are
    // silent to the projection caches until next restart.
    //
    // See: genesis/docs/superpowers/specs/2026-05-23-doorway-access-tier-patterns.md
    if let Some(ref storage_url) = args.storage_url {
        let node_id_str = state.args.node_id.to_string();
        let doorway_id = state
            .args
            .doorway_id
            .as_deref()
            .unwrap_or(&node_id_str)
            .to_string();
        let _events_handle = doorway::projection::storage_events_subscriber::spawn_subscriber_task(
            storage_url.clone(),
            doorway_id,
            state.app_file_cache.clone(),
            Arc::clone(&state.epr_router),
        );
        info!(
            storage_url = %storage_url,
            "Storage events subscriber spawned (Pattern Z.B.2 + B15 projection refresh)"
        );
    }

    // Start Orchestrator background tasks (if enabled)
    // The state is already created and wired to AppState above
    let _orchestrator = if let Some(ref orch_state) = orchestrator_state {
        info!("Starting orchestrator background tasks...");

        let mut orch = Orchestrator::with_state(Arc::clone(orch_state));

        match orch.start().await {
            Ok(()) => {
                info!("Orchestrator started (mDNS discovery, heartbeat, disaster recovery)");
                Some(orch)
            }
            Err(e) => {
                if args.dev_mode {
                    warn!("Orchestrator failed to start (dev mode, continuing): {}", e);
                    None
                } else {
                    error!("Orchestrator failed to start: {}", e);
                    std::process::exit(1);
                }
            }
        }
    } else {
        None
    };

    // Auto-register local elohim-storage with infrastructure DNA (prototype mode)
    // This allows doorway operators to announce their local storage to the network
    let storage_config = StorageRegistrationConfig::from_env();
    if storage_config.enabled {
        let app_url = derive_app_url(&args.conductor_url, args.app_port_min);
        let result =
            register_local_storage(&storage_config, &app_url, &args.installed_app_id).await;

        if result.success {
            info!(
                capabilities = ?result.registered_capabilities,
                "Local storage auto-registration completed"
            );
        } else {
            warn!(
                errors = ?result.errors,
                "Local storage auto-registration had failures"
            );
        }
    }

    // Federation peer discovery (HTTP-based)
    // Queries FEDERATION_PEERS URLs to discover other doorways in the network
    // Uses shared PeerUrlList so admin API can add/remove peers at runtime
    if !args.federation_peers.is_empty() {
        let peer_url_list = state.peer_url_list.clone();
        let self_id = args.doorway_id.clone();
        let cache = state.peer_cache.clone();
        let peer_count = args.federation_peers.len();

        // F-COHERENCE: the cross-edge probe rides the existing discovery loop.
        // It needs the EprRouter (to recompute this edge's self-manifest each
        // tick) and a PeerCoherenceCache to store the per-peer verdicts. The
        // cache is owned by the spawned task here (surfacing it on a route /
        // SelfHealingView is the Wave-F3 X-COH-DIAG follow-on, out of scope).
        let coherence_epr_router = Arc::clone(&state.epr_router);
        let coherence_cache = services::federation::new_peer_coherence_cache();

        services::federation::spawn_peer_discovery_task(
            peer_url_list,
            self_id,
            cache,
            coherence_epr_router,
            coherence_cache,
            std::time::Duration::from_secs(10), // initial delay (let peers boot)
            std::time::Duration::from_secs(60), // refresh interval
        );
        info!(
            "Federation peer discovery started: {} peer(s) configured",
            peer_count
        );

        // T2.2 live wiring: refresh the peer JWKS cache off the peer list the
        // discovery task above already maintains. Same cadence, one tick later
        // so the first refresh sees a populated peer list. This is the ONLY
        // place peer keys are fetched — `verify_token` is network-free.
        services::federation::spawn_peer_jwks_refresh_task(
            state.peer_cache.clone(),
            state.peer_jwks_cache.clone(),
            std::time::Duration::from_secs(15),
            std::time::Duration::from_secs(60),
        );
        info!("Peer JWKS refresh task started (cross-doorway JWT verification)");
    }

    // Federation: register in DHT + start heartbeat task
    // Requires doorway_id + doorway_url to be configured
    if let Some(fed_config) = services::FederationConfig::from_args(&args) {
        let zome_caller = state.zome_caller.clone();
        let fed_state = Arc::clone(&state);
        let fed_config_clone = fed_config.clone();

        if let Some(zome_caller) = zome_caller {
            // Spawn registration with 5s delay (conductor readiness)
            let zc = Arc::clone(&zome_caller);
            let fc = fed_config.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                info!(
                    "Federation: registering doorway '{}' in DHT...",
                    fc.doorway_id
                );

                let mut capabilities = vec!["gateway".to_string()];
                if !fc.doorway_url.is_empty() {
                    capabilities.push("bootstrap".to_string());
                    capabilities.push("signal".to_string());
                }

                if let Err(e) =
                    services::federation::register_doorway_in_dht(&fc, &zc, capabilities).await
                {
                    warn!("Federation registration failed (non-fatal): {}", e);
                }
            });

            // Spawn heartbeat task
            let _heartbeat = services::federation::spawn_heartbeat_task(
                fed_config_clone,
                zome_caller,
                fed_state,
            );
            info!(
                "Federation enabled: doorway_id={}, heartbeat every {}s",
                fed_config.doorway_id, fed_config.heartbeat_interval_secs,
            );
        }
    }

    // SSR bundle-head reconcile tick (Track-4 T4-1): converge each served SSR
    // slug toward its declared serverBlobHash without a restart. Interval is
    // DOORWAY_BUNDLE_RECONCILE_SECS (default 300; 0 disables). Only spawned when
    // the registry actually has a declared-head source (an SSR doorway with at
    // least one configured slug) — an image-baked / SSR-disabled registry has
    // nothing to reconcile.
    {
        let reconcile_secs = std::env::var("DOORWAY_BUNDLE_RECONCILE_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(300);
        if reconcile_secs > 0 && state.renderer_registry.reconcile_enabled() {
            let reconcile_state = Arc::clone(&state);
            tokio::spawn(async move {
                let period = std::time::Duration::from_secs(reconcile_secs);
                loop {
                    tokio::time::sleep(period).await;
                    let outcomes = reconcile_state
                        .renderer_registry
                        .reconcile(&reconcile_state.cache)
                        .await;
                    let refreshed = outcomes.iter().filter(|o| o.outcome == "refreshed").count();
                    let failed = outcomes.iter().filter(|o| o.outcome == "failed").count();
                    let unreachable = outcomes
                        .iter()
                        .filter(|o| o.outcome == "unreachable")
                        .count();
                    if refreshed > 0 || failed > 0 || unreachable > 0 {
                        info!(
                            "SSR bundle reconcile: {} refreshed, {} failed, {} unreachable ({} slugs checked)",
                            refreshed,
                            failed,
                            unreachable,
                            outcomes.len()
                        );
                    } else {
                        let current = outcomes.iter().filter(|o| o.outcome == "current").count();
                        info!(
                            "SSR bundle reconcile: {} refreshed, {} failed, {} unreachable, {} current ({} slugs checked)",
                            refreshed,
                            failed,
                            unreachable,
                            current,
                            outcomes.len()
                        );
                    }
                }
            });
            info!(
                "SSR bundle-head reconcile tick started: every {}s",
                reconcile_secs
            );
        }
    }

    // Run the server
    if let Err(e) = server::run(state).await {
        error!("Server error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Build the ordered storage pool for EPR-projection fetches: the singular
/// primary (`STORAGE_URL`) first, then each distinct `STORAGE_URLS` peer.
///
/// Mirrors the steward-peer registration ordering (see the `peer_urls`
/// construction earlier in `main`) so the EPR router consults the same pool
/// that the route registry already trusts — primary first, peers as reach
/// fallback. journal: `.claude/deliver/journal-resilient-dual-doorway.md` RC #2.
fn epr_storage_pool(args: &Args) -> Vec<String> {
    let mut pool: Vec<String> = Vec::new();
    if let Some(ref primary) = args.storage_url {
        let trimmed = primary.trim().to_string();
        if !trimmed.is_empty() {
            pool.push(trimmed);
        }
    }
    for url in &args.storage_urls {
        let trimmed = url.trim().to_string();
        if !trimmed.is_empty() && !pool.contains(&trimmed) {
            pool.push(trimmed);
        }
    }
    pool
}

/// Resolve the operational EPR-fetch candidate list from notarized doorway
/// registrations, retaining configured storage URLs as bootstrap/fallback.
///
/// The infrastructure zome validates each registration's detached endpoint
/// signature before it can enter the DHT. We therefore consume
/// `get_all_doorways` results directly instead of maintaining a second
/// doorway-local signature or identity authority.
async fn resolve_epr_storage_pool(
    federation_config: Option<&services::FederationConfig>,
    zome_caller: Option<&services::ZomeCaller>,
    configured_urls: &[String],
) -> Vec<String> {
    let (Some(config), Some(zome_caller)) = (federation_config, zome_caller) else {
        return configured_urls.to_vec();
    };

    match services::federation::get_all_doorways(zome_caller, config).await {
        Ok(registrations) => {
            let candidates = doorway::projection::epr_fetch_candidate_urls(
                &registrations,
                &config.doorway_id,
                configured_urls,
            );
            debug!(
                registration_count = registrations.len(),
                candidate_count = candidates.len(),
                configured_count = configured_urls.len(),
                "Resolved EPR projection candidates from signed doorway registrations"
            );
            candidates
        }
        Err(error) => {
            debug!(
                error = %error,
                configured_count = configured_urls.len(),
                "Signed doorway registration resolution unavailable; using configured EPR candidates"
            );
            configured_urls.to_vec()
        }
    }
}

/// Apply a pool-fallback fetch outcome to the EPR router, logging at the level
/// each case warrants (Fix A: the degraded-primary state was invisible at DEBUG
/// for days — the WARN here is part of the fix). `phase` is "boot" or
/// "periodic refresh" for log context.
///
/// - `PrimaryNonEmpty` / `PeerServed` → `replace_all` with the fetched rows.
///   `PeerServed` logs WARN naming the degraded primary and the serving peer.
/// - `AllEmpty` → `replace_all` with empty (genuine empty state), log INFO.
/// - `AllUnreachable` → preserve the last-good table, log WARN.
fn apply_epr_fallback_outcome(
    outcome: FallbackOutcome,
    doorway_id: &str,
    router: &EprRouter,
    phase: &str,
) {
    match outcome {
        FallbackOutcome::PrimaryNonEmpty { url, projections } => {
            // Log the POST-validation truth: a fetched batch may install fewer
            // rows than it carried (poisoned rows skipped per-row). Reporting
            // `installed` (not the input length) is the Fix-2 observability seam.
            let outcome = router.replace_all(projections);
            info!(
                phase,
                installed = outcome.installed,
                rejected = outcome.rejected,
                doorway_id = %doorway_id,
                serving_url = %url,
                "EPR router: loaded projections from primary storage"
            );
        }
        FallbackOutcome::PeerServed {
            primary_url,
            primary_empty,
            serving_url,
            projections,
        } => {
            let outcome = router.replace_all(projections);
            warn!(
                phase,
                installed = outcome.installed,
                rejected = outcome.rejected,
                doorway_id = %doorway_id,
                primary_url = %primary_url,
                primary_state = if primary_empty { "empty" } else { "unreachable" },
                serving_url = %serving_url,
                "EPR router DEGRADED: primary storage gave no projections; a pool peer \
                 supplied them. Router is serving via the fallback peer — heal the primary."
            );
        }
        FallbackOutcome::AllEmpty { urls_tried } => {
            // Genuine empty state — every pool member returned 0 rows. Replace
            // (an honest empty router) and log at INFO.
            router.replace_all(Vec::new());
            info!(
                phase,
                doorway_id = %doorway_id,
                urls_tried = ?urls_tried,
                "EPR router: every storage pool member returned 0 projections — \
                 genuine empty state, router cleared"
            );
        }
        FallbackOutcome::AllUnreachable {
            urls_tried,
            last_error,
        } => {
            // Total fetch failure — preserve the last-good table (never clear on
            // transient unavailability).
            warn!(
                phase,
                doorway_id = %doorway_id,
                urls_tried = ?urls_tried,
                last_error = %last_error,
                "EPR router: entire storage pool unreachable; keeping last-good table"
            );
        }
    }
}

/// Derive admin WebSocket URL from app URL by replacing the port.
/// Delegates to the library implementation so there is a single source of truth.
fn derive_admin_url_from_app(app_url: &str) -> String {
    doorway::derive_admin_url_from_app(app_url)
}

/// Build a [`doorway::worker::TokenMinter`] that re-mints an app auth token
/// from the given admin interface. Handed to worker pools so a connection
/// stuck in unstable sessions (the stale-token signature after a conductor
/// restart) heals itself instead of staying broken until the doorway restarts.
fn make_token_minter(
    admin_url: String,
    installed_app_id: String,
    dev_mode: bool,
) -> doorway::worker::TokenMinter {
    std::sync::Arc::new(
        move || -> futures::future::BoxFuture<'static, Option<Vec<u8>>> {
            let admin_url = admin_url.clone();
            let installed_app_id = installed_app_id.clone();
            Box::pin(
                async move { mint_app_auth_token(&admin_url, &installed_app_id, dev_mode).await },
            )
        },
    )
}

/// Mint an app authentication token by connecting to a conductor's admin interface
/// and calling `issue_app_auth_token`.
///
/// Holochain 0.6's app interface requires every WebSocket connection to authenticate
/// with a token; without it the conductor closes the socket immediately. The
/// WorkerPool's app-interface workers need this token in their `PoolConfig`.
///
/// Strategy:
/// - Retry a handful of times with short backoff to handle conductor warmup
///   (admin interface is the first to come up, but the app may not be installed
///   yet during a fresh boot).
/// - In dev mode, return `None` on failure so the pool still starts (workers will
///   fail to connect, which the existing dev-mode error handling tolerates).
/// - In production, return `None` on failure too — the pool creation step will
///   then start without auth and visibly fail to authenticate, which is preferable
///   to refusing to start at all (same observable failure mode as before this fix,
///   but with a clear log line).
async fn mint_app_auth_token(
    admin_url: &str,
    installed_app_id: &str,
    dev_mode: bool,
) -> Option<Vec<u8>> {
    use doorway::conductor::TypedAdminClient;

    const MAX_ATTEMPTS: u32 = 5;
    const BASE_DELAY: std::time::Duration = std::time::Duration::from_millis(500);
    const TOKEN_EXPIRY_SECS: u64 = 24 * 60 * 60; // 24h — workers reuse on every reconnect

    for attempt in 1..=MAX_ATTEMPTS {
        match TypedAdminClient::connect(admin_url).await {
            Ok(admin) => match admin
                .issue_app_authentication_token(installed_app_id, TOKEN_EXPIRY_SECS)
                .await
            {
                Ok(token) => {
                    info!(
                        admin_url = %admin_url,
                        app_id = %installed_app_id,
                        token_bytes = token.len(),
                        "Minted app auth token for worker pool"
                    );
                    return Some(token);
                }
                Err(e) => {
                    warn!(
                        admin_url = %admin_url,
                        app_id = %installed_app_id,
                        attempt,
                        error = %e,
                        "issue_app_auth_token failed (will retry)"
                    );
                }
            },
            Err(e) => {
                warn!(
                    admin_url = %admin_url,
                    attempt,
                    error = %e,
                    "Admin client connect failed while minting auth token (will retry)"
                );
            }
        }

        if attempt < MAX_ATTEMPTS {
            let delay = BASE_DELAY * 2u32.saturating_pow(attempt - 1);
            tokio::time::sleep(delay.min(std::time::Duration::from_secs(5))).await;
        }
    }

    if dev_mode {
        warn!(
            admin_url = %admin_url,
            app_id = %installed_app_id,
            "Failed to mint app auth token after {} attempts (dev mode, continuing without — workers will retry)",
            MAX_ATTEMPTS
        );
    } else {
        error!(
            admin_url = %admin_url,
            app_id = %installed_app_id,
            "Failed to mint app auth token after {} attempts — app pool will start unauthenticated and likely fail to connect",
            MAX_ATTEMPTS
        );
    }
    None
}

/// Discover existing agents by querying each conductor's admin API.
///
/// Called at startup to populate the ConductorRegistry with pre-existing
/// agent→conductor mappings. Without this, agents installed before the
/// registry existed would have no affinity routing, causing CellMissing
/// errors on multi-conductor setups.
///
/// Stores each agent key under both base64-standard and base64-url-safe
/// encodings so the registry lookup matches regardless of which format
/// the JWT agent_pub_key uses.
async fn discover_existing_agents(registry: &ConductorRegistry, conductor_urls: &[String]) {
    use base64::Engine;
    use std::time::Duration;

    info!(
        "Starting agent discovery across {} conductor(s)...",
        conductor_urls.len()
    );

    let mut total_discovered = 0usize;

    for (i, url) in conductor_urls.iter().enumerate() {
        let conductor_id = format!("conductor-{i}");
        let admin_url = derive_admin_url_from_app(url);
        let admin = match tokio::time::timeout(
            Duration::from_secs(10),
            TypedAdminClient::connect(&admin_url),
        )
        .await
        {
            Ok(Ok(client)) => client,
            Ok(Err(e)) => {
                warn!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    error = %e,
                    "Agent discovery: failed to connect to conductor"
                );
                continue;
            }
            Err(_) => {
                warn!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    "Agent discovery: connection timed out"
                );
                continue;
            }
        };

        // Bound the startup `list_apps` probe with the same per-conductor SLA the
        // ZomeCaller uses. A conductor that accepts the admin connection then
        // stalls (the live NXDOMAIN / half-open class) must NOT wedge boot for the
        // full 120s startup-probe budget — it degrades to a skipped conductor.
        // Safe to skip: this fn only runs for conductor_urls.len() > 1, so one
        // conductor's agents being missed leaves the others' routing intact.
        let list_apps_result = match tokio::time::timeout(
            doorway::services::zome_caller::zome_call_timeout(),
            admin.list_apps(),
        )
        .await
        {
            Ok(inner) => inner,
            Err(_) => {
                warn!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    "list_apps timed out at startup for {conductor_id}; skipping"
                );
                continue;
            }
        };

        match list_apps_result {
            Ok(apps) => {
                let mut conductor_agents = 0usize;
                for app in &apps {
                    // Encode in both formats to match any JWT key encoding
                    let key_std =
                        base64::engine::general_purpose::STANDARD.encode(&app.agent_pub_key);
                    let key_url =
                        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&app.agent_pub_key);

                    // Register under base64-standard encoding (provisioner format)
                    if registry.get_conductor_for_agent(&key_std).is_none() {
                        if let Err(e) = registry
                            .register_agent(&key_std, &conductor_id, &app.installed_app_id)
                            .await
                        {
                            warn!("Failed to register discovered agent (std): {}", e);
                        } else {
                            conductor_agents += 1;
                        }
                    }

                    // Register under base64-url encoding (Holochain display format)
                    if key_url != key_std && registry.get_conductor_for_agent(&key_url).is_none() {
                        if let Err(e) = registry
                            .register_agent(&key_url, &conductor_id, &app.installed_app_id)
                            .await
                        {
                            warn!("Failed to register discovered agent (url): {}", e);
                        }
                    }
                }
                total_discovered += conductor_agents;
                info!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    apps = apps.len(),
                    new_agents = conductor_agents,
                    "Agent discovery completed for conductor"
                );
            }
            Err(e) => {
                warn!(
                    conductor = %conductor_id,
                    admin_url = %admin_url,
                    error = %e,
                    "Agent discovery failed for conductor (affinity routing may be degraded)"
                );
            }
        }
    }

    if total_discovered > 0 {
        info!(
            "Agent discovery complete: {} new agent mapping(s) registered",
            total_discovered
        );
    } else {
        info!("Agent discovery complete: no new agents found (registry may already be populated)");
    }
}

/// Derive app WebSocket URL from conductor admin URL
fn derive_app_url(conductor_url: &str, app_port: u16) -> String {
    // If the URL contains "localhost" or an IP, replace the port
    if let Some(host_start) = conductor_url.find("://") {
        let after_scheme = &conductor_url[host_start + 3..];
        if let Some(port_start) = after_scheme.rfind(':') {
            let host = &after_scheme[..port_start];
            return format!("{}://{}:{}", &conductor_url[..host_start], host, app_port);
        }
    }
    // Fallback: just use the default
    format!("ws://localhost:{app_port}")
}

#[cfg(test)]
mod inbound_max_tests {
    use super::*;

    #[test]
    fn inbound_max_floors_and_defaults() {
        std::env::remove_var("DOORWAY_MAX_INFLIGHT");
        assert_eq!(inbound_max(), DEFAULT_MAX_INFLIGHT, "unset => default");
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "0");
        assert_eq!(
            inbound_max(),
            MIN_MAX_INFLIGHT,
            "0 clamped to floor (anti-deadlock)"
        );
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "3");
        assert_eq!(inbound_max(), MIN_MAX_INFLIGHT, "below floor clamps up");
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "512");
        assert_eq!(inbound_max(), 512, "honored above floor");
        std::env::remove_var("DOORWAY_MAX_INFLIGHT");
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // the const relation IS the pinned invariant
    fn inbound_max_read_floors_and_defaults() {
        std::env::remove_var("DOORWAY_MAX_INFLIGHT_READ");
        assert_eq!(
            inbound_max_read(),
            DEFAULT_MAX_INFLIGHT_READ,
            "unset => read default (larger than write pool)"
        );
        assert!(
            DEFAULT_MAX_INFLIGHT_READ > DEFAULT_MAX_INFLIGHT,
            "reads get a larger floor than writes"
        );
        std::env::set_var("DOORWAY_MAX_INFLIGHT_READ", "0");
        assert_eq!(inbound_max_read(), MIN_MAX_INFLIGHT, "0 clamped to floor");
        std::env::set_var("DOORWAY_MAX_INFLIGHT_READ", "1024");
        assert_eq!(inbound_max_read(), 1024, "honored above floor");
        std::env::remove_var("DOORWAY_MAX_INFLIGHT_READ");
    }
}
