//! Flow planning routes — shell endpoints (501 Not Implemented)
//! All routes return 501 until the flow planning domain is built.

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;

pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    // Log which endpoint was attempted
    tracing::debug!(method = %method, path = %path, "Flow planning endpoint called (not implemented)");

    Ok(not_implemented(path))
}

fn not_implemented(path: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(
            r#"{{"error": "Not implemented", "endpoint": "/api/v1/flow-planning/{path}"}}"#
        ))))
        .unwrap()
}
