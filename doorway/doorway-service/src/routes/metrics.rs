//! `GET /metrics` — the durable Prometheus exposition surface.
//!
//! Cat C node-local Operational state (same legitimate doorway-local class as
//! `/status` and `/admin/self-healing`): a doorway serves its OWN runtime
//! counters. No auth — in-cluster scrape posture, identical to storage's
//! `/metrics`; the PodMonitor scrapes the pod's `gateway-ws` port directly.
//! Operator-only is an ingress property, not enforced here (`/status` and
//! `/admin/self-healing` already set that precedent).
//!
//! Lives on the MAIN listener (8080), NOT the watchdog runtime (8079): the
//! watchdog serves only the three probes. A consequence — during a fatal main
//! wedge `/metrics` cannot answer, so the scrape target's `up == 0` IS the
//! wedge-time signal; this surface is the historical/graphable twin.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Render the process-wide registry as a Prometheus text exposition response.
pub fn handle_metrics() -> Response<Full<Bytes>> {
    let body = crate::metrics::gather_text();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(Full::new(Bytes::from(body)))
        .expect("infallible metrics response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn metrics_route_serves_prometheus_text() {
        crate::metrics::register_all();
        let resp = handle_metrics();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "text/plain; version=0.0.4"
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        // A registered metric renders; storage's per-node surface never leaks in.
        assert!(text.contains("doorway_conductor_sessions"), "{text}");
        assert!(!text.contains("elohim_node_"), "{text}");
    }
}
