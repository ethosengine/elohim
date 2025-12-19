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

    /// Holochain conductor admin WebSocket URL
    #[arg(long, env = "CONDUCTOR_URL", default_value = "ws://localhost:4444")]
    pub conductor_url: String,

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
