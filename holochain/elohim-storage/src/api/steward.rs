//! Steward API controller
//!
//! Routes: `/api/v1/steward/{credentials|gates|grants|access|revenue}[/{id}]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::models::{current_timestamp, NewAccessGrant, NewPremiumGate, NewStewardCredential};
use crate::db::steward_operations;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    AccessGrantView, CreateCredentialInputView, CreateGateInputView, CreateGrantInputView,
    PremiumGateView, StewardCredentialView, StewardRevenueSummaryView,
};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PresenceQuery {
    pub presence_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContentQuery {
    pub content_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResourceTypeQuery {
    pub resource_type: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GateQuery {
    pub gate_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AccessQuery {
    pub gate_id: Option<String>,
    pub grantee_id: Option<String>,
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
        // ====================================================================
        // Credentials
        // ====================================================================

        // POST /api/v1/steward/credentials
        (&Method::POST, "/credentials") => {
            let input: CreateCredentialInputView = parse_body(req).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let new = NewStewardCredential {
                id: &id,
                presence_id: &input.presence_id,
                content_id: &input.content_id,
                affinity_coefficient: input.affinity_coefficient,
                credential_type: &input.credential_type,
                status: "active",
            };
            let mut conn = get_conn(pool)?;
            let result = steward_operations::create_credential(&mut conn, &new)?;
            Ok(response::ok(&StewardCredentialView::from(result)))
        }

        // GET /api/v1/steward/credentials/{id}
        (&Method::GET, p) if p.starts_with("/credentials/") => {
            let sub = p.strip_prefix("/credentials").unwrap_or("");
            let id = extract_id(sub).ok_or_else(|| {
                StorageError::InvalidInput("Credential ID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;
            let result = steward_operations::get_credential(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(StewardCredentialView::from)),
                &format!("Credential {} not found", id),
            ))
        }

        // GET /api/v1/steward/credentials?presenceId=X or ?contentId=X
        (&Method::GET, "/credentials") => {
            let presence_params: PresenceQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_params: ContentQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let mut conn = get_conn(pool)?;

            if let Some(presence_id) = &presence_params.presence_id {
                let results = steward_operations::query_credentials_for_presence(
                    &mut conn,
                    presence_id,
                )?;
                let views: Vec<StewardCredentialView> =
                    results.into_iter().map(StewardCredentialView::from).collect();
                Ok(response::ok(&views))
            } else if let Some(content_id) = &content_params.content_id {
                let results =
                    steward_operations::query_credentials_for_content(&mut conn, content_id)?;
                let views: Vec<StewardCredentialView> =
                    results.into_iter().map(StewardCredentialView::from).collect();
                Ok(response::ok(&views))
            } else {
                Ok(response::bad_request(
                    "presenceId or contentId query param is required",
                ))
            }
        }

        // ====================================================================
        // Gates
        // ====================================================================

        // POST /api/v1/steward/gates
        (&Method::POST, "/gates") => {
            let input: CreateGateInputView = parse_body(req).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let resource_ids_json = serde_json::to_string(&input.gated_resource_ids.0)
                .map_err(|e| {
                    StorageError::InvalidInput(format!("Invalid gated_resource_ids JSON: {}", e))
                })?;
            let new = NewPremiumGate {
                id: &id,
                steward_credential_id: &input.steward_credential_id,
                steward_presence_id: &input.steward_presence_id,
                gated_resource_type: &input.gated_resource_type,
                gated_resource_ids: &resource_ids_json,
                gate_title: &input.gate_title,
                gate_description: input.gate_description.as_deref(),
            };
            let mut conn = get_conn(pool)?;
            let result = steward_operations::create_gate(&mut conn, &new)?;
            Ok(response::ok(&PremiumGateView::from(result)))
        }

        // GET /api/v1/steward/gates/{id}
        (&Method::GET, p) if p.starts_with("/gates/") => {
            let sub = p.strip_prefix("/gates").unwrap_or("");
            let id = extract_id(sub).ok_or_else(|| {
                StorageError::InvalidInput("Gate ID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;
            let result = steward_operations::get_gate(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(PremiumGateView::from)),
                &format!("Gate {} not found", id),
            ))
        }

        // GET /api/v1/steward/gates?resourceType=X
        (&Method::GET, "/gates") => {
            let params: ResourceTypeQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let resource_type = params.resource_type.as_deref().unwrap_or("");
            if resource_type.is_empty() {
                return Ok(response::bad_request(
                    "resourceType query param is required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let results = steward_operations::query_gates_for_resource(&mut conn, resource_type)?;
            let views: Vec<PremiumGateView> =
                results.into_iter().map(PremiumGateView::from).collect();
            Ok(response::ok(&views))
        }

        // ====================================================================
        // Grants
        // ====================================================================

        // POST /api/v1/steward/grants
        (&Method::POST, "/grants") => {
            let input: CreateGrantInputView = parse_body(req).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let now = current_timestamp();
            let new = NewAccessGrant {
                id: &id,
                gate_id: &input.gate_id,
                grantee_presence_id: &input.grantee_presence_id,
                contributor_presence_id: input.contributor_presence_id.as_deref(),
                granted_at: &now,
                expires_at: input.expires_at.as_deref(),
                status: "active",
            };
            let mut conn = get_conn(pool)?;
            let result = steward_operations::create_grant(&mut conn, &new)?;
            Ok(response::ok(&AccessGrantView::from(result)))
        }

        // GET /api/v1/steward/grants/{id}
        (&Method::GET, p) if p.starts_with("/grants/") => {
            let sub = p.strip_prefix("/grants").unwrap_or("");
            let id = extract_id(sub).ok_or_else(|| {
                StorageError::InvalidInput("Grant ID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;
            let result = steward_operations::get_grant(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(AccessGrantView::from)),
                &format!("Grant {} not found", id),
            ))
        }

        // GET /api/v1/steward/grants?gateId=X
        (&Method::GET, "/grants") => {
            let params: GateQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let gate_id = params.gate_id.as_deref().unwrap_or("");
            if gate_id.is_empty() {
                return Ok(response::bad_request("gateId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = steward_operations::query_grants_for_gate(&mut conn, gate_id)?;
            let views: Vec<AccessGrantView> =
                results.into_iter().map(AccessGrantView::from).collect();
            Ok(response::ok(&views))
        }

        // ====================================================================
        // Access check
        // ====================================================================

        // GET /api/v1/steward/access?gateId=X&granteeId=Y
        (&Method::GET, "/access") => {
            let params: AccessQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let gate_id = params.gate_id.as_deref().unwrap_or("");
            let grantee_id = params.grantee_id.as_deref().unwrap_or("");
            if gate_id.is_empty() || grantee_id.is_empty() {
                return Ok(response::bad_request(
                    "gateId and granteeId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let has_access =
                steward_operations::check_access(&mut conn, gate_id, grantee_id)?;
            Ok(response::ok(&serde_json::json!({ "hasAccess": has_access })))
        }

        // ====================================================================
        // Revenue
        // ====================================================================

        // GET /api/v1/steward/revenue/{presenceId}
        (&Method::GET, p) if p.starts_with("/revenue/") => {
            let sub = p.strip_prefix("/revenue").unwrap_or("");
            let presence_id = extract_id(sub).ok_or_else(|| {
                StorageError::InvalidInput("Presence ID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;
            let result = steward_operations::get_revenue_summary(&mut conn, presence_id)?;
            Ok(response::ok(&StewardRevenueSummaryView::from(result)))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown steward route: {} {}",
            method, resource_path
        ))),
    }
}
