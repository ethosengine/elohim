//! Browser-facing catching-up shed page (content-negotiated 503).
//!
//! Cat C node-local OPERATIONAL presentation (p2p-design-gate): no DHT entry,
//! no table, no coordinator fn — a rendering of in-process breaker/admission
//! snapshots, reconstructable by observation. A doorway serving its OWN shed
//! state passes the swap test (a sibling doorway serves its own equivalent).
//! Spec: genesis/docs/superpowers/specs/2026-07-19-doorway-catching-up-page-design.md
//!
//! Negotiation rule: only an explicit `text/html` / `application/xhtml+xml` in
//! `Accept` (a browser navigation) gets the HTML page. Every other client —
//! SDKs, curl, blob/image fetches, `*/*` — keeps the exact legacy JSON body
//! `{"status":"catching-up","retryAfter":N}`. Both variants stay 503 +
//! `Retry-After`; the HTML adds `Cache-Control: no-store`.

use askama::Template;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::HeaderMap;
use hyper::{Response, StatusCode};

use super::upstream_health::UpstreamBreakers;

/// Why a request was shed — drives the page's headline and copy honestly.
#[derive(Debug, Clone)]
pub enum ShedCause {
    /// The path to the storage upstream is unhealthy: circuit open, upstream
    /// backpressure honored, or a connect/timeout failure.
    Upstream {
        endpoint: String,
        /// "closed" | "half-open" | "open" | "unknown"
        circuit: String,
        error_streak: u32,
    },
    /// The doorway's own inbound admission gate is at ceiling.
    Admission,
}

/// Build an [`ShedCause::Upstream`] from the breaker's read-only snapshot for
/// `endpoint`. Uses `snapshot()` (never `is_open`) so observing the cause can
/// never admit a half-open trial as a side effect.
pub fn upstream_cause(breakers: &UpstreamBreakers, endpoint: &str) -> ShedCause {
    match breakers
        .snapshot()
        .into_iter()
        .find(|s| s.endpoint == endpoint)
    {
        Some(s) => ShedCause::Upstream {
            endpoint: endpoint.to_string(),
            circuit: s.circuit.to_string(),
            error_streak: s.error_streak,
        },
        None => ShedCause::Upstream {
            endpoint: endpoint.to_string(),
            circuit: "unknown".to_string(),
            error_streak: 0,
        },
    }
}

/// True when the client is a browser navigation that prefers HTML.
/// Deliberately conservative: a bare `*/*` (curl, SDKs) stays JSON.
pub fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(hyper::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html") || v.contains("application/xhtml+xml"))
        .unwrap_or(false)
}

/// Askama template for the staged catching-up page. Compiled into the binary —
/// zero runtime I/O, zero dependence on the (unavailable) upstream.
#[derive(Template)]
#[template(path = "catching_up.html")]
struct CatchingUpPage {
    retry_after: u64,
    version: &'static str,
    /// "upstream" | "admission" — branches the copy + poll logic.
    cause_kind: &'static str,
    error_streak: u32,
    circuit: String,
}

/// The one shed responder every catching-up site delegates to.
pub fn shed_response(
    wants_html: bool,
    retry_after_secs: u64,
    cause: ShedCause,
) -> Response<Full<Bytes>> {
    if wants_html {
        let (cause_kind, error_streak, circuit) = match &cause {
            ShedCause::Upstream {
                circuit,
                error_streak,
                ..
            } => ("upstream", *error_streak, circuit.clone()),
            ShedCause::Admission => ("admission", 0, "n/a".to_string()),
        };
        let page = CatchingUpPage {
            retry_after: retry_after_secs,
            version: env!("CARGO_PKG_VERSION"),
            cause_kind,
            error_streak,
            circuit,
        };
        // A template render failure falls through to JSON — a shed must never
        // become a 500.
        if let Ok(html) = page.render() {
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Retry-After", retry_after_secs.to_string())
                .header("Cache-Control", "no-store")
                .body(Full::new(Bytes::from(html)))
                .expect("infallible 503 html response");
        }
    }
    let body = serde_json::json!({ "status": "catching-up", "retryAfter": retry_after_secs });
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .header("Retry-After", retry_after_secs.to_string())
        .body(Full::new(Bytes::from(body.to_string())))
        .expect("infallible 503 json response")
}

/// Read-only diagnostic probes that must bypass the breaker entirely: the
/// platform must not blind its own probes during exactly the incident they
/// exist to explain (substrate-trust-contract runbook names these as the
/// authority). Bypassing means neither consulting `is_open` nor feeding
/// `record()` — a pure pass-through bounded by the normal client timeout.
pub fn is_diagnostic_probe(path: &str) -> bool {
    matches!(path, "/p2p/status" | "/db/p2p/conductor-diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{HeaderValue, ACCEPT};

    fn headers_with_accept(v: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(ACCEPT, HeaderValue::from_str(v).unwrap());
        h
    }

    #[test]
    fn browser_accept_gets_html() {
        let h = headers_with_accept(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,*/*;q=0.8",
        );
        assert!(accepts_html(&h));
    }

    #[test]
    fn json_and_wildcard_and_absent_stay_json() {
        assert!(!accepts_html(&headers_with_accept("application/json")));
        assert!(!accepts_html(&headers_with_accept("*/*")));
        assert!(!accepts_html(&headers_with_accept("image/avif,image/webp")));
        assert!(!accepts_html(&HeaderMap::new()));
    }

    #[test]
    fn json_variant_body_shape_unchanged() {
        let resp = shed_response(false, 30, ShedCause::Admission);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers().get("Retry-After").unwrap().to_str().unwrap(),
            "30"
        );
        assert_eq!(
            resp.headers()
                .get("Content-Type")
                .unwrap()
                .to_str()
                .unwrap(),
            "application/json"
        );
    }

    #[tokio::test]
    async fn html_variant_is_503_html_with_retry_after() {
        use http_body_util::BodyExt;
        let resp = shed_response(
            true,
            30,
            ShedCause::Upstream {
                endpoint: "http://storage:8090".into(),
                circuit: "open".into(),
                error_streak: 4,
            },
        );
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers().get("Retry-After").unwrap().to_str().unwrap(),
            "30"
        );
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        assert_eq!(
            resp.headers()
                .get("Cache-Control")
                .unwrap()
                .to_str()
                .unwrap(),
            "no-store"
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Recovery progress"));
        assert!(html.contains("Catching up"));
        // The breaker state is rendered for the visitor.
        assert!(html.contains("open"));
    }

    #[test]
    fn admission_html_renders_capacity_copy() {
        let resp = shed_response(true, 5, ShedCause::Admission);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));
    }

    #[test]
    fn diagnostic_probe_paths() {
        assert!(is_diagnostic_probe("/p2p/status"));
        assert!(is_diagnostic_probe("/db/p2p/conductor-diagnostics"));
        assert!(!is_diagnostic_probe("/db/content"));
        assert!(!is_diagnostic_probe("/"));
    }

    #[test]
    fn upstream_cause_unknown_endpoint_degrades() {
        let b = UpstreamBreakers::default();
        let c = upstream_cause(&b, "http://never-seen:8090");
        match c {
            ShedCause::Upstream {
                circuit,
                error_streak,
                ..
            } => {
                assert_eq!(circuit, "unknown");
                assert_eq!(error_streak, 0);
            }
            ShedCause::Admission => panic!("expected upstream cause"),
        }
    }
}
