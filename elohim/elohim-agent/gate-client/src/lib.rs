//! Gate Client — Wisdom-as-System-Auth for the Elohim Protocol.
//!
//! This crate is the cross-cutting library every relational-impact write path
//! in the protocol calls to ensure wisdom wraps every creation-event.
//!
//! # Architecture
//!
//! The gate is **not a service endpoint**. It is a **protocol invariant**
//! implemented as a library that every network-impacting call site depends on.
//! The elohim-agent-service hosts the wisdom engine; the gate callers are
//! distributed across:
//!
//! - Zome coordinator functions (before DHT commit)
//! - Doorway HTTP POST handlers (via tower::Layer)
//! - libp2p custom-protocol senders
//! - Sync triggers projecting private state to peers
//! - Elohim-agent capability invocations
//! - Advice-seeking flows from users to elohim
//!
//! # Phase
//!
//! This crate ships in two phases:
//!
//! - **DevContext (rehearsal):** `wisdom-invoke` steps are mocked to return
//!   `Allow { phase: DevContext }`. Mechanical gates execute real logic. Every
//!   call-site integration is real. This lets the architecture breathe in its
//!   intended shape before real elohim are live.
//! - **ElohimActive:** `wisdom-invoke` calls a running elohim-agent-service.
//!   Activating is a configuration flip, not a rewrite.
//!
//! See `elohim/elohim-agent/spec/2026-04-18-gate-interface.md` for the full
//! architectural specification.

pub mod dag;
pub mod error;
pub mod events;
pub mod phase;
pub mod space;
pub mod tower;
pub mod transport;
pub mod types;

#[cfg(test)]
pub mod testing;

// ─── Test-only decision override ──────────────────────────────────────────────
//
// Allows integration tests to inject a specific GateDecision for one `check()`
// call without building a full mock infrastructure. Scoped to the current thread
// so parallel tests cannot interfere.
//
// Usage:
//   gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(...)));
//   // ... drive the request ...
//   gate_client::__test_set_decision_override(None); // restore
#[cfg(test)]
thread_local! {
    static DECISION_OVERRIDE: std::cell::RefCell<Option<GateDecision>> =
        const { std::cell::RefCell::new(None) };
}

/// Set a one-shot decision override for the next `check()` call on this thread.
/// Only available in `#[cfg(test)]` builds.
#[cfg(test)]
pub fn __test_set_decision_override(decision: Option<GateDecision>) {
    DECISION_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = decision;
    });
}

// Public re-exports for ergonomic use.
pub use error::{GateError, GateResult};
pub use events::RelationalImpactEvent;
pub use phase::Phase;
pub use space::{SpaceContext, SpaceType};
pub use tower::{tower_layer, GateLayer};
pub use transport::{GateClientConfig, Transport};
pub use types::{
    DeclineGrounds, EscalationTarget, GateDecision, GateStatus, GateTag, Severity, SideEffect,
};

/// Primary entry point for gate checks.
///
/// Called from every relational-impact write path before the side effect is
/// realized. In DevContext phase, returns a mocked `Allow` for boundary-crossing
/// events and `Allow { exempt: true }` for interior events.
pub async fn check(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    // Test-only: consume a one-shot decision override if one is set.
    #[cfg(test)]
    {
        let override_decision = DECISION_OVERRIDE.with(|cell| cell.borrow_mut().take());
        if let Some(decision) = override_decision {
            return Ok(decision);
        }
    }

    let space = space::detect_from_event(&event);

    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }

    // Phase 0 skeleton: returns a mocked Allow.
    // Phase 1 will wire in context-assembly + DAG interpreter.
    // Phase 2 will wire in the universal-band DAG.
    Ok(GateDecision::allow_mocked(Phase::DevContext))
}

/// Synchronous variant for zome coordinator contexts (Holochain WASM).
///
/// Phase 0 stub. Will be implemented when the Rust HDK/WASM integration lands.
pub fn check_blocking(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    // Bridge to async via a minimal executor once we're in WASM context.
    // Phase 0: return the same mock without actually blocking.
    let space = space::detect_from_event(&event);
    if space.is_exempt() {
        return Ok(GateDecision::allow_exempt(Phase::DevContext));
    }
    Ok(GateDecision::allow_mocked(Phase::DevContext))
}

/// Configure the gate client — transport, phase override, trust assessor.
pub fn configure(_config: GateClientConfig) {
    // Phase 0 placeholder — configuration storage wired in Phase 1.
}

/// Queue an escalation for steward / qahal / existential review.
///
/// Phase 0 stub. Implementation in Phase 2 alongside escalate-to-review executor.
pub async fn queue_for_review(
    _target: EscalationTarget,
    _context: serde_json::Value,
) -> GateResult<String> {
    Err(GateError::NotYetImplemented(
        "queue_for_review arrives in Phase 2".to_string(),
    ))
}

// ─── Unit tests for check() and check_blocking() ──────────────────────────────

#[cfg(test)]
mod check_tests {
    use super::*;
    use crate::events::RelationalImpactEvent;

    // ─── Helper constructors for the 8 event variants ─────────────────────────

    fn all_events() -> Vec<RelationalImpactEvent> {
        vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "cid-1".to_string(),
                declared_reach: "public".to_string(),
                author: "agent-a".to_string(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "hash-1".to_string(),
                claim_kind: "brit".to_string(),
                issuer: "agent-b".to_string(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".to_string(),
                provider: "agent-c".to_string(),
                receiver: "agent-d".to_string(),
                quantity: "5".to_string(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "agent-e".to_string(),
                payload_kind: "handshake".to_string(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "manifest-cid".to_string(),
                item_count: 3,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "agent-f".to_string(),
                summary_cid: "sum-cid".to_string(),
                topic: "conflict".to_string(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".to_string(),
                requester: "agent-g".to_string(),
                request_id: "req-1".to_string(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "private-draft".to_string(),
                artifact_ref: "artifact-cid".to_string(),
            },
        ]
    }

    // ─── check(): all 8 events return Allow in DevContext ─────────────────────
    //
    // In Phase 0 / DevContext, none of the current enum variants map to an
    // exempt interior space — all 8 go through the non-exempt `allow_mocked`
    // path. This documents the expected DevContext behaviour at the protocol
    // boundary: every relational-impact event is gated (not short-circuited),
    // and the Phase 0 rehearsal always yields Allow.

    #[tokio::test]
    async fn check_all_events_return_allow_in_dev_context() {
        for event in all_events() {
            let kind = event.kind();
            let result = check(event).await;
            assert!(result.is_ok(), "check() returned Err for {kind}");
            let decision = result.unwrap();
            assert!(
                decision.is_allowed(),
                "check() must return Allow for {kind} in DevContext"
            );
        }
    }

    #[tokio::test]
    async fn check_all_events_return_non_exempt_allow_in_dev_context() {
        // None of the 8 events are exempt interiors — the `allow_mocked` path
        // fires, not `allow_exempt`.
        for event in all_events() {
            let kind = event.kind();
            // Skip PrivateToPublicCrossing if it were somehow exempt (it is not,
            // but this loop documents the invariant for all 8).
            let decision = check(event).await.unwrap();
            assert!(
                !decision.is_exempt(),
                "check() must NOT return exempt Allow for {kind}; \
                 all 8 current variants are boundary-crossing or public"
            );
        }
    }

    #[tokio::test]
    async fn check_all_events_return_dev_context_phase() {
        for event in all_events() {
            let kind = event.kind();
            let decision = check(event).await.unwrap();
            assert!(
                decision.phase.is_dev_context(),
                "check() must return DevContext phase for {kind}"
            );
        }
    }

    #[tokio::test]
    async fn check_mocked_reasoning_has_zero_confidence() {
        // ConstitutionalReasoningSummary::mocked() has confidence 0.0, flagging
        // that this decision carries no reputation weight.
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        let decision = check(event).await.unwrap();
        assert!(
            (decision.reasoning.confidence - 0.0).abs() < f32::EPSILON,
            "mocked reasoning must have confidence 0.0"
        );
        assert_eq!(decision.reasoning.primary_principle, "dev-context-mock");
    }

    // ─── check_blocking(): mirrors check() for all 8 variants ─────────────────

    #[test]
    fn check_blocking_all_events_return_allow_in_dev_context() {
        for event in all_events() {
            let kind = event.kind();
            let result = check_blocking(event);
            assert!(result.is_ok(), "check_blocking() returned Err for {kind}");
            let decision = result.unwrap();
            assert!(
                decision.is_allowed(),
                "check_blocking() must return Allow for {kind} in DevContext"
            );
            assert!(
                !decision.is_exempt(),
                "check_blocking() must return non-exempt Allow for {kind}"
            );
        }
    }

    #[test]
    fn check_blocking_returns_dev_context_phase() {
        let event = RelationalImpactEvent::AttestationWrite {
            subject_hash: "h".to_string(),
            claim_kind: "brit".to_string(),
            issuer: "agent".to_string(),
        };
        let decision = check_blocking(event).unwrap();
        assert!(decision.phase.is_dev_context());
    }

    // ─── check(): decision override mechanism ────────────────────────────────

    #[tokio::test]
    async fn check_override_is_consumed_and_returns_injected_decision() {
        // Inject a Decline decision — check() must consume it and return it,
        // not the default Allow.
        __test_set_decision_override(Some(crate::testing::mock_decline(
            "override-category",
            "injected for test",
        )));

        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        let decision = check(event).await.unwrap();
        assert!(
            !decision.is_allowed(),
            "injected Decline must be returned by check()"
        );
        // Confirm it is specifically a Decline with the injected category.
        let json = serde_json::to_string(&decision.status).unwrap();
        assert!(
            json.contains("override-category"),
            "Decline body must carry the injected category, got: {json}"
        );
    }

    #[tokio::test]
    async fn check_override_is_one_shot_and_cleared_after_first_call() {
        // Inject a Decline.
        __test_set_decision_override(Some(crate::testing::mock_decline(
            "one-shot",
            "should only fire once",
        )));

        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        // First call consumes the override.
        let first = check(event.clone()).await.unwrap();
        assert!(!first.is_allowed(), "first call must return the injected Decline");

        // Second call on the same thread must fall back to the DevContext default Allow.
        let second = check(event).await.unwrap();
        assert!(second.is_allowed(), "second call must return DevContext Allow (override cleared)");
    }
}

// ─── Unit tests for tower header assertions (need override mechanism) ─────────

#[cfg(test)]
mod tower_header_tests {
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

    fn gated_content_router() -> Router {
        Router::new()
            .route("/content", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    fn gated_attestation_router() -> Router {
        Router::new()
            .route("/attestation", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    fn gated_crossing_router() -> Router {
        Router::new()
            .route("/crossing", post(ok_handler))
            .layer(crate::tower::GateLayer::new())
    }

    async fn post_to(router: Router, path: &str) -> (StatusCode, http::HeaderMap, String) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, headers, String::from_utf8_lossy(&bytes).into_owned())
    }

    // ─── M-5: Decline produces 403, x-gate-verdict: decline, grounds in body ──

    #[tokio::test]
    async fn decline_produces_403_with_correct_header_and_grounds_in_body() {
        crate::__test_set_decision_override(Some(crate::testing::mock_decline(
            "harmful-content",
            "content violates existential safety principle",
        )));

        let router = gated_content_router();
        let (status, headers, body) = post_to(router, "/content").await;

        assert_eq!(status, StatusCode::FORBIDDEN, "Decline must produce 403");

        let verdict = headers.get("x-gate-verdict").expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "decline", "x-gate-verdict must be 'decline'");

        assert!(
            body.contains("harmful-content"),
            "body must contain the grounds category, got: {body}"
        );
        assert!(
            body.contains("\"gate\":\"declined\"") || body.contains("\"gate\": \"declined\""),
            "body must carry gate:declined marker, got: {body}"
        );
    }

    // ─── M-6: Escalate produces 202, x-gate-verdict: escalate, target in body ─

    #[tokio::test]
    async fn escalate_produces_202_with_correct_header_and_target_in_body() {
        use crate::types::{EscalationTarget, Severity};

        crate::__test_set_decision_override(Some(crate::testing::mock_escalate(
            EscalationTarget::AppSteward {
                steward_id: "steward-001".to_string(),
            },
            Severity::High,
        )));

        let router = gated_attestation_router();
        let (status, headers, body) = post_to(router, "/attestation").await;

        assert_eq!(status, StatusCode::ACCEPTED, "Escalate must produce 202");

        let verdict = headers.get("x-gate-verdict").expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "escalate", "x-gate-verdict must be 'escalate'");

        // The body JSON must include the target and severity fields.
        assert!(
            body.contains("\"gate\":\"escalated\"") || body.contains("\"gate\": \"escalated\""),
            "body must carry gate:escalated marker, got: {body}"
        );
        assert!(
            body.contains("high") || body.contains("High"),
            "body must include severity, got: {body}"
        );
    }

    // ─── Verdict: passes to inner handler, x-gate-verdict: verdict ───────────

    #[tokio::test]
    async fn verdict_passes_to_inner_handler_with_verdict_header() {
        use crate::types::GateTag;

        crate::__test_set_decision_override(Some(crate::testing::mock_verdict(
            GateTag::StoryPoint {
                valence: "constructive".to_string(),
                magnitude: "medium".to_string(),
                evidence_type: "direct".to_string(),
            },
        )));

        let router = gated_crossing_router();
        let (status, headers, body) = post_to(router, "/crossing").await;

        // Verdict passes through to inner handler — should return 200 from ok_handler.
        assert_eq!(status, StatusCode::OK, "Verdict must pass through to inner handler");

        let verdict = headers.get("x-gate-verdict").expect("x-gate-verdict header must be present");
        assert_eq!(verdict, "verdict", "x-gate-verdict must be 'verdict'");

        assert!(
            body.contains("\"handled\":true") || body.contains("\"handled\": true"),
            "inner handler body must be present, got: {body}"
        );
    }

    // ─── Unmapped path with REGISTERED handler: no x-gate-verdict header ─────
    //
    // When a registered handler is on an unmapped path, the gate falls through
    // (Phase 1 limitation). The x-gate-verdict header should NOT be present
    // because the gate never ran.

    #[tokio::test]
    async fn registered_unmapped_path_has_no_gate_verdict_header() {
        // Register a handler on a path the gate does NOT know about.
        let router = Router::new()
            .route("/unmapped-known-handler", post(ok_handler))
            .layer(crate::tower::GateLayer::new());

        let req = Request::builder()
            .method(Method::POST)
            .uri("/unmapped-known-handler")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK, "handler must respond 200");
        assert!(
            resp.headers().get("x-gate-verdict").is_none(),
            "x-gate-verdict must NOT be present when gate bypassed the path"
        );

        // Confirm the inner handler was actually reached (not some other 200-producing path).
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&bytes).into_owned();
        assert!(
            body.contains("\"handled\":true"),
            "inner handler body must be present, got: {body}"
        );
    }
}
