//! DID document and identity endpoints
//!
//! Serves the doorway's W3C DID Document for federation discovery.
//! See holochain/doorway/DID-FEDERATION.md for architecture details.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::server::AppState;

/// W3C DID Document structure
/// See: https://www.w3.org/TR/did-core/
#[derive(Serialize)]
pub struct DIDDocument {
    /// JSON-LD context
    #[serde(rename = "@context")]
    pub context: Vec<String>,

    /// The DID that this document describes
    pub id: String,

    /// Verification methods (public keys)
    #[serde(rename = "verificationMethod", skip_serializing_if = "Vec::is_empty")]
    pub verification_method: Vec<VerificationMethod>,

    /// Authentication verification method references
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authentication: Vec<String>,

    /// Assertion method references (for signing credentials)
    #[serde(rename = "assertionMethod", skip_serializing_if = "Vec::is_empty")]
    pub assertion_method: Vec<String>,

    /// Service endpoints
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub service: Vec<Service>,

    /// Elohim-specific: capabilities this doorway supports
    #[serde(rename = "elohim:capabilities", skip_serializing_if = "Vec::is_empty")]
    pub elohim_capabilities: Vec<String>,

    /// Elohim-specific: geographic region for routing
    #[serde(rename = "elohim:region", skip_serializing_if = "Option::is_none")]
    pub elohim_region: Option<String>,

    /// Elohim-specific: Holochain cell ID (if connected)
    #[serde(rename = "elohim:holochainCellId", skip_serializing_if = "Option::is_none")]
    pub elohim_holochain_cell_id: Option<String>,
}

/// Verification method (public key) in DID Document
#[derive(Serialize)]
pub struct VerificationMethod {
    /// Full ID of this verification method
    pub id: String,

    /// Type of verification method
    #[serde(rename = "type")]
    pub method_type: String,

    /// Controller of this key
    pub controller: String,

    /// Public key in multibase format
    #[serde(rename = "publicKeyMultibase", skip_serializing_if = "Option::is_none")]
    pub public_key_multibase: Option<String>,
}

/// Service endpoint in DID Document
#[derive(Serialize)]
pub struct Service {
    /// Full ID of this service
    pub id: String,

    /// Type of service
    #[serde(rename = "type")]
    pub service_type: String,

    /// Service endpoint URL
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// Derive the doorway's DID from its configuration
fn derive_doorway_did(state: &AppState) -> String {
    // If doorway_id is set, use it to construct did:web
    if let Some(ref doorway_id) = state.args.doorway_id {
        // doorway_id is like "alpha-elohim-host" or "doorway-a.elohim.host"
        // If it contains dots, it's already a domain; otherwise, convert dashes
        if doorway_id.contains('.') {
            format!("did:web:{}", doorway_id)
        } else {
            // Convert "alpha-elohim-host" to "alpha.elohim.host"
            let domain = doorway_id.replace('-', ".");
            format!("did:web:{}", domain)
        }
    } else if let Some(ref doorway_url) = state.args.doorway_url {
        // Extract domain from URL
        if let Some(domain) = extract_domain(doorway_url) {
            format!("did:web:{}", domain)
        } else {
            // Fallback: use node_id as a local identifier
            format!("did:web:localhost:doorway:{}", state.args.node_id)
        }
    } else {
        // Local development fallback
        format!("did:web:localhost:doorway:{}", state.args.node_id)
    }
}

/// Extract domain from a URL (e.g., "https://alpha.elohim.host" -> "alpha.elohim.host")
fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Take everything before the first "/" or ":"
    let domain = without_scheme
        .split('/')
        .next()?
        .split(':')
        .next()?;

    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

/// Build the DID Document for this doorway
fn build_did_document(state: &AppState) -> DIDDocument {
    let did = derive_doorway_did(state);
    let args = &state.args;

    // Build service endpoints
    let mut services = Vec::new();

    // Blob storage endpoint
    if let Some(ref storage_url) = args.storage_url {
        services.push(Service {
            id: format!("{}#blobs", did),
            service_type: "ElohimBlobStore".to_string(),
            service_endpoint: format!("{}/api/v1/blobs", storage_url),
        });
    } else if let Some(ref doorway_url) = args.doorway_url {
        // If no separate storage, doorway serves blobs
        services.push(Service {
            id: format!("{}#blobs", did),
            service_type: "ElohimBlobStore".to_string(),
            service_endpoint: format!("{}/store", doorway_url),
        });
    }

    // Holochain gateway endpoint
    if let Some(ref doorway_url) = args.doorway_url {
        // Convert https:// to wss://
        let ws_url = doorway_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        services.push(Service {
            id: format!("{}#holochain", did),
            service_type: "HolochainGateway".to_string(),
            service_endpoint: format!("{}/app/{}", ws_url, args.app_port_min),
        });
    }

    // Human registry endpoint (if auth is configured)
    if args.jwt_secret.is_some() {
        if let Some(ref doorway_url) = args.doorway_url {
            services.push(Service {
                id: format!("{}#humans", did),
                service_type: "ElohimHumanRegistry".to_string(),
                service_endpoint: format!("{}/auth", doorway_url),
            });
        }
    }

    // Determine capabilities
    let mut capabilities = vec!["gateway".to_string()];
    if args.storage_url.is_some() {
        capabilities.push("blob-storage".to_string());
    }
    if args.jwt_secret.is_some() {
        capabilities.push("authentication".to_string());
    }
    if state.projection.is_some() {
        capabilities.push("projection".to_string());
    }

    DIDDocument {
        context: vec![
            "https://www.w3.org/ns/did/v1".to_string(),
            "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            "https://elohim-protocol.org/ns/v1".to_string(),
        ],
        id: did.clone(),
        verification_method: vec![
            // Note: In production, this should include actual signing keys
            // For now, we include a placeholder that indicates the verification
            // method type without exposing actual key material
            VerificationMethod {
                id: format!("{}#node-key", did),
                method_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.clone(),
                public_key_multibase: None, // TODO: Add actual key when signing is implemented
            },
        ],
        authentication: vec![format!("{}#node-key", did)],
        assertion_method: vec![format!("{}#node-key", did)],
        service: services,
        elohim_capabilities: capabilities,
        elohim_region: args.region.clone(),
        elohim_holochain_cell_id: None, // TODO: Populate from conductor connection
    }
}

/// Handle GET /.well-known/did.json
///
/// Returns the doorway's W3C DID Document for federation discovery.
/// This endpoint is public and does not require authentication.
pub fn handle_did_document(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let document = build_did_document(&state);

    let body = match serde_json::to_string_pretty(&document) {
        Ok(json) => json,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to serialize DID document: {}"}}"#,
                    e
                ))))
                .unwrap();
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/did+ld+json")
        .header("Cache-Control", "public, max-age=300") // 5 minute cache
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle GET /identity/did (alternative endpoint)
///
/// Same as /.well-known/did.json but at an explicit path.
pub fn handle_did_endpoint(state: Arc<AppState>) -> Response<Full<Bytes>> {
    handle_did_document(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://alpha.elohim.host"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("https://alpha.elohim.host/path"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("https://alpha.elohim.host:8080"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("http://localhost:8080"),
            Some("localhost".to_string())
        );
    }
}
