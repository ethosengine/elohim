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
use tokio::sync::{broadcast, Mutex as TokioMutex, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use elohim_storage::db::init_pool_from_dir;
use elohim_storage::reconcile::pubkey_timeline::PubkeyTimelineCache;
use elohim_storage::reconcile::{HolochainAppSignalStream, ReconcileController};

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

    /// Storage directory (blobs + content.db). In k8s this MUST point at the
    /// storage-data PVC mount (/data) — the default (dirs::data_local_dir) is
    /// the container overlay, wiped on every pod restart: rows get reseeded
    /// but blob bytes don't, leaving dangling blobHash rows that 404 the
    /// projected EPR apps (2026-06-09 regression class).
    #[arg(long, env = "STORAGE_DIR")]
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

    /// Installed app ID for the imagodei DNA (reconcile controller signal subscription).
    /// Defaults to "imagodei". The controller subscribes to this app's signals to
    /// project key-rotation, revocation, and agent-peer-binding events into SQLite.
    /// Set to empty string to disable reconcile controller startup even if --app-url
    /// is configured.
    #[arg(long, env = "IMAGODEI_APP_ID", default_value = "imagodei")]
    imagodei_app_id: String,

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

    /// Path to peer-stewarded availability policy TOML file
    /// Overrides `peer_policy_path` from the storage config file.
    #[arg(long, env = "ELOHIM_STORAGE_PEER_POLICY_PATH")]
    peer_policy_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing BEFORE creating runtimes.
    // JSON output so Grafana Loki can `| json`-extract message/level/target/fields.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("elohim_storage=info".parse()?),
        )
        .init();

    // Structured commit-bearing startup line — the Jenkins build <-> Loki log join key.
    let build_info = elohim_compute::BuildInfo::new("elohim-storage");
    tracing::info!(
        version = %build_info.version,
        commit = %build_info.commit,
        build_time = %build_info.build_time,
        "elohim-storage starting"
    );

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
    if let Some(path) = args.peer_policy_path.clone() {
        config.peer_policy_path = path;
    }

    // Apply env-var overrides for node-shape self-registration (Task C7).
    // When all four are set, the node publishes its shape at boot.
    if let Ok(v) = std::env::var("DEVICE_ARCHETYPE") {
        config.device_archetype = Some(v);
    }
    if let Ok(v) = std::env::var("HOUSEHOLD_ID") {
        config.household_id = Some(v);
    }
    if let Ok(v) = std::env::var("NODE_ROLE") {
        config.node_role = Some(v);
    }
    if let Ok(v) = std::env::var("REGION") {
        config.region = Some(v);
    }
    if let Ok(v) = std::env::var("INVENTORY_BROADCAST_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.inventory_broadcast_seconds = Some(n);
        }
    }
    if let Ok(v) = std::env::var("CUSTODY_SWEEP_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.custody_sweep_seconds = n;
        }
    }
    if let Ok(v) = std::env::var("PLACEMENT_GRACE_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.placement_grace_seconds = n;
        }
    }
    if let Ok(v) = std::env::var("PLACEMENT_GAP_COOLDOWN_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.placement_gap_cooldown_seconds = n;
        }
    }
    if let Ok(v) = std::env::var("KICK_FETCH_PER_PEER_PER_MINUTE") {
        if let Ok(n) = v.parse::<u32>() {
            config.kick_fetch_per_peer_per_minute = n;
        }
    }
    if let Ok(v) = std::env::var("INVENTORY_FRESHNESS_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.inventory_freshness_seconds = n;
        }
    }
    if let Ok(v) = std::env::var("FETCH_BLOB_TIMEOUT_SECONDS") {
        if let Ok(n) = v.parse::<u64>() {
            config.fetch_blob_timeout_seconds = n;
        }
    }
    if let Ok(v) = std::env::var("FETCH_BLOB_PARALLELISM") {
        if let Ok(n) = v.parse::<usize>() {
            if n > 0 {
                config.fetch_blob_parallelism = n;
            }
        }
    }
    if let Ok(v) = std::env::var("SELF_CID") {
        if !v.is_empty() {
            config.self_cid = Some(v);
        }
    }

    // Iroh parallel-stack toggle (see plan
    // genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md).
    // Default `Libp2p` — existing deployments unaffected. `iroh` selects
    // the parallel iroh-blobs path; mutually exclusive at runtime.
    if let Ok(v) = std::env::var("ELOHIM_TRANSPORT_BACKEND") {
        match v.parse::<elohim_storage::config::TransportBackend>() {
            Ok(backend) => config.transport_backend = backend,
            Err(e) => warn!("ignoring ELOHIM_TRANSPORT_BACKEND: {}", e),
        }
    }

    info!(
        storage_dir = %config.storage_dir.display(),
        http_port = config.http_port,
        device_archetype = ?config.device_archetype,
        household_id = ?config.household_id,
        "Starting elohim-storage"
    );

    // Ensure storage directory exists
    tokio::fs::create_dir_all(&config.storage_dir).await?;

    // Provide-loop / re-anchor observability holder. Created here in the
    // composition root so the boot path (self_cid derive + loop spawn) and the
    // re-anchor backfill can write it, and `/p2p/status` can read it via a clone
    // handed to the P2P node below. Mirrors `ProjectionReconcileState`.
    let provide_loop_state = elohim_storage::services::provide_loop_status::ProvideLoopState::new();

    // ── Transport-identity seam + self_cid derivation (Workstream D) ──────────
    //
    // `self_cid` is the node's own steward identity. It is the JOIN KEY three
    // ways: the custody sweep matches `commitment.provider == self_cid`
    // (reconcile/custody.rs), the provide-loop authors with it as `provider`,
    // and the seeder resolves provider/receiver from `GET /p2p/status .peerId`
    // (genesis/seeder/src/peer-id.ts). All three must agree or the snapshot's
    // joins silently empty (project_resilience_snapshot_humans_junction).
    //
    // Both transports answer "what is my self identity / status peerId?" through
    // the `NodeTransport` seam, so the (already transport-neutral) provide-loop
    // below spawns for WHICHEVER backend is active — no branching, no duplicated
    // transport logic. Before the seam, `self_cid` was derived ONLY in libp2p
    // mode, so iroh mode left it `None` → the loop stayed dormant → the
    // resilience card read all zeros.
    //
    // libp2p: `Libp2pTransport` wraps `NodeIdentity::peer_id_string()` read from
    // the SAME `identity.key` the P2P node loads below. `peer_id` depends only on
    // the keypair (not `agent_pubkey`) and `load_or_generate` is idempotent, so
    // the early read here and the node's later read yield byte-identical ids —
    // the libp2p path is behavior-preserving.
    //
    // iroh: `IrohTransport` wraps `secret.public().to_string()` from the SAME
    // `iroh.key` (via the same `load_or_generate_secret_key`) the iroh boot block
    // loads — idempotent (same file ⇒ same id), and equal to the iroh view-fed
    // service's local agent CID, so the resilience join is consistent.
    let node_transport: Option<Arc<dyn elohim_storage::node_transport::NodeTransport>> = if args
        .enable_p2p
        && config.transport_backend == elohim_storage::config::TransportBackend::Libp2p
    {
        let identity_path = config.storage_dir.join("identity.key");
        // agent_pubkey does not affect peer_id; mirror the P2P node's hint
        // so a first-run generate writes the same file the node will load.
        let agent_pubkey = args.agent_pubkey.clone().unwrap_or_else(|| {
            format!(
                "uhCAk_{}",
                &uuid::Uuid::new_v4().to_string().replace('-', "")[..32]
            )
        });
        match NodeIdentity::load_or_generate(&identity_path, agent_pubkey) {
            Ok(identity) => {
                let peer_id = identity.peer_id_string();
                info!(
                    self_cid = %peer_id,
                    "NodeTransport=libp2p — self_cid is the libp2p peer id (matches \
                     /p2p/status peerId join key)"
                );
                Some(Arc::new(
                    elohim_storage::node_transport::Libp2pTransport::new(peer_id),
                ))
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "libp2p transport identity load failed (identity.key unreadable) — \
                     provide-loop stays dormant; resilience card will read zeros"
                );
                None
            }
        }
    } else if args.enable_p2p
        && config.transport_backend == elohim_storage::config::TransportBackend::Iroh
    {
        #[cfg(feature = "p2p-iroh")]
        {
            let iroh_cfg =
                elohim_storage::p2p_iroh::IrohConfig::from_storage_dir(&config.storage_dir);
            match elohim_storage::p2p_iroh::load_or_generate_secret_key(&iroh_cfg.secret_key_path) {
                Ok(secret) => {
                    let node_id = secret.public().to_string();
                    info!(
                        self_cid = %node_id,
                        "NodeTransport=iroh — self_cid is the iroh NodeId (matches the \
                         iroh view-fed agent CID; provide-loop will spawn)"
                    );
                    Some(Arc::new(
                        elohim_storage::node_transport::IrohTransport::new(node_id),
                    ))
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %iroh_cfg.secret_key_path.display(),
                        "iroh transport identity load failed (iroh.key unreadable) — \
                         provide-loop stays dormant; resilience card will read zeros"
                    );
                    None
                }
            }
        }
        #[cfg(not(feature = "p2p-iroh"))]
        {
            warn!(
                "transport_backend=iroh but the p2p-iroh feature is not compiled in — \
                     no transport identity; provide-loop stays dormant"
            );
            None
        }
    } else {
        None
    };

    // Env arm stays highest priority; the transport-derived value only fills a
    // gap when SELF_CID is unset (the dark-card root cause: set in no manifest).
    let self_cid_source = if config.self_cid.is_some() {
        elohim_storage::services::provide_loop_status::SelfCidSource::Env
    } else if let Some(cid) = node_transport.as_ref().and_then(|t| t.self_cid()) {
        config.self_cid = Some(cid);
        elohim_storage::services::provide_loop_status::SelfCidSource::DerivedFromTransport
    } else {
        elohim_storage::services::provide_loop_status::SelfCidSource::Unset
    };
    provide_loop_state
        .set_self_cid_source(self_cid_source)
        .await;

    // Save default config if it doesn't exist
    let config_path = config.config_path();
    if !config_path.exists() {
        config.save(&config_path)?;
        info!(path = %config_path.display(), "Created default config");
    }

    // Initialize the process-wide Diesel pool up front so every consumer
    // (HTTP /db/*, /session, P2P EPR resolution, PeerStatus signal
    // projection, bootstrap-from-session lookup) shares one pool. Previously
    // the pool was created in up to three separate sites, which multiplied
    // SQLite file handles and made /db/stats blind to subscriber activity.
    // Un-gated from --enable-content-db: the pool is a process-wide fixture
    // and the marginal cost of an extra SQLite open is trivial; the content
    // DB flag now only gates HTTP route registration below, not pool init.
    let db_pool = match init_pool_from_dir(&config.storage_dir) {
        Ok(pool) => Some(pool),
        Err(e) => {
            warn!(
                "Failed to initialize database pool: {} (database + session APIs disabled)",
                e
            );
            None
        }
    };

    // Task C7: self-register this node's durable shape from DEVICE_ARCHETYPE
    // + HOUSEHOLD_ID + NODE_ROLE + REGION env vars. No-op when any is unset.
    if let Some(pool) = db_pool.as_ref() {
        // Use the config storage_dir as an agent-pubkey hint until the
        // conductor connection provides the real pubkey (later path).
        let agent_hint = format!("{}-boot", config.storage_dir.display());
        if let Err(e) = elohim_storage::services::boot_registration::register_at_boot(
            pool,
            &config,
            &agent_hint,
        ) {
            warn!(error = %e, "node-shape self-registration failed (non-fatal)");
        }
    }

    // T20: Bootstrap manifest seeding — seed standing-policy and tending-policy
    // manifests on first run. Idempotent: subsequent runs skip kinds already
    // present (peer-authored or previously bootstrapped). Fail-fast: these are
    // foundational protocol manifests; a cold-start system without them cannot
    // apply standing-policy debit weights correctly.
    if let Some(pool) = db_pool.as_ref() {
        match pool.get() {
            Ok(mut conn) => {
                let report =
                    elohim_storage::services::bootstrap_manifests::seed_if_empty(&mut conn)
                        .expect("bootstrap manifests seed must succeed at startup");
                if report.standing_policy_seeded || report.tending_policy_seeded {
                    info!(
                        standing_policy = report.standing_policy_seeded,
                        tending_policy = report.tending_policy_seeded,
                        "Bootstrap manifests seeded on first run"
                    );
                } else {
                    info!("Bootstrap manifests already present — seed skipped");
                }
            }
            Err(e) => {
                // Pool available but connection failed — non-fatal for startup
                // (matching the pool-init warning pattern above), but warn loudly
                // since standing-policy will be absent and fan-out will degrade.
                warn!(error = %e, "Failed to acquire DB connection for bootstrap manifest seeding (non-fatal, fan-out degraded)");
            }
        }
    }

    // One-shot household_id backfill is wired below, AFTER the HcClientRegistry
    // is connected (the replayer reads DHT household memberships via the
    // imagodei conductor client, which is not yet available at this point in
    // boot). See the "household_id backfill" block after the registry wiring.

    // --- Embedded conductor mode ---
    // When enabled, spawn the holochain conductor as a child process and
    // install the hApp before starting storage services. The manager is held
    // in scope as a lifecycle anchor — Drop kills the conductor on exit.
    // Step-zero substrate gossip — agent_info publisher and subscriber both
    // need an `Arc<AdminWebsocket>` that outlives the conductor-ready block.
    // Populated inside the embedded-conductor branch below; consumed downstream
    // where p2p_node is constructed and we wire the publisher/subscriber tasks
    // (gated by ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP).
    let mut agent_info_admin_ws: Option<Arc<holochain_client::AdminWebsocket>> = None;

    let conductor_manager = if args.embedded_conductor {
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

        // Capture an Arc<AdminWebsocket> for downstream use by the step-zero
        // substrate agent_info publisher + subscriber (wired below when the
        // feature flag is on). AdminWebsocket is Clone (its internal state is
        // refcounted), so this clone is cheap.
        agent_info_admin_ws = Some(Arc::new(admin_ws.clone()));

        // Shared (Arc<Mutex>) so the authority-arc actuation endpoint can rewrite
        // the conductor-config + restart it. Held in main for the process
        // lifetime (kill_on_drop on shutdown).
        Some(Arc::new(tokio::sync::Mutex::new(manager)))
    } else {
        None
    };

    // Memory-attribution sampler — the leak-vs-cache discriminator (P-ARC §B gate).
    // The fused `elohim-node` cgroup hides whether the OOM climb is the conductor
    // child's heap, the storage parent's heap, or the SQLite kernel page cache. Emit
    // a cgroup memory.stat breakdown (the VERDICT: anon=heap/leak, file=page-cache)
    // plus a per-process RSS split (the ATTRIBUTION: conductor child vs storage
    // parent) every 60s as structured `tracing` lines under target="memory_attribution".
    // Loki is the surface (no Prometheus app-scrape exists for elohim-storage).
    // Plan: genesis/docs/superpowers/plans/2026-06-16-conductor-memory-attribution-instrument-plan.md
    if let Some(cm) = &conductor_manager {
        let cm = Arc::clone(cm);
        let parent_pid = std::process::id();
        tokio::spawn(async move {
            use elohim_storage::services::system_metrics as sm;
            let cpus = sm::cpu_count().unwrap_or(0);
            // The conductor's own default: calculate_default_db_max_readers = max(2*cpus, 8).
            let db_max_readers = (2 * cpus).max(8);
            info!(
                target: "elohim_storage::memory_attribution",
                event = "boot",
                cpu_count = cpus,
                db_max_readers = db_max_readers,
                cgroup_cpu_quota_cores = ?sm::cgroup_cpu_quota_cores(),
                cgroup_mem_limit_bytes = ?sm::container_memory_limit_bytes(),
                "memory-attribution sampler started"
            );
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tick.tick().await;
                if let Some(b) = sm::cgroup_memory_breakdown() {
                    info!(
                        target: "elohim_storage::memory_attribution",
                        scope = "cgroup",
                        anon_bytes = b.anon,
                        file_bytes = b.file,
                        slab_bytes = b.slab,
                        swap_bytes = ?sm::cgroup_swap_current_bytes(),
                        "cgroup memory breakdown (VERDICT: anon=heap/leak, file=page-cache)"
                    );
                }
                // Read the live child pid under a brief lock, then release before sampling.
                let child_pid = { cm.lock().await.child_pid() };
                for (name, pid) in [
                    ("holochain", child_pid),
                    ("elohim-storage", Some(parent_pid)),
                ] {
                    if let Some(pid) = pid {
                        if let Some(r) = sm::proc_rss(pid) {
                            info!(
                                target: "elohim_storage::memory_attribution",
                                scope = "proc",
                                proc = name,
                                pid = pid,
                                rss_anon_bytes = r.rss_anon,
                                rss_file_bytes = r.rss_file,
                                vm_rss_bytes = r.vm_rss,
                                threads = r.threads,
                                "per-process rss split (attribution)"
                            );
                        }
                    }
                }
            }
        });
    }

    // Initialize blob store
    let blob_store = Arc::new(BlobStore::new(config.blobs_dir()).await?);

    // One-shot shard-manifest backfill (no-p2p build). The p2p build runs the
    // distribution-capable variant later, once the p2p handle exists; here we
    // only record manifests so legacy content stops reading as "unmeasured".
    #[cfg(not(feature = "p2p"))]
    {
        let backfill_pool = db_pool.clone();
        let backfill_blob_store = blob_store.clone();
        tokio::spawn(async move {
            if let Some(pool) = backfill_pool {
                let config =
                    elohim_storage::services::shard_manifest_backfill::BackfillConfig::default();
                if let Err(e) = elohim_storage::services::shard_manifest_backfill::run_once(
                    &pool,
                    &backfill_blob_store,
                    "lamad",
                    &config,
                )
                .await
                {
                    warn!(error = %e, "shard_manifest_backfill failed (non-fatal)");
                }
            }
        });
    }

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
    let peer_policy_path = config.peer_policy_path.clone();
    // Populated inside the Ok(policy_cfg) arm below; read by HTTP layer in Task 5.
    // Underscore-prefixed until Task 5 threads it into the HTTP handler state.
    let mut hc_registry_for_http: Option<
        std::sync::Arc<elohim_storage::hc_client_registry::HcClientRegistry>,
    > = None;
    if let Some(admin_url) = &args.admin_url {
        match elohim_storage::policy::PolicyConfig::load(&peer_policy_path) {
            Ok(policy_cfg) => {
                // Peer-Stewarded Availability — conditionally spawn the
                // conductor forwarder so remote peers can reach this node's
                // internal conductor port. Failure to bind is non-fatal,
                // consistent with the heartbeat startup pattern above: we
                // continue serving HTTP even if peer-status is unavailable.
                if policy_cfg.network.expose_conductor_externally {
                    // App WS forwarder — zome calls from peers (and doorway's
                    // TypedAppClient / conductor-normalizer path).
                    if let Err(e) = elohim_storage::forwarder::spawn_forwarder(
                        &policy_cfg.network.conductor_external_bind,
                        policy_cfg.network.conductor_internal_port,
                    )
                    .await
                    {
                        warn!("conductor app forwarder failed to start: {e}");
                    }

                    // Admin WS forwarder — register agents, install hApps,
                    // list cells. Doorway calls this on /auth/register.
                    if let Err(e) = elohim_storage::forwarder::spawn_forwarder(
                        &policy_cfg.network.conductor_admin_external_bind,
                        policy_cfg.network.conductor_admin_internal_port,
                    )
                    .await
                    {
                        warn!("conductor admin forwarder failed to start: {e}");
                    }
                }

                let registry = elohim_storage::hc_client_registry::HcClientRegistry::connect(
                    &elohim_storage::hc_client_registry::HcRegistryInputs {
                        admin_url: admin_url.clone(),
                        app_url: args.app_url.clone(),
                        app_id: args.app_id.clone(),
                    },
                )
                .await;
                let registry = std::sync::Arc::new(registry);

                // Heartbeat path + infrastructure-role signal subscribers.
                //
                // genesis #1122 pattern (same as the lamad heal-sweep below):
                // do NOT gate this wiring on a boot-time client. The registry's
                // bounded boot ramp connects `infrastructure` FIRST — the
                // coldest window after a pod restart, while the conductor's
                // cells are still CellDisabled — so on a slow boot it is the
                // role most likely to be None-stamped (matthew/jessica failed
                // 5/5 on alpha, 2026-06-11; imagodei, attempted next, landed on
                // attempt 4 ten seconds later). Boot-gating meant the
                // PeerStatus heartbeat AND all the signal subscribers wired in
                // this block (Infrastructure→peer_statuses, REA, ElohimContent,
                // Mishpat, CommitmentByState drain) stayed dead for the pod's
                // lifetime: resilience peer counts dark, projections dark.
                // Acquire the bridge INSIDE the task via connect_role_forever
                // instead — a late connection is exactly as good as a boot one.
                {
                    let infra_boot = registry.infrastructure.clone();
                    let late_inputs = elohim_storage::hc_client_registry::HcRegistryInputs {
                        admin_url: admin_url.clone(),
                        app_url: args.app_url.clone(),
                        app_id: args.app_id.clone(),
                    };
                    let shutdown_tx = shutdown_tx.clone();
                    let blob_store = blob_store.clone();
                    let db_pool = db_pool.clone();
                    let peer_policy_path = peer_policy_path.clone();
                    let device_archetype = config.device_archetype.clone();
                    let infra_app_id = args.app_id.clone();
                    tokio::spawn(async move {
                        let hc = match infra_boot {
                            Some(hc) => hc,
                            None => {
                                info!(
                                    "infrastructure bridge not up at boot — awaiting late connect \
                                 (heartbeat + signal subscribers start once it lands)"
                                );
                                match elohim_storage::hc_client_registry::HcClientRegistry::connect_role_forever(
                                &late_inputs,
                                "infrastructure",
                                shutdown_tx.subscribe(),
                            )
                            .await
                            {
                                Some(hc) => {
                                    info!(
                                        "infrastructure bridge connected (late) — wiring \
                                         heartbeat + signal subscribers"
                                    );
                                    hc
                                }
                                None => return, // only None on shutdown
                            }
                            }
                        };
                        let agent = hc.cell_id().agent_pubkey().clone();
                        let publisher =
                            elohim_storage::heartbeat::ZomeCallPublisher::new(hc.clone(), agent);
                        let probe = elohim_storage::heartbeat::DefaultProbe::new(
                            blob_store.clone(),
                            hc.clone(),
                        );
                        let mut heartbeat = elohim_storage::heartbeat::HeartbeatTask::new(
                            policy_cfg, publisher, probe,
                        );
                        // Task C8: pipe DEVICE_ARCHETYPE through to PeerStatus
                        // so consumers (/shefa/devices, /shefa/dashboard) can
                        // correlate peer vitals with hardware archetype.
                        if let Some(archetype) = device_archetype.clone() {
                            heartbeat = heartbeat.with_archetype_class(archetype);
                        }
                        let hb_shutdown = shutdown_tx.subscribe();
                        tokio::spawn(async move {
                            heartbeat.run(hb_shutdown).await;
                        });
                        info!(
                            policy_path = %peer_policy_path.display(),
                            "PeerStatus heartbeat task started (infrastructure role)"
                        );

                        // Peer-Stewarded Availability — subscribe to the conductor's
                        // signal stream and project `InfrastructureSignal::PeerStatusRecorded`
                        // into the SQLite `peer_statuses` table via `signals::handle_signal`.
                        //
                        // Shares the process-wide Diesel pool (created once at startup)
                        // so subscriber activity is visible via /db/stats and we don't
                        // multiply SQLite file handles. Non-fatal: if the pool is
                        // unavailable we log and the node keeps serving HTTP
                        // (consistent with the heartbeat startup precedent).
                        if let Some(subscriber_pool) = db_pool.clone() {
                            let hc_sub = hc.clone();
                            tokio::spawn(async move {
                                let pool = subscriber_pool;
                                let handle_id = hc_sub
                                .subscribe_infrastructure_signals(
                                    move |signal: elohim_storage::signals::InfrastructureSignal| {
                                        match pool.get() {
                                            Ok(mut conn) => {
                                                if let Err(e) =
                                                    elohim_storage::signals::handle_signal(
                                                        &mut conn, signal,
                                                    )
                                                {
                                                    warn!(
                                                        error = %e,
                                                        "InfrastructureSignal projection failed"
                                                    );
                                                }
                                            }
                                            Err(e) => warn!(
                                                error = %e,
                                                "Failed to acquire DB connection for signal projection"
                                            ),
                                        }
                                    },
                                )
                                .await;
                                info!(
                                    subscription_id = %handle_id,
                                    "InfrastructureSignal subscriber registered (projects PeerStatusRecorded → SQLite)"
                                );
                            });
                        } else {
                            warn!(
                            "InfrastructureSignal subscriber disabled: shared DB pool unavailable"
                        );
                        }

                        // 2026-05-26-substrate-rea-replication-fix Task 6.5 — subscribe to
                        // ReaProjectionSignals (REA commitments, agreements, economic events)
                        // and project them into the local SQL via rea_projection::handle_rea_signal.
                        //
                        // Without this subscriber, the conductor-first HTTP write path in
                        // ReaCommitmentService::create_via_conductor would create the DHT entry
                        // but the bounded SQL-projection poll would time out at 1s — visible to
                        // the operator as "REA commitment X written via conductor but projection
                        // did not land". Same shape that affected /lamad on alpha pre-fix.
                        // Slice-2b T11 — CommitmentByState link author drain.
                        //
                        // The graduation projection (`handle_rea_signal`, sync) flips the
                        // SQL `state` cache proposed→active on a provide event; the DHT
                        // TRUTH is a `CommitmentByState` link, authored async via the
                        // mishpat coordinator. `handle_rea_signal` has no HcClient, so it
                        // pushes the transition onto an mpsc sink; THIS task (which holds
                        // `hc`) drains it and authors the link. Wired here, beside the REA
                        // subscriber, because this is the composition where the HcClient is
                        // available. Without it the SQL cache flip stands alone (honest,
                        // but the lifecycle is not yet DHT-observable).
                        {
                            let hc_link = hc.clone();
                            let (link_tx, mut link_rx) = tokio::sync::mpsc::unbounded_channel::<
                                elohim_storage::rea_projection::PendingStateLink,
                            >();
                            elohim_storage::rea_projection::install_state_link_sink(link_tx);
                            tokio::spawn(async move {
                                while let Some(pending) = link_rx.recv().await {
                                    let input = elohim_storage::services::conductor_writes::CreateCommitmentStateLinkInput {
                                    commitment_cid: pending.commitment_cid.clone(),
                                    state: pending.state.clone(),
                                    event_hash: pending.event_hash.clone(),
                                    signed_at: pending.signed_at.clone(),
                                };
                                    if let Err(e) = elohim_storage::services::conductor_writes::call_create_commitment_state_link(
                                    &hc_link, input,
                                )
                                .await
                                {
                                    // Best-effort: the SQL cache flip already stands.
                                    warn!(
                                        error = %e,
                                        cid = %pending.commitment_cid,
                                        state = %pending.state,
                                        "CommitmentByState link author failed (SQL state cache flip stands)"
                                    );
                                }
                                }
                            });
                            info!(
                                "CommitmentByState link-author drain registered (graduation \
                             proposed→active authors a notarized lifecycle link; SQL state \
                             is a write-through cache)"
                            );
                        }

                        if let Some(subscriber_pool) = db_pool.clone() {
                            let hc_sub = hc.clone();
                            let ctx_sub = elohim_storage::db::AppContext::default_lamad();
                            tokio::spawn(async move {
                                let pool = subscriber_pool;
                                let ctx = ctx_sub;
                                let handle_id = hc_sub
                                .subscribe_rea_projection_signals(
                                    move |signal: elohim_storage::rea_projection::ReaProjectionSignal| {
                                        if let Err(e) =
                                            elohim_storage::rea_projection::handle_rea_signal(
                                                signal, &pool, &ctx,
                                            )
                                        {
                                            warn!(
                                                error = %e,
                                                "ReaProjectionSignal projection failed"
                                            );
                                        }
                                    },
                                )
                                .await;
                                info!(
                                    subscription_id = %handle_id,
                                    "ReaProjectionSignal subscriber registered (projects \
                                     ReaCommitmentCommitted + AgreementCommitted + \
                                     ReaEconomicEventCommitted → SQLite with dht_anchor_hash)"
                                );
                            });
                        } else {
                            warn!(
                            "ReaProjectionSignal subscriber disabled: shared DB pool unavailable \
                             — conductor-first HTTP write path will time out at 1s polling for \
                             SQL projection"
                        );
                        }

                        // Recovery M4 — subscribe to ElohimContentSignals (attestation:* and
                        // governance-action:* Content entries) and fan them through the central
                        // elohim_content_dispatcher to both AttestationProjector + RecoveryFlowProjector.
                        if let Some(subscriber_pool) = db_pool.clone() {
                            let hc_sub = hc.clone();
                            tokio::spawn(async move {
                                let pool = subscriber_pool;
                                let handle_id = hc_sub
                                .subscribe_elohim_content_signals(
                                    move |signal: elohim_storage::signals::ElohimContentSignal| {
                                        match pool.get() {
                                            Ok(mut conn) => {
                                                if let Err(e) =
                                                    elohim_storage::services::elohim_content_dispatcher::dispatch(
                                                        &mut conn, &signal,
                                                    )
                                                {
                                                    warn!(
                                                        kind = %signal.content_type,
                                                        error = %e,
                                                        "ElohimContentSignal dispatch failed"
                                                    );
                                                }
                                            }
                                            Err(e) => warn!(
                                                error = %e,
                                                "Failed to acquire DB connection for elohim content dispatch"
                                            ),
                                        }
                                    },
                                )
                                .await;
                                info!(
                                    subscription_id = %handle_id,
                                    "ElohimContentSignal subscriber registered (dispatches to AttestationProjector + RecoveryFlowProjector)"
                                );
                            });
                        } else {
                            warn!(
                            "ElohimContentSignal subscriber disabled: shared DB pool unavailable"
                        );
                        }

                        // Slice-2b code-review fix — subscribe to MishpatSignals
                        // (CommitmentCommitted + gate-decision/challenge variants) and
                        // project them via signals::handle_mishpat_signal, which parses
                        // the commitment payload and upserts into `mishpat_commitments`
                        // with dht_anchor_hash = action_hash (or sets revoked_at for a
                        // revokes-commitment).
                        //
                        // `on_signal` is app-wide: the mishpat zome lives in the mishpat
                        // role cell (a different DNA), but its post-commit signal still
                        // arrives on THIS client's app websocket because the conductor
                        // delivers every Signal::App from any cell in the installed app.
                        //
                        // Without this, live authoring (the provide-loop tick + HTTP
                        // commitment writes) creates the DHT entry but the projection
                        // never lands. Consequence: the reconciler's
                        // `live_commons_provides_for_provider` dedup stays empty →
                        // re-authors every 60s → unbounded commitment proliferation;
                        // and rea graduation never fires. (Pre-existing 2a gap: the
                        // projection was only ever exercised by direct-row test
                        // fixtures, never a live signal subscriber. Wired here.)
                        if let Some(subscriber_pool) = db_pool.clone() {
                            let hc_sub = hc.clone();
                            let mishpat_app_id = infra_app_id.clone();
                            tokio::spawn(async move {
                                let pool = subscriber_pool;
                                let app_id = mishpat_app_id;
                                let handle_id = hc_sub
                                .subscribe_mishpat_signals(
                                    move |signal: elohim_storage::signals::MishpatSignal| {
                                        match pool.get() {
                                            Ok(mut conn) => {
                                                if let Err(e) =
                                                    elohim_storage::signals::handle_mishpat_signal(
                                                        &mut conn, &app_id, signal,
                                                    )
                                                {
                                                    warn!(
                                                        error = %e,
                                                        "MishpatSignal projection failed"
                                                    );
                                                }
                                            }
                                            Err(e) => warn!(
                                                error = %e,
                                                "Failed to acquire DB connection for mishpat signal projection"
                                            ),
                                        }
                                    },
                                )
                                .await;
                                info!(
                                    subscription_id = %handle_id,
                                    "MishpatSignal subscriber registered (projects CommitmentCommitted → mishpat_commitments with dht_anchor_hash)"
                                );
                            });
                        } else {
                            warn!(
                                "MishpatSignal subscriber disabled: shared DB pool unavailable \
                             — live-authored commitments will create DHT entries but never \
                             project to mishpat_commitments (provide-loop dedup stays empty)"
                            );
                        }
                    });
                }

                // ── Slice-2b provide-loop AUTHORING tick ─────────────────────
                //
                // The P1 reconciliation controller's WRITE half: every ~60s,
                // derive the caught-up commons pin set and author the
                // `replicates-commons` Commitment + ProvideAnnounce for any
                // desired key with no live actual row. Logical-key dedup against
                // the live projection is the author-once guarantee (restart-safe).
                //
                // This lives here (not in P2PNode) because the conductor author
                // seam (`registry.lamad`) is only available in this composition
                // scope — P2PNode::run_provide_reconcile keeps calling observe()
                // for latch hydration only. Requires lamad HcClient + db pool +
                // self_cid; absent any, the loop is not spawned.
                //
                // Caught-up proxy: for an `item` pin, local content presence IS
                // the durable, DB-queryable signal that byte arrival completed
                // (`content_ids_present`, reach-agnostic). The provide-eligibility
                // gate is SEPARATE: a reach-aware classifier pass that admits
                // commons openly and non-commons only with embodied responsibility
                // for the scope. The two sets were previously conflated (commons
                // presence doubled as both proxy and gate); they are now distinct.
                match (
                    registry.lamad.clone(),
                    db_pool.clone(),
                    config.self_cid.clone(),
                ) {
                    (Some(lamad_hc), Some(provide_pool), Some(self_cid))
                        if !self_cid.is_empty() =>
                    {
                        let author = std::sync::Arc::new(
                            elohim_storage::services::conductor_commitment_author::ConductorCommitmentAuthor::new(
                                lamad_hc,
                                self_cid.clone(),
                                provide_pool.clone(),
                            ),
                        );
                        let reconciler = std::sync::Arc::new(
                            elohim_storage::services::provide_reconcile::ProvideReconciler::new(),
                        );
                        // Reach-aware provide-eligibility resolver (lamad pillar).
                        // Commons is openly providable; non-commons requires
                        // embodied responsibility for the scope (classify_pre_authorization).
                        let eligibility = std::sync::Arc::new(
                            elohim_storage::services::provide_reconcile::ClassifierEligibility::new(
                                std::sync::Arc::new(provide_pool.clone()),
                                "lamad",
                            ),
                        );
                        let self_cid_for_log = self_cid.clone();
                        let mut provide_shutdown = shutdown_tx.subscribe();
                        tokio::spawn(async move {
                            use tokio::time::{interval, Duration, MissedTickBehavior};
                            let mut ticker = interval(Duration::from_secs(60));
                            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                            let app_ctx = elohim_storage::db::AppContext::default_lamad();
                            loop {
                                tokio::select! {
                                    _ = ticker.tick() => {
                                        // Derive desired set: active `item` pins
                                        // whose content is locally present AND
                                        // commons-reach (the caught-up proxy).
                                        let desired = {
                                            let mut conn = match provide_pool.get() {
                                                Ok(c) => c,
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "provide author tick: db conn failed (retry next tick)");
                                                    continue;
                                                }
                                            };
                                            let pins = match elohim_storage::db::acquisition_pins::list_active_pins(&mut conn) {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "provide author tick: pin load failed (retry next tick)");
                                                    continue;
                                                }
                                            };
                                            use elohim_storage::services::provide_reconcile::ProvideEligibility;
                                            let head_refs: Vec<String> = pins.iter().map(|p| p.head_ref.clone()).collect();
                                            // Caught-up proxy: reach-agnostic local presence.
                                            let present = elohim_storage::db::content_diesel::content_ids_present(
                                                &mut conn, &app_ctx, &head_refs,
                                            )
                                            .unwrap_or_default();
                                            // Eligibility gate: per-content (head_ref, reach)
                                            // for present content, classified reach-aware.
                                            let candidates: Vec<(String, String)> = elohim_storage::db::content_diesel::content_reaches_for_ids(
                                                &mut conn, &app_ctx, &head_refs,
                                            )
                                            .unwrap_or_default();
                                            let eligible = eligibility.eligible_head_refs(&candidates);
                                            // Stage B: content reach per head_ref → threaded onto
                                            // the desired provide so the author declares the
                                            // content's own reach (not a hardcoded commons).
                                            let reach_by_head_ref: std::collections::HashMap<String, String> =
                                                candidates.iter().cloned().collect();
                                            elohim_storage::services::provide_reconcile::ProvideReconciler::derive_desired(
                                                &pins, &present, &eligible, &reach_by_head_ref,
                                            )
                                        };
                                        if desired.is_empty() {
                                            continue;
                                        }
                                        match reconciler.reconcile_provides(&provide_pool, &*author, &self_cid, &desired).await {
                                            Ok(authored) if authored > 0 => {
                                                tracing::info!(
                                                    target: "elohim_storage::provide",
                                                    authored,
                                                    desired = desired.len(),
                                                    "provide author tick: authored new replicates-commons commitments"
                                                );
                                            }
                                            Ok(_) => {}
                                            Err(e) => {
                                                tracing::warn!(error = %e, "provide author tick: reconcile failed (retry next tick)");
                                            }
                                        }
                                    }
                                    _ = provide_shutdown.recv() => {
                                        tracing::debug!("provide author tick: shutdown signal received, exiting");
                                        break;
                                    }
                                }
                            }
                        });
                        info!(
                            self_cid = %self_cid_for_log,
                            "Slice-2b provide-loop authoring tick started (60s interval, shutdown-aware)"
                        );
                        provide_loop_state.set_active(true).await;
                    }
                    _ => {
                        info!(
                            "Slice-2b provide-loop authoring tick disabled: requires lamad HcClient + db pool + non-empty self_cid"
                        );
                        provide_loop_state.set_active(false).await;
                    }
                }

                // ── Re-anchor backfill (Workstream D — cold-seed recovery) ────
                //
                // Heal content rows the cold-conductor seed left provenance-only
                // (`dht_anchor_hash IS NULL`). Spawned here so `registry.lamad`
                // is in scope; acquires the bridge via the late-connect pattern
                // (same as the infrastructure block above) so a slow conductor
                // that enables its cells AFTER boot still heals — instead of the
                // seeder's circuit latching the card dark forever. One-shot per
                // boot: re-authoring is idempotent, so the next boot's sweep
                // picks up anything this one capped or failed.
                {
                    let reanchor_pool = db_pool.clone();
                    let reanchor_state = provide_loop_state.clone();
                    let reanchor_lamad_boot = registry.lamad.clone();
                    let reanchor_late_inputs =
                        elohim_storage::hc_client_registry::HcRegistryInputs {
                            admin_url: admin_url.clone(),
                            app_url: args.app_url.clone(),
                            app_id: args.app_id.clone(),
                        };
                    let reanchor_shutdown = shutdown_tx.subscribe();
                    tokio::spawn(async move {
                        let Some(pool) = reanchor_pool else {
                            info!("reanchor_backfill skipped: db pool unavailable");
                            return;
                        };
                        // Acquire the lamad bridge — late-connect so a cold
                        // conductor (cells CellDisabled at boot) still heals once
                        // its cells enable.
                        let hc = match reanchor_lamad_boot {
                            Some(hc) => hc,
                            None => {
                                info!(
                                    "reanchor_backfill: lamad bridge not up at boot — awaiting late connect"
                                );
                                match elohim_storage::hc_client_registry::HcClientRegistry::connect_role_forever(
                                    &reanchor_late_inputs,
                                    "lamad",
                                    reanchor_shutdown,
                                )
                                .await
                                {
                                    Some(hc) => hc,
                                    None => {
                                        info!("reanchor_backfill: lamad bridge never came up (shutdown) — skipping");
                                        return;
                                    }
                                }
                            }
                        };
                        // A ContentService scoped to lamad to drive the canonical
                        // re-anchor path (update_via_conductor null-anchor branch).
                        // A throwaway EventBus: the only event this path emits is
                        // ContentUpdated (cache invalidation), and the HTTP cache
                        // does not exist yet at this boot phase — re-anchoring is a
                        // projection write, not a user-visible mutation.
                        let content_service = elohim_storage::services::ContentService::new(
                            pool.clone(),
                            elohim_storage::db::AppContext::default_lamad(),
                            std::sync::Arc::new(elohim_storage::services::events::EventBus::new()),
                        );
                        let cfg =
                            elohim_storage::services::reanchor_backfill::ReanchorConfig::default();
                        match elohim_storage::services::reanchor_backfill::run_once(
                            &pool,
                            &content_service,
                            &hc,
                            &reanchor_state,
                            &cfg,
                        )
                        .await
                        {
                            Ok(report) if report.candidates > 0 => {
                                info!(
                                    candidates = report.candidates,
                                    reanchored = report.reanchored,
                                    failed = report.failed,
                                    remaining = report.remaining,
                                    "reanchor_backfill: cold-seed recovery sweep done"
                                );
                            }
                            Ok(_) => {
                                info!(
                                    "reanchor_backfill: no NULL-anchor content (nothing to heal)"
                                );
                            }
                            Err(e) => {
                                warn!(error = %e, "reanchor_backfill failed (non-fatal)");
                            }
                        }
                    });
                }

                // Stash the registry in shared state for HTTP handlers.
                hc_registry_for_http = Some(registry);
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

    // P1 projection-reconcile stream status. Created here so the node can hold
    // a surfacing-only clone (`/p2p/status`) while the reconcile task (spawned
    // after node construction, where the lamad HcClient is in scope) drives it.
    #[cfg(feature = "p2p")]
    let projection_reconcile_state =
        elohim_storage::p2p::projection_reconcile::ProjectionReconcileState::new();

    // Initialize P2P node if enabled.
    // Gated on `transport_backend == Libp2p` so the iroh path can take over
    // the P2P slot when selected (the two stacks are mutually exclusive at
    // runtime — see plan).
    #[cfg(feature = "p2p")]
    let mut p2p_node = if args.enable_p2p
        && config.transport_backend == elohim_storage::config::TransportBackend::Libp2p
    {
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

        // Optionally load bootstrap URL from active session (stored in SQLite).
        // Uses the shared process-wide pool initialized at startup.
        if args.bootstrap_from_session {
            if let Some(ref pool) = db_pool {
                match pool.get() {
                    Ok(mut conn) => match local_sessions::get_active_session(&mut conn) {
                        Ok(Some(session)) => {
                            if let Some(bootstrap_url) = session.bootstrap_url {
                                info!("  Loading bootstrap from session: {}", bootstrap_url);
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
                    },
                    Err(e) => {
                        warn!("  Failed to get DB connection for bootstrap lookup: {}", e);
                    }
                }
            } else {
                warn!("  Bootstrap-from-session disabled: shared DB pool unavailable");
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
            // T22: wire archetype + cadence override from top-level Config so
            // the broadcaster timer in P2PNode::run can resolve cadence.
            device_archetype: config.device_archetype.clone(),
            inventory_broadcast_seconds: config.inventory_broadcast_seconds,
            // T23: wire custody reconcile sweep parameters from top-level
            // Config so the timer + ConnectionEstablished trigger in
            // P2PNode::run_custody_reconcile see the same values used by the
            // HTTP-side race-fetch path (config.rs defaults).
            self_cid: config.self_cid.clone(),
            custody_sweep_seconds: Some(config.custody_sweep_seconds),
            placement_grace_seconds: config.placement_grace_seconds,
            placement_gap_cooldown_seconds: config.placement_gap_cooldown_seconds,
            inventory_freshness_seconds: config.inventory_freshness_seconds,
            fetch_blob_timeout_seconds: config.fetch_blob_timeout_seconds,
            fetch_blob_parallelism: config.fetch_blob_parallelism,
            ..Default::default()
        };

        // Create P2P node with blob store access
        let mut p2p_node = P2PNode::new(identity, p2p_config, blob_store.clone()).await?;

        // Wire DB pool for EPR Head resolution (if content DB is available).
        // Reuses the shared process-wide pool.
        if args.enable_content_db {
            if let Some(pool) = db_pool.clone() {
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

        // Surface the P1 projection-reconcile status on /p2p/status. The sweep
        // itself runs in a task spawned after this block (it needs the lamad
        // HcClient author seam); the node only publishes the shared snapshot.
        p2p_node = p2p_node.with_projection_reconcile_state(projection_reconcile_state.clone());

        // Surface the provide-loop / re-anchor-backfill status on /p2p/status
        // (Workstream D). The holder was written earlier (self_cid derive + loop
        // spawn) and is written by the re-anchor backfill; the node only
        // publishes the shared snapshot.
        p2p_node = p2p_node.with_provide_loop_state(provide_loop_state.clone());

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

        // Step-zero substrate gossip — wire conductor agent_info propagation
        // across the libp2p mesh so each pod's conductor peer cache survives
        // the Phase 1 doorway-A / doorway-B signal partition. Behind
        // ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP so initial rollout can enable
        // matthew + adam first, verify metrics, then expand cluster-wide.
        //
        // Both publisher and subscriber are spawned here (after p2p_node is
        // constructed so the publisher has a command_sender to the swarm).
        // JoinHandles are intentionally let-bound to keep the tasks alive
        // until shutdown_tx fires; tokio tasks survive handle drop, the
        // bindings just keep the handles in scope for the lifetime of main().
        let enable_agent_info_gossip = std::env::var("ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        if enable_agent_info_gossip {
            if let Some(admin_ws_arc) = agent_info_admin_ws.clone() {
                use elohim_storage::p2p::conductor_agent_info_gossip::{
                    spawn_agent_info_publisher, spawn_agent_info_subscriber_worker,
                    ConductorAgentInfo, SubscriberConfig,
                };
                let cfg = SubscriberConfig::from_env();
                let (ai_tx, ai_rx) =
                    tokio::sync::mpsc::channel::<ConductorAgentInfo>(cfg.queue_capacity);
                p2p_node.set_agent_info_inbound_tx(ai_tx);
                let _subscriber_task = spawn_agent_info_subscriber_worker(
                    admin_ws_arc.clone(),
                    ai_rx,
                    cfg,
                    shutdown_tx.subscribe(),
                );
                let _publisher_task = spawn_agent_info_publisher(
                    admin_ws_arc,
                    p2p_node.handle().command_sender(),
                    std::time::Duration::from_secs(60),
                    shutdown_tx.subscribe(),
                );
                info!(
                    target: "elohim_storage::agent_info",
                    "step-zero substrate agent_info gossip ENABLED (feature flag on)"
                );
            } else {
                warn!(
                    target: "elohim_storage::agent_info",
                    "ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP=true but no embedded conductor — skipping"
                );
            }
        } else {
            info!(
                target: "elohim_storage::agent_info",
                "step-zero substrate agent_info gossip disabled (feature flag off)"
            );
        }

        Some(p2p_node)
    } else if args.enable_p2p
        && config.transport_backend == elohim_storage::config::TransportBackend::Iroh
    {
        info!("P2P transport_backend=iroh — libp2p path skipped (iroh init below)");
        None
    } else {
        info!("P2P networking disabled (use --enable-p2p or ENABLE_P2P=true)");
        None
    };

    #[cfg(not(feature = "p2p"))]
    let p2p_node: Option<()> = None;

    // Initialize extraction cache if enabled. Built before the iroh
    // branch so the iroh-mode `EprServiceBackend` can register it for
    // QueryDelivery cache-tier reporting; the libp2p path picks it up
    // post-hoc via `P2PNode::set_extraction_cache` lower down.
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

    // Phase 11 iroh parallel-stack init — production backends wired in.
    // Per plan genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
    // and spec genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md:
    // each plane's backend dispatches into the same daemon services as the
    // libp2p side. Modes are mutually exclusive at runtime, so the iroh
    // branch constructs its own SyncManager pointing at the same on-disk
    // sled DB the libp2p path would use; only one branch ever opens it.
    #[cfg(feature = "p2p-iroh")]
    let (_iroh_node, iroh_blob_store_for_http): (
        Option<elohim_storage::p2p_iroh::IrohNode>,
        Option<Arc<elohim_storage::p2p_iroh::IrohBlobStore>>,
    ) = if args.enable_p2p
        && config.transport_backend == elohim_storage::config::TransportBackend::Iroh
    {
        use elohim_storage::epr_atom_service::EprAtomService;
        use elohim_storage::epr_service::EprService;
        use elohim_storage::identity_handshake_service::IdentityHandshakeService;
        use elohim_storage::p2p::dedup::DedupLru;
        use elohim_storage::p2p::trust_cache::PeerTrustCache;
        use elohim_storage::p2p_iroh::{
            AlpnRegistration, EprAtomServiceBackend, EprServiceBackend,
            IdentityHandshakeServiceBackend, IrohConfig, IrohEprAtomProtocol, IrohEprProtocol,
            IrohIdentityHandshakeProtocol, IrohNode, IrohShardProtocol, IrohSyncProtocol,
            IrohTrustProtocol, IrohViewFederationProtocol, ShardServiceBackend, SyncManagerBackend,
            TrustServiceBackend, ViewFedServiceBackend, EPR_ALPN, EPR_ATOM_ALPN,
            IDENTITY_HANDSHAKE_ALPN, SHARD_ALPN, SYNC_ALPN, TRUST_ALPN, VIEW_FED_ALPN,
        };
        use elohim_storage::shard_service::ShardService;
        use elohim_storage::sync::{DocStore, StreamTracker, SyncManager};
        use elohim_storage::trust_service::TrustService;
        use elohim_storage::view_fed_service::{libp2p_keypair_from_ed25519_bytes, ViewFedService};

        let iroh_cfg = IrohConfig::from_storage_dir(&config.storage_dir);

        // Sync backend — opens the same `sync.sled` directory the libp2p
        // path uses (mirrors src/p2p/mod.rs:1472). Mode-exclusive at
        // runtime so the two paths never contend for the lock.
        let sync_sled_path = config.storage_dir.join("sync.sled");
        let doc_store = match DocStore::at_path(&sync_sled_path).await {
            Ok(store) => Arc::new(store),
            Err(e) => {
                error!(error = %e, path = %sync_sled_path.display(), "iroh: failed to open sync DocStore");
                return Err(Box::new(e));
            }
        };
        let stream_tracker = Arc::new(StreamTracker::new());
        let sync_manager = Arc::new(SyncManager::new(doc_store, stream_tracker));
        let sync_backend: Arc<dyn elohim_storage::p2p_iroh::SyncBackend> =
            Arc::new(SyncManagerBackend::new(sync_manager));
        let sync_handler = IrohSyncProtocol::new(sync_backend);

        // EPR backend — transport-neutral service shared with the libp2p
        // path. Read-side only in iroh mode for now: Announce (Kademlia
        // put_record on libp2p) is stubbed pending pkarr / iroh-gossip
        // identity-binding per the complementarity spec n0 mitigation
        // roadmap. PeerTrustCache starts empty in iroh mode (trust
        // handshake is libp2p-canonical per the spec); the slow-path
        // reach-auth still runs correctly via DB lookups.
        let epr_db_pool = if args.enable_content_db {
            db_pool.clone()
        } else {
            None
        };
        let epr_policy = epr_db_pool.as_ref().map(|pool| {
            let policy_cache = elohim_storage::db::policy_cache::PolicyCache::new(pool.clone());
            Arc::new(elohim_storage::db::policy_cache::PolicyEnforcement::new(
                policy_cache,
            ))
        });
        let epr_service = Arc::new(EprService::new(
            epr_db_pool,
            epr_policy,
            extraction_cache.clone(),
            PeerTrustCache::new(),
        ));
        let epr_backend: Arc<dyn elohim_storage::p2p_iroh::EprBackend> =
            Arc::new(EprServiceBackend::new(epr_service));
        let epr_handler = IrohEprProtocol::new(epr_backend);

        // EPR-atom backend — transport-neutral service shared with the
        // libp2p path. Caller identity defaults to Anonymous in iroh
        // mode pending the cross-stack peer-map graduation; serves
        // Commons/Public atoms correctly and falls through to NotFound
        // for tighter reach tiers (leak-free, matches libp2p semantics
        // for unauthenticated callers).
        let dedup = Arc::new(DedupLru::new());
        let epr_atom_service = Arc::new(EprAtomService::new(
            if args.enable_content_db {
                db_pool.clone()
            } else {
                None
            },
            dedup,
        ));
        let epr_atom_backend: Arc<dyn elohim_storage::p2p_iroh::EprAtomBackend> =
            Arc::new(EprAtomServiceBackend::new(epr_atom_service));
        let epr_atom_handler = IrohEprAtomProtocol::new(epr_atom_backend);

        // Shard backend — transport-neutral service shared with the
        // libp2p path. Wraps the same BlobStore the libp2p path uses;
        // mode-exclusive at runtime so no contention. Note: this is the
        // legacy SHA-256 sharded fetch plane, NOT the iroh-blobs
        // BLAKE3-streamed plane (the latter mounts on the Router
        // automatically under iroh_blobs::ALPN). Per spec, the shard
        // ALPN exists for libp2p-fallback peers that can't use
        // iroh-blobs.
        let shard_service = Arc::new(ShardService::new(
            blob_store.clone(),
            if args.enable_content_db {
                db_pool.clone()
            } else {
                None
            },
        ));
        let shard_backend: Arc<dyn elohim_storage::p2p_iroh::ShardBackend> =
            Arc::new(ShardServiceBackend::new(shard_service));
        let shard_handler = IrohShardProtocol::new(shard_backend);

        // View-federation backend — transport-neutral service that
        // bundles this node's identity, keypair, and DB pool. The
        // signing keypair is derived from the iroh SecretKey's
        // ed25519 bytes and rebuilt as a libp2p::identity::Keypair so
        // the same `build_response_slice` path runs identically (the
        // service still expects a libp2p Keypair pending signer-trait
        // refactor; both wrap the same ed25519 crypto so signatures
        // are byte-identical).
        let iroh_secret =
            elohim_storage::p2p_iroh::load_or_generate_secret_key(&iroh_cfg.secret_key_path)
                .map_err(|e| {
                    error!(error = %e, path = %iroh_cfg.secret_key_path.display(),
                           "iroh: failed to load secret key for view-fed signer");
                    e
                })?;
        let mut iroh_secret_bytes = iroh_secret.to_bytes();
        let view_fed_signer =
            libp2p_keypair_from_ed25519_bytes(&mut iroh_secret_bytes).map_err(|e| {
                error!(error = %e, "iroh: failed to derive libp2p keypair from iroh secret");
                std::io::Error::other(format!("iroh keypair derivation: {e}"))
            })?;
        let iroh_node_id_str = iroh_secret.public().to_string();
        // Local agent CID in iroh mode mirrors the iroh NodeId until
        // the cross-stack peer-map graduates DHT-anchored identity
        // into a unified handle. View-fed responses are verifiable by
        // the receiver against the agent CID's public key (which is
        // the same ed25519 key in both transport modes).
        let view_fed_service = Arc::new(ViewFedService::new(
            iroh_node_id_str.clone(),
            iroh_node_id_str.clone(),
            view_fed_signer,
            if args.enable_content_db {
                db_pool.clone()
            } else {
                None
            },
        ));
        let view_fed_backend: Arc<dyn elohim_storage::p2p_iroh::ViewFederationBackend> = Arc::new(
            ViewFedServiceBackend::new(view_fed_service, iroh_node_id_str.clone()),
        );
        let view_fed_handler = IrohViewFederationProtocol::new(view_fed_backend);

        // Identity-handshake backend — transport-neutral service
        // sharing the libp2p path's verify+persist sequence. Per spec
        // dual-stack permanent; integrity via DHT-anchored signed
        // wire frames (Track 1), not transport-level security.
        let identity_handshake_service =
            Arc::new(IdentityHandshakeService::new(if args.enable_content_db {
                db_pool.clone()
            } else {
                None
            }));
        let identity_handshake_backend: Arc<
            dyn elohim_storage::p2p_iroh::IdentityHandshakeBackend,
        > = Arc::new(IdentityHandshakeServiceBackend::new(
            identity_handshake_service,
            iroh_node_id_str.clone(),
        ));
        let identity_handshake_handler =
            IrohIdentityHandshakeProtocol::new(identity_handshake_backend);

        // Trust backend — transport-neutral response builder. Cache
        // insertion (libp2p-keyed peer_trust_cache) is intentionally
        // skipped in iroh mode pending Phase 12 cross-stack peer-map
        // graduation; iroh-mode reach-auth fast path falls through to
        // slow-path DB lookups (correct, just not ambient-cached).
        let trust_service = Arc::new(TrustService::new());
        let trust_backend: Arc<dyn elohim_storage::p2p_iroh::TrustBackend> =
            Arc::new(TrustServiceBackend::new(trust_service));
        let trust_handler = IrohTrustProtocol::new(trust_backend);

        let extras: Vec<AlpnRegistration> = vec![
            (SYNC_ALPN.to_vec(), Box::new(sync_handler)),
            (EPR_ALPN.to_vec(), Box::new(epr_handler)),
            (EPR_ATOM_ALPN.to_vec(), Box::new(epr_atom_handler)),
            (SHARD_ALPN.to_vec(), Box::new(shard_handler)),
            (VIEW_FED_ALPN.to_vec(), Box::new(view_fed_handler)),
            (
                IDENTITY_HANDSHAKE_ALPN.to_vec(),
                Box::new(identity_handshake_handler),
            ),
            (TRUST_ALPN.to_vec(), Box::new(trust_handler)),
        ];

        match IrohNode::start_with_protocols(iroh_cfg, extras).await {
            Ok(node) => {
                info!(
                    "Iroh parallel-stack node started (Phase 11 — sync + EPR + EPR-atom \
                     + shard + view-fed + identity-handshake + trust backends wired)"
                );
                info!("  Node ID: {}", node.node_id());
                info!(
                    "  ALPNs: iroh-blobs, iroh-gossip, /elohim/sync/2.0.0, \
                     /elohim/epr/2.0.0, /elohim/epr-atom/2.0.0, /elohim/shard/2.0.0, \
                     /elohim/view-federation/2.0.0, /elohim/identity-handshake/2.0.0, \
                     /elohim/trust/2.0.0"
                );
                // Cutover gate #2 (Plan 2): clone blob store for HTTP server wiring.
                let iroh_blob_store_for_http = Arc::new(node.store().clone());
                (Some(node), Some(iroh_blob_store_for_http))
            }
            Err(e) => {
                error!(error = %e, "Failed to start iroh node");
                return Err(Box::new(e));
            }
        }
    } else {
        (None, None)
    };

    #[cfg(not(feature = "p2p-iroh"))]
    let (_iroh_node, _iroh_blob_store_for_http): (Option<()>, Option<()>) = (None, None);

    // Plan 4: Wire DualGossipPublisher into the libp2p P2PNode so inventory snapshots
    // are published to both transports. Must happen after iroh node creation, before
    // P2PNode::run(). No-op when either stack is absent.
    #[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
    if let (Some(libp2p_n), Some(iroh_n)) = (p2p_node.as_mut(), _iroh_node.as_ref()) {
        let libp2p_tx = libp2p_n.handle().command_sender();
        let libp2p_pub: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> = Arc::new(
            elohim_storage::p2p::adapters::LibP2PGossipPublisher::new(libp2p_tx),
        );
        let iroh_pub: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> = Arc::new(
            elohim_storage::p2p_iroh::dual_publish::IrohGossipPublisher::spawn(
                iroh_n.gossip().clone(),
            ),
        );
        let dual: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> = Arc::new(
            elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher::new(
                Some(libp2p_pub),
                Some(iroh_pub),
            ),
        );
        libp2p_n.set_gossip_publisher(dual);
        info!(
            "Plan 4: DualGossipPublisher wired into P2PNode — inventory snapshots now dual-stack"
        );
    }

    // Start HTTP server for shard API
    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.http_port).parse()?;

    // Load operator-configured elohim capability once at startup (Category C — operational).
    // None = no ELOHIM_CAPABILITY_CONFIG_FILE set, or file unreadable/invalid.
    // Warnings are emitted by load_elohim_capability_from_env() so operators see diagnostics.
    let elohim_capability = elohim_storage::load_elohim_capability_from_env();
    if elohim_capability.is_some() {
        info!("Elohim capability profile loaded from ELOHIM_CAPABILITY_CONFIG_FILE");
    } else {
        info!("No elohim capability profile — node operates as storage/relay only (set ELOHIM_CAPABILITY_CONFIG_FILE to enable)");
    }

    let render_capability = elohim_storage::load_render_capability_from_url().await;
    if render_capability.is_some() {
        tracing::info!("render_capability loaded from DOORWAY_CAPABILITY_URL");
    } else if std::env::var("DOORWAY_CAPABILITY_URL").is_ok() {
        tracing::warn!("DOORWAY_CAPABILITY_URL set but capability could not be loaded — peer-status will show null");
    }

    // Tier-2 extensions are not yet populated by any source; leave None for now.
    // (Future: registered capability owners populate this map at startup.)
    let extensions: Option<elohim_storage::CapabilityExtensions> = None;

    // EPR Phase 2B Task C.6 — compose the 4-layer write-through state.
    //
    // Layer 1 (manifest defaults) — loaded from pillar manifest files on disk.
    // ELOHIM_PILLAR_MANIFEST_DIR env var overrides the default path.
    // Failure degrades gracefully to an empty layer (same as before Phase 4 T12).
    //
    // Layer 2 (policy.toml) — read from PolicyConfig::write_through if present.
    //
    // Layer 3 (env vars) — `ELOHIM_WRITE_THROUGH_<PILLAR>=on|off|true|false|1|0`.
    //
    // Layer 4 (admin override) — mutated live via `POST /admin/write-through`.
    let write_through_state = {
        let manifest_dir = std::env::var("ELOHIM_PILLAR_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("elohim/sdk/domains"));
        let manifest_layer =
            elohim_storage::services::manifest_registry::load_pillar_manifest_layer1(&manifest_dir)
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        target = "phase4::manifest_layer1",
                        error = %e,
                        "failed to load pillar manifest layer-1; layer stays empty"
                    );
                    std::collections::HashMap::new()
                });
        let mut state =
            elohim_storage::write_through::WriteThroughState::from_manifest(manifest_layer);
        // Layer 2 — policy.toml. Try to load; failures are warned, not fatal.
        if let Ok(policy_cfg) = elohim_storage::policy::PolicyConfig::load(&config.peer_policy_path)
        {
            if let Some(policy_override) =
                elohim_storage::policy::write_through_override_from_policy(
                    &policy_cfg.write_through,
                )
            {
                info!(
                    pillars = policy_override.pillars.len(),
                    "write-through layer 2 (policy.toml) applied"
                );
                state = state.with_policy_override(policy_override);
            }
        }
        // Layer 3 — env vars.
        let env_override = elohim_storage::write_through::WriteThroughOverride::from_env();
        if !env_override.is_empty() {
            info!(
                pillars = env_override.pillars.len(),
                "write-through layer 3 (env) applied"
            );
            state = state.with_env_override(env_override);
        }
        Arc::new(state)
    };

    let mut http_server = HttpServer::new(blob_store.clone(), http_addr)
        .with_progress_hub(Arc::clone(&progress_hub))
        .with_elohim_capability(elohim_capability)
        .with_render_capability(render_capability)
        .with_extensions(extensions)
        .with_write_through_state(write_through_state.clone())
        // T17: race-fetch parameters (timeout, parallelism, self-CID).
        .with_fetch_config(&config);

    // Transport-identity seam — lets `/p2p/status` report the active transport's
    // peerId (iroh mode) instead of 503, so the resilience join works on iroh.
    // No effect on libp2p, where `p2p_handle.status()` stays authoritative.
    if let Some(ref transport) = node_transport {
        http_server = http_server.with_node_transport(transport.clone());
    }

    if args.embedded_conductor {
        http_server = http_server.with_embedded_conductor();
    }
    // Wire the conductor manager so POST /admin/arc-policy/actuate can rewrite
    // the conductor-config + restart it (authority-arc actuation, spec §5).
    if let Some(ref cm) = conductor_manager {
        http_server = http_server.with_conductor_manager(Arc::clone(cm));
    }

    // Cutover gate #2 (Plan 2 Task 3): wire iroh blob store into HTTP server.
    // When the iroh node started successfully, thread its blob store so that
    // GET /blob/{hash} can serve BLAKE3-addressed blobs from iroh for
    // iroh-capable callers. Absent store → every request degrades to legacy SHA256 path.
    #[cfg(feature = "p2p-iroh")]
    if let Some(iroh_blobs) = iroh_blob_store_for_http.clone() {
        http_server = http_server.with_iroh_blob_store(iroh_blobs);
        info!(
            "Iroh blob store wired into HTTP server (cutover gate #2 — iroh-first for BLAKE3-capable callers)"
        );
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

    // Wire the shared Diesel pool into the HTTP server. Pool creation happens
    // once at startup (see `db_pool` above); this section only gates which
    // route groups register against it. `/db/*` requires --enable-content-db;
    // `/session` is always on so long as the pool initialized.
    if let Some(pool) = db_pool.clone() {
        if args.enable_content_db {
            // Create services with the shared pool
            let services = Arc::new(Services::new(pool.clone()));
            http_server = http_server.with_services(services);
            // Wire policy enforcement for content filtering
            let policy_cache = elohim_storage::db::policy_cache::PolicyCache::new(pool.clone());
            let enforcement = Arc::new(elohim_storage::db::policy_cache::PolicyEnforcement::new(
                policy_cache,
            ));
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
        } else {
            http_server = http_server.with_db_pool(pool);
            info!("Session API:");
            info!("  GET    /session       - Get active session");
            info!("  POST   /session       - Create session");
            info!("  DELETE /session       - Delete session");
            info!("  GET    /session/all   - List all sessions");
            info!("Content database disabled (use --enable-content-db or ENABLE_CONTENT_DB=true)");
        }
    } else {
        error!("Database + session APIs disabled: shared pool unavailable");
    }

    // Wire P2P services into HTTP server
    #[cfg(feature = "p2p")]
    if let Some(ref node) = p2p_node {
        http_server = http_server.with_sync_manager(node.sync_manager().clone());
        http_server = http_server.with_p2p_handle(node.handle());
        info!("P2P node wired to HTTP server — Sync API and /p2p/status active");
    }

    // T22: Construct EprFanOutCtx and inject into HTTP layer.
    //
    // The fan-out context activates the FeedbackSignal arrival pipeline:
    //   1. project_signal  — debit-weighted standing update
    //   2. back_prop_one_hop — forward signal one hop upstream (unseals predecessor records)
    //   3. flood_feedback  — gossip-flood to content reach topic
    //
    // Construction is gated on db_pool being available. P2P adapters are wired
    // from the swarm command channel; without P2P the outbound_sink and
    // gossip_publisher remain None and those two steps are skipped gracefully.
    //
    // Sealing keys: generated ephemerally here for dev/test. These keys are used
    // to seal predecessor records in SQLite. In production, they should be loaded
    // from a persisted node-key file so predecessor records survive process
    // restart. See TODO(T22-followup) in api/epr.rs::UnsealingKeyBundle.
    if let Some(ref pool) = db_pool {
        use dryoc::classic::crypto_box::crypto_box_seed_keypair;
        use elohim_storage::api::epr::{EprFanOutCtx, SealingKeyPair, UnsealingKeyBundle};
        use elohim_storage::services::sealed_against_self::{
            ImagodeiPubKey, ImagodeiSecretKey, MishpatQuorumPubKey, MishpatQuorumSecretKey,
        };

        // Load (or generate) manifest registry from DB for standing-policy debit weights.
        let manifest_registry = match pool.get() {
            Ok(mut conn) => {
                let registry = elohim_storage::services::manifest_registry::ManifestRegistry::new();
                match registry.load_from_db(&mut conn) {
                    Ok(count) => {
                        info!(
                            pillar_mappings = count,
                            "T22: ManifestRegistry loaded from DB (standing-policy debit weights active)"
                        );
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            "T22: ManifestRegistry load failed (debit weights will use defaults)"
                        );
                    }
                }
                Some(Arc::new(registry))
            }
            Err(e) => {
                warn!(error = %e, "T22: DB connection failed for ManifestRegistry (fan-out degraded)");
                None
            }
        };

        // Ephemeral 2-of-2 sealing keys for local dev.
        // TODO(T22-followup): load from persisted node-key file so predecessor
        // records survive process restart. Ephemeral keys are acceptable for dev
        // since back_prop_one_hop only needs them to read records written in the
        // same process lifetime.
        let mishpat_seed = [0xA0u8; 32]; // dev-only deterministic seed
        let imagodei_seed = [0xB0u8; 32]; // dev-only deterministic seed
        let (mishpat_pk_raw, mishpat_sk_raw) = crypto_box_seed_keypair(&mishpat_seed);
        let (imagodei_pk_raw, imagodei_sk_raw) = crypto_box_seed_keypair(&imagodei_seed);

        let mishpat_pk = MishpatQuorumPubKey(mishpat_pk_raw);
        let mishpat_sk = MishpatQuorumSecretKey(mishpat_sk_raw);
        let imagodei_pk = ImagodeiPubKey(imagodei_pk_raw);
        let imagodei_sk = ImagodeiSecretKey(imagodei_sk_raw);

        let sealing_keys = Arc::new(SealingKeyPair {
            mishpat_pk: MishpatQuorumPubKey(mishpat_pk.0),
            imagodei_pk: ImagodeiPubKey(imagodei_pk.0),
        });
        let unsealing_keys = Arc::new(UnsealingKeyBundle {
            mishpat_pk,
            mishpat_sk,
            imagodei_pk,
            imagodei_sk,
        });

        // W2A: inject sealing keys into the libp2p P2PNode so that
        // handle_epr_atom_request can call record_predecessor on Content-kind
        // Announce ingests. The same SealingKeyPair is also passed to
        // EprFanOutCtx below for the HTTP fan-out path.
        #[cfg(feature = "p2p")]
        if let Some(ref mut node) = p2p_node {
            node.set_sealing_keys(sealing_keys.clone());
            info!("W2A: sealing keys wired into P2PNode — record_predecessor active on Content-kind Announce");
        }

        // P2P adapters — only wired when a live swarm is available.
        #[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
        let (outbound_sink, gossip_publisher, local_peer_id_opt) = if let Some(ref node) = p2p_node
        {
            let tx = node.handle().command_sender();
            let sink: Arc<dyn elohim_storage::services::back_prop::OutboundSink> = Arc::new(
                elohim_storage::p2p::adapters::LibP2POutboundSink::new(tx.clone()),
            );
            let libp2p_publisher: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> =
                Arc::new(elohim_storage::p2p::adapters::LibP2PGossipPublisher::new(
                    tx,
                ));

            // Plan 4: wrap in DualGossipPublisher — fans out to iroh when the iroh
            // node is running, degrades to libp2p-only when it is absent (None).
            let iroh_publisher_opt: Option<
                Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher>,
            > = _iroh_node.as_ref().map(|iroh| {
                Arc::new(
                    elohim_storage::p2p_iroh::dual_publish::IrohGossipPublisher::spawn(
                        iroh.gossip().clone(),
                    ),
                )
                    as Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher>
            });

            let publisher: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> =
                Arc::new(
                    elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher::new(
                        Some(libp2p_publisher),
                        iroh_publisher_opt,
                    ),
                );

            let peer_id = node.handle().local_peer_id();
            (Some(sink), Some(publisher), Some(peer_id))
        } else {
            (None, None, None)
        };

        // p2p without iroh: use LibP2PGossipPublisher directly (no dual fan-out).
        #[cfg(all(feature = "p2p", not(feature = "p2p-iroh")))]
        let (outbound_sink, gossip_publisher, local_peer_id_opt) = if let Some(ref node) = p2p_node
        {
            let tx = node.handle().command_sender();
            let sink: Arc<dyn elohim_storage::services::back_prop::OutboundSink> = Arc::new(
                elohim_storage::p2p::adapters::LibP2POutboundSink::new(tx.clone()),
            );
            let publisher: Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher> =
                Arc::new(elohim_storage::p2p::adapters::LibP2PGossipPublisher::new(
                    tx,
                ));
            let peer_id = node.handle().local_peer_id();
            (Some(sink), Some(publisher), Some(peer_id))
        } else {
            (None, None, None)
        };

        #[cfg(not(feature = "p2p"))]
        let (outbound_sink, gossip_publisher, local_peer_id_opt) = (
            None::<Arc<dyn elohim_storage::services::back_prop::OutboundSink>>,
            None::<Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher>>,
            None::<String>,
        );

        // Derive the standing-policy CID from the bootstrap manifest constant.
        // Phase 4 will expose a way to discover the highest-revision policy CID
        // from the ManifestRegistry; for now we hardcode the bootstrap CID.
        let standing_policy_cid = Some("bootstrap:standing-policy:v1".to_string());

        // Local pubkey: use the agent pubkey from config (a placeholder in dev;
        // Phase 4 / conductor connection will provide the real ed25519 pubkey).
        // project_signal uses this to scope the standing_view evaluator column.
        // For now we derive 32 placeholder bytes from the storage dir path hash.
        let local_pubkey = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            config.storage_dir.hash(&mut hasher);
            let h = hasher.finish();
            let mut bytes = vec![0u8; 32];
            bytes[..8].copy_from_slice(&h.to_le_bytes());
            Some(bytes)
        };

        // Graph engine init (graph-native feature). Opens CozoDB sled instance at
        // storage_dir/graph.db, applies core schema, then applies any graph extensions
        // declared in pillar manifest.json files. Failures are non-fatal — projection
        // is skipped gracefully and the node continues serving relational views.
        #[cfg(feature = "graph-native")]
        let graph_engine_arc: Option<
            std::sync::Arc<elohim_storage::graph::engine::GraphEngine>,
        > = {
            use elohim_storage::graph::engine::GraphEngine;
            use elohim_storage::graph::registry::{apply_graph_extension, GraphExtension};
            use elohim_storage::graph::schema::apply_core_schema;

            let graph_db_path = config.storage_dir.join("graph.db");
            match GraphEngine::open(&graph_db_path) {
                Ok(eng) => {
                    if let Err(e) = apply_core_schema(&eng) {
                        tracing::warn!(
                            error = %e,
                            "graph schema apply failed; graph projection disabled"
                        );
                        None
                    } else {
                        // Apply graph extensions from pillar manifests on disk.
                        // Same manifest_dir as write-through layer-1 loader.
                        let manifest_dir = std::env::var("ELOHIM_PILLAR_MANIFEST_DIR")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("elohim/sdk/domains"));

                        if let Ok(entries) = std::fs::read_dir(&manifest_dir) {
                            for entry in entries.flatten() {
                                let pillar_name = entry.file_name().to_string_lossy().to_string();
                                let manifest_path = entry.path().join("manifest.json");
                                if !manifest_path.exists() {
                                    continue;
                                }
                                let Ok(body) = std::fs::read_to_string(&manifest_path) else {
                                    continue;
                                };
                                let Ok(json) = serde_json::from_str::<serde_json::Value>(&body)
                                else {
                                    continue;
                                };
                                if let Some(graph_val) = json.get("graph") {
                                    match serde_json::from_value::<GraphExtension>(
                                        graph_val.clone(),
                                    ) {
                                        Ok(ext) => {
                                            if let Err(e) =
                                                apply_graph_extension(&eng, &pillar_name, &ext)
                                            {
                                                tracing::warn!(
                                                    pillar = %pillar_name,
                                                    error = %e,
                                                    "graph extension apply failed; pillar rules skipped"
                                                );
                                            } else {
                                                info!(
                                                    pillar = %pillar_name,
                                                    "graph extension applied from manifest"
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                pillar = %pillar_name,
                                                error = %e,
                                                "graph extension parse failed; pillar rules skipped"
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        info!(
                            path = ?graph_db_path,
                            "graph-native: GraphEngine opened + core schema applied"
                        );
                        Some(std::sync::Arc::new(eng))
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = ?graph_db_path,
                        error = %e,
                        "graph engine open failed; graph projection disabled"
                    );
                    None
                }
            }
        };

        let fan_out_ctx = Arc::new(EprFanOutCtx {
            manifest_registry,
            outbound_sink,
            gossip_publisher,
            sealing_keys: Some(sealing_keys),
            unsealing_keys: Some(unsealing_keys),
            local_peer_id: local_peer_id_opt,
            local_pubkey,
            standing_policy_cid,
            #[cfg(feature = "graph-native")]
            graph_engine: graph_engine_arc,
        });

        http_server = http_server.with_fan_out_ctx(fan_out_ctx);
        info!("T22: EprFanOutCtx constructed and injected — FeedbackSignal fan-out active");
    }

    // Wire HcClientRegistry into HTTP server for zome forwarding (Phase 11 Task 5).
    if let Some(registry) = hc_registry_for_http.as_ref() {
        http_server = http_server.with_hc_registry(registry.clone());
    }

    // One-shot household_id backfill — fills legacy `humans.household_id IS NULL`
    // rows from DHT household memberships, the same value the live reconcile path
    // stamps (`on_membership_projected`). The replayer reads household collectives
    // from the local projection, then reads each household's members back from the
    // imagodei conductor. Best-effort: a missing imagodei client or unreachable
    // conductor degrades to an empty/partial mapping — never a startup failure.
    // The backfill is idempotent and NULL-only, so it never overwrites a
    // create-time or live-stamped value.
    if let (Some(pool), Some(imagodei)) = (
        db_pool.as_ref(),
        hc_registry_for_http
            .as_ref()
            .and_then(|r| r.imagodei.clone()),
    ) {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            use elohim_storage::services::holochain_humans_replayer::{
                snapshot_household_ids, ConductorMembershipReader,
            };
            // humans/collectives projections are lamad-app-scoped (see the
            // reconcile controller's `default_lamad` ctx for the same junction).
            let ctx = elohim_storage::db::AppContext::default_lamad();
            let reader = ConductorMembershipReader {
                hc_client: imagodei,
            };
            match snapshot_household_ids(&pool_clone, &ctx, &reader).await {
                Ok(mapping) => {
                    if let Err(e) =
                        elohim_storage::services::household_backfill::run_once_by_membership(
                            &pool_clone,
                            mapping,
                        )
                    {
                        warn!(error = %e, "household_backfill failed (non-fatal)");
                    }
                }
                Err(e) => {
                    warn!(error = %e, "household_backfill snapshot failed (non-fatal)");
                }
            }
        });
    } else {
        tracing::debug!(
            "household_id backfill skipped: db pool or imagodei conductor client unavailable"
        );
    }

    // Load slug index for HTML5 app caching
    http_server.load_slug_index().await;

    let http_server = Arc::new(http_server);

    // Save admin_url before the import handler moves it out of args.
    // The reconcile controller startup (below) needs this value after args.admin_url
    // is potentially moved into the import config.
    let saved_admin_url: Option<String> = args.admin_url.clone();

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

    // ---------------------------------------------------------------------------
    // Reconcile controller startup (Task A.11)
    //
    // Connects to the imagodei conductor app interface and subscribes to signals.
    // The controller is spawned as a background tokio task. Non-fatal: if the
    // imagodei conductor is not yet reachable, we log a warning and continue
    // serving HTTP + blob storage without reconciliation.
    //
    // The controller requires:
    //   1. --app-url / HOLOCHAIN_APP_URL (already used for ImportApi)
    //   2. --admin-url / HOLOCHAIN_ADMIN_URL (for signing credentials)
    //   3. --imagodei-app-id / IMAGODEI_APP_ID (default "imagodei")
    //   4. --enable-content-db (for db_pool and compromise_at derivation; optional)
    //
    // In production all four are set via the elohim-node Helm chart / systemd unit.
    // In dev / blob-only mode any of 1-3 may be absent; the no-op path is correct.
    // ---------------------------------------------------------------------------
    if !args.imagodei_app_id.is_empty() {
        if let Some(admin_url) = saved_admin_url.clone() {
            let app_url_clone = args.app_url.clone();
            let admin_url_clone = admin_url;
            // genesis #1123 root-cause: this used to be passed as the INSTALLED
            // APP ID. No manifest sets IMAGODEI_APP_ID, so the default literal
            // "imagodei" was asked of a conductor whose installed app is the
            // elohim happ (with imagodei as a ROLE inside it) — the controller
            // failed "app 'imagodei' not found" on every boot since inception
            // and the forever-retry would have retried the wrong id forever.
            // The registry's working connect uses (app_id=args.app_id,
            // role="imagodei"); mirror it: imagodei_app_id is the ROLE name.
            let installed_app_id_clone = args.app_id.clone();
            let imagodei_role_clone = args.imagodei_app_id.clone();
            let reconcile_pool = db_pool.clone().map(Arc::new);

            // Pubkey cache is shared between the controller and the EPR verify
            // path (Task A.7). Created here with a generous capacity so the
            // controller and concurrent verifications never contend on a small LRU.
            let pubkey_cache = Arc::new(TokioMutex::new(PubkeyTimelineCache::with_capacity(512)));

            // Capture a P2P command sender if the p2p feature is enabled and the
            // node is running. The controller uses this to gossip new AgentPeerBinding
            // entries to subscribed peers (Task A.10).
            //
            // When p2p feature is disabled, the controller runs without a swarm_tx
            // and gossip publish is silently skipped (logged at debug level per the
            // no-swarm-path in on_agent_peer_binding).
            #[cfg(feature = "p2p")]
            let swarm_tx_opt = p2p_node.as_ref().map(|n| n.handle().command_sender());

            // genesis #1122 fix: the imagodei conductor needs 6+ minutes to
            // enable its cells after a rolling restart. The old code connected
            // ONCE and, on failure ("app 'imagodei' not found" / CellDisabled),
            // permanently disabled the reconcile controller for the pod's whole
            // lifetime — so MembershipProjected → humans-junction stamps never
            // landed and projection-reconcile heals reported conductor_missing
            // on every non-genesis pod until the next pod delete.
            //
            // Now: retry the signal-stream connect FOREVER with capped
            // exponential backoff (`hc_client_registry::reconnect_backoff`,
            // the same policy the registry late-connect uses). When it lands —
            // early OR late — the controller is built and run_loop started with
            // EXACTLY the same wiring the boot-success path used (the whole
            // build-controller + run_loop is self-contained in this task, so
            // late success is byte-for-byte as good as early success). If the
            // conductor later disconnects mid-loop, we fall back into the same
            // reconnect loop rather than giving up.
            let mut reconcile_shutdown = shutdown_tx.subscribe();
            tokio::spawn(async move {
                use elohim_storage::hc_client_registry::{
                    reconnect_backoff, should_warn_still_down,
                };
                loop {
                    let mut attempt: u32 = 0;
                    let stream = loop {
                        attempt = attempt.saturating_add(1);
                        match HolochainAppSignalStream::connect(
                            &admin_url_clone,
                            &app_url_clone,
                            &installed_app_id_clone,
                            &imagodei_role_clone, // role within the installed happ
                            reconcile_pool.clone(),
                        )
                        .await
                        {
                            Ok(stream) => {
                                if attempt == 1 {
                                    info!(
                                        app_id = %installed_app_id_clone,
                                        "Reconcile controller connected to imagodei conductor — starting loop"
                                    );
                                } else {
                                    info!(
                                        app_id = %installed_app_id_clone,
                                        attempt,
                                        "Reconcile controller connected to imagodei conductor (late) — \
                                         starting loop with same wiring as boot-success path"
                                    );
                                }
                                break stream;
                            }
                            Err(e) => {
                                let delay = reconnect_backoff(attempt);
                                if should_warn_still_down(attempt) {
                                    warn!(
                                        error = %e,
                                        app_id = %installed_app_id_clone,
                                        attempt,
                                        delay_secs = delay.as_secs(),
                                        "Reconcile controller bridge still down — retrying forever \
                                         (conductor cells may still be CellDisabled after a rolling \
                                         restart; storage keeps serving blobs/HTTP meanwhile)"
                                    );
                                } else {
                                    info!(
                                        error = %e,
                                        app_id = %installed_app_id_clone,
                                        attempt,
                                        delay_secs = delay.as_secs(),
                                        "Reconcile controller bridge reconnect: retrying"
                                    );
                                }
                                tokio::select! {
                                    _ = tokio::time::sleep(delay) => {}
                                    _ = reconcile_shutdown.recv() => {
                                        info!(
                                            app_id = %installed_app_id_clone,
                                            "Reconcile controller reconnect loop exiting (shutdown)"
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                    };

                    // Build controller. new_with_storage when db_pool available
                    // (enables sweep + compromise_at derivation); new() otherwise.
                    let mut controller = match reconcile_pool.clone() {
                        Some(pool) => {
                            let c = ReconcileController::new_with_storage(
                                stream,
                                pool,
                                Arc::clone(&pubkey_cache),
                            );
                            #[cfg(feature = "p2p")]
                            let c = match swarm_tx_opt.clone() {
                                Some(tx) => c.with_swarm_tx(tx),
                                None => c,
                            };
                            c
                        }
                        None => {
                            // No db_pool — sweep ops will be no-ops (logged by controller).
                            let c = ReconcileController::new(stream);
                            #[cfg(feature = "p2p")]
                            let c = match swarm_tx_opt.clone() {
                                Some(tx) => c.with_swarm_tx(tx),
                                None => c,
                            };
                            c
                        }
                    };

                    if let Err(e) = controller.run_loop().await {
                        warn!(error = %e, "Reconcile controller run_loop exited with error — re-entering reconnect loop");
                    } else {
                        info!(
                            "Reconcile controller run_loop exited cleanly (conductor disconnected) — re-entering reconnect loop"
                        );
                    }

                    // The conductor dropped (clean or error). Rather than
                    // giving up for the pod's lifetime, fall back into the
                    // reconnect loop — the same forever-survival guarantee.
                    tokio::select! {
                        _ = tokio::time::sleep(reconnect_backoff(1)) => {}
                        _ = reconcile_shutdown.recv() => {
                            info!("Reconcile controller exiting after disconnect (shutdown)");
                            return;
                        }
                    }
                }
            });
        } else {
            info!(
                "Reconcile controller disabled: no --admin-url / HOLOCHAIN_ADMIN_URL set \
                 (set it to enable imagodei signal subscription)"
            );
        }
    } else {
        info!("Reconcile controller disabled: --imagodei-app-id is empty");
    }

    // P1 projection-reconcile stream — REA commitments converge from THIS node's
    // OWN conductor, with peers used as discovery only. Cures the edge-triggered
    // projection gap (a missed ReaProjectionSignal left adam divergent for 10
    // days; reseeds collapse to 409 on the originator so the signal never
    // re-fires). See `.claude/deliver/journal-resilient-dual-doorway.md` rc #2
    // and `p2p::projection_reconcile`.
    //
    // Lives here (not in P2PNode) for the same reason as the provide-loop: the
    // lamad HcClient author/read seam (`registry.lamad`) is only in this
    // composition scope. Requires the libp2p P2P handle + lamad HcClient + db
    // pool; absent any, the task is not spawned (the node still surfaces a null
    // projectionReconcile on /p2p/status).
    //
    // Cadence: boot sweep after a 30s settle (peers + conductor up), then every
    // PROJECTION_RECONCILE_SECS (default 300; 0 disables). Read once here, never
    // on the hot path.
    #[cfg(feature = "p2p")]
    {
        let reconcile_secs: u64 = std::env::var("PROJECTION_RECONCILE_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        let p2p_handle = p2p_node.as_ref().map(|n| n.handle());

        // One-shot shard-manifest backfill — gives legacy content (seeded
        // before distribute-at-ingest existed) a shard manifest + distribution
        // so the resilience snapshot stops reading it as "unmeasured" and
        // shard_locations can populate. Non-fatal, paced, skip-and-warn on
        // missing blobs. Distributes when a p2p handle is present.
        {
            let backfill_pool = db_pool.clone();
            let backfill_blob_store = blob_store.clone();
            let backfill_handle = p2p_handle.clone();
            tokio::spawn(async move {
                if let Some(pool) = backfill_pool {
                    let config =
                        elohim_storage::services::shard_manifest_backfill::BackfillConfig::default(
                        );
                    if let Err(e) = elohim_storage::services::shard_manifest_backfill::run_once(
                        &pool,
                        &backfill_blob_store,
                        "lamad",
                        &config,
                        backfill_handle,
                    )
                    .await
                    {
                        warn!(error = %e, "shard_manifest_backfill failed (non-fatal)");
                    }
                }
            });
        }

        // genesis #1122 fix (lamad heal leg): DON'T gate task spawn on a
        // boot-time lamad HcClient. If the registry's bounded boot ramp
        // None-stamped lamad (conductor cells still CellDisabled), the
        // projection-reconcile heal sweep — the leg that asks the LOCAL
        // conductor and was reporting conductor_missing on every non-genesis
        // pod — would never spawn. Instead we acquire the lamad bridge INSIDE
        // the task via `connect_role_forever`, so the heal sweep starts the
        // moment lamad lands, late or early.
        let lamad_hc_boot = hc_registry_for_http.as_ref().and_then(|r| r.lamad.clone());
        let late_inputs = saved_admin_url.clone().map(|admin_url| {
            elohim_storage::hc_client_registry::HcRegistryInputs {
                admin_url,
                app_url: args.app_url.clone(),
                app_id: args.app_id.clone(),
            }
        });
        match (reconcile_secs, p2p_handle, db_pool.clone()) {
            (0, _, _) => {
                info!("projection-reconcile: disabled (PROJECTION_RECONCILE_SECS=0)");
            }
            (secs, Some(handle), Some(pool)) => {
                let state = projection_reconcile_state.clone();
                let mut reconcile_shutdown = shutdown_tx.subscribe();
                let late_inputs_for_task = late_inputs.clone();
                tokio::spawn(async move {
                    use tokio::time::{interval, Duration, MissedTickBehavior};
                    // Boot settle: let peers connect + conductor finish init
                    // before the first sweep (other boot tasks behave likewise).
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(30)) => {}
                        _ = reconcile_shutdown.recv() => {
                            tracing::debug!("projection-reconcile: shutdown during boot settle");
                            return;
                        }
                    }

                    // Acquire the lamad bridge. Fast path: boot-connect already
                    // landed it. Late path: boot-connect None-stamped it, so we
                    // retry forever (capped backoff) — the heal sweep is exactly
                    // as good once it lands late as if it had landed at boot.
                    let hc = match lamad_hc_boot {
                        Some(hc) => hc,
                        None => match late_inputs_for_task {
                            Some(inputs) => {
                                info!(
                                    "projection-reconcile: lamad bridge not up at boot — \
                                     awaiting late connect (heal sweep starts once it lands)"
                                );
                                match elohim_storage::hc_client_registry::HcClientRegistry::connect_role_forever(
                                    &inputs,
                                    "lamad",
                                    reconcile_shutdown.resubscribe(),
                                )
                                .await
                                {
                                    Some(hc) => {
                                        info!("projection-reconcile: lamad bridge connected (late) — heal sweep enabled");
                                        hc
                                    }
                                    None => {
                                        // Only None on shutdown.
                                        tracing::debug!("projection-reconcile: shutdown while awaiting lamad late connect");
                                        return;
                                    }
                                }
                            }
                            None => {
                                info!(
                                    "projection-reconcile: disabled (no admin-url for lamad late connect)"
                                );
                                return;
                            }
                        },
                    };

                    let mut ticker = interval(Duration::from_secs(secs));
                    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    loop {
                        tokio::select! {
                            _ = ticker.tick() => {
                                elohim_storage::p2p::projection_reconcile::run_sweep(
                                    &handle, &hc, &pool, &state,
                                )
                                .await;
                            }
                            _ = reconcile_shutdown.recv() => {
                                tracing::debug!("projection-reconcile: shutdown signal received, exiting");
                                break;
                            }
                        }
                    }
                });
                info!(
                    interval_secs = secs,
                    "projection-reconcile stream started (boot sweep +30s, then periodic, shutdown-aware; lamad bridge acquired in-task with forever-retry)"
                );
            }
            _ => {
                info!("projection-reconcile: disabled (requires libp2p P2P handle + db pool)");
            }
        }
    }

    // T21: Tending TTL sweep task — 5-minute interval, shutdown-aware.
    //
    // Deletes expired non-Safety attention_tending rows. Safety classification
    // is never swept (§2.8 constitutional floor protection). Errors are logged
    // at warn level; sweep failure is recoverable on next tick.
    if let Some(pool) = db_pool.clone() {
        let mut sweep_shutdown = shutdown_tx.subscribe();
        tokio::spawn(async move {
            use tokio::time::{interval, MissedTickBehavior};
            let mut ticker = interval(std::time::Duration::from_secs(300));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        match pool.get() {
                            Ok(mut conn) => {
                                match elohim_storage::services::tending::sweep_expired(&mut conn) {
                                    Ok(deleted) => {
                                        if deleted > 0 {
                                            tracing::info!(
                                                deleted,
                                                "tending TTL sweep: removed expired rows"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            "tending TTL sweep failed (recoverable next tick)"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "tending TTL sweep: failed to acquire DB connection (recoverable next tick)"
                                );
                            }
                        }
                    }
                    _ = sweep_shutdown.recv() => {
                        tracing::debug!("tending TTL sweep task: shutdown signal received, exiting");
                        break;
                    }
                }
            }
        });
        info!("Tending TTL sweep task started (5-min interval, shutdown-aware)");
    }

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
