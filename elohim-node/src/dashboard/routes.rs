//! Dashboard HTTP routes
//!
//! Handlers for dashboard pages and API endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use super::{
    metrics::{collect_metrics, NodeMetrics},
    setup::{setup_doorway, setup_join_network, DoorwayConfig, JoinNetworkConfig, SetupResult},
    DiscoveredPeer, PairingRequest, PairingStatus, SharedState,
};
use crate::network::{
    sync_state::ConnectedAppsSummary, ConnectedApp, Operator, RegistrationStatus, SyncProgress,
};
use crate::update::{UpdateStatus, CURRENT_VERSION};

/// Dashboard index page
pub async fn index(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;

    if state.setup_complete {
        Html(include_str!("../../static/dashboard.html"))
    } else {
        Html(include_str!("../../static/setup.html"))
    }
}

/// Setup wizard page
pub async fn setup_page() -> impl IntoResponse {
    Html(include_str!("../../static/setup.html"))
}

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    "OK"
}

// === API Endpoints ===

/// Node status response
#[derive(Serialize)]
pub struct StatusResponse {
    pub node_id: String,
    pub hostname: String,
    pub setup_complete: bool,
    pub cluster_name: Option<String>,
    pub cluster_role: Option<String>,
    pub uptime_secs: u64,
    pub version: String,
}

/// GET /api/status
pub async fn api_status(State(state): State<SharedState>) -> Json<StatusResponse> {
    let state = state.read().await;

    Json(StatusResponse {
        node_id: state.config.node.id.clone(),
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        setup_complete: state.setup_complete,
        cluster_name: if state.setup_complete {
            Some(state.config.node.cluster_name.clone())
        } else {
            None
        },
        cluster_role: if state.setup_complete {
            Some("replica".to_string())
        } else {
            None
        },
        uptime_secs: 0, // TODO: Track actual uptime
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// GET /api/metrics
pub async fn api_metrics(State(state): State<SharedState>) -> Json<NodeMetrics> {
    let state = state.read().await;
    Json(collect_metrics(&state.config.node.id, state.setup_complete))
}

/// GET /api/discovery/peers
pub async fn api_discovered_peers(State(state): State<SharedState>) -> Json<Vec<DiscoveredPeer>> {
    let state = state.read().await;
    Json(state.discovered_peers.clone())
}

/// POST /api/discovery/scan - Trigger network scan
#[derive(Serialize)]
pub struct ScanResponse {
    pub peers_found: usize,
    pub peers: Vec<DiscoveredPeer>,
}

pub async fn api_scan_network(State(state): State<SharedState>) -> Json<ScanResponse> {
    // TODO: Implement actual mDNS scan
    let state = state.read().await;

    Json(ScanResponse {
        peers_found: state.discovered_peers.len(),
        peers: state.discovered_peers.clone(),
    })
}

/// GET /api/pairing/requests
pub async fn api_pairing_requests(State(state): State<SharedState>) -> Json<Vec<PairingRequest>> {
    let state = state.read().await;
    Json(state.pairing_requests.clone())
}

/// POST /api/pairing/approve
#[derive(Deserialize)]
pub struct ApproveRequest {
    pub request_id: String,
}

#[derive(Serialize)]
pub struct ApproveResponse {
    pub success: bool,
    pub message: String,
}

pub async fn api_approve_pairing(
    State(state): State<SharedState>,
    Json(req): Json<ApproveRequest>,
) -> Json<ApproveResponse> {
    let mut state = state.write().await;

    if let Some(request) = state
        .pairing_requests
        .iter_mut()
        .find(|r| r.request_id == req.request_id)
    {
        request.status = PairingStatus::Approved;

        // TODO: Send approval message to peer with operator keys

        Json(ApproveResponse {
            success: true,
            message: format!("Approved pairing request {}", req.request_id),
        })
    } else {
        Json(ApproveResponse {
            success: false,
            message: "Pairing request not found".to_string(),
        })
    }
}

/// POST /api/pairing/reject
#[derive(Deserialize)]
pub struct RejectRequest {
    pub request_id: String,
    #[allow(dead_code)]
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct RejectResponse {
    pub success: bool,
    pub message: String,
}

pub async fn api_reject_pairing(
    State(state): State<SharedState>,
    Json(req): Json<RejectRequest>,
) -> Json<RejectResponse> {
    let mut state = state.write().await;

    if let Some(request) = state
        .pairing_requests
        .iter_mut()
        .find(|r| r.request_id == req.request_id)
    {
        request.status = PairingStatus::Rejected;

        // TODO: Send rejection message to peer

        Json(RejectResponse {
            success: true,
            message: format!("Rejected pairing request {}", req.request_id),
        })
    } else {
        Json(RejectResponse {
            success: false,
            message: "Pairing request not found".to_string(),
        })
    }
}

/// POST /api/setup/join - Join existing network
pub async fn api_setup_join(
    State(state): State<SharedState>,
    Json(config): Json<JoinNetworkConfig>,
) -> Json<SetupResult> {
    let result = setup_join_network(config).await;

    if result.success {
        let mut state = state.write().await;
        state.setup_complete = true;
    }

    Json(result)
}

/// POST /api/setup/doorway - Become a doorway node
pub async fn api_setup_doorway(
    State(state): State<SharedState>,
    Json(config): Json<DoorwayConfig>,
) -> Json<SetupResult> {
    let result = setup_doorway(config).await;

    if result.success {
        let mut state = state.write().await;
        state.setup_complete = true;
    }

    Json(result)
}

// === Update API Endpoints ===

/// Update status response
#[derive(Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub status: UpdateStatus,
    pub auto_update_enabled: bool,
}

/// GET /api/update/status
pub async fn api_update_status(State(state): State<SharedState>) -> Json<UpdateStatusResponse> {
    let state = state.read().await;

    Json(UpdateStatusResponse {
        current_version: CURRENT_VERSION.to_string(),
        status: UpdateStatus::UpToDate, // TODO: Get from update service
        auto_update_enabled: state.config.update.enabled,
    })
}

/// Check for updates request
#[derive(Deserialize)]
pub struct CheckUpdatesRequest {
    pub doorway_url: Option<String>,
}

/// POST /api/update/check - Check for updates
pub async fn api_update_check(
    State(state): State<SharedState>,
    Json(req): Json<CheckUpdatesRequest>,
) -> Json<UpdateStatusResponse> {
    use crate::update::UpdateService;

    let state_guard = state.read().await;
    let config = state_guard.config.update.clone();
    drop(state_guard);

    let mut update_service = UpdateService::new(config.clone());

    // Use provided doorway URL or try to get from config
    if let Some(url) = req.doorway_url {
        update_service.set_doorway(url);
    }

    let status = match update_service.check_for_updates().await {
        Ok(s) => s,
        Err(e) => UpdateStatus::Failed {
            error: e.to_string(),
        },
    };

    Json(UpdateStatusResponse {
        current_version: CURRENT_VERSION.to_string(),
        status,
        auto_update_enabled: config.enabled,
    })
}

/// POST /api/update/apply - Apply available update
pub async fn api_update_apply(
    State(state): State<SharedState>,
    Json(req): Json<CheckUpdatesRequest>,
) -> Json<UpdateStatusResponse> {
    use crate::update::UpdateService;

    let state_guard = state.read().await;
    let config = state_guard.config.update.clone();
    drop(state_guard);

    let mut update_service = UpdateService::new(config.clone());

    if let Some(url) = req.doorway_url {
        update_service.set_doorway(url);
    }

    // First check for updates
    if let Err(e) = update_service.check_for_updates().await {
        return Json(UpdateStatusResponse {
            current_version: CURRENT_VERSION.to_string(),
            status: UpdateStatus::Failed {
                error: e.to_string(),
            },
            auto_update_enabled: config.enabled,
        });
    }

    // Apply update
    let status = match update_service.apply_update().await {
        Ok(_) => update_service.status().clone(),
        Err(e) => UpdateStatus::Failed {
            error: e.to_string(),
        },
    };

    Json(UpdateStatusResponse {
        current_version: CURRENT_VERSION.to_string(),
        status,
        auto_update_enabled: config.enabled,
    })
}

/// POST /api/update/rollback - Rollback to previous version
pub async fn api_update_rollback(State(state): State<SharedState>) -> Json<UpdateStatusResponse> {
    use crate::update::UpdateService;

    let state_guard = state.read().await;
    let config = state_guard.config.update.clone();
    drop(state_guard);

    let mut update_service = UpdateService::new(config.clone());

    let status = match update_service.rollback() {
        Ok(_) => UpdateStatus::PendingRestart {
            version: "previous".to_string(),
        },
        Err(e) => UpdateStatus::Failed {
            error: e.to_string(),
        },
    };

    Json(UpdateStatusResponse {
        current_version: CURRENT_VERSION.to_string(),
        status,
        auto_update_enabled: config.enabled,
    })
}

// === Network API Endpoints ===

/// Network membership response
#[derive(Serialize)]
pub struct NetworkStatusResponse {
    pub is_registered: bool,
    pub status: RegistrationStatus,
    pub operator: Option<Operator>,
    pub cluster_name: Option<String>,
    pub cluster_role: Option<String>,
    pub doorways: Vec<DoorwaySummary>,
    pub sync_progress: SyncProgress,
    pub connected_apps: ConnectedAppsSummary,
    pub registered_at: Option<u64>,
    pub last_heartbeat: Option<u64>,
}

/// Summary of a doorway connection
#[derive(Serialize)]
pub struct DoorwaySummary {
    pub url: String,
    pub is_primary: bool,
    pub status: String,
    pub last_contact: u64,
}

/// GET /api/network/status - Get network membership status
pub async fn api_network_status(State(state): State<SharedState>) -> Json<NetworkStatusResponse> {
    let state = state.read().await;
    let network = &state.network;

    let doorways: Vec<DoorwaySummary> = network
        .doorways
        .iter()
        .map(|d| DoorwaySummary {
            url: d.url.clone(),
            is_primary: d.is_primary,
            status: format!("{:?}", d.status),
            last_contact: d.last_contact,
        })
        .collect();

    let cluster_name = network.cluster.as_ref().map(|c| c.name.clone());
    let cluster_role = network.cluster.as_ref().map(|c| format!("{:?}", c.role));

    Json(NetworkStatusResponse {
        is_registered: network.is_registered(),
        status: network.status.clone(),
        operator: network.operator.clone(),
        cluster_name,
        cluster_role,
        doorways,
        sync_progress: network.sync_progress.clone(),
        connected_apps: ConnectedAppsSummary::from_apps(&network.connected_apps),
        registered_at: network.registered_at,
        last_heartbeat: network.last_heartbeat,
    })
}

/// GET /api/network/apps - Get connected apps
pub async fn api_connected_apps(State(state): State<SharedState>) -> Json<Vec<ConnectedApp>> {
    let state = state.read().await;
    Json(state.network.connected_apps.clone())
}

/// Node info for doorway/operator queries
#[derive(Serialize)]
pub struct NodeInfoResponse {
    pub node_id: String,
    pub hostname: String,
    pub version: String,
    pub cluster_name: Option<String>,
    pub cluster_role: Option<String>,
    pub status: String,
    pub sync_position: u64,
    pub sync_state: String,
    pub connected_apps: usize,
    pub uptime_secs: u64,
    pub hardware: HardwareInfo,
}

#[derive(Serialize)]
pub struct HardwareInfo {
    pub cpu_cores: usize,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub arch: String,
    pub os: String,
}

/// GET /api/node/info - Get node info for doorway queries
pub async fn api_node_info(State(state): State<SharedState>) -> Json<NodeInfoResponse> {
    let state = state.read().await;

    let cluster_name = state.network.cluster.as_ref().map(|c| c.name.clone());
    let cluster_role = state
        .network
        .cluster
        .as_ref()
        .map(|c| format!("{:?}", c.role));
    let sync_state = format!("{:?}", state.network.sync_progress.state);

    Json(NodeInfoResponse {
        node_id: state.config.node.id.clone(),
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        version: CURRENT_VERSION.to_string(),
        cluster_name,
        cluster_role,
        status: format!("{:?}", state.network.status),
        sync_position: state.network.sync_progress.position,
        sync_state,
        connected_apps: state.network.connected_apps.len(),
        uptime_secs: 0, // TODO: Track uptime
        hardware: HardwareInfo {
            cpu_cores: num_cpus::get(),
            memory_bytes: 0,
            storage_bytes: 0,
            arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
        },
    })
}

// === Node Discovery for Multi-Node Dashboard ===

/// Information about a node on the local network
#[derive(Serialize)]
pub struct NetworkNodeInfo {
    pub node_id: String,
    pub hostname: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub is_local: bool,
    pub status: String,
    pub version: Option<String>,
    pub cluster_name: Option<String>,
}

/// GET /api/nodes - List all nodes discovered on the local network
pub async fn api_list_nodes(State(state): State<SharedState>) -> Json<Vec<NetworkNodeInfo>> {
    let state = state.read().await;

    let mut nodes = Vec::new();

    // Add local node first
    let local_hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    nodes.push(NetworkNodeInfo {
        node_id: state.config.node.id.clone(),
        hostname: local_hostname,
        addresses: vec!["127.0.0.1".to_string()],
        port: state.config.api.http_port,
        is_local: true,
        status: "online".to_string(),
        version: Some(CURRENT_VERSION.to_string()),
        cluster_name: Some(state.config.node.cluster_name.clone()),
    });

    // Add discovered peers that are elohim-nodes
    for peer in &state.discovered_peers {
        if matches!(peer.node_type, super::PeerType::Node) {
            nodes.push(NetworkNodeInfo {
                node_id: peer.peer_id.clone(),
                hostname: peer
                    .hostname
                    .clone()
                    .unwrap_or_else(|| peer.peer_id.clone()),
                addresses: peer.addresses.clone(),
                port: state.config.api.http_port, // Assume same port
                is_local: false,
                status: "online".to_string(),
                version: None, // Would need to query
                cluster_name: None,
            });
        }
    }

    Json(nodes)
}

/// Proxy request to a remote node
#[derive(Deserialize)]
pub struct ProxyRequest {
    pub node_address: String,
    pub endpoint: String,
}

/// POST /api/proxy - Proxy a request to another node's API
pub async fn api_proxy_node(
    Json(req): Json<ProxyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use reqwest::Client;

    let client = Client::new();
    let url = format!("http://{}{}", req.node_address, req.endpoint);

    match client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => Ok(Json(data)),
                    Err(e) => Err((
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to parse response: {}", e),
                    )),
                }
            } else {
                Err((
                    StatusCode::BAD_GATEWAY,
                    format!("Remote node returned: {}", response.status()),
                ))
            }
        }
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            format!("Failed to reach node: {}", e),
        )),
    }
}
