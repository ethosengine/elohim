//! Risk API controller
//!
//! Routes:
//!   GET  `/api/v1/risk/vulnerability/{placeId}` — compute vulnerability assessment
//!   GET  `/api/v1/risk/alerts`                  — list risk alerts
//!   PATCH `/api/v1/risk/alerts/{id}`            — update alert (ack, resolve, escalate)
//!
//! Category C (operational, SQLite-only). No DHT provenance.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::risk_alerts::RiskAlertQuery;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{RiskAlertView, UpdateRiskAlertInputView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/risk*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    // Route dispatch:
    //   vulnerability/{placeId}  → GET vulnerability
    //   alerts                   → GET list
    //   alerts/{id}              → PATCH update
    if let Some(place_id) = path.strip_prefix("vulnerability/") {
        if method == Method::GET && !place_id.is_empty() {
            return Ok(handle_vulnerability(place_id, pool, ctx).await);
        }
    } else if path == "alerts" && method == Method::GET {
        return Ok(handle_list_alerts(req, pool, ctx).await);
    } else if let Some(alert_id) = path.strip_prefix("alerts/") {
        if method == Method::PATCH && !alert_id.is_empty() {
            return Ok(handle_update_alert(req, alert_id, pool, ctx).await);
        }
    }

    Ok(response::not_found(&format!(
        "Unknown risk route: {} /api/v1/risk/{}",
        method, path
    )))
}

async fn handle_vulnerability(
    place_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    // Compute assessment (async — may fetch weather)
    let assessment =
        crate::services::vulnerability::compute_vulnerability(pool, ctx, place_id).await;

    // Side-effect: generate any newly-triggered alerts
    match get_conn(pool) {
        Ok(mut conn) => {
            if let Err(e) = crate::services::risk_alert::evaluate_and_generate_alerts(
                &mut conn,
                ctx,
                place_id,
                &assessment,
            ) {
                tracing::warn!("Alert generation failed for place {}: {}", place_id, e);
            }
        }
        Err(e) => {
            tracing::warn!("Could not get conn for alert generation: {}", e);
        }
    }

    response::ok(&assessment)
}

async fn handle_list_alerts(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: RiskAlertQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::risk_alerts::list_risk_alerts(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<RiskAlertView> = items.into_iter().map(RiskAlertView::from).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_update_alert(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let v: UpdateRiskAlertInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for update risk alert"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // Derive acknowledged_at from current timestamp if acknowledged_by is being set
    let acknowledged_at = v
        .acknowledged_by
        .as_ref()
        .map(|_| crate::db::models::current_timestamp());

    // resolved_at only set when status transitions to "resolved"
    let resolved_at = v
        .status
        .as_deref()
        .filter(|&s| s == "resolved")
        .map(|_| crate::db::models::current_timestamp());

    match crate::db::risk_alerts::update_risk_alert(
        &mut conn,
        ctx,
        id,
        v.status,
        v.acknowledged_by,
        acknowledged_at,
        resolved_at,
        v.escalated_to,
    ) {
        Ok(alert) => response::ok(&RiskAlertView::from(alert)),
        Err(e) => response::error_response(e),
    }
}
