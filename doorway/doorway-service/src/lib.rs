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
pub mod metrics;
pub mod nats;
pub mod orchestrator;
pub mod projection;
pub mod proxy;
pub mod render;
pub mod routes;
pub mod server;
pub mod services;
pub mod signal;
pub mod signing;
pub mod ssr;
pub mod types;
pub mod worker;

pub use config::Args;
pub use server::{run, AppState};
pub use types::{DoorwayError, Result};

/// Derive the app-interface WebSocket URL from a conductor URL by replacing the
/// port with `app_port`.
///
/// The inverse of [`derive_admin_url_from_app`], and the reason BOTH exist: the
/// doorway is configured with two DIFFERENT conventions and they must not be
/// confused. `CONDUCTOR_URL` (singular) is the **admin** URL — that is the
/// flag's own documented contract, and it is what `hc-mesh.sh` / `hc-start.sh`
/// pass. `CONDUCTOR_URLS` (plural) is a list of **app** URLs — that is what the
/// deploy pipeline's `computeConductorUrls` emits (`:8445`, the socat app port).
/// Anything that needs the other side derives it; nothing assumes.
pub fn derive_app_url_from_conductor(conductor_url: &str, app_port: u16) -> String {
    if let Some(host_start) = conductor_url.find("://") {
        let after_scheme = &conductor_url[host_start + 3..];
        if let Some(port_start) = after_scheme.rfind(':') {
            let host = &after_scheme[..port_start];
            return format!("{}://{}:{}", &conductor_url[..host_start], host, app_port);
        }
    }
    format!("ws://localhost:{app_port}")
}

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
