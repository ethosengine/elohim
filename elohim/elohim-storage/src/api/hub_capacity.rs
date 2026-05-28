//! GET /api/v1/hub/{hub_id}/capacity. Spec §7.2.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::hub_capacity_service::compute_hub_capacity;

pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    hub_id: &str,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from(r#"{"error":"GET only"}"#)))
            .unwrap());
    }
    let hub_id_owned = hub_id.to_string();
    let pool = pool.clone();
    let view = tokio::task::spawn_blocking(move || -> Result<_, StorageError> {
        let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
        compute_hub_capacity(&mut conn, &hub_id_owned)
    })
    .await
    .map_err(|e| StorageError::Internal(e.to_string()))??;
    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
