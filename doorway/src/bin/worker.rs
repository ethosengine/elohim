//! Doorway Worker - Backend processor for NATS Holochain requests
//!
//! Run this binary alongside each Holochain conductor to process requests
//! from the Doorway gateway.
//!
//! Usage:
//!   doorway-worker --nats-url nats://localhost:4222 --conductor-url ws://localhost:4444
//!
//! Environment variables:
//!   NATS_URL - NATS server URL (default: nats://127.0.0.1:4222)
//!   CONDUCTOR_URL - Holochain conductor admin URL (default: ws://localhost:4444)
//!   WORKER_ID - Unique worker identifier (default: auto-generated UUID)
//!   REQUEST_TIMEOUT_MS - Request timeout in milliseconds (default: 30000)
//!   MAX_CONCURRENT - Maximum concurrent requests (default: 10)

use clap::Parser;
use doorway::worker::processor::{Worker, WorkerConfig};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "doorway-worker")]
#[command(about = "Backend worker for Doorway NATS request processing")]
#[command(version)]
struct Args {
    /// NATS server URL
    #[arg(long, env = "NATS_URL", default_value = "nats://127.0.0.1:4222")]
    nats_url: String,

    /// Holochain conductor admin URL
    #[arg(long, env = "CONDUCTOR_URL", default_value = "ws://localhost:4444")]
    conductor_url: String,

    /// Unique worker ID (auto-generated if not provided)
    #[arg(long, env = "WORKER_ID")]
    worker_id: Option<String>,

    /// Request timeout in milliseconds
    #[arg(long, env = "REQUEST_TIMEOUT_MS", default_value = "30000")]
    request_timeout_ms: u64,

    /// Maximum concurrent requests
    #[arg(long, env = "MAX_CONCURRENT", default_value = "10")]
    max_concurrent: usize,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("info,doorway=debug")
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse arguments
    let args = Args::parse();

    let config = WorkerConfig {
        worker_id: args.worker_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        nats_url: args.nats_url,
        conductor_url: args.conductor_url,
        request_timeout_ms: args.request_timeout_ms,
        max_concurrent: args.max_concurrent,
    };

    info!(
        "Starting Doorway worker {} (NATS: {}, Conductor: {})",
        config.worker_id, config.nats_url, config.conductor_url
    );

    // Create and run the worker
    match Worker::new(config).await {
        Ok(worker) => {
            // Handle shutdown signals
            let worker_handle = tokio::spawn(async move {
                if let Err(e) = worker.run().await {
                    error!("Worker error: {}", e);
                }
            });

            // Wait for shutdown signal
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                }
                result = worker_handle => {
                    if let Err(e) = result {
                        error!("Worker task error: {}", e);
                    }
                }
            }

            info!("Worker shutting down");
        }
        Err(e) => {
            error!("Failed to create worker: {}", e);
            std::process::exit(1);
        }
    }
}
