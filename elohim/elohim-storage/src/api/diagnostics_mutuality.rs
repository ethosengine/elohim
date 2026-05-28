//! GET /api/v1/diagnostics/mutuality-audit?hub={hub_id}. Spec §6.3.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::db::models::MutualityAuditLogRow;
use crate::db::mutuality_audit_log;
use crate::db::DbPool;
use crate::error::StorageError;

#[derive(Serialize)]
struct MutualityAuditView {
    rows: Vec<MutualityAuditLogRowSerial>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MutualityAuditLogRowSerial {
    commitment_cid: String,
    provider_dwelling_hub_id: String,
    recipient_dwelling_hub_id: String,
    reciprocity_status: String,
    days_since_authored: i32,
    grace_period_days: i32,
    signaled_at: Option<String>,
    swept_at: String,
}

impl From<MutualityAuditLogRow> for MutualityAuditLogRowSerial {
    fn from(r: MutualityAuditLogRow) -> Self {
        Self {
            commitment_cid: r.commitment_cid,
            provider_dwelling_hub_id: r.provider_dwelling_hub_id,
            recipient_dwelling_hub_id: r.recipient_dwelling_hub_id,
            reciprocity_status: r.reciprocity_status,
            days_since_authored: r.days_since_authored,
            grace_period_days: r.grace_period_days,
            signaled_at: r.signaled_at,
            swept_at: r.swept_at,
        }
    }
}

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from(r#"{"error":"GET only"}"#)))
            .unwrap());
    }
    // Capture query string before req is consumed by spawn_blocking move
    let query_str = req.uri().query().unwrap_or("").to_string();
    let hub_id = parse_query_param(&query_str, "hub")
        .ok_or_else(|| StorageError::InvalidInput("?hub=<hub_id> required".into()))?;
    let pool = pool.clone();
    let rows =
        tokio::task::spawn_blocking(move || -> Result<Vec<MutualityAuditLogRow>, StorageError> {
            let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
            mutuality_audit_log::list_recent_for_recipient(&mut conn, &hub_id, 100)
        })
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))??;
    let view = MutualityAuditView {
        rows: rows.into_iter().map(Into::into).collect(),
    };
    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

fn parse_query_param(query_str: &str, key: &str) -> Option<String> {
    for pair in query_str.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == key && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}
