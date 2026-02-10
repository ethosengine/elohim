//! Configuration for Doorway
//!
//! CLI arguments and environment variable handling using clap.
//! Pattern adapted from holo-host/rust/holo-gateway/src/lib.rs

use clap::Parser;
use std::net::SocketAddr;
use uuid::Uuid;

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

    /// Public URL of this doorway for cross-doorway validation
    /// (e.g., "https://alpha.elohim.host")
    #[arg(long, env = "DOORWAY_URL")]
    pub doorway_url: Option<String>,

    /// URL of elohim-storage for blob storage
    /// (e.g., "http://localhost:8091")
    /// Doorway forwards blobs here for authoritative storage
    #[arg(long, env = "STORAGE_URL")]
    pub storage_url: Option<String>,

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

    /// Admin port for orchestrator mDNS advertisement (defaults to conductor admin port)
    #[arg(long, env = "ORCHESTRATOR_ADMIN_PORT", default_value = "8888")]
    pub orchestrator_admin_port: u16,

    /// Bootstrap URL for P2P discovery (Holochain kitsune bootstrap)
    /// Returned in native-handoff response for Tauri clients to join network
    #[arg(long, env = "BOOTSTRAP_URL")]
    pub bootstrap_url: Option<String>,

    /// Signal relay URL for WebRTC signaling
    /// Returned in native-handoff response for Tauri clients
    #[arg(long, env = "SIGNAL_URL")]
    pub signal_url: Option<String>,
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

impl Args {
    /// Get effective admin URL (falls back to conductor_url if not set)
    pub fn admin_url(&self) -> &str {
        self.conductor_admin_url.as_deref().unwrap_or(&self.conductor_url)
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

    /// Get the list of conductor app interface URLs
    /// Prefers CONDUCTOR_URLS (multi-conductor) over single CONDUCTOR_URL
    pub fn conductor_url_list(&self) -> Vec<String> {
        if let Some(ref urls) = self.conductor_urls {
            urls.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![self.conductor_url.clone()]
        }
    }

    /// Check if an app port is within the allowed range
    pub fn is_valid_app_port(&self, port: u16) -> bool {
        port >= self.app_port_min && port <= self.app_port_max
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.dev_mode {
            if self.jwt_secret.is_none() {
                return Err("JWT_SECRET is required in production mode".to_string());
            }
        }

        if self.app_port_min > self.app_port_max {
            return Err("APP_PORT_MIN must be less than or equal to APP_PORT_MAX".to_string());
        }

        Ok(())
    }
}
