//! Contributor API controller
//!
//! Routes: `/api/v1/contributors/{id}/{dashboard|impact|recognition}`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::contributors;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    ContributorDashboardView, ContributorImpactView, ContributorRecognitionView, EconomicEventView,
};

use super::get_conn;

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
    let _query_str = req.uri().query().unwrap_or("");

    match (&method, path_only) {
        // GET /api/v1/contributors/me/dashboard
        (&Method::GET, "/me/dashboard") => {
            // TODO: extract presence_id from auth context
            let presence_id = "me";
            let mut conn = get_conn(pool)?;
            let result = contributors::get_dashboard(&mut conn, presence_id)?;
            Ok(response::from_option(
                Ok(result.map(ContributorDashboardView::from)),
                "Contributor dashboard not found",
            ))
        }

        // GET /api/v1/contributors/{id}/dashboard
        (&Method::GET, p) if p.ends_with("/dashboard") => {
            let id_part = p.strip_suffix("/dashboard").unwrap_or("");
            let id = extract_id(id_part)
                .ok_or_else(|| StorageError::InvalidInput("Contributor ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = contributors::get_dashboard(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ContributorDashboardView::from)),
                &format!("Dashboard for {} not found", id),
            ))
        }

        // GET /api/v1/contributors/{id}/impact
        (&Method::GET, p) if p.ends_with("/impact") => {
            let id_part = p.strip_suffix("/impact").unwrap_or("");
            let id = extract_id(id_part)
                .ok_or_else(|| StorageError::InvalidInput("Contributor ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = contributors::get_impact(&mut conn, id)?;
            Ok(response::ok(&ContributorImpactView::from(result)))
        }

        // GET /api/v1/contributors/{id}/recognition
        (&Method::GET, p) if p.ends_with("/recognition") => {
            let id_part = p.strip_suffix("/recognition").unwrap_or("");
            let id = extract_id(id_part)
                .ok_or_else(|| StorageError::InvalidInput("Contributor ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let events = contributors::get_recognition_history(&mut conn, id)?;
            let view = ContributorRecognitionView {
                presence_id: id.to_string(),
                events: events.into_iter().map(EconomicEventView::from).collect(),
            };
            Ok(response::ok(&view))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown contributors route: {} {}",
            method, resource_path
        ))),
    }
}
