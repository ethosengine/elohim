//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8

use clap::Parser;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use doorway::{
    config::Args,
    db::MongoClient,
    nats::NatsClient,
    orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorState},
    projection::{EngineConfig, ProjectionEngine, SubscriberConfig, spawn_engine_task, spawn_subscriber},
    server,
    services::{self, DiscoveryConfig, StorageRegistrationConfig, register_local_storage, spawn_discovery_task},
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
                .unwrap_or_else(|_| format!("doorway={},info", log_level).into()),
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
    info!("Mode: {}", if args.dev_mode { "DEVELOPMENT" } else { "PRODUCTION" });
    let startup_app_url = derive_app_url(&args.conductor_url, args.app_port_min);
    info!("Conductor admin: {} (discovery, list_apps)", args.admin_url());
    info!("Conductor app: {} (zome calls)", startup_app_url);
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
                warn!("MongoDB connection failed (dev mode, continuing without): {}", e);
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
                warn!("NATS connection failed (dev mode, continuing without): {}", e);
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
                warn!("App worker pool failed to start (dev mode, using direct proxy): {}", e);
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
                warn!("Admin worker pool failed to start (dev mode, using direct proxy): {}", e);
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

    let state = Arc::new(state);

    // Start zome capability discovery (import configs, cache rules)
    // This populates zome_configs and import_config_store for route matching
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

    // Start Projection Engine (if projection store is available)
    // Note: In dev mode, the signal subscriber is disabled because:
    // 1. Holochain 0.3+ requires app interface authentication (IssueAppAuthenticationToken)
    // 2. The projection store is memory-only without MongoDB anyway
    // Production mode uses app_auth module for proper IssueAppAuthenticationToken flow
    let _projection_handle = if let Some(ref projection_store) = state.projection {
        if args.dev_mode {
            // In dev mode, create engine without signal subscriber
            // The projection store can still be queried, just won't receive real-time signals
            info!("Projection engine started (dev mode: signal subscriber disabled, app interface requires auth)");

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
            // Production mode: require proper app interface authentication
            // Use conductor URL for admin interface, derive app URL from port
            let app_url = derive_app_url(&args.conductor_url, args.app_port_min);

            info!(
                "Starting projection engine (admin: {}, app: {})",
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

            info!("Projection engine started");
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
        let result = register_local_storage(
            &storage_config,
            &app_url,
            &args.installed_app_id,
        )
        .await;

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

    // Run the server
    if let Err(e) = server::run(state).await {
        error!("Server error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
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
    format!("ws://localhost:{}", app_port)
}
