//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8

use clap::Parser;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use doorway::{
    conductor::{
        admin_client::AdminClient, ConductorInfo, ConductorPoolMap, ConductorRegistry,
        ConductorRouter,
    },
    config::Args,
    db::MongoClient,
    nats::NatsClient,
    orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorState},
    projection::{
        spawn_engine_task, spawn_subscriber, EngineConfig, ProjectionEngine, SubscriberConfig,
    },
    server,
    services::{
        self, register_local_storage, spawn_discovery_task, DiscoveryConfig,
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

    // APP pool - for zome calls
    let worker_app_url = derive_app_url(&args.conductor_url, args.app_port_min);
    let app_pool = match WorkerPool::new(PoolConfig {
        worker_count: args.worker_count,
        conductor_url: worker_app_url.clone(),
        request_timeout_ms: args.request_timeout_ms,
        max_queue_size: 1000,
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
    let admin_url = args.admin_url().to_string();
    let admin_pool = match WorkerPool::new(PoolConfig {
        worker_count: args.worker_count,
        conductor_url: admin_url.clone(),
        request_timeout_ms: args.request_timeout_ms,
        max_queue_size: 1000,
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
            match WorkerPool::new(PoolConfig {
                worker_count: 2, // Per-conductor pools are smaller than the main pool
                conductor_url: app_url.clone(),
                request_timeout_ms: args.request_timeout_ms,
                max_queue_size: 500,
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
                                peer_count: status["connected_peers"].as_u64().unwrap_or(0)
                                    as usize,
                                peer_id: status["peer_id"].as_str().map(|s| s.to_string()),
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

            let _discovery_handle = spawn_discovery_task(
                discovery_config,
                Arc::clone(&state.zome_configs),
                Arc::clone(import_config_store),
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
            // Production mode + projection_writer=true: start signal subscriber
            let app_url = derive_app_url(&args.conductor_url, args.app_port_min);

            info!(
                "Starting projection engine with signal subscriber (admin: {}, app: {})",
                args.conductor_url, app_url
            );

            // Start signal subscriber with proper authentication
            let subscriber_config = SubscriberConfig {
                admin_url: args.conductor_url.clone(),
                app_url,
                installed_app_id: args.installed_app_id.clone(),
                ..SubscriberConfig::default()
            };
            let (subscriber, subscriber_handle) = spawn_subscriber(subscriber_config);

            // Create and start projection engine
            let engine = Arc::new(ProjectionEngine::new(
                projection_store.clone(),
                EngineConfig::default(),
            ));
            let signal_rx = subscriber.subscribe();
            let engine_handle = spawn_engine_task(engine, signal_rx);

            info!("Projection engine started (writer mode)");
            Some((subscriber_handle, engine_handle))
        }
    } else {
        warn!("Projection engine not started (no projection store)");
        None
    };

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
    if !args.federation_peers.is_empty() {
        let peer_urls = args.federation_peers.clone();
        let self_id = args.doorway_id.clone();
        let cache = state.peer_cache.clone();
        let peer_count = peer_urls.len();

        services::federation::spawn_peer_discovery_task(
            peer_urls,
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

/// Derive admin WebSocket URL from app URL by replacing the port
fn derive_admin_url_from_app(app_url: &str) -> String {
    if let Some(host_start) = app_url.find("://") {
        let after_scheme = &app_url[host_start + 3..];
        if let Some(port_start) = after_scheme.rfind(':') {
            let host = &after_scheme[..port_start];
            let port_str = &after_scheme[port_start + 1..];
            // Admin port = app port - 1 (socat convention: 8444=admin, 8445=app; 4444/4445)
            let admin_port = port_str
                .parse::<u16>()
                .map(|p| p.saturating_sub(1))
                .unwrap_or(4444);
            return format!("{}://{}:{}", &app_url[..host_start], host, admin_port);
        }
    }
    "ws://localhost:4444".to_string()
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
        let admin = AdminClient::new(admin_url.clone()).with_timeout(Duration::from_secs(10));

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
