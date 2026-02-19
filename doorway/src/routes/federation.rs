//! Federation Routes
//!
//! HTTP endpoints for doorway federation:
//! - `GET /api/v1/federation/doorways` — list known doorways from DHT
//! - `GET /.well-known/doorway-keys` — public signing key in JWKS format
//! - `GET /admin/federation/peers` — configured peer URLs with status
//! - `POST /admin/federation/peers` — add a federation peer
//! - `DELETE /admin/federation/peers` — remove a federation peer
//! - `POST /admin/federation/peers/refresh` — force peer cache refresh

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::AppState;
use crate::services::federation::{self, FederationConfig};

// =============================================================================
// Response Types
// =============================================================================

/// Doorway list response
#[derive(Serialize)]
pub struct FederationDoorwaysResponse {
    pub doorways: Vec<DoorwaySummary>,
    pub self_id: Option<String>,
    pub total: usize,
}

/// Summary of a doorway for the list endpoint
#[derive(Serialize)]
pub struct DoorwaySummary {
    pub id: String,
    pub url: String,
    pub region: Option<String>,
    pub tier: String,
    pub capabilities: Vec<String>,
    pub status: String,
}

/// JWKS response for doorway public keys
#[derive(Serialize)]
pub struct JwksResponse {
    pub keys: Vec<JwkKey>,
}

/// Single JWK entry
#[derive(Serialize)]
pub struct JwkKey {
    pub kty: String,
    pub crv: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub kid: String,
    pub x: String,
}

// =============================================================================
// Handlers
// =============================================================================

/// Handle GET /api/v1/federation/doorways
///
/// Lists known doorways from the infrastructure DHT.
/// If federation is not configured, returns only self (if doorway_id is set).
pub async fn handle_federation_doorways(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let self_id = state.args.doorway_id.clone();

    // Try to query DHT if we have a ZomeCaller
    let mut doorways = if let Some(ref zome_caller) = state.zome_caller {
        if let Some(config) = FederationConfig::from_args(&state.args) {
            match federation::get_all_doorways(zome_caller, &config).await {
                Ok(infos) => infos
                    .into_iter()
                    .map(|d| {
                        let capabilities: Vec<String> =
                            serde_json::from_str(&d.capabilities_json).unwrap_or_default();
                        DoorwaySummary {
                            id: d.id,
                            url: d.url,
                            region: d.region,
                            tier: d.tier,
                            capabilities,
                            status: "online".to_string(),
                        }
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Failed to query doorways from DHT: {}", e);
                    build_self_only_doorway(&state)
                }
            }
        } else {
            build_self_only_doorway(&state)
        }
    } else {
        build_self_only_doorway(&state)
    };

    // Merge in peer-discovered doorways from HTTP federation
    let peer_doorways = crate::services::federation::get_cached_peers(&state.peer_cache).await;
    let mut seen_ids: std::collections::HashSet<String> =
        doorways.iter().map(|d| d.id.clone()).collect();
    for peer in peer_doorways {
        if seen_ids.insert(peer.id.clone()) {
            doorways.push(DoorwaySummary {
                id: peer.id,
                url: peer.url,
                region: peer.region,
                tier: "Federated".to_string(),
                capabilities: peer.capabilities,
                status: "online".to_string(),
            });
        }
    }

    let total = doorways.len();
    let response = FederationDoorwaysResponse {
        doorways,
        self_id,
        total,
    };

    match serde_json::to_string_pretty(&response) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "public, max-age=30")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error": "Serialization failed: {e}"}}"#
            ))))
            .unwrap(),
    }
}

/// Handle GET /.well-known/doorway-keys
///
/// Returns public signing key in JWKS (JSON Web Key Set) format.
/// Used by other doorways to verify signatures from this doorway.
pub fn handle_doorway_keys(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let mut keys = Vec::new();

    // Add node signing key if available
    if let Some(ref verifying_key) = state.node_verifying_key {
        let pub_bytes = verifying_key.to_bytes();
        let x = base64_url_encode(&pub_bytes);

        keys.push(JwkKey {
            kty: "OKP".to_string(),
            crv: "Ed25519".to_string(),
            key_use: "sig".to_string(),
            kid: "node-key-1".to_string(),
            x,
        });
    }

    let response = JwksResponse { keys };

    match serde_json::to_string_pretty(&response) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "public, max-age=300")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error": "Serialization failed: {e}"}}"#
            ))))
            .unwrap(),
    }
}

// =============================================================================
// P2P Peer Advertisement
// =============================================================================

/// P2P peer info for bootstrap discovery
#[derive(Serialize)]
pub struct P2PPeerInfo {
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub capabilities: Vec<String>,
    pub nat_status: Option<String>,
}

/// Response for GET /api/v1/federation/p2p-peers
#[derive(Serialize)]
pub struct P2PPeersResponse {
    pub peers: Vec<P2PPeerInfo>,
    pub total: usize,
}

/// Handle GET /api/v1/federation/p2p-peers
///
/// Returns P2P peer information for desktop stewards to bootstrap into the mesh.
/// Queries local elohim-storage's /p2p/status endpoint and transforms the result.
/// For StatefulSet pods (replicas > 1), iterates headless DNS names.
pub async fn handle_federation_p2p_peers(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let storage_url = state
        .args
        .storage_url
        .clone()
        .unwrap_or_else(|| "http://localhost:8090".to_string());

    let mut peers = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Query local elohim-storage's P2P status
    match query_storage_p2p_status(&client, &storage_url).await {
        Ok(peer) => peers.push(peer),
        Err(e) => {
            tracing::debug!("Failed to query local storage P2P status: {}", e);
        }
    }

    // For K8s StatefulSet, also query headless DNS peers
    // Pattern: elohim-edgenode-{env}-{N}.elohim-edgenode-{env}-headless:8090
    if let Some(ref headless_base) = state.args.headless_service_base {
        let replicas = state.args.statefulset_replicas.unwrap_or(2);
        for i in 0..replicas {
            let peer_url = format!(
                "http://{}-{}.{}-headless:8090",
                headless_base, i, headless_base
            );
            // Skip if same as local storage
            if peer_url.contains("localhost") {
                continue;
            }
            match query_storage_p2p_status(&client, &peer_url).await {
                Ok(peer) => {
                    // Deduplicate by peer_id
                    if !peers.iter().any(|p| p.peer_id == peer.peer_id) {
                        peers.push(peer);
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to query peer at {}: {}", peer_url, e);
                }
            }
        }
    }

    let total = peers.len();
    let response = P2PPeersResponse { peers, total };

    match serde_json::to_string_pretty(&response) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "public, max-age=30")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error": "Serialization failed: {e}"}}"#
            ))))
            .unwrap(),
    }
}

/// Query a single elohim-storage instance's /p2p/status and transform to P2PPeerInfo
async fn query_storage_p2p_status(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<P2PPeerInfo, String> {
    let url = format!("{}/p2p/status", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Non-200 status: {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let peer_id = body["peer_id"]
        .as_str()
        .ok_or("Missing peer_id")?
        .to_string();

    let multiaddrs: Vec<String> = body["listen_addresses"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let nat_status = body["nat_status"].as_str().map(String::from);

    let relay_mode = body["relay_mode"].as_str().unwrap_or("client");
    let mut capabilities = vec!["shard".to_string(), "sync".to_string()];
    if relay_mode == "server" || relay_mode == "both" {
        capabilities.push("relay".to_string());
    }

    Ok(P2PPeerInfo {
        peer_id,
        multiaddrs,
        capabilities,
        nat_status,
    })
}

// =============================================================================
// Helpers
// =============================================================================

/// Build a doorway list containing only self (when DHT unavailable)
fn build_self_only_doorway(state: &AppState) -> Vec<DoorwaySummary> {
    if let Some(ref doorway_id) = state.args.doorway_id {
        let mut capabilities = vec!["gateway".to_string()];
        if state.args.storage_url.is_some() {
            capabilities.push("blob-storage".to_string());
        }
        if state.args.bootstrap_enabled {
            capabilities.push("bootstrap".to_string());
        }
        if state.args.signal_enabled {
            capabilities.push("signal".to_string());
        }
        if state.projection.is_some() {
            capabilities.push("projection".to_string());
        }

        vec![DoorwaySummary {
            id: doorway_id.clone(),
            url: state.args.doorway_url.clone().unwrap_or_default(),
            region: state.args.region.clone(),
            tier: "Emerging".to_string(),
            capabilities,
            status: "online".to_string(),
        }]
    } else {
        vec![]
    }
}

/// Base64url encode without padding (for JWKS "x" parameter)
fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

// =============================================================================
// Admin Federation Peer Management
// =============================================================================

/// A configured federation peer with enriched status from the peer cache
#[derive(Serialize)]
pub struct FederationPeerConfigEntry {
    pub url: String,
    pub reachable: bool,
    pub doorway_id: Option<String>,
    pub region: Option<String>,
    pub capabilities: Vec<String>,
}

/// Response for GET /admin/federation/peers
#[derive(Serialize)]
pub struct FederationPeersConfigResponse {
    pub peers: Vec<FederationPeerConfigEntry>,
    pub total: usize,
    pub self_id: Option<String>,
}

/// Request body for POST /admin/federation/peers
#[derive(Deserialize)]
pub struct AddPeerRequest {
    pub url: String,
}

/// Request body for DELETE /admin/federation/peers
#[derive(Deserialize)]
pub struct RemovePeerRequest {
    pub url: String,
}

/// Generic mutation response for admin operations
#[derive(Serialize)]
pub struct AdminMutationResponse {
    pub success: bool,
    pub message: String,
}

/// Handle GET /admin/federation/peers
///
/// Returns configured peer URLs enriched with reachability and identity
/// from the peer cache.
pub async fn handle_admin_federation_peers(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let urls = federation::get_peer_urls(&state.peer_url_list).await;
    let cached_peers = federation::get_cached_peers(&state.peer_cache).await;

    let peers: Vec<FederationPeerConfigEntry> = urls
        .iter()
        .map(|url| {
            // Cross-reference with cached peer data to enrich
            let normalized = url.trim_end_matches('/');
            let matching_peer = cached_peers.iter().find(|p| {
                p.source_peer.trim_end_matches('/') == normalized
                    || p.url.trim_end_matches('/') == normalized
            });

            FederationPeerConfigEntry {
                url: url.clone(),
                reachable: matching_peer.is_some(),
                doorway_id: matching_peer.map(|p| p.id.clone()),
                region: matching_peer.and_then(|p| p.region.clone()),
                capabilities: matching_peer
                    .map(|p| p.capabilities.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    let total = peers.len();
    let response = FederationPeersConfigResponse {
        peers,
        total,
        self_id: state.args.doorway_id.clone(),
    };

    json_response(&response)
}

/// Handle POST /admin/federation/peers
///
/// Add a new federation peer URL. Triggers immediate discovery for the new peer.
pub async fn handle_admin_add_federation_peer(
    state: Arc<AppState>,
    body: Bytes,
) -> Response<Full<Bytes>> {
    let request: AddPeerRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return json_error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {e}"));
        }
    };

    // Basic URL validation
    if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return json_error_response(
            StatusCode::BAD_REQUEST,
            "URL must start with http:// or https://",
        );
    }

    let added = federation::add_peer_url(&state.peer_url_list, request.url.clone()).await;
    if !added {
        return json_response(&AdminMutationResponse {
            success: false,
            message: "Peer URL already configured".to_string(),
        });
    }

    // Trigger immediate refresh for the new peer
    federation::refresh_peer_cache(
        std::slice::from_ref(&request.url),
        state.args.doorway_id.as_deref(),
        &state.peer_cache,
    )
    .await;

    json_response(&AdminMutationResponse {
        success: true,
        message: format!("Peer added: {}", request.url),
    })
}

/// Handle DELETE /admin/federation/peers
///
/// Remove a federation peer URL and clean matching entries from the peer cache.
pub async fn handle_admin_remove_federation_peer(
    state: Arc<AppState>,
    body: Bytes,
) -> Response<Full<Bytes>> {
    let request: RemovePeerRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return json_error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {e}"));
        }
    };

    let removed = federation::remove_peer_url(&state.peer_url_list, &request.url).await;
    if !removed {
        return json_response(&AdminMutationResponse {
            success: false,
            message: "Peer URL not found in configuration".to_string(),
        });
    }

    // Clean matching entries from the peer cache
    {
        let normalized = request.url.trim_end_matches('/');
        let mut cache = state.peer_cache.write().await;
        cache.retain(|p| p.source_peer.trim_end_matches('/') != normalized);
    }

    json_response(&AdminMutationResponse {
        success: true,
        message: format!("Peer removed: {}", request.url),
    })
}

/// Handle POST /admin/federation/peers/refresh
///
/// Force an immediate refresh of the peer cache from all configured peer URLs.
pub async fn handle_admin_refresh_federation_peers(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let urls = federation::get_peer_urls(&state.peer_url_list).await;

    federation::refresh_peer_cache(&urls, state.args.doorway_id.as_deref(), &state.peer_cache)
        .await;

    let cached = federation::get_cached_peers(&state.peer_cache).await;

    json_response(&AdminMutationResponse {
        success: true,
        message: format!(
            "Refreshed {} peer URL(s), discovered {} doorway(s)",
            urls.len(),
            cached.len()
        ),
    })
}

/// Helper: serialize to JSON response
fn json_response<T: Serialize>(data: &T) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(data) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(e) => json_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Serialization failed: {e}"),
        ),
    }
}

/// Helper: JSON error response
fn json_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(
            r#"{{"error": "{message}"}}"#
        ))))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_url_encode() {
        let data = [0u8; 32]; // 32 zero bytes
        let encoded = base64_url_encode(&data);
        assert!(!encoded.contains('='), "Should not have padding");
        assert!(!encoded.contains('+'), "Should not have + (url-safe)");
        assert!(!encoded.contains('/'), "Should not have / (url-safe)");
    }

    #[test]
    fn test_jwks_serialization() {
        let response = JwksResponse {
            keys: vec![JwkKey {
                kty: "OKP".to_string(),
                crv: "Ed25519".to_string(),
                key_use: "sig".to_string(),
                kid: "node-key-1".to_string(),
                x: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"kty\":\"OKP\""));
        assert!(json.contains("\"crv\":\"Ed25519\""));
        assert!(json.contains("\"use\":\"sig\""));
    }
}
