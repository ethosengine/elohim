//! Mastery API controller
//!
//! Routes: `/api/v1/mastery[/{contentId}][/engagement|/assessment|/batch|/path/{pathId}|/stats|/check-privilege]`
//!
//! Delegates to `db::content_mastery` for Diesel queries.
//! Practice pool, challenge, and points endpoints are stubbed as TODO.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::{Deserialize, Serialize};

use crate::db::content_mastery::{
    self, CreateMasteryInput, MasteryAdvancement, MasteryQuery, PathMasterySummary,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::ContentMasteryView;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Request types (camelCase API boundary)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeMasteryRequest {
    pub human_id: String,
    pub content_id: String,
    #[serde(default)]
    pub mastery_level: Option<String>,
    #[serde(default)]
    pub content_version_at_mastery: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordEngagementRequest {
    pub human_id: String,
    pub content_id: String,
    pub engagement_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordAssessmentRequest {
    pub human_id: String,
    pub content_id: String,
    pub new_level: String,
    #[serde(default)]
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchQueryRequest {
    pub human_id: String,
    pub content_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckPrivilegeRequest {
    pub human_id: String,
    pub content_id: String,
}

// ---------------------------------------------------------------------------
// Response types (camelCase API boundary)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MasteryAdvancementResponse {
    pub mastery: ContentMasteryView,
    pub previous_level: String,
    pub new_level: String,
    pub is_advancement: bool,
}

impl From<MasteryAdvancement> for MasteryAdvancementResponse {
    fn from(a: MasteryAdvancement) -> Self {
        Self {
            mastery: ContentMasteryView::from(a.mastery),
            previous_level: a.previous_level,
            new_level: a.new_level,
            is_advancement: a.is_advancement,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathMasterySummaryResponse {
    pub path_id: String,
    pub human_id: String,
    pub total_content: usize,
    pub mastered_content: usize,
    pub average_mastery_index: f32,
    pub average_freshness: f32,
    pub needs_refresh_count: usize,
    pub by_level: Vec<LevelCount>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LevelCount {
    pub level: String,
    pub count: usize,
}

impl From<PathMasterySummary> for PathMasterySummaryResponse {
    fn from(s: PathMasterySummary) -> Self {
        Self {
            path_id: s.path_id,
            human_id: s.human_id,
            total_content: s.total_content,
            mastered_content: s.mastered_content,
            average_mastery_index: s.average_mastery_index,
            average_freshness: s.average_freshness,
            needs_refresh_count: s.needs_refresh_count,
            by_level: s
                .by_level
                .into_iter()
                .map(|(level, count)| LevelCount { level, count })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MasteryStatsResponse {
    pub total_count: i64,
    pub refresh_needed_count: i64,
    pub average_freshness: f64,
    pub by_level: Vec<LevelCountI64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LevelCountI64 {
    pub level: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivilegeCheckResponse {
    pub has_mastery: bool,
    pub mastery_level: Option<String>,
    pub mastery_level_index: Option<i32>,
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

/// Extract a query parameter value from an optional query string.
fn extract_query_param(query: Option<&str>, key: &str) -> Option<String> {
    let q = query?;
    for pair in q.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == key {
                return Some(v.to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Handler functions
// ---------------------------------------------------------------------------

/// POST /api/v1/mastery — initialize or upsert mastery
async fn initialize_mastery(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
    // Currently null for direct storage writes. Backfill needed for pre-coherence data.
    let body: InitializeMasteryRequest = parse_body(req).await?;

    let input = CreateMasteryInput {
        id: None,
        human_id: body.human_id,
        content_id: body.content_id,
        mastery_level: body
            .mastery_level
            .unwrap_or_else(|| "not_started".to_string()),
        content_version_at_mastery: body.content_version_at_mastery,
    };

    let mut conn = get_conn(pool)?;
    let mastery = content_mastery::upsert_mastery(&mut conn, ctx, input)?;
    Ok(response::created(&ContentMasteryView::from(mastery)))
}

/// POST /api/v1/mastery/engagement — record engagement
async fn handle_record_engagement(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: RecordEngagementRequest = parse_body(req).await?;

    let mut conn = get_conn(pool)?;
    let mastery = content_mastery::record_engagement(
        &mut conn,
        ctx,
        &body.human_id,
        &body.content_id,
        &body.engagement_type,
    )?;
    Ok(response::ok(&ContentMasteryView::from(mastery)))
}

/// POST /api/v1/mastery/assessment — record assessment (advance mastery)
async fn handle_record_assessment(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: RecordAssessmentRequest = parse_body(req).await?;

    let evidence_json = body
        .evidence
        .map(|v| serde_json::to_string(&v))
        .transpose()
        .map_err(|e| StorageError::InvalidInput(format!("Invalid evidence: {}", e)))?;

    let mut conn = get_conn(pool)?;
    let advancement = content_mastery::advance_mastery(
        &mut conn,
        ctx,
        &body.human_id,
        &body.content_id,
        &body.new_level,
        evidence_json.as_deref(),
    )?;
    Ok(response::ok(&MasteryAdvancementResponse::from(advancement)))
}

/// GET /api/v1/mastery/{contentId}?humanId=... — get mastery for content
async fn get_mastery_for_content(
    content_id: &str,
    req: &Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let human_id = extract_query_param(req.uri().query(), "humanId");
    let human_id = match human_id {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(response::bad_request("humanId query parameter is required")),
    };

    let mut conn = get_conn(pool)?;
    let result = content_mastery::get_mastery_for_content(&mut conn, ctx, &human_id, content_id)?;
    Ok(response::from_option(
        Ok(result.map(ContentMasteryView::from)),
        &format!(
            "Mastery not found for human {} content {}",
            human_id, content_id
        ),
    ))
}

/// GET /api/v1/mastery — list mastery records
async fn list_mastery(
    req: &Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let query: MasteryQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let records = content_mastery::list_mastery(&mut conn, ctx, &query)?;
    let views: Vec<ContentMasteryView> =
        records.into_iter().map(ContentMasteryView::from).collect();
    Ok(response::ok(&views))
}

/// POST /api/v1/mastery/batch — batch query mastery for multiple content IDs
async fn handle_batch_query(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: BatchQueryRequest = parse_body(req).await?;

    let mut conn = get_conn(pool)?;
    let records = content_mastery::get_mastery_for_contents(
        &mut conn,
        ctx,
        &body.human_id,
        &body.content_ids,
    )?;
    let views: Vec<ContentMasteryView> =
        records.into_iter().map(ContentMasteryView::from).collect();
    Ok(response::ok(&views))
}

/// GET /api/v1/mastery/path/{pathId}?humanId=...&contentIds=id1,id2,...
async fn handle_path_mastery(
    path_id: &str,
    req: &Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let human_id = extract_query_param(req.uri().query(), "humanId");
    let human_id = match human_id {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(response::bad_request("humanId query parameter is required")),
    };

    let content_ids_str = extract_query_param(req.uri().query(), "contentIds");
    let content_ids: Vec<String> = content_ids_str
        .map(|s| s.split(',').map(|id| id.trim().to_string()).collect())
        .unwrap_or_default();

    if content_ids.is_empty() {
        return Ok(response::bad_request(
            "contentIds query parameter is required (comma-separated)",
        ));
    }

    let mut conn = get_conn(pool)?;
    let summary =
        content_mastery::calculate_path_mastery(&mut conn, ctx, &human_id, path_id, &content_ids)?;
    Ok(response::ok(&PathMasterySummaryResponse::from(summary)))
}

/// GET /api/v1/mastery/stats — stats dashboard
async fn handle_stats(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let total_count = content_mastery::mastery_count(&mut conn, ctx)?;
    let refresh_needed = content_mastery::refresh_needed_count(&mut conn, ctx)?;
    let avg_freshness = content_mastery::average_freshness(&mut conn, ctx)?;
    let by_level_raw = content_mastery::stats_by_level(&mut conn, ctx)?;

    let by_level: Vec<LevelCountI64> = by_level_raw
        .into_iter()
        .map(|(level, count)| LevelCountI64 { level, count })
        .collect();

    Ok(response::ok(&MasteryStatsResponse {
        total_count,
        refresh_needed_count: refresh_needed,
        average_freshness: avg_freshness,
        by_level,
    }))
}

/// POST /api/v1/mastery/check-privilege — check privilege based on mastery
async fn handle_check_privilege(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: CheckPrivilegeRequest = parse_body(req).await?;

    let mut conn = get_conn(pool)?;
    let result =
        content_mastery::get_mastery_for_content(&mut conn, ctx, &body.human_id, &body.content_id)?;

    let resp = match result {
        Some(mastery) => PrivilegeCheckResponse {
            has_mastery: true,
            mastery_level: Some(mastery.mastery_level),
            mastery_level_index: Some(mastery.mastery_level_index),
        },
        None => PrivilegeCheckResponse {
            has_mastery: false,
            mastery_level: None,
            mastery_level_index: None,
        },
    };

    Ok(response::ok(&resp))
}

/// Return 501 Not Implemented for stubbed endpoints
fn not_implemented(description: &str) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::json_response(
        hyper::StatusCode::NOT_IMPLEMENTED,
        &serde_json::json!({
            "error": format!("Not implemented: {}", description),
            "status": 501
        }),
    ))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/mastery*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // resource_path arrives without the "mastery" prefix, e.g. "", "/", "/{contentId}", "/engagement"
    let trimmed = resource_path.trim_end_matches('/');

    match (&method, trimmed) {
        // GET /api/v1/mastery — list mastery records
        (&Method::GET, "") | (&Method::GET, "/") => list_mastery(&req, pool, ctx).await,

        // POST /api/v1/mastery — initialize mastery
        (&Method::POST, "") | (&Method::POST, "/") => initialize_mastery(req, pool, ctx).await,

        // POST /api/v1/mastery/engagement — record engagement
        (&Method::POST, "/engagement") => handle_record_engagement(req, pool, ctx).await,

        // POST /api/v1/mastery/assessment — record assessment
        (&Method::POST, "/assessment") => handle_record_assessment(req, pool, ctx).await,

        // POST /api/v1/mastery/batch — batch query
        (&Method::POST, "/batch") => handle_batch_query(req, pool, ctx).await,

        // POST /api/v1/mastery/check-privilege — privilege check
        (&Method::POST, "/check-privilege") => handle_check_privilege(req, pool, ctx).await,

        // GET /api/v1/mastery/stats — stats dashboard
        (&Method::GET, "/stats") => handle_stats(pool, ctx).await,

        // ---------------------------------------------------------------
        // Stubbed endpoints (TODO: implement)
        // ---------------------------------------------------------------

        // POST /api/v1/mastery/pool — practice pool
        (&Method::POST, "/pool") => not_implemented("practice pool creation"),

        // POST /api/v1/mastery/challenge/* — challenge routes
        (&Method::POST, path) if path.starts_with("/challenge") => {
            not_implemented("challenge endpoints")
        }

        // POST /api/v1/mastery/points/* — points routes
        (&Method::POST, path) if path.starts_with("/points") => not_implemented("points endpoints"),

        // GET /api/v1/mastery/pool/* — practice pool queries
        (&Method::GET, path) if path.starts_with("/pool") => {
            not_implemented("practice pool queries")
        }

        // ---------------------------------------------------------------
        // Routes with path segments: /path/{pathId} and /{contentId}
        // ---------------------------------------------------------------
        _ => {
            // GET /api/v1/mastery/path/{pathId}
            if method == Method::GET && trimmed.starts_with("/path/") {
                let path_id = trimmed.strip_prefix("/path/").unwrap_or("");
                if path_id.is_empty() {
                    return Ok(response::bad_request("pathId is required"));
                }
                return handle_path_mastery(path_id, &req, pool, ctx).await;
            }

            // GET /api/v1/mastery/{contentId}
            if method == Method::GET {
                let content_id = match extract_id(trimmed) {
                    Some(id) => id,
                    None => {
                        return Ok(response::not_found(&format!(
                            "Unknown mastery route: {} {}",
                            method, resource_path
                        )))
                    }
                };
                return get_mastery_for_content(content_id, &req, pool, ctx).await;
            }

            Ok(response::not_found(&format!(
                "Unknown mastery route: {} {}",
                method, resource_path
            )))
        }
    }
}
