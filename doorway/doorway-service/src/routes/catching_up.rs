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
use hyper::{Method, Response, StatusCode};

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
    // The same `cause` the HTML page renders, carried for programmatic clients.
    //
    // The 2026-07-19 design deliberately left the JSON opaque ("for an API client
    // that is correct backpressure") and fixed only the human's dead end. The
    // 2026-08-20 investigation is the counter-evidence: a breaker-open shed (the
    // upstream was NEVER called) and a genuine admission shed are indistinguishable
    // on the wire, so every machine consumer reads an outage as benign churn. That
    // one ambiguity is why saga ch04/ch06/ch10 reds were repeatedly diagnosed as
    // post-deploy churn, and why the fleet-quiesce gate cannot tell "still settling"
    // from "upstream is down" (measured: doorway_upstream_breaker_open_total 66->69
    // in six minutes while admission shedTotal stayed 0 on BOTH doorways).
    //
    // Additive only: `status` and `retryAfter` keep their exact legacy meaning and
    // values, so any client reading those two is unaffected. What is new is that a
    // reader who WANTS to tell the two apart now can.
    let (cause_kind, error_streak, circuit) = match &cause {
        ShedCause::Upstream {
            circuit,
            error_streak,
            ..
        } => ("upstream", *error_streak, circuit.as_str()),
        ShedCause::Admission => ("admission", 0, "n/a"),
    };
    let body = serde_json::json!({
        "status": "catching-up",
        "retryAfter": retry_after_secs,
        "cause": cause_kind,
        "circuit": circuit,
        "errorStreak": error_streak,
    });
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

/// True for the notary HEAD-declare write route (`POST /db/content/{id}/head`)
/// — mirrors elohim-storage's `is_head_declare_write` (`elohim/elohim-storage/
/// src/http.rs`) byte-for-byte in shape. Doorway carries NO authority logic of
/// its own (the route registry's `auth_required` metadata is declared but
/// unenforced — see `genesis/data/timeline/backlog/
/// doorway-auth-required-metadata-unenforced.md`), so its only correct move
/// for this route is to always PROXY through to storage and let storage's own
/// auth-first ordering (fixed in ab316cad7) decide: 401/403 for a non-author,
/// or its own catching-up 503 for a confirmed author under genuine write-pool
/// exhaustion. A blind doorway-side shed — e.g. the per-upstream circuit
/// breaker opening after a run of failures while a peer is catching up —
/// would otherwise mask that authority refusal behind an opaque 503 no matter
/// what storage would have decided, because the request never reaches
/// storage at all (measured on alpha-A: "Non-author move of HEAD" expected
/// 401/403, got 503 — the doorway breaker was open and shed the call).
///
/// Deliberately excludes `/head-record` (no author gate; a read surface) and
/// `/canonical-head` (god-mode staging tier, no author gate) — those stay
/// behind the normal catching-up shed unchanged, same as storage's exclusion.
pub fn is_head_declare_write(method: &Method, path: &str) -> bool {
    *method == Method::POST && path.starts_with("/db/content/") && path.ends_with("/head")
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
    fn json_variant_keeps_status_and_retry_after() {
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

    /// The discriminator the whole 2026-08-20 investigation turned on: a
    /// breaker-open shed (upstream never called) and a genuine admission shed
    /// must not be the same bytes. `status`/`retryAfter` keep their legacy
    /// meaning; `cause` is what lets a probe or a CI gate tell an outage from
    /// churn instead of waiting out a fleet that is never coming back.
    #[tokio::test]
    async fn json_variant_names_its_cause() {
        use http_body_util::BodyExt;

        let read = |resp: Response<Full<Bytes>>| async move {
            let bytes = resp.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        };

        let admission = read(shed_response(false, 2, ShedCause::Admission)).await;
        assert_eq!(admission["status"], "catching-up");
        assert_eq!(admission["retryAfter"], 2);
        assert_eq!(admission["cause"], "admission");

        let upstream = read(shed_response(
            false,
            30,
            ShedCause::Upstream {
                endpoint: "http://storage:8090".into(),
                circuit: "open".into(),
                error_streak: 4,
            },
        ))
        .await;
        assert_eq!(upstream["status"], "catching-up");
        assert_eq!(upstream["retryAfter"], 30);
        assert_eq!(upstream["cause"], "upstream");
        assert_eq!(upstream["circuit"], "open");
        assert_eq!(upstream["errorStreak"], 4);

        // The endpoint is deliberately NOT serialized: the HTML page does not
        // show it either, and it is an internal cluster address.
        assert!(upstream.get("endpoint").is_none());
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
    fn head_declare_write_matches_post_head() {
        assert!(is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/head"
        ));
    }

    #[test]
    fn head_declare_write_excludes_get() {
        assert!(!is_head_declare_write(
            &Method::GET,
            "/db/content/some-id/head"
        ));
    }

    #[test]
    fn head_declare_write_excludes_head_record() {
        assert!(!is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/head-record"
        ));
    }

    #[test]
    fn head_declare_write_excludes_canonical_head() {
        assert!(!is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/canonical-head"
        ));
    }

    #[test]
    fn head_declare_write_excludes_unrelated_write_route() {
        assert!(!is_head_declare_write(&Method::POST, "/db/content/bulk"));
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
