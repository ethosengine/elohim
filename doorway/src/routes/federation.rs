//! Federation Routes
//!
//! HTTP endpoints for doorway federation:
//! - `GET /api/v1/federation/doorways` — list known doorways from DHT
//! - `GET /.well-known/doorway-keys` — public signing key in JWKS format

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
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
                r#"{{"error": "Serialization failed: {}"}}"#,
                e
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
                r#"{{"error": "Serialization failed: {}"}}"#,
                e
            ))))
            .unwrap(),
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
