//! Attestation API controller
//!
//! Routes: `/api/v1/attestations[/{id}[/revoke]]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::content_attestations;
use crate::db::models::NewContentAttestation;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    ContentAttestationView, CreateAttestationInputView, RevokeAttestationInputView,
};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AttestationQuery {
    pub content_id: Option<String>,
    pub attestor_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let (path_only, _query) = resource_path.split_once('?').unwrap_or((resource_path, ""));
    let query_str = req.uri().query().unwrap_or("");

    match (&method, path_only) {
        // POST /api/v1/attestations/ → create
        (&Method::POST, "/" | "") => {
            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let input: CreateAttestationInputView = parse_body(req).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let evidence_json = input
                .evidence
                .as_ref()
                .map(|v| serde_json::to_string(&v.0))
                .transpose()
                .map_err(|e| StorageError::InvalidInput(format!("Invalid evidence JSON: {}", e)))?;
            let grantor_json = input
                .grantor
                .as_ref()
                .map(|v| serde_json::to_string(&v.0))
                .transpose()
                .map_err(|e| StorageError::InvalidInput(format!("Invalid grantor JSON: {}", e)))?;
            let new = NewContentAttestation {
                id: &id,
                content_id: &input.content_id,
                attestor_presence_id: &input.attestor_presence_id,
                scope: &input.scope,
                attestation_type: &input.attestation_type,
                evidence: evidence_json.as_deref(),
                grantor: grantor_json.as_deref(),
            };
            let mut conn = get_conn(pool)?;
            let result = content_attestations::create_attestation(&mut conn, &new)?;
            Ok(response::ok(&ContentAttestationView::from(result)))
        }

        // POST /api/v1/attestations/{id}/revoke
        (&Method::POST, p) if p.ends_with("/revoke") => {
            let id_part = p.strip_suffix("/revoke").unwrap_or("");
            let id = extract_id(id_part)
                .ok_or_else(|| StorageError::InvalidInput("Attestation ID required".to_string()))?;
            let _input: RevokeAttestationInputView = parse_body(req).await?;
            let mut conn = get_conn(pool)?;
            let result = content_attestations::revoke_attestation(&mut conn, id)?;
            Ok(response::ok(&ContentAttestationView::from(result)))
        }

        // GET /api/v1/attestations?contentId=X or ?attestorId=X
        (&Method::GET, "/" | "") => {
            let params: AttestationQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let mut conn = get_conn(pool)?;
            if let Some(content_id) = &params.content_id {
                let results =
                    content_attestations::query_attestations_for_content(&mut conn, content_id)?;
                let views: Vec<ContentAttestationView> = results
                    .into_iter()
                    .map(ContentAttestationView::from)
                    .collect();
                Ok(response::ok(&views))
            } else if let Some(attestor_id) = &params.attestor_id {
                let results =
                    content_attestations::query_attestations_by_attestor(&mut conn, attestor_id)?;
                let views: Vec<ContentAttestationView> = results
                    .into_iter()
                    .map(ContentAttestationView::from)
                    .collect();
                Ok(response::ok(&views))
            } else {
                Ok(response::bad_request(
                    "contentId or attestorId query param is required",
                ))
            }
        }

        // GET /api/v1/attestations/{id}
        (&Method::GET, p) => {
            let id = extract_id(p)
                .ok_or_else(|| StorageError::InvalidInput("Attestation ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = content_attestations::get_attestation(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ContentAttestationView::from)),
                &format!("Attestation {} not found", id),
            ))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown attestations route: {} {}",
            method, resource_path
        ))),
    }
}
