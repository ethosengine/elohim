//! Health check endpoints
//!
//! Provides /health and /healthz endpoints for Kubernetes probes.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;

use crate::config::Args;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub mode: String,
    pub node_id: String,
}

/// Handle health check request
pub fn health_check(args: &Args) -> Response<Full<Bytes>> {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        mode: if args.dev_mode {
            "development".to_string()
        } else {
            "production".to_string()
        },
        node_id: args.node_id.to_string(),
    };

    let body = serde_json::to_string(&response).unwrap_or_else(|_| r#"{"status":"ok"}"#.to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
