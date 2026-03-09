//! Flow planning routes — transparent proxy to elohim-storage `/api/v1/flow-planning/*`

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::server::AppState;

pub async fn handle_flow_planning_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Flow-planning proxy called but STORAGE_URL not configured");
            return service_unavailable("Storage service not configured. Set STORAGE_URL env var.");
        }
    };
    forward_to_storage(req, &storage_url, path).await
}

async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding flow-planning request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        Method::PATCH => client.patch(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward flow-planning request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

fn service_unavailable(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error": "{msg}"}}"#))))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;

    #[test]
    fn service_unavailable_returns_503() {
        let resp = service_unavailable("test error");
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn storage_url_built_with_query() {
        let base = "http://localhost:8090";
        let path = "/api/v1/flow-planning";
        let query = "agent=abc";
        let url = format!("{}{}?{}", base.trim_end_matches('/'), path, query);
        assert_eq!(url, "http://localhost:8090/api/v1/flow-planning?agent=abc");
    }

    #[test]
    fn storage_url_trailing_slash_trimmed() {
        let base = "http://localhost:8090/";
        let path = "/api/v1/flow-planning/abc-123";
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        assert_eq!(url, "http://localhost:8090/api/v1/flow-planning/abc-123");
    }
}
