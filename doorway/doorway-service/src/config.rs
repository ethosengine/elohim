//! Configuration for Doorway
//!
//! CLI arguments and environment variable handling using clap.
//! Pattern adapted from holo-host/rust/holo-gateway/src/lib.rs

use clap::Parser;
use std::net::SocketAddr;
use uuid::Uuid;

/// Enforcement mode for the delegates-compute op-gate (`DELEGATES_COMPUTE_OP_GATE`).
///
/// Controls whether `POST /db/content` and `POST /db/content/bulk` are gated by a
/// pre-dispatch call to storage's `POST /api/v1/authorize-operation` before forwarding.
///
/// - `off` (default): no call, no latency — today's passthrough behavior.
/// - `observe`: gate fires; verdict is logged but request always forwards.
/// - `enforce`: gate fires; denied verdict or infra error returns 403 (fail-closed).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OpGateMode {
    /// Passthrough — no storage call, no latency.
    #[default]
    Off,
    /// Gate fires; verdicts are logged but request always forwards.
    Observe,
    /// Gate fires; denied verdict or infra error → 403 (fail-closed).
    Enforce,
}

impl std::str::FromStr for OpGateMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "off" | "" => Ok(Self::Off),
            "observe" => Ok(Self::Observe),
            "enforce" => Ok(Self::Enforce),
            other => Err(format!(
                "unknown op-gate mode {other:?}; expected off|observe|enforce"
            )),
        }
    }
}

impl std::fmt::Display for OpGateMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Observe => write!(f, "observe"),
            Self::Enforce => write!(f, "enforce"),
        }
    }
}

/// Doorway - WebSocket gateway for Elohim Holochain
///
/// "Knock and it shall be opened" - Matthew 7:7-8
#[derive(Parser, Debug, Clone)]
#[command(name = "doorway")]
#[command(about = "WebSocket gateway for Elohim Holochain infrastructure")]
pub struct Args {
    /// Unique node identifier for this gateway instance
    #[arg(long, env = "NODE_ID", default_value_t = Uuid::new_v4())]
    pub node_id: Uuid,

    /// Address to listen on
    #[arg(long, env = "LISTEN", default_value = "0.0.0.0:8080")]
    pub listen: SocketAddr,

    /// Holochain conductor admin WebSocket URL (default port 4444)
    /// Used for admin operations (list_apps, attach_app_interface, discovery)
    /// The app interface URL for zome calls is derived by replacing port with APP_PORT_MIN
    #[arg(long, env = "CONDUCTOR_URL", default_value = "ws://localhost:4444")]
    pub conductor_url: String,

    /// Explicit admin URL override (optional)
    /// If set, used for admin operations instead of CONDUCTOR_URL
    /// Useful when admin and app interfaces have different hosts
    #[arg(long, env = "CONDUCTOR_ADMIN_URL")]
    pub conductor_admin_url: Option<String>,

    /// Minimum app interface port
    #[arg(long, env = "APP_PORT_MIN", default_value = "4445")]
    pub app_port_min: u16,

    /// Maximum app interface port
    #[arg(long, env = "APP_PORT_MAX", default_value = "65535")]
    pub app_port_max: u16,

    /// Enable development mode (disables auth, enables passthrough)
    #[arg(long, env = "DEV_MODE", default_value = "false")]
    pub dev_mode: bool,

    /// Opt in to the multi-peer signal subscriber while in dev mode.
    /// Dev mode skips the subscriber by default because many dev contexts run
    /// with no conductor at all and would spin reconnect noise. Auth is NOT the
    /// blocker — the subscriber self-authenticates via TypedAppClient — so set
    /// this when the dev stack fronts real conductors (e.g. hc-mesh.sh).
    /// No effect outside dev mode; PROJECTION_WRITER=false still disables.
    #[arg(long, env = "DEV_SIGNAL_SUBSCRIBER", default_value = "false")]
    pub dev_signal_subscriber: bool,

    /// NATS configuration
    #[command(flatten)]
    pub nats: NatsArgs,

    /// MongoDB connection URI
    #[arg(long, env = "MONGODB_URI", default_value = "mongodb://localhost:27017")]
    pub mongodb_uri: String,

    /// MongoDB database name
    #[arg(long, env = "MONGODB_DB", default_value = "doorway")]
    pub mongodb_db: String,

    /// JWT secret for token signing (required in production)
    #[arg(long, env = "JWT_SECRET")]
    pub jwt_secret: Option<String>,

    /// JWT token expiry in seconds
    #[arg(long, env = "JWT_EXPIRY_SECONDS", default_value = "3600")]
    pub jwt_expiry_seconds: u64,

    /// API key for authenticated access (optional, for backward compat)
    #[arg(long, env = "API_KEY_AUTHENTICATED")]
    pub api_key_authenticated: Option<String>,

    /// API key for admin access (optional, for backward compat)
    #[arg(long, env = "API_KEY_ADMIN")]
    pub api_key_admin: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Request timeout in milliseconds
    #[arg(long, env = "REQUEST_TIMEOUT_MS", default_value = "30000")]
    pub request_timeout_ms: u64,

    /// Number of internal worker tasks (for concurrent request processing)
    #[arg(long, env = "WORKER_COUNT", default_value = "4")]
    pub worker_count: usize,

    /// Enable bootstrap service for agent discovery
    #[arg(long, env = "BOOTSTRAP_ENABLED", default_value = "true")]
    pub bootstrap_enabled: bool,

    /// Enable signal service for WebRTC signaling
    #[arg(long, env = "SIGNAL_ENABLED", default_value = "true")]
    pub signal_enabled: bool,

    /// Maximum signal connections
    #[arg(long, env = "SIGNAL_MAX_CLIENTS")]
    pub signal_max_clients: Option<usize>,

    /// Signal rate limit in kbps per IP
    #[arg(long, env = "SIGNAL_RATE_LIMIT_KBPS")]
    pub signal_rate_limit_kbps: Option<i32>,

    /// Signal idle timeout in milliseconds
    #[arg(long, env = "SIGNAL_IDLE_TIMEOUT_MS")]
    pub signal_idle_timeout_ms: Option<i32>,

    /// Enable orchestrator for cluster management (mDNS discovery, node provisioning)
    #[arg(long, env = "ORCHESTRATOR_ENABLED", default_value = "false")]
    pub orchestrator_enabled: bool,

    /// Region identifier for this doorway instance (for locality-aware routing)
    #[arg(long, env = "REGION")]
    pub region: Option<String>,

    /// Doorway identifier for federation (e.g., "alpha-elohim-host")
    /// Used in JWT claims to identify token issuer
    #[arg(long, env = "DOORWAY_ID")]
    pub doorway_id: Option<String>,

    /// Enable the self-hostable pkarr resolver endpoint at /pkarr/{key}.
    /// See genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
    /// (cutover gate #10).
    #[arg(long, env = "DOORWAY_PKARR_RESOLVER_ENABLED", default_value_t = false)]
    pub pkarr_resolver_enabled: bool,

    /// LRU cache capacity for pkarr packets. Default 1000.
    #[arg(long, env = "DOORWAY_PKARR_CACHE_CAPACITY")]
    pub pkarr_cache_capacity: Option<usize>,

    /// If set, persist the pkarr cache to <dir>/packets.bin across restarts.
    #[arg(long, env = "DOORWAY_PKARR_CACHE_DIR")]
    pub pkarr_cache_dir: Option<std::path::PathBuf>,

    /// Public URL of this doorway for cross-doorway validation
    /// (e.g., "https://alpha.elohim.host")
    #[arg(long, env = "DOORWAY_URL")]
    pub doorway_url: Option<String>,

    /// Additional public gateway addresses for this same doorway identity.
    /// Lower list positions receive higher priority in the signed registration.
    #[arg(long, env = "DOORWAY_URLS", value_delimiter = ',')]
    pub doorway_urls: Vec<String>,

    /// Cache TTL carried by signed doorway endpoint records.
    #[arg(long, env = "DOORWAY_ENDPOINT_TTL_SECS", default_value_t = 300)]
    pub doorway_endpoint_ttl_secs: u32,

    /// URL of elohim-storage for blob storage
    /// (e.g., "http://localhost:8091")
    /// Doorway forwards blobs here for authoritative storage
    #[arg(long, env = "STORAGE_URL")]
    pub storage_url: Option<String>,

    /// Additional storage peer URLs (comma-separated)
    /// Each URL registers as a peer in the route registry via /manifest
    #[arg(long, env = "STORAGE_URLS", value_delimiter = ',')]
    pub storage_urls: Vec<String>,

    /// URL of elohim-agent-sdk sidecar for AI agent invocation
    /// (e.g., "http://localhost:8095")
    /// Doorway proxies /api/v1/elohim/invoke requests here
    #[arg(
        long,
        env = "ELOHIM_AGENT_URL",
        default_value = "http://localhost:8095"
    )]
    pub elohim_agent_url: String,

    /// URL of the elohim-node compute endpoint (e.g., "http://localhost:8091")
    /// When set, doorway tries elohim-node first for /api/v1/elohim/* requests,
    /// falling back to elohim-agent-sdk sidecar if node is unreachable.
    #[arg(long, env = "ELOHIM_NODE_URL", default_value = "")]
    pub elohim_node_url: String,

    /// URL of doorway-app for operator dashboard
    /// (e.g., "http://localhost:8081")
    /// Doorway proxies /threshold/* requests here
    #[arg(long, env = "THRESHOLD_URL", default_value = "http://localhost:8081")]
    pub threshold_url: String,

    /// Whether this instance runs the projection signal subscriber
    /// When true (default): starts signal subscriber to populate projection from DHT signals
    /// When false: reads projection from shared MongoDB, no subscriber (read replica mode)
    #[arg(long, env = "PROJECTION_WRITER", default_value = "true")]
    pub projection_writer: bool,

    /// Comma-separated list of conductor app interface URLs for multi-conductor pool
    /// e.g. "ws://cond-0:4445,ws://cond-1:4445"
    /// If set, takes precedence over CONDUCTOR_URL for the conductor pool
    #[arg(long, env = "CONDUCTOR_URLS")]
    pub conductor_urls: Option<String>,

    /// Holochain installed app ID for projections and signal subscriptions
    #[arg(long, env = "INSTALLED_APP_ID", default_value = "elohim")]
    pub installed_app_id: String,

    /// Path to the hApp bundle for auto-provisioning new agents
    #[arg(long, env = "HAPP_BUNDLE_PATH", default_value = "/app/elohim.happ")]
    pub happ_bundle_path: String,

    /// Admin port for orchestrator mDNS advertisement (defaults to conductor admin port)
    #[arg(long, env = "ORCHESTRATOR_ADMIN_PORT", default_value = "8888")]
    pub orchestrator_admin_port: u16,

    /// Comma-separated list of allowed CORS origins.
    /// In dev mode this is ignored (all origins allowed).
    /// e.g. "https://elohim.host,https://alpha.elohim.host,tauri://localhost"
    #[arg(long, env = "CORS_ORIGINS", value_delimiter = ',')]
    pub cors_origins: Vec<String>,

    /// Comma-separated list of peer doorway URLs for federation discovery
    /// Each peer is queried at startup and periodically for cross-doorway awareness
    /// e.g. "https://doorway-staging.elohim.host,https://doorway.elohim.host"
    #[arg(long, env = "FEDERATION_PEERS", value_delimiter = ',')]
    pub federation_peers: Vec<String>,

    /// Bootstrap URL for P2P discovery (Holochain kitsune bootstrap)
    /// Returned in native-handoff response for Tauri clients to join network
    #[arg(long, env = "BOOTSTRAP_URL")]
    pub bootstrap_url: Option<String>,

    /// Signal relay URL for WebRTC signaling
    /// Returned in native-handoff response for Tauri clients
    #[arg(long, env = "SIGNAL_URL")]
    pub signal_url: Option<String>,

    /// Headless service base name for StatefulSet peer discovery
    /// e.g., "elohim-edgenode-staging" → queries elohim-edgenode-staging-{N}.elohim-edgenode-staging-headless:8090
    #[arg(long, env = "HEADLESS_SERVICE_BASE")]
    pub headless_service_base: Option<String>,

    /// Number of StatefulSet replicas for P2P peer enumeration
    #[arg(long, env = "STATEFULSET_REPLICAS")]
    pub statefulset_replicas: Option<u32>,

    /// Delegates-compute op-gate enforcement mode.
    ///
    /// Controls whether `POST /db/content` and `POST /db/content/bulk` are gated by a
    /// pre-dispatch call to storage's `POST /api/v1/authorize-operation`.
    ///
    /// - `off` (default): no call, no latency — existing passthrough behavior.
    /// - `observe`: gate fires; result is logged but request always forwards.
    /// - `enforce`: gate fires; denied verdict or infra error returns 403 (fail-closed).
    #[arg(
        long = "delegates-compute-op-gate",
        env = "DELEGATES_COMPUTE_OP_GATE",
        default_value = "off"
    )]
    pub op_gate_mode: OpGateMode,

    /// Run in CHE-facing (keyless peer-client) mode.
    ///
    /// When `true`, the doorway refuses to start unless:
    /// - `DELEGATES_COMPUTE_OP_GATE=enforce`
    /// - `DEV_MODE` is off
    /// - `JWT_SECRET` is set and at least 32 characters
    #[arg(long, env = "CHE_FACING", default_value = "false")]
    pub che_facing: bool,
}

/// NATS connection configuration
#[derive(Parser, Debug, Clone)]
pub struct NatsArgs {
    /// NATS server URL
    #[arg(long, env = "NATS_URL", default_value = "nats://127.0.0.1:4222")]
    pub nats_url: String,

    /// NATS username (optional)
    #[arg(long, env = "NATS_USER")]
    pub nats_user: Option<String>,

    /// NATS password (optional)
    #[arg(long, env = "NATS_PASSWORD")]
    pub nats_password: Option<String>,
}

/// Parse the port out of a `scheme://host:port` URL. `None` when absent or
/// unparseable — the caller then falls back to deriving rather than guessing.
fn url_port(url: &str) -> Option<u16> {
    let after_scheme = url.split("://").nth(1)?;
    let host_port = after_scheme.split('/').next()?;
    host_port.rsplit(':').next()?.parse::<u16>().ok()
}

impl Args {
    /// The DECLARED storage peers, in declared order: `--storage-url` first,
    /// then `--storage-urls`, trimmed, de-duplicated, empties dropped.
    ///
    /// This is the ONE derivation of peer declaration order. It is what
    /// `RouteRegistry::declare_peer_priority` is fed at boot (index = rank,
    /// 0 = primary), what the EPR storage pool is built from, and what
    /// `ServingHealth` reads to decide which upstream is this doorway's
    /// PRIMARY. Those three used to derive the same order independently; a
    /// drift between any two of them would make the health surface classify
    /// against a different primary than the router routes to.
    ///
    /// Empty means "no storage peer declared" — a real third answer, not a
    /// primary of `""`.
    pub fn declared_storage_peers(&self) -> Vec<String> {
        let mut peers: Vec<String> = Vec::new();
        let declared = self
            .storage_url
            .iter()
            .map(String::as_str)
            .chain(self.storage_urls.iter().map(String::as_str));
        for url in declared {
            let trimmed = url.trim().to_string();
            if !trimmed.is_empty() && !peers.contains(&trimmed) {
                peers.push(trimmed);
            }
        }
        peers
    }

    /// This doorway's DECLARED PRIMARY storage endpoint (rank 0), normalized
    /// the way the breaker map and `declare_peer_priority` key endpoints
    /// (trailing slash stripped). `None` when nothing is declared.
    pub fn declared_primary_storage_peer(&self) -> Option<String> {
        self.declared_storage_peers()
            .into_iter()
            .next()
            .map(|url| url.trim_end_matches('/').to_string())
    }

    /// Get effective admin URL (falls back to conductor_url if not set)
    pub fn admin_url(&self) -> &str {
        self.conductor_admin_url
            .as_deref()
            .unwrap_or(&self.conductor_url)
    }

    /// Get effective JWT secret (uses default in dev mode)
    pub fn jwt_secret(&self) -> String {
        if self.dev_mode {
            self.jwt_secret
                .clone()
                .unwrap_or_else(|| "dev-only-insecure-secret".to_string())
        } else {
            self.jwt_secret
                .clone()
                .expect("JWT_SECRET is required in production mode")
        }
    }

    /// Get the list of conductor **app interface** URLs.
    ///
    /// Prefers `CONDUCTOR_URLS` (multi-conductor, already app URLs — the deploy
    /// pipeline's `computeConductorUrls` emits `:8445`) over the single
    /// `CONDUCTOR_URL`, which by its own flag documentation is the **admin**
    /// URL and therefore has to be CONVERTED, not passed through.
    ///
    /// The pass-through was a real cross-convention bug (measured 2026-08-22 on
    /// the household mesh, where `--conductor-url ws://localhost:4444` is the
    /// admin socket): the single admin URL entered this app-URL list verbatim,
    /// so the conductor registry then ran `derive_admin_url_from_app` (port − 1)
    /// on an ADMIN url and registered `ws://localhost:4443` — a port nothing
    /// listens on. Every registry-keyed admin consumer (the projection signal
    /// subscriber, agent-registry refresh, the post-restart app-auth re-mint)
    /// dialled that dead socket forever: `Conductor connection failed: Admin
    /// WebSocket connect to ws://localhost:4443 failed: Connection refused`,
    /// climbing its own backoff ladder and never healing.
    pub fn conductor_url_list(&self) -> Vec<String> {
        if let Some(ref urls) = self.conductor_urls {
            urls.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![self.single_conductor_app_url()]
        }
    }

    /// The single `CONDUCTOR_URL` expressed as an APP-interface URL.
    ///
    /// The port decides, so BOTH live conventions keep working and neither has
    /// to be guessed at: a URL already inside the configured app-port range is
    /// an app URL and passes through untouched (alpha's
    /// `--conductor-url ws://elohim-matthew-alpha:4445`, and any socat topology
    /// whose app port is not `APP_PORT_MIN`); anything else is the admin URL the
    /// flag documents (`hc-mesh.sh` / `hc-start.sh` pass `:4444`) and is
    /// converted to `APP_PORT_MIN`.
    fn single_conductor_app_url(&self) -> String {
        if let Some(port) = url_port(&self.conductor_url) {
            if self.is_valid_app_port(port) {
                return self.conductor_url.clone();
            }
        }
        crate::derive_app_url_from_conductor(&self.conductor_url, self.app_port_min)
    }

    /// Check if an app port is within the allowed range
    pub fn is_valid_app_port(&self, port: u16) -> bool {
        port >= self.app_port_min && port <= self.app_port_max
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.dev_mode && self.jwt_secret.is_none() {
            return Err("JWT_SECRET is required in production mode".to_string());
        }

        if self.app_port_min > self.app_port_max {
            return Err("APP_PORT_MIN must be less than or equal to APP_PORT_MAX".to_string());
        }

        // CHE_FACING: enforce a minimal security baseline for keyless peer-client deployments.
        // I4: mode must be enforce; I5: dev_mode off + jwt_secret present and ≥32 chars.
        if self.che_facing {
            if self.op_gate_mode != OpGateMode::Enforce {
                return Err("CHE_FACING=1 requires DELEGATES_COMPUTE_OP_GATE=enforce".to_string());
            }
            if self.dev_mode {
                return Err("CHE_FACING=1 is incompatible with DEV_MODE=true".to_string());
            }
            match &self.jwt_secret {
                None => {
                    return Err("CHE_FACING=1 requires JWT_SECRET to be set".to_string());
                }
                Some(s) if s.len() < 32 => {
                    return Err(format!(
                        "CHE_FACING=1 requires JWT_SECRET to be at least 32 characters (got {})",
                        s.len()
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod conductor_url_convention_tests {
    use super::*;
    use clap::Parser;

    fn args_with(argv: &[&str]) -> Args {
        let mut full = vec!["doorway", "--listen", "127.0.0.1:0"];
        full.extend_from_slice(argv);
        Args::parse_from(full)
    }

    /// The mesh convention (and the flag's own documentation):
    /// `--conductor-url` is the ADMIN socket. The registry consumes this list as
    /// APP urls and runs `derive_admin_url_from_app` (port − 1) over it, so an
    /// admin URL passed through verbatim became `4443` — a port nothing listens
    /// on, and the reason the projection subscriber, the agent-registry refresh
    /// and the post-restart app-auth re-mint never connected on the household
    /// mesh (measured 2026-08-22).
    #[test]
    fn an_admin_conductor_url_is_converted_to_the_app_port() {
        let args = args_with(&["--conductor-url", "ws://localhost:4444"]);
        assert_eq!(args.conductor_url_list(), vec!["ws://localhost:4445"]);
        assert_eq!(
            crate::derive_admin_url_from_app(&args.conductor_url_list()[0]),
            "ws://localhost:4444",
            "and the registry's own inverse now lands back on the real admin port"
        );
    }

    /// The alpha convention: `--conductor-url ws://elohim-matthew-alpha:4445` is
    /// already an APP url. A URL inside the app-port range passes through
    /// untouched, so this fix cannot move a working deployment.
    #[test]
    fn an_app_conductor_url_passes_through_untouched() {
        let args = args_with(&["--conductor-url", "ws://elohim-matthew-alpha:4445"]);
        assert_eq!(
            args.conductor_url_list(),
            vec!["ws://elohim-matthew-alpha:4445"]
        );
    }

    /// A socat topology whose app port is not `APP_PORT_MIN` (the 8444/8445
    /// convention `derive_admin_url_from_app` documents) still passes through —
    /// the port range, not a hardcoded number, is what decides.
    #[test]
    fn a_non_default_app_port_inside_the_range_passes_through() {
        let args = args_with(&["--conductor-url", "ws://peer:8445"]);
        assert_eq!(args.conductor_url_list(), vec!["ws://peer:8445"]);
    }

    /// `CONDUCTOR_URLS` (plural) is authoritative and already app URLs — the
    /// deploy pipeline's `computeConductorUrls` emits them. Untouched.
    #[test]
    fn the_plural_list_is_never_rewritten() {
        let args = args_with(&["--conductor-urls", "ws://a:4445,ws://b:8445"]);
        assert_eq!(
            args.conductor_url_list(),
            vec!["ws://a:4445", "ws://b:8445"]
        );
    }

    #[test]
    fn url_port_parses_or_declines() {
        assert_eq!(url_port("ws://localhost:4444"), Some(4444));
        assert_eq!(url_port("ws://localhost:4444/path"), Some(4444));
        assert_eq!(url_port("ws://localhost"), None);
        assert_eq!(url_port("localhost:4444"), None);
    }
}
