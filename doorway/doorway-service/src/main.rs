//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8

use clap::Parser;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use doorway::{
    conductor::{
        ConductorInfo, ConductorPoolMap, ConductorRegistry, ConductorRouter, TypedAdminClient,
    },
    config::Args,
    db::MongoClient,
    nats::NatsClient,
    orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorState},
    projection::{
        spawn_engine_task, spawn_subscriber, EngineConfig, ProjectionEngine, ProjectionSignal,
        SubscriberConfig,
    },
    server,
    services::{
        self, register_local_storage, spawn_discovery_task_with_signal, DiscoveryConfig,
        StorageRegistrationConfig,
    },
    worker::{PoolConfig, WorkerPool},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file if present
    let _ = dotenvy::dotenv();

    // Parse command line arguments
    let args = Args::parse();

    // Initialize tracing/logging
    let log_level = args.log_level.clone();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("doorway={log_level},info").into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
    let import_client = services::ImportClient::with_defaults(import_app_url.clone());
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
            capacity_max: 50,
        });
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
        let pool_map = ConductorPoolMap::new(Arc::clone(default_pool));

        let mut pools_created = 0usize;
        for (i, url) in conductor_urls.iter().enumerate() {
            let conductor_id = format!("conductor-{i}");
            // Use URL as-is from CONDUCTOR_URLS — it already contains the correct port.
            // derive_app_url would replace the port with app_port_min (4445), which breaks
            // headless k8s services where the socat proxy listens on a different port (e.g. 8445).
            let app_url = url.clone();
            // Mint a token from this conductor's admin interface — the app interface
            // will close any unauthenticated connection.
            let admin_url_for_pool = derive_admin_url_from_app(&app_url);
            let pool_auth_token =
                mint_app_auth_token(&admin_url_for_pool, &args.installed_app_id, args.dev_mode)
                    .await;
            match WorkerPool::new(PoolConfig {
                worker_count: 2, // Per-conductor pools are smaller than the main pool
                conductor_url: app_url.clone(),
                request_timeout_ms: args.request_timeout_ms,
                max_queue_size: 500,
                auth_token: pool_auth_token,
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
        let (_, verifying_key) = doorway::custodial_keys::crypto::generate_keypair();
        state.node_verifying_key = Some(verifying_key);
        info!("Node signing key generated for federation");
    }

    // Create ZomeCaller for federation + service registration
    {
        let admin_url = args.admin_url().to_string();
        let app_url = derive_app_url(&args.conductor_url, args.app_port_min);
        let zome_caller = services::ZomeCaller::new(&admin_url, &app_url, &args.installed_app_id);
        state.zome_caller = Some(Arc::new(zome_caller));
        info!(
            "ZomeCaller created for federation (admin: {}, app: {})",
            admin_url, app_url
        );
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
                            let health = doorway::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: status["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: status["peerId"].as_str().map(|s| s.to_string()),
                            };
                            *p2p_health.write().await = Some(health);
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
    // Non-fatal: if a storage peer is not yet up, its routes are simply unavailable until
    // the operator restarts doorway or peers register manually.
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
    let mut failed = 0usize;
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
                    "Failed to register steward peer — its routes unavailable"
                );
                failed += 1;
            }
        }
    }
    if !peer_urls.is_empty() {
        tracing::info!(registered, failed, "Steward peer registration complete");
    }

    // Create WarmupState before Arc::new(state) so it can be stored in AppState
    // and also passed to spawn_stream_task later.
    if args.projection_writer && !peer_urls.is_empty() && state.projection.is_some() {
        state.warmup_state = Some(Arc::new(
            doorway::projection::warm_stream::WarmupState::new(),
        ));
    }

    // Create discovery readiness channel before Arc::new(state)
    let (discovery_tx, discovery_rx) = tokio::sync::watch::channel(false);
    state.discovery_ready = discovery_rx;

    let state = Arc::new(state);

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
    // In dev mode, the signal subscriber is always disabled (app interface requires auth).
    let _projection_handle = if let Some(ref projection_store) = state.projection {
        if args.dev_mode || !args.projection_writer {
            if !args.projection_writer {
                info!("Projection reader: using shared MongoDB (PROJECTION_WRITER=false)");
            } else {
                info!("Projection engine started (dev mode: signal subscriber disabled, app interface requires auth)");
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
                                // Note: record_reconnect() is available but not wired here
                                // because there is no reconnect loop yet. When subscriber
                                // reconnection is implemented, it should call
                                // peer_health.record_reconnect() on each attempt.
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

        services::federation::spawn_peer_discovery_task(
            peer_url_list,
            self_id,
            cache,
            std::time::Duration::from_secs(10), // initial delay (let peers boot)
            std::time::Duration::from_secs(60), // refresh interval
        );
        info!(
            "Federation peer discovery started: {} peer(s) configured",
            peer_count
        );
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

    // Run the server
    if let Err(e) = server::run(state).await {
        error!("Server error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Derive admin WebSocket URL from app URL by replacing the port.
/// Delegates to the library implementation so there is a single source of truth.
fn derive_admin_url_from_app(app_url: &str) -> String {
    doorway::derive_admin_url_from_app(app_url)
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

        match admin.list_apps().await {
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
