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
    server,
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
    info!("Conductor: {}", args.conductor_url);
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

    // Create worker pool for scalable request routing
    // This provides connection pooling and request queuing without needing NATS
    let pool = match WorkerPool::new(PoolConfig {
        worker_count: args.worker_count,
        conductor_url: args.conductor_url.clone(),
        request_timeout_ms: args.request_timeout_ms,
        max_queue_size: 1000,
    })
    .await
    {
        Ok(p) => {
            info!(
                "Worker pool started with {} workers (conductor: {})",
                args.worker_count, args.conductor_url
            );
            Some(Arc::new(p))
        }
        Err(e) => {
            if args.dev_mode {
                warn!("Worker pool failed to start (dev mode, using direct proxy): {}", e);
                None
            } else {
                error!("Worker pool failed to start: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Create application state
    let state = if let Some(p) = pool {
        Arc::new(server::AppState::with_pool(args, mongo, nats, p))
    } else {
        Arc::new(server::AppState::with_services(args, mongo, nats))
    };

    // Run the server
    if let Err(e) = server::run(state).await {
        error!("Server error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
