//! Dashboard - Web UI for node status, setup, and management
//!
//! Provides:
//! - System resource metrics (CPU, memory, disk, network)
//! - Node discovery status
//! - Setup wizard (join network or become doorway)
//! - Cluster health overview

pub mod discovery;
pub mod metrics;
pub mod routes;
pub mod setup;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::network::NetworkMembership;

/// Dashboard state shared across handlers
pub struct DashboardState {
    pub config: Config,
    pub setup_complete: bool,
    pub discovered_peers: Vec<DiscoveredPeer>,
    pub pairing_requests: Vec<PairingRequest>,
    pub network: NetworkMembership,
}

/// A peer discovered on the local network
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredPeer {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub hostname: Option<String>,
    pub node_type: PeerType,
    pub discovered_at: u64,
}

/// Type of discovered peer
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub enum PeerType {
    /// elohim-node (always-on node)
    Node,
    /// elohim-app (Tauri desktop/mobile)
    App,
    /// doorway (bootstrap server)
    Doorway,
    /// Unknown peer type
    Unknown,
}

/// A pairing request from a discovered peer
#[derive(Debug, Clone, serde::Serialize)]
pub struct PairingRequest {
    pub request_id: String,
    pub from_peer: DiscoveredPeer,
    pub requested_at: u64,
    pub status: PairingStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub enum PairingStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

pub type SharedState = Arc<RwLock<DashboardState>>;

/// Create the dashboard router
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        // Dashboard pages
        .route("/", get(routes::index))
        .route("/setup", get(routes::setup_page))
        // API endpoints
        .route("/api/status", get(routes::api_status))
        .route("/api/metrics", get(routes::api_metrics))
        .route("/api/discovery/peers", get(routes::api_discovered_peers))
        .route("/api/discovery/scan", post(routes::api_scan_network))
        .route("/api/pairing/requests", get(routes::api_pairing_requests))
        .route("/api/pairing/approve", post(routes::api_approve_pairing))
        .route("/api/pairing/reject", post(routes::api_reject_pairing))
        .route("/api/setup/join", post(routes::api_setup_join))
        .route("/api/setup/doorway", post(routes::api_setup_doorway))
        // Update API
        .route("/api/update/status", get(routes::api_update_status))
        .route("/api/update/check", post(routes::api_update_check))
        .route("/api/update/apply", post(routes::api_update_apply))
        .route("/api/update/rollback", post(routes::api_update_rollback))
        // Network API
        .route("/api/network/status", get(routes::api_network_status))
        .route("/api/network/apps", get(routes::api_connected_apps))
        .route("/api/node/info", get(routes::api_node_info))
        // Multi-node dashboard API
        .route("/api/nodes", get(routes::api_list_nodes))
        .route("/api/proxy", post(routes::api_proxy_node))
        // Health check
        .route("/health", get(routes::health))
        // Static files
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .with_state(state)
}

impl DashboardState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            setup_complete: false,
            discovered_peers: Vec::new(),
            pairing_requests: Vec::new(),
            network: NetworkMembership::new(),
        }
    }
}
