//! elohim-node: Always-on infrastructure runtime for the Elohim Protocol
//!
//! This daemon runs on family hardware (plug-and-play nodes in a rack) and provides:
//! - Device-to-node sync (phones, laptops → family node)
//! - Cluster-to-cluster sync (family → family)
//! - Backup and replication based on reach levels
//! - Cluster orchestration via the pod module
//!
//! See README.md and ARCHITECTURE.md for details.

mod config;
mod dashboard;
mod update;
mod network;
mod pod;

mod sync;
mod cluster;
mod storage;
mod p2p;
mod api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{info, error};

use config::Config;
use dashboard::{create_router, DashboardState};
use pod::{Pod, PodConfig};

#[derive(Parser)]
#[command(name = "elohim-node")]
#[command(about = "Always-on infrastructure runtime for the Elohim Protocol")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "elohim-node.toml")]
    config: String,

    /// Data directory
    #[arg(short, long, env = "ELOHIM_DATA_DIR")]
    data_dir: Option<String>,

    /// Node ID (overrides config file)
    #[arg(long, env = "ELOHIM_NODE_ID")]
    node_id: Option<String>,

    /// Cluster name (overrides config file)
    #[arg(long, env = "ELOHIM_CLUSTER_NAME")]
    cluster_name: Option<String>,

    /// Pod subcommand for manual operations
    #[command(subcommand)]
    pod_cmd: Option<pod::cli::PodCommands>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("elohim_node=info".parse()?)
        )
        .init();

    let cli = Cli::parse();

    info!("Starting elohim-node");
    info!("Config file: {}", cli.config);

    // Load or create default config
    let mut config = if std::path::Path::new(&cli.config).exists() {
        let content = std::fs::read_to_string(&cli.config)?;
        toml::from_str(&content)?
    } else {
        info!("Config file not found, using defaults");
        Config::default()
    };

    // Apply CLI overrides
    if let Some(node_id) = cli.node_id {
        config.node.id = node_id;
    }
    if let Some(cluster_name) = cli.cluster_name {
        config.node.cluster_name = cluster_name;
    }
    if let Some(data_dir) = cli.data_dir {
        config.node.data_dir = PathBuf::from(data_dir);
    }

    info!("Node ID: {}", config.node.id);
    info!("Cluster: {}", config.node.cluster_name);
    info!("Data dir: {}", config.node.data_dir.display());

    // Handle pod subcommand if present
    if let Some(pod_cmd) = cli.pod_cmd {
        let pod_config = PodConfig {
            enabled: config.pod.enabled,
            decision_interval_secs: config.pod.decision_interval_secs,
            rules_file: config.pod.rules_file.clone(),
            max_actions_per_hour: config.pod.max_actions_per_hour,
            dry_run: config.pod.dry_run,
        };

        let mut pod = Pod::new(config.node.id.clone(), pod_config);

        let result = pod::cli::execute_command(&mut pod, pod_cmd).await;

        match result {
            Ok(output) => {
                println!("{}", output);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Create dashboard state
    let dashboard_state = Arc::new(RwLock::new(DashboardState::new(config.clone())));

    // Create pod instance
    let pod_config = PodConfig {
        enabled: config.pod.enabled,
        decision_interval_secs: config.pod.decision_interval_secs,
        rules_file: config.pod.rules_file.clone(),
        max_actions_per_hour: config.pod.max_actions_per_hour,
        dry_run: config.pod.dry_run,
    };
    let pod = Arc::new(RwLock::new(Pod::new(config.node.id.clone(), pod_config)));

    // Start pod in background
    if config.pod.enabled {
        let pod_clone = pod.clone();
        tokio::spawn(async move {
            let mut pod = pod_clone.write().await;
            if let Err(e) = pod.start().await {
                tracing::error!(error = %e, "Pod failed to start");
            }
        });
        info!("Pod started in background");
    } else {
        info!("Pod is disabled");
    }

    // --- P2P Layer ---
    // Build libp2p swarm
    let data_dir = &config.node.data_dir;
    match p2p::build_swarm(&config.p2p, data_dir) {
        Ok(swarm) => {
            info!(peer_id = %swarm.local_peer_id(), "P2P swarm built");

            // Create sync engine
            let engine = Arc::new(Mutex::new(
                sync::SyncEngine::new(data_dir)
                    .expect("Failed to initialize sync engine"),
            ));

            // Create sync coordinator
            let coordinator = sync::SyncCoordinator::new(
                engine.clone(),
                config.sync.sync_interval_ms,
            );

            // Channels between swarm event loop and coordinator
            let (swarm_event_tx, swarm_event_rx) = mpsc::channel(256);
            let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel::<sync::coordinator::SwarmCommand>(256);

            // Spawn swarm event loop
            let mut swarm_handle = swarm;
            tokio::spawn(async move {
                // Forward commands to swarm in parallel with event loop
                // We split: the swarm.run() consumes self and pushes events
                // But we also need to process commands. Use a wrapper approach:
                swarm_handle.run(swarm_event_tx).await;
            });

            // Spawn coordinator
            tokio::spawn(async move {
                coordinator.run(swarm_event_rx, swarm_cmd_tx).await;
            });

            info!("P2P sync layer started");
        }
        Err(e) => {
            error!(error = %e, "Failed to build P2P swarm, running without P2P");
        }
    }

    // Create dashboard router with pod
    let app = create_router(dashboard_state);

    // Bind to HTTP port
    let addr = SocketAddr::from(([0, 0, 0, 0], config.api.http_port));
    info!("Dashboard listening on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
