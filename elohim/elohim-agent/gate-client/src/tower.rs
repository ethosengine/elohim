//! Tower middleware integration for the gate-client.
//!
//! Wraps Axum (or any `tower`-compatible) HTTP POST routes so the gate fires
//! before the inner service handles the request. Every doorway HTTP POST route
//! can opt-in with a single `.layer(gate_client::tower_layer())` call.
//!
//! # Phase 1 limitation: path-based event inference
//!
//! In Phase 1 the `RelationalImpactEvent` is inferred from the request path
//! segment rather than parsed from the request body. This keeps the tower layer
//! body-agnostic and avoids consuming the body before the inner service sees it.
//!
//! Known limitation: if a path does not match any of the eight registered
//! prefixes the request falls through to the inner service WITHOUT a gate
//! check. This is a deliberate Phase 1 trade-off. Phase 7 will wire full
//! body-extracted event context when doorway integrates. Any new POST route
//! that carries relational impact and is NOT listed below MUST be added here —
//! treat a missing mapping as a code-review gate.
//!
//! Registered path → event mappings (prefix match on the first path segment):
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

use http::{Method, Request, Response, StatusCode};
use tower::{Layer, Service};

use crate::{
    events::RelationalImpactEvent,
    types::{GateDecision, GateStatus},
};

// ─── Public factory ───────────────────────────────────────────────────────────

/// Create a `tower::Layer` that wraps every POST request in a gate check.
///
/// Usage:
/// ```rust,ignore
/// Router::new()
///     .route("/content", post(create_content))
///     .layer(gate_client::tower_layer());
/// ```
pub fn tower_layer() -> GateLayer {
    GateLayer::new()
}

// ─── Layer ────────────────────────────────────────────────────────────────────

/// A `tower::Layer` that injects a gate check before every POST request.
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
    ResBody: Default + Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Only gate POST requests. Other methods pass straight through.
        if req.method() != Method::POST {
            let fut = self.inner.call(req);
            return Box::pin(async move { fut.await });
        }

        let path = req.uri().path().to_owned();
        let event_opt = infer_event_from_path(&path);

        // No event mapping → fall through without gate check (Phase 1 limitation).
        // See module-level doc comment for the rationale and upgrade path.
        if event_opt.is_none() {
            let fut = self.inner.call(req);
            return Box::pin(async move { fut.await });
        }

        let event = event_opt.unwrap();

        // Clone the inner service so we can move it into the async block.
        // `poll_ready` was already called on `self.inner` above; we use a
        // clone here because tower's service contract requires that the service
        // used for `call` is the one that was polled ready.  For Phase 1 the
        // inner service is cheap to clone (Axum Router).
        let mut inner = self.inner.clone();

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
                    resp.headers_mut().insert(
                        "x-gate-verdict",
                        http::HeaderValue::from_static("allow"),
                    );
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
                    let body_bytes =
                        serde_json::to_vec(&body_json).unwrap_or_else(|_| b"{}".to_vec());

                    // We need a ResBody from bytes. For Phase 1 we use axum's
                    // axum::body::Body when compiled with axum, but GateService is
                    // generic over ResBody — the short-circuit body must come from
                    // ResBody::default(). The 403 body is attached as a bytes
                    // extension on the response parts so callers can read it.
                    //
                    // Limitation: the default ResBody carries no bytes. Full body
                    // propagation requires constraining ResBody to From<Bytes>, which
                    // is a Phase 7 refinement. In Phase 1 the 403 status code alone
                    // is the gate signal; the body is best-effort.
                    let mut resp = Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header("content-type", "application/json")
                        .extension(body_bytes)
                        .body(ResBody::default())
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(ResBody::default())
                                .expect("infallible response build")
                        });
                    resp.headers_mut().insert(
                        "x-gate-verdict",
                        http::HeaderValue::from_static("decline"),
                    );
                    Ok(resp)
                }

                GateStatus::Escalate { target, severity } => {
                    let body_json = serde_json::json!({
                        "gate": "escalated",
                        "target": target,
                        "severity": severity,
                    });
                    let body_bytes =
                        serde_json::to_vec(&body_json).unwrap_or_else(|_| b"{}".to_vec());

                    let mut resp = Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .header("content-type", "application/json")
                        .extension(body_bytes)
                        .body(ResBody::default())
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::ACCEPTED)
                                .body(ResBody::default())
                                .expect("infallible response build")
                        });
                    resp.headers_mut().insert(
                        "x-gate-verdict",
                        http::HeaderValue::from_static("escalate"),
                    );
                    Ok(resp)
                }

                GateStatus::Verdict(_tag) => {
                    // Evaluator-shape gates: pass through; attach verdict header.
                    let mut resp = inner.call(req).await?;
                    resp.headers_mut().insert(
                        "x-gate-verdict",
                        http::HeaderValue::from_static("verdict"),
                    );
                    Ok(resp)
                }
            }
        })
    }
}

// ─── Path → Event inference ───────────────────────────────────────────────────

/// Infer a `RelationalImpactEvent` from an HTTP path.
///
/// Returns `None` for paths that have no declared mapping. See module-level
/// doc for the full list and the Phase 1 fall-through contract.
fn infer_event_from_path(path: &str) -> Option<RelationalImpactEvent> {
    // Normalise: strip trailing slash, match on first-segment prefix.
    let p = path.trim_end_matches('/');

    if p.starts_with("/content") {
        return Some(RelationalImpactEvent::ContentPublish {
            content_cid: "inferred".to_string(),
            declared_reach: "public".to_string(),
            author: "unknown".to_string(),
        });
    }
    if p.starts_with("/attestation") {
        return Some(RelationalImpactEvent::AttestationWrite {
            subject_hash: "inferred".to_string(),
            claim_kind: "inferred".to_string(),
            issuer: "unknown".to_string(),
        });
    }
    if p.starts_with("/economic-event") {
        return Some(RelationalImpactEvent::EconomicEventEmit {
            event_kind: "inferred".to_string(),
            provider: "unknown".to_string(),
            receiver: "unknown".to_string(),
            quantity: "0".to_string(),
        });
    }
    if p.starts_with("/peer-message") {
        return Some(RelationalImpactEvent::PeerMessage {
            recipient: "unknown".to_string(),
            payload_kind: "inferred".to_string(),
        });
    }
    if p.starts_with("/sync") {
        return Some(RelationalImpactEvent::SyncToPeers {
            manifest_cid: "inferred".to_string(),
            item_count: 0,
        });
    }
    if p.starts_with("/advice") {
        return Some(RelationalImpactEvent::AdviceSought {
            requester: "unknown".to_string(),
            summary_cid: "inferred".to_string(),
            topic: "inferred".to_string(),
        });
    }
    if p.starts_with("/agent/invoke") {
        return Some(RelationalImpactEvent::CapabilityInvoke {
            capability: "inferred".to_string(),
            requester: "unknown".to_string(),
            request_id: "inferred".to_string(),
        });
    }
    if p.starts_with("/crossing") {
        return Some(RelationalImpactEvent::PrivateToPublicCrossing {
            source_space: "inferred".to_string(),
            artifact_ref: "inferred".to_string(),
        });
    }

    None
}
