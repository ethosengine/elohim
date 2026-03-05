//! Presence lifecycle routes
//!
//! HTTP API for contributor presence management. Exposes the IPresenceLifecycle
//! interface as REST endpoints under `/api/v1/presences`.
//!
//! Lifecycle: UNCLAIMED -> STEWARDED -> CLAIMED
//!
//! ## Routes
//!
//! - `POST   /api/v1/presences`                  - Create a new presence
//! - `POST   /api/v1/presences/{id}/stewardship`  - Begin stewardship
//! - `GET    /api/v1/presences/stewarded`          - Get my stewarded presences
//! - `POST   /api/v1/presences/{id}/claim`         - Initiate a claim
//! - `POST   /api/v1/presences/{id}/verify`        - Verify and finalize a claim
//! - `GET    /api/v1/presences/{id}`               - Get presence by ID
//! - `GET    /api/v1/presences?state=`             - Get presences by state

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

// ---------------------------------------------------------------------------
// Serde Types
// ---------------------------------------------------------------------------

/// Contributor presence lifecycle state.
///
/// Maps to TypeScript `PresenceState` in `presence.model.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PresenceState {
    Unclaimed,
    Stewarded,
    Claimed,
}

/// External identifier for linking to other platforms.
///
/// Maps to TypeScript `ExternalIdentifier` in `presence.model.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalIdentifier {
    pub provider: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

/// Request body for `POST /api/v1/presences`.
///
/// Maps to TypeScript `CreatePresenceRequest` in `presence.model.ts`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePresenceRequest {
    pub display_name: String,
    #[serde(default)]
    pub external_identifiers: Option<Vec<ExternalIdentifier>>,
    #[serde(default)]
    pub establishing_content_ids: Option<Vec<String>>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
}

/// Request body for `POST /api/v1/presences/{id}/stewardship`.
///
/// Maps to TypeScript `BeginStewardshipRequest` in `presence.model.ts`.
/// The `presenceId` is extracted from the URL path, not the body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginStewardshipRequest {
    /// Agent public key of the steward (required).
    pub steward_agent_id: String,
    #[serde(default)]
    pub commitment_note: Option<String>,
}

/// Request body for `POST /api/v1/presences/{id}/claim`.
///
/// Maps to TypeScript `InitiateClaimRequest` in `presence.model.ts`.
/// The `presenceId` is extracted from the URL path, not the body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitiateClaimRequest {
    /// Agent public key of the claiming agent (required).
    pub claiming_agent_id: String,
    pub claim_evidence: serde_json::Value,
    pub verification_method: String,
}

/// Contributor presence view returned in API responses.
///
/// Maps to TypeScript `ContributorPresenceView` in `presence.model.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributorPresenceView {
    pub id: String,
    pub display_name: String,
    pub presence_state: PresenceState,
    pub external_identifiers: Vec<ExternalIdentifier>,
    pub establishing_content_ids: Vec<String>,
    pub established_at: String,
    // Recognition metrics
    pub affinity_total: f64,
    pub unique_engagers: u64,
    pub citation_count: u64,
    pub recognition_score: f64,
    pub accumulating_since: String,
    pub last_recognition_at: String,
    // Stewardship (populated when state is Stewarded or Claimed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stewardship_started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stewardship_quality_score: Option<f64>,
    // Claim details (populated when state is Claimed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_initiated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_verified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_verification_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_agent_id: Option<String>,
    // Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Payload sent to elohim-storage for CREATE
// ---------------------------------------------------------------------------

/// Payload forwarded to storage for `POST /db/presences`.
///
/// Matches `CreateContributorPresenceInputView` in elohim-storage/views.rs.
/// Storage accepts camelCase with parsed arrays — no JSON-string fields needed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageCreatePresencePayload {
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_identifiers: Option<Vec<ExternalIdentifier>>,
    pub establishing_content_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

// ---------------------------------------------------------------------------
// Payload sent to elohim-storage for STEWARDSHIP
// ---------------------------------------------------------------------------

/// Payload forwarded to storage for `POST /db/presences/{id}/stewardship`.
///
/// Matches `InitiateStewardshipInput` in elohim-storage/db/contributor_presences.rs.
/// Note: that struct has no `serde(rename_all)`, so fields must be snake_case.
#[derive(Debug, Clone, Serialize)]
struct StorageStewardshipPayload {
    pub steward_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stewardship_commitment_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Payload sent to elohim-storage for CLAIM
// ---------------------------------------------------------------------------

/// Payload forwarded to storage for `POST /db/presences/{id}/claim`.
///
/// Matches `InitiateClaimInputView` in elohim-storage/views.rs (camelCase).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageClaimPayload {
    pub claiming_agent_id: String,
    pub verification_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn bad_request(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error":"{msg}"}}"#))))
        .unwrap()
}

fn service_unavailable(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error":"{msg}"}}"#))))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Storage Proxy Helpers
// ---------------------------------------------------------------------------

/// Forward a presence request to elohim-storage, preserving query string.
/// Used for GET and DELETE requests that need no body transformation.
async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);

    let query = req.uri().query();
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding presence request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT) {
        match req.collect().await {
            Ok(collected) => {
                let body_bytes = collected.to_bytes();
                builder = builder.body(body_bytes.to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    send_storage_request(builder, storage_path).await
}

/// Execute a reqwest request and map the response to a doorway response.
async fn send_storage_request(
    builder: reqwest::RequestBuilder,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        size = body.len(),
                        path = %storage_path,
                        "Forwarded presence response"
                    );
                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %storage_path, "Failed to forward presence request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

/// Handle `POST /api/v1/presences` - Create a new contributor presence.
///
/// Validates that displayName is non-empty, then forwards to storage as
/// `CreateContributorPresenceInputView` (camelCase, parsed arrays).
async fn handle_create_presence(
    req: Request<Incoming>,
    storage_url: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read create-presence request body");
            return bad_request(&format!("Failed to read request body: {e}"));
        }
    };

    let request: CreatePresenceRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to parse create-presence request");
            return bad_request(&format!("Invalid JSON: {e}"));
        }
    };

    if request.display_name.trim().is_empty() {
        return bad_request("displayName must not be empty");
    }

    let payload = StorageCreatePresencePayload {
        display_name: request.display_name,
        external_identifiers: request.external_identifiers,
        establishing_content_ids: request.establishing_content_ids.unwrap_or_default(),
        note: request.note,
        image: request.image,
    };

    let storage_endpoint = format!("{}/db/presences", storage_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut builder = client
        .post(&storage_endpoint)
        .header("Content-Type", "application/json");

    let body_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "Failed to serialize create-presence payload");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to serialize payload: {e}"}}"#
                ))))
                .unwrap();
        }
    };

    builder = builder.body(body_bytes);

    debug!(display_name = %payload.display_name, "Creating contributor presence");
    send_storage_request(builder, "/db/presences").await
}

/// Handle `POST /api/v1/presences/{id}/stewardship` - Begin stewardship.
///
/// Validates stewardAgentId is present, remaps to storage's `InitiateStewardshipInput`
/// format (snake_case fields: steward_id, stewardship_commitment_id).
async fn handle_begin_stewardship(
    req: Request<Incoming>,
    storage_url: &str,
    presence_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read stewardship request body");
            return bad_request(&format!("Failed to read request body: {e}"));
        }
    };

    let request: BeginStewardshipRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to parse stewardship request");
            return bad_request(&format!("Invalid JSON: {e}"));
        }
    };

    if request.steward_agent_id.trim().is_empty() {
        return bad_request("stewardAgentId must not be empty");
    }

    // Remap camelCase client payload → snake_case storage payload
    // Storage's InitiateStewardshipInput has no serde rename_all, uses snake_case field names
    let payload = StorageStewardshipPayload {
        steward_id: request.steward_agent_id,
        stewardship_commitment_id: None,
    };

    let storage_path = format!("/db/presences/{}/stewardship", presence_id);
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);

    let body_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "Failed to serialize stewardship payload");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to serialize payload: {e}"}}"#
                ))))
                .unwrap();
        }
    };

    let client = reqwest::Client::new();
    let builder = client
        .post(&storage_endpoint)
        .header("Content-Type", "application/json")
        .body(body_bytes);

    debug!(presence_id, "Beginning stewardship of contributor presence");
    send_storage_request(builder, &storage_path).await
}

/// Handle `POST /api/v1/presences/{id}/claim` - Initiate a claim.
///
/// Validates presenceId and claimingAgentId, serializes claimEvidence to the
/// `InitiateClaimInputView` shape that storage expects (camelCase, parsed evidence).
async fn handle_initiate_claim(
    req: Request<Incoming>,
    storage_url: &str,
    presence_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read claim request body");
            return bad_request(&format!("Failed to read request body: {e}"));
        }
    };

    let request: InitiateClaimRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to parse claim request");
            return bad_request(&format!("Invalid JSON: {e}"));
        }
    };

    if presence_id.trim().is_empty() {
        return bad_request("presenceId must not be empty");
    }

    if request.claiming_agent_id.trim().is_empty() {
        return bad_request("claimingAgentId must not be empty");
    }

    // Forward as InitiateClaimInputView (camelCase, evidence as parsed Value not string)
    let evidence = if request.claim_evidence.is_null() {
        None
    } else {
        Some(request.claim_evidence)
    };

    let payload = StorageClaimPayload {
        claiming_agent_id: request.claiming_agent_id,
        verification_method: request.verification_method,
        evidence,
    };

    let storage_path = format!("/db/presences/{}/claim", presence_id);
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);

    let body_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "Failed to serialize claim payload");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to serialize payload: {e}"}}"#
                ))))
                .unwrap();
        }
    };

    let client = reqwest::Client::new();
    let builder = client
        .post(&storage_endpoint)
        .header("Content-Type", "application/json")
        .body(body_bytes);

    debug!(presence_id, "Initiating claim on contributor presence");
    send_storage_request(builder, &storage_path).await
}

// ---------------------------------------------------------------------------
// Route Dispatcher
// ---------------------------------------------------------------------------

/// Handle all `/api/v1/presences/*` requests.
pub async fn handle_presence_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Presence proxy called but STORAGE_URL not configured");
            return service_unavailable("Storage service not configured. Set STORAGE_URL env var.");
        }
    };

    let method = req.method().clone();
    // Strip both singular (/api/v1/presence) and plural (/api/v1/presences) prefixes
    let sub_path = path
        .strip_prefix("/api/v1/presences")
        .or_else(|| path.strip_prefix("/api/v1/presence"))
        .unwrap_or(path)
        .trim_start_matches('/');

    // Dispatch to appropriate handler based on method + sub_path
    match (&method, sub_path) {
        // POST /api/v1/presences -> POST /db/presences (create)
        (&Method::POST, "") => handle_create_presence(req, &storage_url).await,

        // GET /api/v1/presences -> GET /db/presences (list, supports ?state= query)
        (&Method::GET, "") => forward_to_storage(req, &storage_url, "/db/presences").await,

        // GET /api/v1/presences/unclaimed -> GET /db/presences/unclaimed
        (&Method::GET, "unclaimed") => {
            forward_to_storage(req, &storage_url, "/db/presences/unclaimed").await
        }

        // GET /api/v1/presences/stewarded -> GET /db/presences/stewarded
        (&Method::GET, "stewarded") => {
            forward_to_storage(req, &storage_url, "/db/presences/stewarded").await
        }

        // GET /api/v1/presences/steward/{stewardId} -> GET /db/presences/steward/{stewardId}
        (&Method::GET, p) if p.starts_with("steward/") => {
            let storage_path = format!("/db/presences/{p}");
            forward_to_storage(req, &storage_url, &storage_path).await
        }

        // POST /api/v1/presences/{id}/stewardship -> validate + remap -> POST /db/presences/{id}/stewardship
        (&Method::POST, p) if p.ends_with("/stewardship") => {
            let id = p.trim_end_matches("/stewardship");
            handle_begin_stewardship(req, &storage_url, id).await
        }

        // POST /api/v1/presences/{id}/claim -> validate + remap -> POST /db/presences/{id}/claim
        (&Method::POST, p) if p.ends_with("/claim") => {
            let id = p.trim_end_matches("/claim");
            handle_initiate_claim(req, &storage_url, id).await
        }

        // POST /api/v1/presences/{id}/verify -> POST /db/presences/{id}/verify-claim
        // (storage uses /verify-claim not /verify)
        (&Method::POST, p) if p.ends_with("/verify") => {
            let id = p.trim_end_matches("/verify");
            let storage_path = format!("/db/presences/{id}/verify-claim");
            forward_to_storage(req, &storage_url, &storage_path).await
        }

        // GET /api/v1/presences/{id}/history -> GET /db/presences/{id}/history
        (&Method::GET, p) if p.ends_with("/history") => {
            let storage_path = format!("/db/presences/{p}");
            forward_to_storage(req, &storage_url, &storage_path).await
        }

        // GET /api/v1/presences/{id} -> GET /db/presences/{id}
        (&Method::GET, id) if !id.is_empty() => {
            let storage_path = format!("/db/presences/{id}");
            forward_to_storage(req, &storage_url, &storage_path).await
        }

        // DELETE /api/v1/presences/{id} -> DELETE /db/presences/{id}
        (&Method::DELETE, id) if !id.is_empty() => {
            let storage_path = format!("/db/presences/{id}");
            forward_to_storage(req, &storage_url, &storage_path).await
        }

        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"Unknown presence route: {method} {path}"}}"#
            ))))
            .unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_state_serializes_camel_case() {
        let json = serde_json::to_string(&PresenceState::Unclaimed).unwrap();
        assert_eq!(json, r#""unclaimed""#);

        let json = serde_json::to_string(&PresenceState::Stewarded).unwrap();
        assert_eq!(json, r#""stewarded""#);

        let json = serde_json::to_string(&PresenceState::Claimed).unwrap();
        assert_eq!(json, r#""claimed""#);
    }

    #[test]
    fn test_presence_state_deserializes_camel_case() {
        let state: PresenceState = serde_json::from_str(r#""unclaimed""#).unwrap();
        assert_eq!(state, PresenceState::Unclaimed);

        let state: PresenceState = serde_json::from_str(r#""stewarded""#).unwrap();
        assert_eq!(state, PresenceState::Stewarded);

        let state: PresenceState = serde_json::from_str(r#""claimed""#).unwrap();
        assert_eq!(state, PresenceState::Claimed);
    }

    #[test]
    fn test_create_presence_request_deserializes() {
        let json = r#"{
            "displayName": "Lynn Foster",
            "externalIdentifiers": [
                { "provider": "github", "value": "lynn-foster" }
            ],
            "establishingContentIds": ["content-123"],
            "note": "REA pioneer"
        }"#;

        let request: CreatePresenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.display_name, "Lynn Foster");
        assert!(request.external_identifiers.is_some());
        assert_eq!(request.external_identifiers.as_ref().unwrap().len(), 1);
        assert_eq!(request.establishing_content_ids.as_ref().unwrap().len(), 1);
        assert_eq!(request.note.as_deref(), Some("REA pioneer"));
        assert!(request.image.is_none());
    }

    #[test]
    fn test_begin_stewardship_request_deserializes() {
        let json = r#"{ "stewardAgentId": "uhCAk-agent-key", "commitmentNote": "I will steward this presence" }"#;
        let request: BeginStewardshipRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.steward_agent_id, "uhCAk-agent-key");
        assert_eq!(
            request.commitment_note.as_deref(),
            Some("I will steward this presence")
        );
    }

    #[test]
    fn test_stewardship_payload_remaps_to_snake_case() {
        let payload = StorageStewardshipPayload {
            steward_id: "uhCAk-agent-key".to_string(),
            stewardship_commitment_id: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        // Storage InitiateStewardshipInput uses snake_case (no rename_all)
        assert!(json.contains("steward_id"));
        assert!(json.contains("uhCAk-agent-key"));
        assert!(!json.contains("stewardAgentId"));
    }

    #[test]
    fn test_initiate_claim_request_deserializes() {
        let json = r#"{
            "claimingAgentId": "uhCAk-agent-key",
            "claimEvidence": { "signedMessage": "abc123" },
            "verificationMethod": "signed-message"
        }"#;

        let request: InitiateClaimRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.verification_method, "signed-message");
        assert_eq!(request.claiming_agent_id, "uhCAk-agent-key");
        assert!(request.claim_evidence.is_object());
    }

    #[test]
    fn test_claim_payload_remaps_evidence_to_parsed_value() {
        let evidence = serde_json::json!({ "signedMessage": "abc123" });
        let payload = StorageClaimPayload {
            claiming_agent_id: "uhCAk-agent".to_string(),
            verification_method: "signed-message".to_string(),
            evidence: Some(evidence.clone()),
        };
        let json_str = serde_json::to_string(&payload).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // evidence is a parsed object, not a JSON string
        assert!(parsed["evidence"].is_object());
        assert_eq!(parsed["evidence"]["signedMessage"], "abc123");
        assert_eq!(parsed["claimingAgentId"], "uhCAk-agent");
        assert_eq!(parsed["verificationMethod"], "signed-message");
    }

    #[test]
    fn test_create_presence_payload_omits_empty_optionals() {
        let payload = StorageCreatePresencePayload {
            display_name: "Lynn Foster".to_string(),
            external_identifiers: None,
            establishing_content_ids: vec![],
            note: None,
            image: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("displayName"));
        assert!(!json.contains("externalIdentifiers"));
        assert!(!json.contains("note"));
        assert!(!json.contains("image"));
    }

    #[test]
    fn test_contributor_presence_view_serializes() {
        let view = ContributorPresenceView {
            id: "presence-1".to_string(),
            display_name: "Lynn Foster".to_string(),
            presence_state: PresenceState::Unclaimed,
            external_identifiers: vec![ExternalIdentifier {
                provider: "github".to_string(),
                value: "lynn-foster".to_string(),
                verified: None,
                verified_at: None,
            }],
            establishing_content_ids: vec!["content-123".to_string()],
            established_at: "2026-03-05T00:00:00Z".to_string(),
            affinity_total: 0.0,
            unique_engagers: 0,
            citation_count: 0,
            recognition_score: 0.0,
            accumulating_since: "2026-03-05T00:00:00Z".to_string(),
            last_recognition_at: "2026-03-05T00:00:00Z".to_string(),
            steward_id: None,
            stewardship_started_at: None,
            stewardship_quality_score: None,
            claim_initiated_at: None,
            claim_verified_at: None,
            claim_verification_method: None,
            claimed_agent_id: None,
            note: Some("REA pioneer".to_string()),
            image: None,
            created_at: "2026-03-05T00:00:00Z".to_string(),
            updated_at: "2026-03-05T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&view).unwrap();
        // Verify camelCase serialization
        assert!(json.contains("displayName"));
        assert!(json.contains("presenceState"));
        assert!(json.contains("externalIdentifiers"));
        assert!(json.contains("recognitionScore"));
        assert!(json.contains("accumulatingSince"));
        // Verify None fields are omitted
        assert!(!json.contains("stewardId"));
        assert!(!json.contains("claimInitiatedAt"));
        assert!(!json.contains("image"));
    }
}
