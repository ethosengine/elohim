//! Elohim Agent SDK routes — transparent proxy to elohim-agent-sdk sidecar `/invoke`

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::server::AppState;

pub async fn handle_elohim_agent_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let agent_url = state.args.elohim_agent_url.clone();
    // Strip "/api/v1/elohim" prefix — the remainder becomes the sidecar path.
    // e.g. /api/v1/elohim/invoke        → /invoke
    //      /api/v1/elohim/invoke/health  → /invoke/health
    let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
    forward_to_agent(req, &agent_url, sidecar_path).await
}

async fn forward_to_agent(
    req: Request<Incoming>,
    agent_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let agent_endpoint = format!("{}{}", agent_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{agent_endpoint}?{q}"),
        None => agent_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding request to elohim-agent-sdk sidecar");

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
            let budget_remaining = response
                .headers()
                .get("x-elohim-budget-remaining")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            match response.bytes().await {
                Ok(body) => {
                    let mut resp = Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin");
                    if let Some(budget) = budget_remaining {
                        resp = resp.header("X-Elohim-Budget-Remaining", budget);
                    }
                    resp.body(Full::new(Bytes::from(body.to_vec()))).unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read agent response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read agent response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward request to elohim-agent-sdk sidecar");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to agent sidecar: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_path_mapping() {
        let path = "/api/v1/elohim/invoke";
        let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
        assert_eq!(sidecar_path, "/invoke");
    }

    #[test]
    fn test_health_path_mapping() {
        let path = "/api/v1/elohim/invoke/health";
        let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
        assert_eq!(sidecar_path, "/invoke/health");
    }

    #[test]
    fn test_url_built_with_query() {
        let base = "http://localhost:8095";
        let path = "/invoke";
        let query = "dry_run=true";
        let url = format!("{}{}?{}", base.trim_end_matches('/'), path, query);
        assert_eq!(url, "http://localhost:8095/invoke?dry_run=true");
    }

    #[test]
    fn test_url_built_without_query() {
        let base = "http://localhost:8095/";
        let path = "/invoke";
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        assert_eq!(url, "http://localhost:8095/invoke");
    }
}
