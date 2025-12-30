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

use clap::Parser;
use elohim_storage::{BlobStore, Config, HttpServer, ImportHandler, ImportHandlerConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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

    /// Holochain conductor admin WebSocket URL
    #[arg(long, env = "HOLOCHAIN_ADMIN_URL")]
    admin_url: Option<String>,

    /// Installed Holochain app ID
    #[arg(long, env = "HOLOCHAIN_APP_ID", default_value = "elohim")]
    app_id: String,

    /// Zome name for import calls
    #[arg(long, default_value = "content_store")]
    zome_name: String,

    /// Disable import handler (HTTP only mode)
    #[arg(long)]
    no_import: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("elohim_storage=info".parse()?),
        )
        .init();

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

    // Start HTTP server for shard API
    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.http_port).parse()?;
    let http_server = Arc::new(HttpServer::new(blob_store.clone(), http_addr));

    info!("HTTP API available at http://{}", http_addr);
    info!("Endpoints:");
    info!("  GET  /health           - Health check");
    info!("  PUT  /shard/{{hash}}     - Store a shard");
    info!("  GET  /shard/{{hash}}     - Retrieve a shard");
    info!("  HEAD /shard/{{hash}}     - Check if shard exists");
    info!("  PUT  /blob/{{hash}}      - Store blob (auto-sharding)");
    info!("  GET  /blob/{{hash}}      - Reassemble blob from shards");
    info!("  GET  /manifest/{{hash}}  - Get shard manifest");

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
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutting down...");
    };

    // Run HTTP server with graceful shutdown
    tokio::select! {
        result = http_server.run() => {
            if let Err(e) = result {
                error!(error = %e, "HTTP server error");
            }
        }
        _ = shutdown => {}
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
