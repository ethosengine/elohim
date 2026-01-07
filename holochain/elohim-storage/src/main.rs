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
use elohim_storage::{BlobStore, Config, ContentDb, HttpServer, ImportHandler, ImportHandlerConfig};
use elohim_storage::{ProgressHub, ProgressHubConfig};
use elohim_storage::import_api::{ImportApi, ImportApiConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "p2p")]
use elohim_storage::p2p::{P2PConfig, P2PNode};
#[cfg(feature = "p2p")]
use elohim_storage::identity::NodeIdentity;

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

    /// Disable import handler (HTTP only mode)
    #[arg(long)]
    no_import: bool,

    /// Enable HTTP Import API (for doorway forwarding)
    /// When enabled, exposes /import/* endpoints for batch imports
    #[arg(long, env = "ENABLE_IMPORT_API")]
    enable_import_api: bool,

    /// Enable SQLite content database
    /// When enabled, exposes /db/* endpoints for content and paths
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

async fn async_main(import_runtime: tokio::runtime::Handle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    // Initialize blob store
    let blob_store = Arc::new(BlobStore::new(config.blobs_dir()).await?);

    // Create progress hub for WebSocket streaming
    let progress_hub = Arc::new(ProgressHub::new(ProgressHubConfig::default()));
    info!("Progress hub initialized for WebSocket streaming");

    // Initialize P2P node if enabled
    #[cfg(feature = "p2p")]
    let p2p_node = if args.enable_p2p {
        let agent_pubkey = args.agent_pubkey.clone().unwrap_or_else(|| {
            // Generate a placeholder agent key if none provided
            format!("uhCAk_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..32].to_string())
        });

        // Load or create P2P identity
        let identity_path = config.storage_dir.join("identity.key");
        let identity = NodeIdentity::load_or_generate(&identity_path, agent_pubkey)?;

        info!(peer_id = %identity.peer_id(), "P2P identity loaded");

        // Configure P2P
        let p2p_config = P2PConfig {
            listen_addresses: if args.p2p_port == 0 {
                vec!["/ip4/0.0.0.0/tcp/0".to_string()]
            } else {
                vec![format!("/ip4/0.0.0.0/tcp/{}", args.p2p_port)]
            },
            enable_mdns: true,
            ..Default::default()
        };

        // Create P2P node with blob store access
        let p2p_node = P2PNode::new(identity, p2p_config, blob_store.clone()).await?;

        // Start listening
        p2p_node.start().await?;

        info!("P2P networking enabled");
        info!("  Peer ID: {}", p2p_node.peer_id());
        info!("  mDNS discovery: enabled");
        info!("  Shard protocol: /elohim/shard/1.0.0");

        Some(p2p_node)
    } else {
        info!("P2P networking disabled (use --enable-p2p or ENABLE_P2P=true)");
        None
    };

    #[cfg(not(feature = "p2p"))]
    let p2p_node: Option<()> = None;

    // Initialize SQLite content database if enabled
    let content_db: Option<Arc<ContentDb>> = if args.enable_content_db {
        info!("SQLite content database enabled");
        match ContentDb::open(&config.storage_dir) {
            Ok(db) => {
                let db = Arc::new(db);
                if let Ok(stats) = db.stats() {
                    info!("  Content: {} items", stats.content_count);
                    info!("  Paths: {} items", stats.path_count);
                    info!("  Steps: {} items", stats.step_count);
                    info!("  Tags: {} unique", stats.unique_tags);
                }
                Some(db)
            }
            Err(e) => {
                error!("Failed to open content database: {}", e);
                None
            }
        }
    } else {
        info!("SQLite content database disabled (use --enable-content-db or ENABLE_CONTENT_DB=true)");
        None
    };

    // Start HTTP server for shard API
    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.http_port).parse()?;
    let mut http_server = HttpServer::new(blob_store.clone(), http_addr)
        .with_progress_hub(Arc::clone(&progress_hub));

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
        info!("  Chunk size: {} items (min: {})", args.import_chunk_size, args.import_min_chunk_size);
        info!("  Chunk delay: {}ms", args.import_chunk_delay_ms);
        info!("  Slow threshold: {}ms (triggers chunk reduction)", args.import_slow_threshold_ms);
        info!("  Import processing on dedicated runtime (4 workers)");

        // HcClient handles cell discovery and signing internally
        // No need for manual cell_id discovery - it happens on connect
        let mut import_api = ImportApi::new(
            ImportApiConfig {
                admin_url: args.admin_url.clone().unwrap_or_else(|| "ws://localhost:4444".to_string()),
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
                warn!("  ⚠️ Conductor connection failed: {} (imports will queue locally)", e);
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

    // Attach ContentDb to HttpServer if enabled
    if let Some(ref db) = content_db {
        http_server = http_server.with_content_db(Arc::clone(db));
        info!("Database API:");
        info!("  GET  /db/stats           - Database statistics");
        info!("  GET  /db/content         - List content");
        info!("  GET  /db/content/{{id}}    - Get content by ID");
        info!("  POST /db/content         - Create content");
        info!("  POST /db/content/bulk    - Bulk create content");
        info!("  GET  /db/paths           - List paths");
        info!("  GET  /db/paths/{{id}}      - Get path with steps");
        info!("  POST /db/paths           - Create path");
        info!("  POST /db/paths/bulk      - Bulk create paths");
    }

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

            // Create shutdown channel
            let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
            import_handler.set_shutdown(shutdown_rx);

            info!("Import handler enabled");
            info!("  App ID: {}", args.app_id);
            info!("  Zome: {}", args.zome_name);

            let handle = tokio::spawn(async move {
                if let Err(e) = import_handler.run().await {
                    error!(error = %e, "Import handler failed");
                }
            });

            Some((handle, shutdown_tx))
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
    let p2p_shutdown_rx = p2p_node.as_ref().map(|node| node.shutdown_sender().subscribe());

    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutting down...");
    };

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

    // Signal import handler to stop
    if let Some((handle, shutdown_tx)) = import_handle {
        let _ = shutdown_tx.send(());
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
