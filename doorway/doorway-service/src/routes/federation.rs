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
    pub identity_root: Option<String>,
    pub signing_key: Option<String>,
    pub endpoints: Vec<infrastructure_types::DoorwayEndpoint>,
    pub record_serial: Option<u64>,
    pub record_signature: Option<Vec<u8>>,
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
                            identity_root: Some(d.identity_root),
                            signing_key: Some(d.signing_key),
                            endpoints: d.endpoints,
                            record_serial: Some(d.record_serial),
                            record_signature: Some(d.record_signature),
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
                endpoints: vec![infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: peer.url.clone(),
                    priority: 0,
                    ttl_secs: 30,
                }],
                url: peer.url,
                identity_root: None,
                signing_key: None,
                record_serial: None,
                record_signature: None,
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
///
/// `kid` is this doorway's own `doorway_id` — cross-doorway JWT verification
/// (`auth::jwt::JwtValidator`) keys its peer cache by `doorway_id`, so a
/// mismatch here would make every foreign-issued token unverifiable. Falls
/// back to the pre-federation placeholder id only when `doorway_id` isn't
/// configured (dev/standalone mode).
pub fn handle_doorway_keys(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let mut keys = Vec::new();

    // Add node signing key if available
    if let Some(ref verifying_key) = state.node_verifying_key {
        let pub_bytes = verifying_key.to_bytes();
        let x = base64_url_encode(&pub_bytes);
        let kid = state
            .args
            .doorway_id
            .clone()
            .unwrap_or_else(|| "node-key-1".to_string());

        keys.push(JwkKey {
            kty: "OKP".to_string(),
            crv: "Ed25519".to_string(),
            key_use: "sig".to_string(),
            kid,
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
#[derive(Serialize, Debug)]
pub struct P2PPeerInfo {
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub capabilities: Vec<String>,
    pub nat_status: Option<String>,
}

/// Response for GET /api/v1/federation/p2p-peers
#[derive(Serialize, Debug)]
pub struct P2PPeersResponse {
    pub peers: Vec<P2PPeerInfo>,
    pub total: usize,
    /// Honest mesh peer count from the routed storage's `connectedPeers`
    /// (`/p2p/status`). `None` when storage is unreachable or the field is
    /// absent. `total` mirrors this when present (the OLD `total` =
    /// backend-count bug: it reported 1 even when 13 peers were connected —
    /// see plan §1). Emitted as `connectedPeerCount` (camelCase wire key —
    /// snake_case never leaves the Rust boundary).
    #[serde(rename = "connectedPeerCount")]
    pub connected_peer_count: Option<usize>,
}

/// Handle GET /api/v1/federation/p2p-peers
///
/// Returns P2P peer information for desktop stewards to bootstrap into the mesh.
/// Fetches the routed storage's `/p2p/status` ONCE and projects the mesh
/// `connectedPeers` count honestly (NOT one row per storage backend — the old
/// behavior reported `total:1` while 13 peers were connected because it never
/// read `connectedPeers`; see plan F-EDGE §1).
///
/// Cat-C node-local Operational read-model: any doorway computes its own view
/// from its own routed storage `/p2p/status`. The `connectedPeers` field name
/// is the dataplane P-DIAGNOSTIC `/p2p/status` contract (X-EDGE-PEERS); this
/// projection consumes it verbatim, never a parallel vocabulary.
pub async fn handle_federation_p2p_peers(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let storage_url = state
        .args
        .storage_url
        .clone()
        .unwrap_or_else(|| "http://localhost:8090".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Fetch the routed storage's /p2p/status once; project the mesh view.
    // On any fetch/parse failure project from a null value → honest zero
    // (peers: [], total: 0, connectedPeerCount: null) — preserves the old
    // debug-log-and-continue posture without faking a self row.
    let status = match fetch_storage_p2p_status(&client, &storage_url).await {
        Ok(value) => value,
        Err(e) => {
            tracing::debug!("Failed to query local storage P2P status: {}", e);
            serde_json::Value::Null
        }
    };
    let response = project_p2p_peers(&status);

    match serde_json::to_string_pretty(&response) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            // /p2p-peers is mutable (mesh state changes) — short TTL, NOT
            // CDN-cacheable. Keep the 30s window.
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

/// Fetch one storage instance's `/p2p/status` body as a raw JSON value.
/// Side-effecting (HTTP); the pure projection lives in `project_p2p_peers`.
async fn fetch_storage_p2p_status(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/p2p/status", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Non-200 status: {}", resp.status()));
    }

    resp.json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))
}

/// Pure projection of one storage `/p2p/status` body → the federation peers
/// response. `total` = mesh `connectedPeers` when present, else the self-row
/// count (degraded honesty: "we can only see ourselves"). When `peerId` is
/// absent (storage unreachable / null value) there is no self row and the
/// view is an honest zero. Cat-C node-local read-model — no DHT entry, no
/// table, no coordinator fn.
fn project_p2p_peers(status: &serde_json::Value) -> P2PPeersResponse {
    let mut peers = Vec::new();
    if let Some(peer_id) = status["peerId"].as_str() {
        let multiaddrs = status["listenAddresses"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let nat_status = status["natStatus"].as_str().map(String::from);
        let relay_mode = status["relayMode"].as_str().unwrap_or("client");
        let mut capabilities = vec!["shard".to_string(), "sync".to_string()];
        if relay_mode == "server" || relay_mode == "both" {
            capabilities.push("relay".to_string());
        }
        peers.push(P2PPeerInfo {
            peer_id: peer_id.to_string(),
            multiaddrs,
            capabilities,
            nat_status,
        });
    }

    let connected_peer_count = status["connectedPeers"].as_u64().map(|n| n as usize);
    let total = connected_peer_count.unwrap_or(peers.len());

    P2PPeersResponse {
        peers,
        total,
        connected_peer_count,
    }
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
            identity_root: None,
            signing_key: None,
            endpoints: FederationConfig::from_args(&state.args)
                .map(|config| config.endpoints)
                .unwrap_or_default(),
            record_serial: None,
            record_signature: None,
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

// =============================================================================
// Hosted-at binding resolution (doorway-federation-failover T2.2)
// =============================================================================

/// Handle `GET /api/v1/federation/hosted-binding/{agent_pub_key}`.
///
/// Serves this node's `hosted_agent_bindings` projection of the imagodei
/// Category-A2 `hosted-at` link, so a sibling doorway (or an operator) can ask
/// "where is this agent hosted?" and get an answer grounded in substrate truth
/// rather than one doorway's private Mongo.
///
/// This is a SINGLE-TARGET read of this node's own storage — it never iterates
/// peers and never fans out (`doorway/CLAUDE.md` no-fan-out rule). A sibling
/// that wants a different node's view asks that node.
///
/// - 200 — the projected binding (`doorwayId`, `doorwayUrl`, `installedAppId`,
///   `dhtAnchorHash`, `boundAt`).
/// - 404 `{"error":"no-hosted-binding"}` — HONEST ABSENCE. The Chaperone
///   degrades this to its existing 409 `hosted-elsewhere` rather than guessing.
/// - 502 — storage unreachable. Distinct from 404 on purpose: "we could not
///   ask" must never read as "there is no binding."
pub async fn handle_federation_hosted_binding(
    state: Arc<AppState>,
    agent_pub_key: &str,
) -> Response<Full<Bytes>> {
    if agent_pub_key.trim().is_empty() {
        return hosted_binding_json(
            StatusCode::BAD_REQUEST,
            r#"{"error":"agent_pub_key required"}"#.to_string(),
        );
    }

    let storage_url = state
        .args
        .storage_url
        .clone()
        .unwrap_or_else(|| "http://localhost:8090".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!(
        "{}/api/v1/federation/hosted-binding/{}",
        storage_url.trim_end_matches('/'),
        agent_pub_key
    );

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => hosted_binding_json(StatusCode::OK, body),
            Err(e) => {
                tracing::debug!(error = %e, "Hosted-binding body read failed");
                hosted_binding_json(
                    StatusCode::BAD_GATEWAY,
                    r#"{"error":"hosted-binding-unreadable"}"#.to_string(),
                )
            }
        },
        Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => hosted_binding_json(
            StatusCode::NOT_FOUND,
            r#"{"error":"no-hosted-binding"}"#.to_string(),
        ),
        Ok(resp) => {
            tracing::debug!(status = %resp.status(), "Hosted-binding query returned non-success");
            hosted_binding_json(
                StatusCode::BAD_GATEWAY,
                r#"{"error":"hosted-binding-unavailable"}"#.to_string(),
            )
        }
        Err(e) => {
            tracing::debug!(error = %e, "Hosted-binding query failed to reach storage");
            hosted_binding_json(
                StatusCode::BAD_GATEWAY,
                r#"{"error":"hosted-binding-unavailable"}"#.to_string(),
            )
        }
    }
}

fn hosted_binding_json(status: StatusCode, body: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        // Never cache a hosting-location answer: a human's home doorway can
        // change (migration), and a stale cached redirect target would send
        // them to a doorway that no longer hosts them.
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(body)))
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

    #[test]
    fn p2p_peers_reports_mesh_connected_count_not_backend_count() {
        // storage /p2p/status shape: one backend, but 13 connected mesh peers.
        let status = serde_json::json!({
            "peerId": "12D3KooSELF",
            "listenAddresses": ["/ip4/10.0.0.1/tcp/4001"],
            "natStatus": "public",
            "relayMode": "client",
            "connectedPeers": 13
        });
        let resp = project_p2p_peers(&status);
        // self is one row; the mesh count is surfaced honestly and is 13, not 1.
        assert_eq!(resp.connected_peer_count, Some(13), "{resp:?}");
        assert_eq!(
            resp.total, 13,
            "total must be mesh count, not backend count"
        );
        assert_eq!(resp.peers.len(), 1, "self row still present");
        assert_eq!(resp.peers[0].peer_id, "12D3KooSELF");
    }

    #[test]
    fn p2p_peers_tolerates_missing_connected_peers_field() {
        let status = serde_json::json!({
            "peerId": "12D3KooSELF",
            "listenAddresses": [],
            "relayMode": "client"
        });
        let resp = project_p2p_peers(&status);
        assert_eq!(resp.connected_peer_count, None);
        assert_eq!(resp.total, 1, "fallback: self-only when mesh count absent");
    }

    #[test]
    fn p2p_peers_unreachable_storage_is_honest_zero() {
        // A null/empty status (storage unreachable or no peerId) → no self row,
        // honest zero, no mesh count claimed.
        let resp = project_p2p_peers(&serde_json::Value::Null);
        assert_eq!(resp.peers.len(), 0);
        assert_eq!(resp.total, 0);
        assert_eq!(resp.connected_peer_count, None);
    }

    #[test]
    fn p2p_peers_relay_capability_when_relay_mode_server() {
        let status = serde_json::json!({
            "peerId": "12D3KooRELAY",
            "listenAddresses": ["/ip4/10.0.0.2/tcp/4001"],
            "relayMode": "server",
            "connectedPeers": 4
        });
        let resp = project_p2p_peers(&status);
        assert!(
            resp.peers[0].capabilities.contains(&"relay".to_string()),
            "relay capability advertised when relayMode=server"
        );
    }

    #[test]
    fn p2p_peers_response_serializes_camelcase_connected_peer_count() {
        // snake_case never leaves the Rust boundary: the new field must emit
        // `connectedPeerCount`, not `connected_peer_count`.
        let status = serde_json::json!({
            "peerId": "12D3KooSELF",
            "connectedPeers": 7
        });
        let resp = project_p2p_peers(&status);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(
            json.contains("connectedPeerCount"),
            "must emit camelCase wire key: {json}"
        );
        assert!(
            !json.contains("connected_peer_count"),
            "must NOT leak snake_case: {json}"
        );
    }
}
