//! Routing & Distribution API controller
//!
//! Routes:
//! - `POST /api/v1/routing/compute` — compute a route between points
//! - `POST /api/v1/routing/distribution` — compute a distribution plan

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::distribution::DistributionRequestView;
use crate::services::response;
use crate::services::routing::ComputeRouteInputView;

use super::parse_body;

/// Handle `/api/v1/routing*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        (&Method::POST, "compute") => handle_compute_route(req).await,
        (&Method::POST, "distribution") => handle_distribution(req).await,
        _ => response::not_found(&format!(
            "Unknown routing route: {} /api/v1/routing/{}",
            method, path
        )),
    })
}

async fn handle_compute_route(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let input: ComputeRouteInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for compute route"),
    };

    let result = crate::services::routing::compute_route(&input).await;
    response::ok(&result)
}

async fn handle_distribution(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let input: DistributionRequestView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for distribution request"),
    };

    let result = crate::services::distribution::compute_distribution(&input).await;
    response::ok(&result)
}
