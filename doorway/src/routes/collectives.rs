//! Collective (Governance) API Routes
//!
//! Business-logic route stubs for collective governance operations.
//! These endpoints will eventually layer governance logic on top of
//! the raw elohim-storage CRUD — consent validation, graduated intimacy
//! enforcement, constitutional parent checks, etc.
//!
//! ## Routes
//!
//! - `GET    /api/v1/collectives`                              → List collectives
//! - `GET    /api/v1/collectives/{id}`                         → Get a collective
//! - `POST   /api/v1/collectives`                              → Create a collective
//! - `GET    /api/v1/collectives/{id}/participants`             → List participants
//! - `POST   /api/v1/collectives/{id}/participants`             → Add participant
//! - `DELETE /api/v1/collectives/{id}/participants/{humanId}`   → Depart collective
//! - `GET    /api/v1/humans/{humanId}/collectives`              → Collectives for a human

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

// ---------------------------------------------------------------------------
// Serde types — mirror the storage-client TypeScript / Rust views.rs types
// ---------------------------------------------------------------------------

/// Collective response view (mirrors `CollectiveView` from elohim-storage).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveView {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub governance_layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constitutional_parent_id: Option<String>,
    pub reach: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dissolved_at: Option<String>,
}

/// Input for creating a new collective.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectiveInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub governance_layer: String,
    #[serde(default)]
    pub constitutional_parent_id: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub created_by: Option<String>,
}

/// Collective participation response view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveParticipationView {
    pub id: String,
    pub collective_id: String,
    pub human_id: String,
    pub intimacy_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_context: Option<String>,
    pub governance_weight: f64,
    pub consent_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub joined_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departed_at: Option<String>,
}

/// Input for adding a participant to a collective.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddParticipantInput {
    pub human_id: String,
    #[serde(default)]
    pub intimacy_level: Option<String>,
    #[serde(default)]
    pub role_context: Option<String>,
}

/// Query filters for listing collectives.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveQuery {
    #[serde(default)]
    pub governance_layer: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub active_only: Option<bool>,
}

// ---------------------------------------------------------------------------
// Storage proxy helper
// ---------------------------------------------------------------------------

/// Forward a collectives API request to elohim-storage.
///
/// Maps `/api/v1/collectives/*` → `{storage_url}/db/collectives/*`
/// Maps `/api/v1/humans/{id}/collectives` → `{storage_url}/db/humans/{id}/collectives`
async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    // Build the storage endpoint URL
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);

    // Preserve query string
    let query = req.uri().query();
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding collectives request to elohim-storage");

    // Build the forwarded request
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

    // Forward content-type header if present
    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    // Forward authorization header if present
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    // Forward body for POST/PUT
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

    // Send the request
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
                        "Forwarded collectives response"
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
            warn!(error = %e, url = %full_url, "Failed to forward collectives request to storage");
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
// Route Dispatcher
// ---------------------------------------------------------------------------

/// Handle all `/api/v1/collectives/*` and `/api/v1/humans/{id}/collectives` requests.
pub async fn handle_collectives_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Collectives proxy called but storage_url not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured"}"#,
                )))
                .unwrap();
        }
    };

    let method = req.method().clone();

    // Handle cross-domain route: /api/v1/humans/{humanId}/collectives
    // Maps to /db/humans/{humanId}/collectives on storage
    if path.starts_with("/api/v1/humans/") {
        if let Some(rest) = path.strip_prefix("/api/v1/humans/") {
            if rest.ends_with("/collectives") && method == Method::GET {
                let storage_path = format!("/db/humans/{rest}");
                return forward_to_storage(req, &storage_url, &storage_path).await;
            }
        }
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"Unknown collectives route: {method} {path}"}}"#
            ))))
            .unwrap();
    }

    // Strip /api/v1 prefix → /db prefix for storage routing
    // /api/v1/collectives/... → /db/collectives/...
    let storage_path = match path.strip_prefix("/api/v1") {
        Some(rest) => format!("/db{rest}"),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Invalid path"}"#)))
                .unwrap();
        }
    };

    forward_to_storage(req, &storage_url, &storage_path).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collective_view_serializes_camel_case() {
        let view = CollectiveView {
            id: "coll-1".into(),
            name: "Test Collective".into(),
            description: None,
            governance_layer: "community".into(),
            constitutional_parent_id: None,
            reach: "local".into(),
            metadata: None,
            created_by: Some("human-1".into()),
            created_at: "2026-03-05T00:00:00Z".into(),
            updated_at: "2026-03-05T00:00:00Z".into(),
            dissolved_at: None,
        };

        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("governanceLayer"));
        assert!(json.contains("createdBy"));
        assert!(!json.contains("governance_layer"));
        assert!(!json.contains("created_by"));
        // Optional None fields should be omitted
        assert!(!json.contains("description"));
        assert!(!json.contains("dissolvedAt"));
    }

    #[test]
    fn create_collective_input_deserializes_camel_case() {
        let json = r#"{
            "id": "coll-2",
            "name": "New Collective",
            "governanceLayer": "faith",
            "reach": "regional"
        }"#;

        let input: CreateCollectiveInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.id, "coll-2");
        assert_eq!(input.governance_layer, "faith");
        assert_eq!(input.reach, Some("regional".into()));
        assert!(input.description.is_none());
        assert!(input.constitutional_parent_id.is_none());
    }

    #[test]
    fn participation_view_serializes_camel_case() {
        let view = CollectiveParticipationView {
            id: "part-1".into(),
            collective_id: "coll-1".into(),
            human_id: "human-1".into(),
            intimacy_level: "recognition".into(),
            role_context: None,
            governance_weight: 1.0,
            consent_state: "consented".into(),
            metadata: None,
            joined_at: "2026-03-05T00:00:00Z".into(),
            updated_at: "2026-03-05T00:00:00Z".into(),
            departed_at: None,
        };

        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("collectiveId"));
        assert!(json.contains("humanId"));
        assert!(json.contains("intimacyLevel"));
        assert!(json.contains("governanceWeight"));
        assert!(json.contains("consentState"));
        assert!(!json.contains("collective_id"));
    }

    #[test]
    fn add_participant_input_deserializes_camel_case() {
        let json = r#"{
            "humanId": "human-42",
            "intimacyLevel": "connection",
            "roleContext": "steward"
        }"#;

        let input: AddParticipantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.human_id, "human-42");
        assert_eq!(input.intimacy_level, Some("connection".into()));
        assert_eq!(input.role_context, Some("steward".into()));
    }

    #[test]
    fn collective_query_defaults_to_none() {
        let json = "{}";
        let query: CollectiveQuery = serde_json::from_str(json).unwrap();
        assert!(query.governance_layer.is_none());
        assert!(query.reach.is_none());
        assert!(query.active_only.is_none());
    }

    #[test]
    fn collective_query_deserializes_filters() {
        let json = r#"{
            "governanceLayer": "education",
            "reach": "global",
            "activeOnly": true
        }"#;

        let query: CollectiveQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.governance_layer, Some("education".into()));
        assert_eq!(query.reach, Some("global".into()));
        assert_eq!(query.active_only, Some(true));
    }
}
