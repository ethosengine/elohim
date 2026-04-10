//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8
//!
//! Doorway provides authenticated WebSocket access to Holochain conductors
//! with support for multiple operators via NATS messaging and MongoDB storage.
//!
//! ## Services
//!
//! - **Gateway**: WebSocket proxy to Holochain admin and app interfaces
//! - **Bootstrap**: Agent discovery service for DHT networks
//! - **Signal**: WebRTC signaling relay (SBD protocol)
//! - **Cache**: In-memory content caching for REST API
//! - **Projection**: DHT → MongoDB projection engine for fast reads
//! - **Orchestrator**: Plug-n-play node management with mDNS discovery

pub mod getrandom_custom;

pub mod auth;
pub mod bootstrap;
pub mod cache;
pub mod conductor;
pub mod config;
pub mod cors;
pub mod custodial_keys;
pub mod db;
pub mod hosts;
pub mod logging;
pub mod nats;
pub mod orchestrator;
pub mod projection;
pub mod proxy;
pub mod routes;
pub mod server;
pub mod services;
pub mod signal;
pub mod signing;
pub mod types;
pub mod worker;

pub use config::Args;
pub use server::{run, AppState};
pub use types::{DoorwayError, Result};

/// Derive admin WebSocket URL from app URL by replacing the port.
///
/// The admin port is the app port minus 1 (socat convention: 8444=admin, 8445=app).
/// Exposed at crate root so both `routes::zome_helpers` and `routes::auth_routes` can
/// construct temporary `ZomeCaller` instances without depending on `main`.
pub fn derive_admin_url_from_app(app_url: &str) -> String {
    if let Some(host_start) = app_url.find("://") {
        let after_scheme = &app_url[host_start + 3..];
        if let Some(port_start) = after_scheme.rfind(':') {
            let host = &after_scheme[..port_start];
            let port_str = &after_scheme[port_start + 1..];
            let admin_port = port_str
                .parse::<u16>()
                .map(|p| p.saturating_sub(1))
                .unwrap_or(4444);
            return format!("{}://{}:{}", &app_url[..host_start], host, admin_port);
        }
    }
    "ws://localhost:4444".to_string()
}
