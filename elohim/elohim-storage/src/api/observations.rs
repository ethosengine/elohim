//! HTTP routes for observation read-side access.
//!
//! Routes:
//!   - `GET /api/v1/observations/by-subject?subjectCid=X&kind=Y`
//!     → `Vec<ObservationView>` (newest first)
//!   - `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
//!     → `Vec<ObservationView>` (newest first)
//!   - `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
//!     → `Option<ObservationDiversitySummaryView>`
//!
//! Source of truth for `observations`: libp2p gossip plane (agent-authored,
//! projected into SQLite by the observation manager). Category B.
//! Source of truth for `observation_diversity_summary`: SQLite aggregation
//! view over `observations`. Category C.
//!
//! Template: `agreements.rs` (serde_urlencoded query parsing,
//! `get_conn` + `response::ok` response helpers).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::diesel_schema::{observation_diversity_summary, observations};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{ObservationDiversitySummaryView, ObservationView};
use diesel::prelude::*;

use super::get_conn;

// ---------------------------------------------------------------------------
// Query param structs
// ---------------------------------------------------------------------------

/// Query params for `GET /api/v1/observations/by-subject`.
/// camelCase because `CLAUDE.md` §Query Parameter Convention applies.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BySubjectQuery {
    pub subject_cid: String,
    pub kind: String,
}

/// Query params for `GET /api/v1/observations/by-observer`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByObserverQuery {
    pub observer_cid: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Query params for `GET /api/v1/observations/diversity`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiversityQuery {
    pub subject_cid: String,
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/observations*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }

    let path = resource_path.trim_start_matches('/');

    match path {
        "by-subject" => handle_by_subject(req, pool).await,
        "by-observer" => handle_by_observer(req, pool).await,
        "diversity" => handle_diversity(req, pool).await,
        _ => Ok(response::not_found(&format!(
            "Unknown observations route: /api/v1/observations/{}",
            path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// `GET /api/v1/observations/by-subject?subjectCid=X&kind=Y`
async fn handle_by_subject(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: BySubjectQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/by-subject: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;
    let rows: Vec<crate::db::models::ObservationRow> = observations::table
        .filter(observations::subject_cid.eq(&q.subject_cid))
        .filter(observations::observation_kind.eq(&q.kind))
        .order(observations::observed_at.desc())
        .load(&mut conn)
        .map_err(|e| StorageError::Internal(format!("observations query failed: {}", e)))?;

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    Ok(response::ok(&views))
}

/// `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
async fn handle_by_observer(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: ByObserverQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/by-observer: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;

    // Build a boxed query so we can optionally filter by kind.
    let base = observations::table
        .filter(observations::observer_cid.eq(&q.observer_cid))
        .order(observations::observed_at.desc())
        .into_boxed();

    let rows: Vec<crate::db::models::ObservationRow> = if let Some(kind) = &q.kind {
        base.filter(observations::observation_kind.eq(kind))
            .load(&mut conn)
    } else {
        base.load(&mut conn)
    }
    .map_err(|e| StorageError::Internal(format!("observations query failed: {}", e)))?;

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    Ok(response::ok(&views))
}

/// `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
async fn handle_diversity(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: DiversityQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/diversity: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;
    let row: Option<crate::db::models::ObservationDiversitySummaryRow> =
        observation_diversity_summary::table
            .filter(observation_diversity_summary::subject_cid.eq(&q.subject_cid))
            .filter(observation_diversity_summary::observation_kind.eq(&q.kind))
            .first(&mut conn)
            .optional()
            .map_err(|e| {
                StorageError::Internal(format!("observation_diversity_summary query failed: {}", e))
            })?;

    Ok(response::ok(&row.map(ObservationDiversitySummaryView::from)))
}
