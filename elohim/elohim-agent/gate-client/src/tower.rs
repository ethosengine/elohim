//! Tower middleware integration for the gate-client.
//!
//! Wraps Axum (or any `tower`-compatible) HTTP POST/PUT/PATCH/DELETE routes so
//! the gate fires before the inner service handles the request. Every doorway
//! HTTP state-changing route can opt-in with a single
//! `.layer(gate_client::tower_layer())` call.
//!
//! # Phase 1 limitation: path-based event inference
//!
//! In Phase 1 the `RelationalImpactEvent` is inferred from the request path
//! segment rather than parsed from the request body. This keeps the tower layer
//! body-agnostic and avoids consuming the body before the inner service sees it.
//!
//! **IMPORTANT — path-inferred events carry sentinel field values.**
//! When Phase 2 lands the DAG interpreter, `context-assemble` steps will read
//! event fields. Consumers MUST NOT treat field values from path-inferred events
//! as reliable — only the variant *kind* is meaningful. Call
//! `is_path_inferred(&event)` to detect these events at any Phase 2 call site.
//!
//! Phase 7 will wire full body-extracted event context when doorway integrates.
//! Any new state-changing route that carries relational impact and is NOT listed
//! below MUST be added here — treat a missing mapping as a code-review gate.
//!
//! Known limitation: if a path does not match any of the eight registered
//! segments the request falls through to the inner service WITHOUT a gate
//! check. This is a deliberate Phase 1 trade-off.
//!
//! Registered path → event mappings (first path segment match):
//! - `/content`        → `ContentPublish`
//! - `/attestation`    → `AttestationWrite`
//! - `/economic-event` → `EconomicEventEmit`
//! - `/peer-message`   → `PeerMessage`
//! - `/sync`           → `SyncToPeers`
//! - `/advice`         → `AdviceSought`
//! - `/agent/invoke`   → `CapabilityInvoke`
//! - `/crossing`       → `PrivateToPublicCrossing`

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use tower::{Layer, Service};

use crate::{
    events::RelationalImpactEvent,
    types::{GateDecision, GateStatus},
};

// ─── Public factory ───────────────────────────────────────────────────────────

/// Create a `tower::Layer` that wraps every state-changing request in a gate check.
///
/// Gated methods: POST, PUT, PATCH, DELETE. GET/HEAD/OPTIONS pass straight through.
///
/// This is the entry point for **Pattern B** at the doorway HTTP boundary —
/// see the [crate-level "Pattern B" docs](crate#pattern-b--doorway-http-post-handler-towerlayer)
/// for a complete example.
///
/// The layer infers a [`RelationalImpactEvent`] from the request path (see
/// the module-level table), calls [`crate::check`], and on non-Allow decisions
/// short-circuits with:
///
/// - **Decline** → HTTP 403 with JSON grounds in the body and
///   `x-gate-verdict: decline` header.
/// - **Escalate** → HTTP 202 with the review target / severity in the body
///   and `x-gate-verdict: escalate` header.
/// - **Allow / Verdict** → pass through to the inner service with
///   `x-gate-verdict: allow` or `x-gate-verdict: verdict` added.
///
/// Usage sketch:
///
/// ```rust,no_run
/// use axum::{routing::post, Router};
///
/// async fn create_content() -> &'static str { "ok" }
///
/// let app: Router = Router::new()
///     .route("/content", post(create_content))
///     .layer(gate_client::tower_layer());
/// ```
pub fn tower_layer() -> GateLayer {
    GateLayer::new()
}

// ─── Path-inference marker ────────────────────────────────────────────────────

/// Returns `true` if this event was produced by Phase 1 path inference.
///
/// Path-inferred events carry sentinel field values (`"inferred"`, `"unknown"`,
/// `0`, etc.). The *variant kind* is reliable; individual field values are NOT.
///
/// Any Phase 2 `context-assemble` step that reads event fields MUST check this
/// first and skip field-level reads when `true`.
///
/// Phase 7: body parsing replaces path inference. Consumers MUST NOT read field
/// values from path-inferred events as reliable — only the variant kind.
pub fn is_path_inferred(event: &RelationalImpactEvent) -> bool {
    // All events produced by `infer_event_from_path` carry the sentinel string
    // "inferred" as the primary identifier field. This is the structural marker
    // of Phase 1 path inference — maintained until body parsing lands in Phase 7.
    match event {
        RelationalImpactEvent::ContentPublish { content_cid, .. } => content_cid == "inferred",
        RelationalImpactEvent::AttestationWrite { subject_hash, .. } => subject_hash == "inferred",
        RelationalImpactEvent::EconomicEventEmit { event_kind, .. } => event_kind == "inferred",
        RelationalImpactEvent::PeerMessage { payload_kind, .. } => payload_kind == "inferred",
        RelationalImpactEvent::SyncToPeers { manifest_cid, .. } => manifest_cid == "inferred",
        RelationalImpactEvent::AdviceSought { summary_cid, .. } => summary_cid == "inferred",
        RelationalImpactEvent::CapabilityInvoke { capability, .. } => capability == "inferred",
        RelationalImpactEvent::PrivateToPublicCrossing { source_space, .. } => {
            source_space == "inferred"
        }
    }
}

// ─── Layer ────────────────────────────────────────────────────────────────────

/// A `tower::Layer` that injects a gate check before every state-changing request.
#[derive(Clone, Debug, Default)]
pub struct GateLayer;

impl GateLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for GateLayer {
    type Service = GateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GateService { inner }
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

/// A `tower::Service` that calls the gate before delegating to the inner service.
#[derive(Clone, Debug)]
pub struct GateService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GateService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    ReqBody: Send + 'static,
    ResBody: From<Bytes> + Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Per tower contract: the service polled ready is the one we must call.
        // Clone now, replace self.inner with the clone, use the owned original.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        // Only gate state-changing methods. Read-only methods pass straight through.
        let is_state_changing = matches!(
            req.method(),
            &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
        );
        if !is_state_changing {
            return Box::pin(async move { inner.call(req).await });
        }

        let path = req.uri().path().to_owned();
        let event_opt = infer_event_from_path(&path);

        // No event mapping → fall through without gate check (Phase 1 limitation).
        // See module-level doc comment for the rationale and upgrade path.
        let Some(event) = event_opt else {
            return Box::pin(async move { inner.call(req).await });
        };

        Box::pin(async move {
            let decision: GateDecision = match crate::check(event).await {
                Ok(d) => d,
                Err(e) => {
                    // Gate transport error — fail open with a log in dev-context.
                    // Phase 2 will introduce configurable fail-open vs fail-closed.
                    tracing::warn!(error = %e, "gate-client transport error; failing open");
                    GateDecision::allow_mocked(crate::Phase::DevContext)
                }
            };

            match &decision.status {
                GateStatus::Allow { .. } => {
                    // Attach the verdict phase as a debug response header after
                    // the inner service responds.
                    let mut resp = inner.call(req).await?;
                    resp.headers_mut()
                        .insert("x-gate-verdict", http::HeaderValue::from_static("allow"));
                    Ok(resp)
                }

                GateStatus::Decline { grounds } => {
                    let body_json = serde_json::json!({
                        "gate": "declined",
                        "grounds": {
                            "category": grounds.category,
                            "summary": grounds.summary,
                            "principleRefs": grounds.principle_refs,
                        }
                    });
                    let body_bytes: Bytes = serde_json::to_vec(&body_json)
                        .unwrap_or_else(|_| b"{}".to_vec())
                        .into();

                    let mut resp = Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header("content-type", "application/json")
                        .body(ResBody::from(body_bytes))
                        .expect("infallible 403 response build");
                    resp.headers_mut()
                        .insert("x-gate-verdict", http::HeaderValue::from_static("decline"));
                    Ok(resp)
                }

                GateStatus::Escalate { target, severity } => {
                    let body_json = serde_json::json!({
                        "gate": "escalated",
                        "target": target,
                        "severity": severity,
                    });
                    let body_bytes: Bytes = serde_json::to_vec(&body_json)
                        .unwrap_or_else(|_| b"{}".to_vec())
                        .into();

                    let mut resp = Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .header("content-type", "application/json")
                        .body(ResBody::from(body_bytes))
                        .expect("infallible 202 response build");
                    resp.headers_mut()
                        .insert("x-gate-verdict", http::HeaderValue::from_static("escalate"));
                    Ok(resp)
                }

                GateStatus::Verdict(_tag) => {
                    // Evaluator-shape gates: pass through; attach verdict header.
                    let mut resp = inner.call(req).await?;
                    resp.headers_mut()
                        .insert("x-gate-verdict", http::HeaderValue::from_static("verdict"));
                    Ok(resp)
                }
            }
        })
    }
}

// ─── Path → Event inference ───────────────────────────────────────────────────

/// Infer a `RelationalImpactEvent` from an HTTP path.
///
/// Matches on the **first path segment** (not a prefix) to avoid false matches
/// (e.g. `/contentious` must NOT match `/content`).
///
/// Returns `None` for paths that have no declared mapping. See module-level
/// doc for the full list and the Phase 1 fall-through contract.
///
/// **WARNING — sentinel field values.**  All returned events carry `"inferred"` /
/// `"unknown"` / `0` as field values. These are structural sentinels, not real
/// data. See `is_path_inferred()` and the module-level doc.
///
/// Phase 7: body parsing replaces path inference. Consumers MUST NOT read field
/// values from path-inferred events as reliable — only the variant kind.
fn infer_event_from_path(path: &str) -> Option<RelationalImpactEvent> {
    // Strip leading slash, then split into segments. Match on first (and
    // optionally second) segment — no prefix matching to avoid over-matching.
    let trimmed = path.trim_start_matches('/');
    let mut segments = trimmed.split('/');
    let first = segments.next();
    let second = segments.next();

    match (first, second) {
        (Some("content"), _) => Some(RelationalImpactEvent::ContentPublish {
            content_cid: "inferred".to_string(),
            declared_reach: "public".to_string(),
            author: "unknown".to_string(),
        }),
        (Some("attestation"), _) => Some(RelationalImpactEvent::AttestationWrite {
            subject_hash: "inferred".to_string(),
            claim_kind: "inferred".to_string(),
            issuer: "unknown".to_string(),
        }),
        (Some("economic-event"), _) => Some(RelationalImpactEvent::EconomicEventEmit {
            event_kind: "inferred".to_string(),
            provider: "unknown".to_string(),
            receiver: "unknown".to_string(),
            quantity: "0".to_string(),
        }),
        (Some("peer-message"), _) => Some(RelationalImpactEvent::PeerMessage {
            recipient: "unknown".to_string(),
            payload_kind: "inferred".to_string(),
        }),
        (Some("sync"), _) => Some(RelationalImpactEvent::SyncToPeers {
            manifest_cid: "inferred".to_string(),
            item_count: 0,
        }),
        (Some("advice"), _) => Some(RelationalImpactEvent::AdviceSought {
            requester: "unknown".to_string(),
            summary_cid: "inferred".to_string(),
            topic: "inferred".to_string(),
        }),
        (Some("agent"), Some("invoke")) => Some(RelationalImpactEvent::CapabilityInvoke {
            capability: "inferred".to_string(),
            requester: "unknown".to_string(),
            request_id: "inferred".to_string(),
        }),
        (Some("crossing"), _) => Some(RelationalImpactEvent::PrivateToPublicCrossing {
            source_space: "inferred".to_string(),
            artifact_ref: "inferred".to_string(),
        }),
        _ => None,
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

/// Service-level tests for the gate tower layer, including the Decline path.
/// These tests require axum (dev-dependency) and tokio.
#[cfg(test)]
mod tower_service_tests {
    use axum::{body::Body, routing::post, Router};
    use http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn ok_handler() -> axum::response::Response {
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(r#"{"handled":true}"#))
            .unwrap()
    }

    fn build_gated_router() -> Router {
        Router::new()
            .route("/content", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    async fn post_to(router: Router, path: &str) -> (StatusCode, String) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn decline_returns_403_with_json_body() {
        // Inject a Decline decision for this thread.
        crate::__test_set_decision_override(Some(crate::testing::mock_decline(
            "test-category",
            "test decline reason",
        )));

        let router = build_gated_router();
        let (status, body) = post_to(router, "/content").await;

        assert_eq!(status, StatusCode::FORBIDDEN, "Decline must produce 403");
        assert!(
            body.contains("\"gate\":\"declined\"") || body.contains("\"gate\": \"declined\""),
            "403 response must carry gate JSON body, got: {body}"
        );
        assert!(
            body.contains("test-category"),
            "403 body must include grounds category, got: {body}"
        );
    }

    #[tokio::test]
    async fn decline_403_is_absent_when_gate_allows() {
        // No override — default DevContext always returns Allow.
        let router = build_gated_router();
        let (status, _body) = post_to(router, "/content").await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Allow must reach inner handler and return 200"
        );
    }

    #[tokio::test]
    async fn put_is_gated_and_reaches_inner_handler() {
        // State-changing verbs other than POST must also pass through the gate.
        // With DevContext Always-Allow, PUT should reach the inner service.
        let router = Router::new()
            .route("/content", axum::routing::put(ok_handler))
            .layer(crate::tower::GateLayer::new());

        let req = Request::builder()
            .method(Method::PUT)
            .uri("/content")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "PUT should reach inner handler when gate allows"
        );
    }

    #[tokio::test]
    async fn patch_is_gated_and_reaches_inner_handler() {
        // PATCH must also pass through the gate.
        let router = Router::new()
            .route("/content", axum::routing::patch(ok_handler))
            .layer(crate::tower::GateLayer::new());

        let req = Request::builder()
            .method(Method::PATCH)
            .uri("/content")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "PATCH should reach inner handler when gate allows"
        );
    }
}

// ─── Unit tests for path inference ───────────────────────────────────────────

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn content_segment_matches() {
        assert!(infer_event_from_path("/content").is_some());
        assert!(infer_event_from_path("/content/foo").is_some());
        // Note: in real usage `req.uri().path()` strips the query string before
        // calling this function, so `/content?x=1` never reaches here with the
        // query attached. The gate layer calls `uri().path()` which returns only
        // the path component. This function receives clean paths.
    }

    #[test]
    fn contentious_does_not_match_content() {
        // The critical regression: starts_with would have matched this wrongly.
        assert!(infer_event_from_path("/contentious").is_none());
        assert!(infer_event_from_path("/contentious/bar").is_none());
    }

    #[test]
    fn agent_invoke_requires_second_segment() {
        assert!(infer_event_from_path("/agent/invoke").is_some());
        assert!(infer_event_from_path("/agent/invoke/extra").is_some());
        // /agent alone has no mapping (no second segment "invoke")
        assert!(infer_event_from_path("/agent").is_none());
        assert!(infer_event_from_path("/agent/other").is_none());
    }

    #[test]
    fn unmapped_paths_return_none() {
        assert!(infer_event_from_path("/unmapped").is_none());
        assert!(infer_event_from_path("/").is_none());
        assert!(infer_event_from_path("").is_none());
    }

    #[test]
    fn is_path_inferred_detects_sentinel_events() {
        let event = infer_event_from_path("/content").unwrap();
        assert!(is_path_inferred(&event));
    }

    // ─── All 8 registered paths resolve to the correct event variant ──────────

    #[test]
    fn all_registered_paths_resolve() {
        let cases: &[(&str, &str)] = &[
            ("/content", "content-publish"),
            ("/attestation", "attestation-write"),
            ("/economic-event", "economic-event-emit"),
            ("/peer-message", "peer-message"),
            ("/sync", "sync-to-peers"),
            ("/advice", "advice-sought"),
            ("/agent/invoke", "capability-invoke"),
            ("/crossing", "private-to-public-crossing"),
        ];
        for (path, expected_kind) in cases {
            let event = infer_event_from_path(path)
                .unwrap_or_else(|| panic!("path '{path}' must resolve to an event"));
            assert_eq!(
                event.kind(),
                *expected_kind,
                "path '{path}' must infer event kind '{expected_kind}'"
            );
        }
    }

    // ─── Path inference edge cases ────────────────────────────────────────────

    #[test]
    fn empty_path_returns_none() {
        assert!(infer_event_from_path("").is_none());
    }

    #[test]
    fn bare_slash_returns_none() {
        assert!(infer_event_from_path("/").is_none());
    }

    #[test]
    fn trailing_slash_on_registered_path_still_resolves() {
        // "/content/" → first segment is "content", second is "". Still matches.
        assert!(infer_event_from_path("/content/").is_some());
    }

    #[test]
    fn deeply_nested_registered_path_resolves_by_first_segment() {
        assert!(infer_event_from_path("/content/a/b/c/d").is_some());
        let event = infer_event_from_path("/content/a/b/c/d").unwrap();
        assert_eq!(event.kind(), "content-publish");
    }

    #[test]
    fn agent_without_invoke_returns_none() {
        assert!(infer_event_from_path("/agent").is_none());
        assert!(infer_event_from_path("/agent/").is_none());
    }

    #[test]
    fn agent_with_wrong_second_segment_returns_none() {
        assert!(infer_event_from_path("/agent/invoke-not").is_none());
        assert!(infer_event_from_path("/agent/status").is_none());
        assert!(infer_event_from_path("/agent/metrics").is_none());
    }

    #[test]
    fn agent_invoke_with_extra_segments_resolves() {
        // "/agent/invoke/foo/bar" — first two segments are "agent"/"invoke", matches.
        let event = infer_event_from_path("/agent/invoke/extra").unwrap();
        assert_eq!(event.kind(), "capability-invoke");
    }

    #[test]
    fn is_path_inferred_is_true_for_all_inferred_events() {
        let paths = [
            "/content",
            "/attestation",
            "/economic-event",
            "/peer-message",
            "/sync",
            "/advice",
            "/agent/invoke",
            "/crossing",
        ];
        for path in &paths {
            let event = infer_event_from_path(path)
                .unwrap_or_else(|| panic!("expected event for path {path}"));
            assert!(
                is_path_inferred(&event),
                "is_path_inferred must be true for event inferred from '{path}'"
            );
        }
    }

    #[test]
    fn manually_constructed_real_event_is_not_path_inferred() {
        // An event with real (non-sentinel) field values must NOT be flagged as
        // path-inferred. This guards against false positives in Phase 2 logic.
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "bafkreiabc123".to_string(),
            declared_reach: "public".to_string(),
            author: "agent-real".to_string(),
        };
        assert!(
            !is_path_inferred(&event),
            "real events with non-sentinel fields must not be flagged as path-inferred"
        );
    }
}
