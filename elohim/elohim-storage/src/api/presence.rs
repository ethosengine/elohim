//! Presence API controller
//!
//! Routes: `/api/v1/presence[/{id}][/stewardship|/claim|/verify-claim]`
//!
//! Delegates to `PresenceService` for business logic, which calls
//! `db::contributor_presences` for Diesel queries.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::contributor_presences::{
    ContributorPresenceQuery, CreateContributorPresenceInput, InitiateClaimInput,
    InitiateStewardshipInput,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::PresenceService;
use crate::views::ContributorPresenceView;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Request types (camelCase API boundary)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePresenceRequest {
    pub display_name: String,
    #[serde(default)]
    pub external_identifiers: Option<serde_json::Value>,
    #[serde(default)]
    pub establishing_content_ids: Option<Vec<String>>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeginStewardshipRequest {
    pub steward_agent_id: String,
    #[serde(default)]
    pub stewardship_commitment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitiateClaimRequest {
    pub claiming_agent_id: String,
    pub verification_method: String,
    #[serde(default)]
    pub claim_evidence: Option<serde_json::Value>,
    #[serde(default)]
    pub facilitated_by: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Extract `{id}` from a resource path like `/{id}`, `/{id}/stewardship`, etc.
fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    // Split on '/' — the first segment is the ID
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// Extract sub-action after `/{id}/` from a resource path
fn extract_action(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let _ = parts.next(); // skip id
    parts.next().filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Handler functions
// ---------------------------------------------------------------------------

async fn list_presences(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Parse query params from URL
    let query_str = req.uri().query().unwrap_or("");
    let query: ContributorPresenceQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let presences = PresenceService::list_presences(&mut conn, ctx, &query)?;
    let views: Vec<ContributorPresenceView> = presences
        .into_iter()
        .map(ContributorPresenceView::from)
        .collect();
    Ok(response::ok(&views))
}

async fn get_presence(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let result = PresenceService::get_presence(&mut conn, ctx, id)?;
    Ok(response::from_option(
        Ok(result.map(ContributorPresenceView::from)),
        &format!("Presence {} not found", id),
    ))
}

async fn create_presence(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
    // Currently null for direct storage writes. Backfill needed for pre-coherence data.
    let body: CreatePresenceRequest = parse_body(req).await?;

    let external_identifiers_json = body
        .external_identifiers
        .map(|v| serde_json::to_string(&v))
        .transpose()
        .map_err(|e| StorageError::InvalidInput(format!("Invalid externalIdentifiers: {}", e)))?;

    let input = CreateContributorPresenceInput {
        id: None,
        display_name: body.display_name,
        external_identifiers_json,
        establishing_content_ids: body.establishing_content_ids.unwrap_or_default(),
        image: body.image,
        note: body.note,
        metadata_json: None,
    };

    let mut conn = get_conn(pool)?;
    let presence = PresenceService::create_presence(&mut conn, ctx, input)?;
    Ok(response::created(&ContributorPresenceView::from(presence)))
}

async fn begin_stewardship(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: BeginStewardshipRequest = parse_body(req).await?;

    let input = InitiateStewardshipInput {
        steward_id: body.steward_agent_id,
        stewardship_commitment_id: body.stewardship_commitment_id,
    };

    let mut conn = get_conn(pool)?;
    let presence = PresenceService::begin_stewardship(&mut conn, ctx, id, input)?;
    Ok(response::ok(&ContributorPresenceView::from(presence)))
}

async fn initiate_claim(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: InitiateClaimRequest = parse_body(req).await?;

    let evidence_json = body
        .claim_evidence
        .map(|v| serde_json::to_string(&v))
        .transpose()
        .map_err(|e| StorageError::InvalidInput(format!("Invalid claimEvidence: {}", e)))?;

    let input = InitiateClaimInput {
        claiming_agent_id: body.claiming_agent_id,
        verification_method: body.verification_method,
        evidence_json,
        facilitated_by: body.facilitated_by,
    };

    let mut conn = get_conn(pool)?;
    let presence = PresenceService::initiate_claim(&mut conn, ctx, id, input)?;
    Ok(response::ok(&ContributorPresenceView::from(presence)))
}

async fn verify_claim(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let presence = PresenceService::verify_claim(&mut conn, ctx, id)?;
    Ok(response::ok(&ContributorPresenceView::from(presence)))
}

async fn delete_presence(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(response::from_delete_bool_result(
        PresenceService::delete_presence(&mut conn, ctx, id),
        &format!("Presence {} not found", id),
    ))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/presence*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // resource_path arrives without the "presence" prefix, e.g. "", "/", "/{id}", "/{id}/stewardship"
    let trimmed = resource_path.trim_end_matches('/');

    match (&method, trimmed) {
        // GET /api/v1/presence  — list
        (&Method::GET, "") | (&Method::GET, "/") => list_presences(req, pool, ctx).await,

        // POST /api/v1/presence — create
        (&Method::POST, "") | (&Method::POST, "/") => create_presence(req, pool, ctx).await,

        // Routes with an ID segment
        _ => {
            let id = match extract_id(trimmed) {
                Some(id) => id,
                None => {
                    return Ok(response::not_found(&format!(
                        "Unknown presence route: {} {}",
                        method, resource_path
                    )))
                }
            };
            let action = extract_action(trimmed);

            match (&method, action) {
                // GET /api/v1/presence/{id}
                (&Method::GET, None) => get_presence(id, pool, ctx).await,

                // DELETE /api/v1/presence/{id}
                (&Method::DELETE, None) => delete_presence(id, pool, ctx).await,

                // POST /api/v1/presence/{id}/stewardship
                (&Method::POST, Some("stewardship")) => begin_stewardship(id, req, pool, ctx).await,

                // POST /api/v1/presence/{id}/claim
                (&Method::POST, Some("claim")) => initiate_claim(id, req, pool, ctx).await,

                // POST /api/v1/presence/{id}/verify-claim
                (&Method::POST, Some("verify-claim")) => verify_claim(id, pool, ctx).await,

                _ => Ok(response::not_found(&format!(
                    "Unknown presence route: {} {}",
                    method, resource_path
                ))),
            }
        }
    }
}
