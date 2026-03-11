//! Governance API controller
//!
//! Routes: `/api/v1/governance/{state|states|challenges|proposals|precedents|discussions}[/{id}]`
//!
//! Governance is per-entity, not app-scoped. Delegates directly to `db::governance`
//! CRUD functions.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::governance;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    ChallengeView, DiscussionView, GovernanceStateView, PrecedentView, ProposalView,
};

use super::get_conn;

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GovernanceStateQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContentQuery {
    pub content_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ProposalQuery {
    pub content_id: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Extract `{id}` from a resource path like `/{id}`
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

/// Handle `/api/v1/governance*` requests
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
        // GET /api/v1/governance/state?entityType=X&entityId=Y
        (&Method::GET, "/state") => {
            let params: GovernanceStateQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            let entity_id = params.entity_id.as_deref().unwrap_or("");
            if entity_type.is_empty() || entity_id.is_empty() {
                return Ok(response::bad_request(
                    "entityType and entityId query params are required",
                ));
            }
            let mut conn = get_conn(pool)?;
            let result = governance::get_governance_state(&mut conn, entity_type, entity_id)?;
            Ok(response::from_option(
                Ok(result.map(GovernanceStateView::from)),
                &format!(
                    "Governance state not found for {}:{}",
                    entity_type, entity_id
                ),
            ))
        }

        // GET /api/v1/governance/states?entityType=X
        (&Method::GET, "/states") => {
            let params: GovernanceStateQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let entity_type = params.entity_type.as_deref().unwrap_or("");
            if entity_type.is_empty() {
                return Ok(response::bad_request("entityType query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_governance_states(&mut conn, entity_type)?;
            let views: Vec<GovernanceStateView> =
                results.into_iter().map(GovernanceStateView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/challenges/{id}
        (&Method::GET, p) if p.starts_with("/challenges/") => {
            let sub = p.strip_prefix("/challenges").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Challenge ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_challenge(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ChallengeView::from)),
                &format!("Challenge {} not found", id),
            ))
        }

        // GET /api/v1/governance/challenges?contentId=X
        (&Method::GET, "/challenges") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_challenges(&mut conn, content_id)?;
            let views: Vec<ChallengeView> = results.into_iter().map(ChallengeView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/proposals/{id}
        (&Method::GET, p) if p.starts_with("/proposals/") => {
            let sub = p.strip_prefix("/proposals").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_proposal(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ProposalView::from)),
                &format!("Proposal {} not found", id),
            ))
        }

        // GET /api/v1/governance/proposals?contentId=X&status=Y
        (&Method::GET, "/proposals") => {
            let params: ProposalQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results =
                governance::query_proposals(&mut conn, content_id, params.status.as_deref())?;
            let views: Vec<ProposalView> = results.into_iter().map(ProposalView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/precedents/{id}
        (&Method::GET, p) if p.starts_with("/precedents/") => {
            let sub = p.strip_prefix("/precedents").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Precedent ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_precedent(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(PrecedentView::from)),
                &format!("Precedent {} not found", id),
            ))
        }

        // GET /api/v1/governance/precedents?contentId=X
        (&Method::GET, "/precedents") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_precedents(&mut conn, content_id)?;
            let views: Vec<PrecedentView> = results.into_iter().map(PrecedentView::from).collect();
            Ok(response::ok(&views))
        }

        // GET /api/v1/governance/discussions/{id}
        (&Method::GET, p) if p.starts_with("/discussions/") => {
            let sub = p.strip_prefix("/discussions").unwrap_or("");
            let id = extract_id(sub)
                .ok_or_else(|| StorageError::InvalidInput("Discussion ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = governance::get_discussion(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(DiscussionView::from)),
                &format!("Discussion {} not found", id),
            ))
        }

        // GET /api/v1/governance/discussions?contentId=X
        (&Method::GET, "/discussions") => {
            let params: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
            let content_id = params.content_id.as_deref().unwrap_or("");
            if content_id.is_empty() {
                return Ok(response::bad_request("contentId query param is required"));
            }
            let mut conn = get_conn(pool)?;
            let results = governance::query_discussions(&mut conn, content_id)?;
            let views: Vec<DiscussionView> =
                results.into_iter().map(DiscussionView::from).collect();
            Ok(response::ok(&views))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown governance route: {} {}",
            method, resource_path
        ))),
    }
}
