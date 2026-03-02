//! EPR Head proxy routes
//!
//! Proxies EPR Head requests to elohim-storage with Accept header forwarding.
//!
//! - `GET /api/epr-head/{id}` → `GET {storage_url}/epr-head/{id}`
//! - `PUT /api/epr-head/{id}` → `PUT {storage_url}/epr-head/{id}`

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{header, Method, Request, Response, StatusCode};
use tracing::debug;

/// Handle EPR Head proxy requests.
///
/// Forwards to elohim-storage's `/epr-head/{id}` endpoint, preserving
/// the Accept header for content negotiation (JSON vs DAG-CBOR).
pub async fn handle_epr_head_request(
    req: Request<Incoming>,
    storage_url: &str,
    id: &str,
) -> Result<Response<Full<Bytes>>, String> {
    if id.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
            .unwrap());
    }

    let method = req.method().clone();
    let upstream_url = format!(
        "{}/epr-head/{}",
        storage_url.trim_end_matches('/'),
        urlencoding::encode(id)
    );

    debug!(method = %method, id = %id, upstream = %upstream_url, "Proxying EPR Head request");

    let client = reqwest::Client::new();

    match method {
        Method::GET => {
            // Forward Accept header for content negotiation
            let accept = req
                .headers()
                .get(header::ACCEPT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            let response = client
                .get(&upstream_url)
                .header("Accept", &accept)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| format!("Upstream request failed: {e}"))?;

            proxy_response(response).await
        }
        Method::PUT => {
            // Extract Content-Type before consuming the body
            let content_type = req
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            let body = req
                .collect()
                .await
                .map_err(|e| format!("Failed to read body: {e}"))?;
            let data = body.to_bytes();

            let response = client
                .put(&upstream_url)
                .header("Content-Type", &content_type)
                .body(data.to_vec())
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| format!("Upstream request failed: {e}"))?;

            proxy_response(response).await
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from("Method not allowed")))
            .unwrap()),
    }
}

/// Convert a reqwest response to a hyper response.
async fn proxy_response(response: reqwest::Response) -> Result<Response<Full<Bytes>>, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read upstream body: {e}"))?;

    Ok(Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        .header(header::CONTENT_TYPE, &content_type)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::from(data.to_vec())))
        .unwrap())
}
