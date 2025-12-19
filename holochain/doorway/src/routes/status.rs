//! Status endpoint for Doorway
//!
//! Provides runtime status information including active connections.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::server::AppState;

/// Status response payload
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Service name
    pub service: &'static str,
    /// Service version
    pub version: &'static str,
    /// Node ID
    pub node_id: String,
    /// Whether dev mode is enabled
    pub dev_mode: bool,
    /// Number of available hosts in router
    pub available_hosts: usize,
    /// MongoDB connection status
    pub mongodb_connected: bool,
    /// NATS connection status
    pub nats_connected: bool,
}

/// Handle status request
pub async fn status_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let available_hosts = state.router.available_count().await;

    let status = StatusResponse {
        service: "doorway",
        version: env!("CARGO_PKG_VERSION"),
        node_id: state.args.node_id.to_string(),
        dev_mode: state.args.dev_mode,
        available_hosts,
        mongodb_connected: state.mongo.is_some(),
        nats_connected: state.nats.is_some(),
    };

    match serde_json::to_string_pretty(&status) {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize status")))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = StatusResponse {
            service: "doorway",
            version: "0.1.0",
            node_id: "test-node".to_string(),
            dev_mode: true,
            available_hosts: 3,
            mongodb_connected: true,
            nats_connected: true,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("doorway"));
        assert!(json.contains("test-node"));
    }
}
