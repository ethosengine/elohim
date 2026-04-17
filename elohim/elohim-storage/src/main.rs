//! Elohim Storage Daemon
//!
//! Runs alongside Holochain conductor to provide blob storage and import processing.
//!
//! ## Usage
//!
//! ```bash
//! # Start with defaults (HTTP server + import handler)
//! elohim-storage
//!
//! # Start with custom config
//! elohim-storage --config /path/to/config.toml
//!
//! # Start with custom HTTP port
//! elohim-storage --http-port 8091
//!
//! # Start with custom storage directory
//! elohim-storage --storage-dir /data/blobs
//!
//! # Connect to specific conductor
//! elohim-storage --admin-url ws://localhost:4444 --app-id elohim
//! ```
//!
//! ## HTTP API
//!
//! - `GET /health` - Health check
//! - `PUT /shard/{hash}` - Store a shard
//! - `GET /shard/{hash}` - Retrieve a shard
//! - `HEAD /shard/{hash}` - Check if shard exists
//! - `PUT /blob/{hash}` - Store blob (auto-creates manifest)
//! - `GET /blob/{hash}` - Reassemble blob from shards
//! - `GET /manifest/{hash}` - Get shard manifest
//!
//! ## Import Processing
//!
//! Listens for ImportBatchQueued signals and processes batches by:
//! 1. Reading blob from local storage
//! 2. Parsing items JSON
//! 3. Sending chunks to zome via process_import_chunk
//!
//! ## Runtime Isolation
//!
//! Uses dedicated tokio runtimes to prevent import processing from starving HTTP/WebSocket:
//! - **Server runtime (2 workers)**: HTTP/WebSocket server - always responsive for upgrades
//! - **Import runtime (4 workers)**: Zome call processing - can saturate without blocking server

use clap::Parser;
use elohim_storage::import_api::{ImportApi, ImportApiConfig};
use elohim_storage::{BlobStore, Config, HttpServer, ImportHandler, ImportHandlerConfig};
use elohim_storage::{ProgressHub, ProgressHubConfig, Services};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use elohim_storage::db::init_pool_from_dir;

#[cfg(feature = "p2p")]
use elohim_storage::db::local_sessions;
#[cfg(feature = "p2p")]
use elohim_storage::identity::NodeIdentity;
#[cfg(feature = "p2p")]
use elohim_storage::p2p::{P2PConfig, P2PNode, RelayMode};

#[derive(Parser, Debug)]
#[command(name = "elohim-storage")]
#[command(about = "Blob storage sidecar for Elohim nodes")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Storage directory
    #[arg(long)]
    storage_dir: Option<PathBuf>,

    /// HTTP API port for shard storage
    #[arg(long)]
    http_port: Option<u16>,

    /// Holochain conductor admin WebSocket URL (for signal-based import handler)
    #[arg(long, env = "HOLOCHAIN_ADMIN_URL")]
    admin_url: Option<String>,

    /// Holochain conductor app WebSocket URL (for ImportApi HTTP handler)
    /// Used by doorway to forward import requests
    #[arg(long, env = "HOLOCHAIN_APP_URL", default_value = "ws://localhost:4445")]
    app_url: String,

    /// Installed Holochain app ID
    #[arg(long, env = "HOLOCHAIN_APP_ID", default_value = "elohim")]
    app_id: String,

    /// Zome name for import calls
    #[arg(long, default_value = "content_store")]
    zome_name: String,

    // --- Embedded conductor mode ---
    /// Enable embedded conductor mode. When set, elohim-storage spawns and
    /// manages the holochain conductor as a child process instead of
    /// connecting to an external conductor.
    #[arg(long, env = "EMBEDDED_CONDUCTOR")]
    embedded_conductor: bool,

    /// Path to the holochain conductor binary.
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_BINARY", default_value = "holochain")]
    conductor_binary: PathBuf,

    /// Path to the conductor configuration YAML file.
    /// Only used when --embedded-conductor is set.
    #[arg(
        long,
        env = "CONDUCTOR_CONFIG_PATH",
        default_value = "/etc/holochain/conductor-config.yaml"
    )]
    conductor_config_path: PathBuf,

    /// Conductor data root directory (lair keystore, chain data).
    /// Only used when --embedded-conductor is set.
    #[arg(
        long,
        env = "CONDUCTOR_DATA_DIR",
        default_value = "/var/local/lib/holochain"
    )]
    conductor_data_dir: PathBuf,

    /// Path to the hApp bundle file.
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "HAPP_PATH", default_value = "/opt/holochain/elohim.happ")]
    happ_path: PathBuf,

    /// Maximum retries waiting for conductor readiness (2s between retries).
    /// Only used when --embedded-conductor is set.
    #[arg(long, env = "CONDUCTOR_MAX_RETRIES", default_value_t = 60)]
    conductor_max_retries: u32,

    /// Disable import handler (HTTP only mode)
    #[arg(long)]
    no_import: bool,

    /// Enable HTTP Import API (for doorway forwarding)
    /// When enabled, exposes /import/* endpoints for batch imports
    #[arg(long, env = "ENABLE_IMPORT_API")]
    enable_import_api: bool,

    /// Enable SQLite content database
    /// When enabled, exposes /db/* endpoints for content
    #[arg(long, env = "ENABLE_CONTENT_DB")]
    enable_content_db: bool,

    /// Import chunk size (items per chunk)
    #[arg(long, env = "IMPORT_CHUNK_SIZE", default_value = "30")]
    import_chunk_size: usize,

    /// Delay between import chunks in milliseconds (conductor breathing room)
    #[arg(long, env = "IMPORT_CHUNK_DELAY_MS", default_value = "300")]
    import_chunk_delay_ms: u64,

    /// Minimum chunk size (floor for adaptive reduction)
    #[arg(long, env = "IMPORT_MIN_CHUNK_SIZE", default_value = "10")]
    import_min_chunk_size: usize,

    /// Response time threshold (ms) to trigger chunk reduction
    #[arg(long, env = "IMPORT_SLOW_THRESHOLD_MS", default_value = "30000")]
    import_slow_threshold_ms: u64,

    // P2P options
    /// Enable P2P networking for shard transfer
    #[arg(long, env = "ENABLE_P2P")]
    #[cfg(feature = "p2p")]
    enable_p2p: bool,

    /// P2P listen port (0 for random)
    #[arg(long, env = "P2P_PORT", default_value = "0")]
    #[cfg(feature = "p2p")]
    p2p_port: u16,

    /// Agent public key for P2P identity (required for P2P)
    #[arg(long, env = "AGENT_PUBKEY")]
    #[cfg(feature = "p2p")]
    agent_pubkey: Option<String>,

    /// P2P bootstrap nodes (multiaddr format)
    /// Can be specified multiple times or comma-separated
    /// Format: /ip4/1.2.3.4/tcp/9876/p2p/12D3KooW...
    #[arg(long, env = "P2P_BOOTSTRAP_NODES", value_delimiter = ',')]
    #[cfg(feature = "p2p")]
    bootstrap_nodes: Vec<String>,

    /// Disable mDNS local network discovery
    #[arg(long, env = "DISABLE_MDNS")]
    #[cfg(feature = "p2p")]
    disable_mdns: bool,

    /// Load P2P bootstrap URL from active session in database
    /// Useful for Tauri apps where bootstrap URL comes from doorway handoff
    #[arg(long, env = "P2P_BOOTSTRAP_FROM_SESSION")]
    #[cfg(feature = "p2p")]
    bootstrap_from_session: bool,

    /// Relay mode: client (desktop steward), server (K8s pod), both (doorway host)
    /// Client: connect through relay servers for NAT traversal
    /// Server: accept relay reservations from NAT-ed peers
    /// Both: relay client + server
    #[arg(long, env = "RELAY_MODE", default_value = "client")]
    #[cfg(feature = "p2p")]
    relay_mode: String,

    /// Addresses to announce to the network (multiaddr format)
    /// Used by K8s pods to advertise stable DNS/IP addresses
    /// Format: /dns4/edgenode-0.headless.svc/tcp/9876
    #[arg(long, env = "ANNOUNCE_ADDRS", value_delimiter = ',')]
    #[cfg(feature = "p2p")]
    announce_addrs: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing BEFORE creating runtimes
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("elohim_storage=info".parse()?),
        )
        .init();

    // Create dedicated server runtime - small, always responsive for HTTP/WebSocket
    let server_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("http-server")
        .enable_all()
        .build()
        .expect("Failed to create server runtime");

    // Create dedicated import runtime - larger, for heavy zome call processing
    let import_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("import-worker")
        .enable_all()
        .build()
        .expect("Failed to create import runtime");

    // Get handle to import runtime for spawning import tasks
    let import_handle = import_rt.handle().clone();

    info!(
        server_workers = 2,
        import_workers = 4,
        "Runtime isolation enabled: HTTP/WebSocket on server runtime, imports on dedicated runtime"
    );

    // Run the main async logic on server runtime
    server_rt.block_on(async_main(import_handle))
}

async fn async_main(
    import_runtime: tokio::runtime::Handle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    // Load config
    let mut config = if let Some(config_path) = &args.config {
        Config::load(config_path)?
    } else {
        Config::default()
    };

    // Apply CLI overrides
    if let Some(dir) = args.storage_dir {
        config.storage_dir = dir;
    }
    if let Some(port) = args.http_port {
        config.http_port = port;
    }

    info!(
        storage_dir = %config.storage_dir.display(),
        http_port = config.http_port,
        "Starting elohim-storage"
    );

    // Ensure storage directory exists
    tokio::fs::create_dir_all(&config.storage_dir).await?;

    // Save default config if it doesn't exist
    let config_path = config.config_path();
    if !config_path.exists() {
        config.save(&config_path)?;
        info!(path = %config_path.display(), "Created default config");
    }

    // --- Embedded conductor mode ---
    // When enabled, spawn the holochain conductor as a child process and
    // install the hApp before starting storage services. The manager is held
    // in scope as a lifecycle anchor — Drop kills the conductor on exit.
    let _conductor_manager = if args.embedded_conductor {
        use elohim_storage::conductor::ConductorManager;
        use elohim_storage::happ_manager;

        info!("Embedded conductor mode enabled");

        let mut manager = ConductorManager::new(
            args.conductor_binary.clone(),
            args.conductor_config_path.clone(),
            args.conductor_data_dir.clone(),
            4444, // admin port — must match conductor config
        );

        // Start conductor as child process
        manager.start()?;

        // Wait for conductor readiness
        let admin_ws = manager.wait_for_ready(args.conductor_max_retries).await?;

        // Install/validate hApp
        happ_manager::ensure_happ_installed(&admin_ws, &args.happ_path, &args.app_id).await?;

        info!("Embedded conductor ready, hApp installed");
        Some(manager)
    } else {
        None
    };

    // Initialize blob store
    let blob_store = Arc::new(BlobStore::new(config.blobs_dir()).await?);

    // Graceful-shutdown broadcast channel — shared by the import handler,
    // the PeerStatus heartbeat, and any other task that needs to react to
    // ctrl-c. Created here (before the tasks that subscribe) so a single
    // send fans out to every consumer.
    let (shutdown_tx, _shutdown_rx_root) = broadcast::channel::<()>(1);

    // Peer-Stewarded Availability — spawn the PeerStatus heartbeat task.
    //
    // Connects a dedicated HcClient to the `infrastructure` role so we can
    // issue signed `record_peer_status` zome calls on a 60s cadence and
    // publish one final `leaving` snapshot at shutdown. Failure to connect
    // (e.g. embedded conductor not ready in some dev flows) is logged and
    // does NOT abort startup — the node still serves HTTP/blobs.
    //
    // TODO(Task 16): replace the hard-coded path with `cfg.peer_policy_path`
    // once the storage config gains that field.
    let peer_policy_path = std::path::PathBuf::from("./config/peer-policy.toml");
    if let Some(admin_url) = &args.admin_url {
        match elohim_storage::policy::PolicyConfig::load(&peer_policy_path) {
            Ok(policy_cfg) => {
                // Peer-Stewarded Availability — conditionally spawn the
                // conductor forwarder so remote peers can reach this node's
                // internal conductor port. Failure to bind is non-fatal,
                // consistent with the heartbeat startup pattern above: we
                // continue serving HTTP even if peer-status is unavailable.
                if policy_cfg.network.expose_conductor_externally {
                    if let Err(e) = elohim_storage::forwarder::spawn_forwarder(
                        &policy_cfg.network.conductor_external_bind,
                        policy_cfg.network.conductor_internal_port,
                    )
                    .await
                    {
                        warn!("conductor forwarder failed to start: {e}");
                    }
                }

                match elohim_storage::hc_client::HcClient::connect(
                    elohim_storage::hc_client::HcClientConfig {
                        admin_url: admin_url.clone(),
                        app_url: args.app_url.clone(),
                        app_id: args.app_id.clone(),
                        role: Some("infrastructure".to_string()),
                    },
                )
                .await
                {
                    Ok(hc) => {
                        let hc = Arc::new(hc);
                        let agent = hc.cell_id().agent_pubkey().clone();
                        let publisher = elohim_storage::heartbeat::ZomeCallPublisher::new(
                            hc.clone(),
                            agent,
                        );
                        let probe = elohim_storage::heartbeat::DefaultProbe::new(
                            blob_store.clone(),
                            hc.clone(),
                        );
                        let heartbeat = elohim_storage::heartbeat::HeartbeatTask::new(
                            policy_cfg, publisher, probe,
                        );
                        let hb_shutdown = shutdown_tx.subscribe();
                        tokio::spawn(async move {
                            heartbeat.run(hb_shutdown).await;
                        });
                        info!(
                            policy_path = %peer_policy_path.display(),
                            "PeerStatus heartbeat task started (infrastructure role)"
                        );
                    }
                    Err(e) => {
                        warn!(
                            "PeerStatus heartbeat disabled: infrastructure HcClient connect failed: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    policy_path = %peer_policy_path.display(),
                    "PeerStatus heartbeat disabled: policy config load failed: {}",
                    e
                );
            }
        }
    } else {
        info!("PeerStatus heartbeat disabled: no --admin-url / HOLOCHAIN_ADMIN_URL set");
    }

    // Create progress hub for WebSocket streaming
    let progress_hub = Arc::new(ProgressHub::new(ProgressHubConfig::default()));
    info!("Progress hub initialized for WebSocket streaming");

    // Initialize P2P node if enabled
    #[cfg(feature = "p2p")]
    let mut p2p_node = if args.enable_p2p {
        let agent_pubkey = args.agent_pubkey.clone().unwrap_or_else(|| {
            // Generate a placeholder agent key if none provided
            format!(
                "uhCAk_{}",
                &uuid::Uuid::new_v4().to_string().replace("-", "")[..32]
            )
        });

        // Load or create P2P identity
        let identity_path = config.storage_dir.join("identity.key");
        let identity = NodeIdentity::load_or_generate(&identity_path, agent_pubkey)?;

        info!(peer_id = %identity.peer_id(), "P2P identity loaded");

        // Collect bootstrap nodes from CLI args
        let mut bootstrap_nodes = args.bootstrap_nodes.clone();

        // Optionally load bootstrap URL from active session (stored in SQLite)
        if args.bootstrap_from_session {
            // Initialize Diesel pool to read session data
            match init_pool_from_dir(&config.storage_dir) {
                Ok(pool) => {
                    match pool.get() {
                        Ok(mut conn) => {
                            match local_sessions::get_active_session(&mut conn) {
                                Ok(Some(session)) => {
                                    if let Some(bootstrap_url) = session.bootstrap_url {
                                        info!(
                                            "  Loading bootstrap from session: {}",
                                            bootstrap_url
                                        );
                                        // Bootstrap URL from doorway handoff (libp2p multiaddr format)
                                        bootstrap_nodes.push(bootstrap_url);
                                    }
                                }
                                Ok(None) => {
                                    info!("  No active session found for bootstrap");
                                }
                                Err(e) => {
                                    warn!("  Failed to load session for bootstrap: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("  Failed to get DB connection for bootstrap lookup: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("  Failed to initialize DB pool for bootstrap lookup: {}", e);
                }
            }
        }

        // Configure P2P
        let enable_mdns = !args.disable_mdns;
        let relay_mode: RelayMode = args.relay_mode.parse().unwrap_or_else(|e| {
            warn!("Invalid relay mode: {}, defaulting to client", e);
            RelayMode::Client
        });
        let p2p_config = P2PConfig {
            listen_addresses: if args.p2p_port == 0 {
                vec!["/ip4/0.0.0.0/tcp/0".to_string()]
            } else {
                vec![format!("/ip4/0.0.0.0/tcp/{}", args.p2p_port)]
            },
            bootstrap_nodes: bootstrap_nodes.clone(),
            enable_mdns,
            storage_dir: config.storage_dir.clone(),
            relay_mode,
            announce_addresses: args.announce_addrs.clone(),
            ..Default::default()
        };

        // Create P2P node with blob store access
        let mut p2p_node = P2PNode::new(identity, p2p_config, blob_store.clone()).await?;

        // Wire DB pool for EPR Head resolution (if content DB is available)
        if args.enable_content_db {
            if let Ok(pool) = init_pool_from_dir(&config.storage_dir) {
                p2p_node = p2p_node.with_db_pool(pool.clone());
                // Wire policy enforcement for content filtering on P2P path
                let policy_cache = elohim_storage::db::policy_cache::PolicyCache::new(pool);
                let enforcement = Arc::new(
                    elohim_storage::db::policy_cache::PolicyEnforcement::new(policy_cache),
                );
                p2p_node = p2p_node.with_policy_enforcement(enforcement);
                info!("  P2P EPR resolution: DB pool + policy enforcement wired");
            }
        }

        info!("P2P networking enabled");
        info!("  Peer ID: {}", p2p_node.peer_id());
        info!("  Relay mode: {}", relay_mode);
        info!(
            "  mDNS discovery: {}",
            if enable_mdns { "enabled" } else { "disabled" }
        );
        info!(
            "  Bootstrap nodes: {}",
            if bootstrap_nodes.is_empty() {
                "none".to_string()
            } else {
                bootstrap_nodes.len().to_string()
            }
        );
        if !args.announce_addrs.is_empty() {
            info!("  Announce addresses: {}", args.announce_addrs.join(", "));
        }
        info!("  Protocols: /elohim/shard/1.0.0, /elohim/storage-sync/1.0.0, /elohim/epr/1.0.0, /elohim/id/1.0.0");

        Some(p2p_node)
    } else {
        info!("P2P networking disabled (use --enable-p2p or ENABLE_P2P=true)");
        None
    };

    #[cfg(not(feature = "p2p"))]
    let p2p_node: Option<()> = None;

    // Initialize extraction cache if enabled
    let extraction_cache = if config.extraction_cache.enabled {
        let cache_dir = config.extraction_cache_dir();
        tokio::fs::create_dir_all(&cache_dir).await?;

        match elohim_cache_core::extraction::DiskBackend::new(cache_dir.clone()).await {
            Ok(backend) => {
                let mut cache_config = config.extraction_cache.clone();
                cache_config.cache_dir = cache_dir;
                let cache = Arc::new(elohim_cache_core::extraction::ExtractionCache::new(
                    Box::new(backend),
                    cache_config,
                ));
                info!(
                    budget_mb = config.extraction_cache.budget_bytes / (1024 * 1024),
                    ttl_secs = config.extraction_cache.ttl_secs,
                    "Extraction cache enabled"
                );
                Some(cache)
            }
            Err(e) => {
                warn!(
                    "Failed to create extraction cache backend: {} (continuing without cache)",
                    e
                );
                None
            }
        }
    } else {
        info!("Extraction cache disabled");
        None
    };

    // Start HTTP server for shard API
    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.http_port).parse()?;
    let mut http_server =
        HttpServer::new(blob_store.clone(), http_addr).with_progress_hub(Arc::clone(&progress_hub));

    if args.embedded_conductor {
        http_server = http_server.with_embedded_conductor();
    }

    if let Some(ref cache) = extraction_cache {
        http_server = http_server.with_extraction_cache(Arc::clone(cache));
    }

    // Wire extraction cache into P2P node for delivery capability queries
    #[cfg(feature = "p2p")]
    if let (Some(ref mut node), Some(ref cache)) = (&mut p2p_node, &extraction_cache) {
        node.set_extraction_cache(Arc::clone(cache));
        info!("  P2P delivery capability: extraction cache wired");
    }

    info!("HTTP API available at http://{}", http_addr);
    info!("Endpoints:");
    info!("  GET  /health           - Health check");
    info!("  PUT  /shard/{{hash}}     - Store a shard");
    info!("  GET  /shard/{{hash}}     - Retrieve a shard");
    info!("  HEAD /shard/{{hash}}     - Check if shard exists");
    info!("  PUT  /blob/{{hash}}      - Store blob (auto-sharding)");
    info!("  GET  /blob/{{hash}}      - Reassemble blob from shards");
    info!("  GET  /manifest/{{hash}}  - Get shard manifest");

    // Initialize Import API if enabled
    let import_api: Option<Arc<RwLock<ImportApi>>> = if args.enable_import_api {
        info!("Import API enabled");
        info!("  POST /import/queue           - Queue import batch");
        info!("  GET  /import/status/{{batch}} - Get import status");
        info!("  WS   /import/progress        - WebSocket progress stream");
        info!("  Conductor app URL: {}", args.app_url);
        info!(
            "  Chunk size: {} items (min: {})",
            args.import_chunk_size, args.import_min_chunk_size
        );
        info!("  Chunk delay: {}ms", args.import_chunk_delay_ms);
        info!(
            "  Slow threshold: {}ms (triggers chunk reduction)",
            args.import_slow_threshold_ms
        );
        info!("  Import processing on dedicated runtime (4 workers)");

        // HcClient handles cell discovery and signing internally
        // No need for manual cell_id discovery - it happens on connect
        let mut import_api = ImportApi::new(
            ImportApiConfig {
                admin_url: args
                    .admin_url
                    .clone()
                    .unwrap_or_else(|| "ws://localhost:4444".to_string()),
                app_url: args.app_url.clone(),
                app_id: args.app_id.clone(),
                role: Some("lamad".to_string()),
                zome_name: args.zome_name.clone(),
                chunk_size: args.import_chunk_size,
                chunk_delay: std::time::Duration::from_millis(args.import_chunk_delay_ms),
                min_chunk_size: args.import_min_chunk_size,
                slow_response_threshold_ms: args.import_slow_threshold_ms,
                ..Default::default()
            },
            blob_store.clone(),
        )
        .with_progress_hub(Arc::clone(&progress_hub))
        .with_import_runtime(import_runtime.clone());

        // Connect to conductor
        match import_api.connect_conductor().await {
            Ok(_) => {
                info!("  ✅ Conductor connected");
            }
            Err(e) => {
                warn!(
                    "  ⚠️ Conductor connection failed: {} (imports will queue locally)",
                    e
                );
            }
        }

        Some(Arc::new(RwLock::new(import_api)))
    } else {
        info!("Import API disabled (use --enable-import-api or ENABLE_IMPORT_API=true)");
        None
    };

    // Attach ImportApi to HttpServer if enabled
    if let Some(ref api) = import_api {
        http_server = http_server.with_import_api(Arc::clone(api));
    }

    // Initialize and attach Node Registry API
    info!("Initializing Node Registry API connection...");
    match elohim_storage::NodeRegistryApi::connect(
        args.admin_url
            .clone()
            .unwrap_or_else(|| "ws://localhost:4444".to_string()),
        args.app_url.clone(),
        args.app_id.clone(),
    )
    .await
    {
        Ok(api) => {
            info!("  ✅ Node Registry API connected");
            http_server = http_server.with_node_registry_api(Arc::new(api));
        }
        Err(e) => {
            warn!("  ⚠️ Node Registry API connection failed: {}", e);
        }
    }

    // Initialize Diesel connection pool for all database operations
    if args.enable_content_db {
        match init_pool_from_dir(&config.storage_dir) {
            Ok(pool) => {
                // Create services with the pool
                let services = Arc::new(Services::new(pool.clone()));
                http_server = http_server.with_services(services);
                // Wire policy enforcement for content filtering
                let policy_cache = elohim_storage::db::policy_cache::PolicyCache::new(pool.clone());
                let enforcement = Arc::new(
                    elohim_storage::db::policy_cache::PolicyEnforcement::new(policy_cache),
                );
                http_server = http_server.with_policy_enforcement(enforcement);
                http_server = http_server.with_db_pool(pool);

                info!("Database API:");
                info!("  GET  /db/stats           - Database statistics");
                info!("  GET  /db/content         - List content");
                info!("  GET  /db/content/{{id}}    - Get content by ID");
                info!("  POST /db/content         - Create content");
                info!("  POST /db/content/bulk    - Bulk create content");
                info!("Session API:");
                info!("  GET    /session       - Get active session");
                info!("  POST   /session       - Create session");
                info!("  DELETE /session       - Delete session");
                info!("  GET    /session/all   - List all sessions");
            }
            Err(e) => {
                error!(
                    "Failed to initialize database pool: {} (database API disabled)",
                    e
                );
            }
        }
    } else {
        // Even without content DB, initialize pool for session management
        match init_pool_from_dir(&config.storage_dir) {
            Ok(pool) => {
                http_server = http_server.with_db_pool(pool);
                info!("Session API:");
                info!("  GET    /session       - Get active session");
                info!("  POST   /session       - Create session");
                info!("  DELETE /session       - Delete session");
                info!("  GET    /session/all   - List all sessions");
            }
            Err(e) => {
                warn!(
                    "Failed to initialize session pool: {} (session API disabled)",
                    e
                );
            }
        }
        info!("Content database disabled (use --enable-content-db or ENABLE_CONTENT_DB=true)");
    }

    // Wire P2P services into HTTP server
    #[cfg(feature = "p2p")]
    if let Some(ref node) = p2p_node {
        http_server = http_server.with_sync_manager(node.sync_manager().clone());
        http_server = http_server.with_p2p_handle(node.handle());
        info!("P2P node wired to HTTP server — Sync API and /p2p/status active");
    }

    // Load slug index for HTML5 app caching
    http_server.load_slug_index().await;

    let http_server = Arc::new(http_server);

    // Start import handler if enabled
    let import_handle = if !args.no_import {
        if let Some(admin_url) = args.admin_url {
            let import_config = ImportHandlerConfig {
                admin_url,
                installed_app_id: args.app_id.clone(),
                zome_name: args.zome_name.clone(),
                ..ImportHandlerConfig::default()
            };

            let mut import_handler = ImportHandler::new(import_config, blob_store.clone());

            // Subscribe to the shared graceful-shutdown broadcast created
            // earlier in main(). The outer `shutdown_tx.send(())` call in the
            // ctrl-c handler fans out to this receiver and to the heartbeat.
            import_handler.set_shutdown(shutdown_tx.subscribe());

            info!("Import handler enabled");
            info!("  App ID: {}", args.app_id);
            info!("  Zome: {}", args.zome_name);

            let handle = tokio::spawn(async move {
                if let Err(e) = import_handler.run().await {
                    error!(error = %e, "Import handler failed");
                }
            });

            Some(handle)
        } else {
            warn!("Import handler disabled: no --admin-url or HOLOCHAIN_ADMIN_URL set");
            info!("  To enable import processing, set HOLOCHAIN_ADMIN_URL or use --admin-url");
            None
        }
    } else {
        info!("Import handler disabled via --no-import");
        None
    };

    info!("Press Ctrl+C to stop.");

    // Handle shutdown signal
    // Create P2P shutdown channel
    #[cfg(feature = "p2p")]
    let p2p_shutdown_rx = p2p_node
        .as_ref()
        .map(|node| node.shutdown_sender().subscribe());

    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutting down...");
    };

    // Start the P2P node immediately before entering the event loop.
    // This shrinks the window between queuing bootstrap dials and polling
    // the swarm to near-zero. Decoupled from the shutdown-channel guard
    // below so that a missing shutdown channel cannot silently skip start().
    #[cfg(feature = "p2p")]
    if let Some(ref node) = p2p_node {
        if let Err(e) = node.start().await {
            error!(error = %e, "Failed to start P2P node");
            return Err(e.into());
        }
        info!("P2P node started — dials queued, event loop about to poll");
    }

    // Run HTTP server (and optionally P2P) with graceful shutdown
    #[cfg(feature = "p2p")]
    {
        if let (Some(node), Some(shutdown_rx)) = (p2p_node.as_ref(), p2p_shutdown_rx) {
            tokio::select! {
                result = http_server.run() => {
                    if let Err(e) = result {
                        error!(error = %e, "HTTP server error");
                    }
                }
                _ = node.run(shutdown_rx) => {
                    info!("P2P node stopped");
                }
                _ = shutdown => {
                    // Signal P2P to stop
                    if let Some(ref node) = p2p_node {
                        let _ = node.shutdown_sender().send(());
                    }
                }
            }
        } else {
            tokio::select! {
                result = http_server.run() => {
                    if let Err(e) = result {
                        error!(error = %e, "HTTP server error");
                    }
                }
                _ = shutdown => {}
            }
        }
    }

    #[cfg(not(feature = "p2p"))]
    {
        tokio::select! {
            result = http_server.run() => {
                if let Err(e) = result {
                    error!(error = %e, "HTTP server error");
                }
            }
            _ = shutdown => {}
        }
    }

    // Signal all graceful-shutdown subscribers (import handler, PeerStatus
    // heartbeat). A single broadcast fans out to every subscriber so the
    // heartbeat publishes its final `leaving` snapshot in parallel with the
    // import handler draining.
    let _ = shutdown_tx.send(());
    if let Some(handle) = import_handle {
        let _ = handle.await;
    }

    // Print stats before exit
    if let Ok(stats) = blob_store.stats().await {
        info!(
            blobs = stats.total_blobs,
            bytes = stats.total_bytes,
            "Final storage stats"
        );
    }

    Ok(())
}
