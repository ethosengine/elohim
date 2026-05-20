//! valueflows-bridge — the hREA / VF-GraphQL bridge.
//!
//! Mounted by elohim-storage at `/api/v1/vf-graphql`. M1 ships a stub handler
//! that returns 503 unimplemented for any non-trivial query and fixture
//! `EconomicEvent` data for the M1 tracer-bullet query.
//!
//! See `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

pub mod schema;

/// Handle a single HTTP request arriving at `/api/v1/vf-graphql`.
///
/// Mirrors the existing `elohim-storage::graphql::server::handle_graphql` shape
/// so elohim-storage can route to either endpoint with the same pattern.
///
/// M1: returns 503 for any non-tracer-bullet query; fixture `EconomicEvent`
/// data for the M1 tracer-bullet query. M2+ will wire identity bridge,
/// authority gate, EPR atom emit, and real hREA projection.
pub async fn handle_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, BridgeError> {
    if req.method() != Method::POST {
        return Ok(error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "POST required for /api/v1/vf-graphql",
        ));
    }

    // Collect body bytes (same pattern as elohim-storage::graphql::server).
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| BridgeError::ReadBody(e.to_string()))?
        .to_bytes();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_graphql_request",
                &format!("could not parse body as GraphQL request: {e}"),
            ));
        }
    };

    let schema = schema::build_schema();
    let gql_response = schema.execute(gql_request).await;

    let body = serde_json::to_vec(&gql_response)
        .map_err(|e| BridgeError::SerializeResponse(e.to_string()))?;

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| BridgeError::BuildResponse(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("could not read request body: {0}")]
    ReadBody(String),
    #[error("could not serialize response: {0}")]
    SerializeResponse(String),
    #[error("could not build response: {0}")]
    BuildResponse(String),
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "errors": [{
            "message": message,
            "extensions": { "code": code }
        }]
    });
    let body_bytes = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .expect("static response always builds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_carries_expected_status_and_content_type() {
        let resp = error_response(
            StatusCode::BAD_REQUEST,
            "invalid_graphql_request",
            "boom",
        );
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get("content-type").unwrap().to_str().unwrap(),
            "application/json"
        );
    }

    // End-to-end request/response tests live in the valueflows-tests crate
    // (tests/m1_tracer_bullet.rs + tests/m1_http_smoke.rs) — hyper::body::Incoming
    // is not constructible directly, so a test helper (handle_request_for_test)
    // is added in Task 9 to exercise the parse → schema → response wire.
}
