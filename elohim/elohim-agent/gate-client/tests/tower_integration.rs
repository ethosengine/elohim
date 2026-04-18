//! Integration tests for the gate-client `tower::Layer` integration.
//!
//! These tests spin up a minimal Axum router wrapped by `gate_client::tower_layer()`
//! and drive it via `tower::ServiceExt::oneshot` — no actual TCP binding required.
//!
//! In Phase 1 / DevContext the `check()` function always returns a mocked Allow,
//! so every mapped and unmapped POST reaches the inner handler with a 200 OK.
//! The tests confirm:
//!   1. A mapped path (`/content`) reaches the inner service (200 + handled body).
//!   2. A boundary-crossing advice path (`/advice`) also reaches the inner service.
//!   3. An unmapped path (`/unmapped-path`) falls through to the inner service
//!      (Phase 1 fall-through contract — documented in src/tower.rs).

use axum::{body::Body, routing::post, Router};
use http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Trivial handler that always returns 200 OK with a fixed JSON body.
async fn test_handler() -> axum::response::Response {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"handled":true}"#))
        .unwrap()
}

/// Build the test router: two named routes + one fallback route, all wrapped by
/// the gate layer.
fn build_router() -> Router {
    Router::new()
        .route("/content", post(test_handler))
        .route("/advice", post(test_handler))
        // `/unmapped-path` is not registered — this exercises the 404 / fall-through path.
        .layer(gate_client::tower_layer())
}

// ─── Helper ───────────────────────────────────────────────────────────────────

async fn send_post(router: Router, path: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes).into_owned();
    (status, body)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_to_content_passes_through_gate_and_reaches_inner_handler() {
    let router = build_router();
    let (status, body) = send_post(router, "/content").await;

    assert_eq!(status, StatusCode::OK, "expected 200 from inner handler");
    assert!(
        body.contains("\"handled\":true") || body.contains("\"handled\": true"),
        "expected handler body, got: {body}"
    );
}

#[tokio::test]
async fn post_to_advice_passes_through_gate_and_reaches_inner_handler() {
    let router = build_router();
    let (status, body) = send_post(router, "/advice").await;

    assert_eq!(status, StatusCode::OK, "expected 200 from inner handler");
    assert!(
        body.contains("\"handled\":true") || body.contains("\"handled\": true"),
        "expected handler body, got: {body}"
    );
}

#[tokio::test]
async fn post_to_unmapped_path_falls_through_without_gate_check() {
    // Phase 1 fall-through: the path "/unmapped-path" has no registered route
    // so Axum returns 404. The gate layer does not intercept it (no event
    // mapping), which is the intended behaviour documented in src/tower.rs.
    let router = build_router();
    let (status, _body) = send_post(router, "/unmapped-path").await;

    // Axum returns 404 because no route is registered for this path.
    // The gate layer correctly falls through — confirmed by the absence of a
    // 403 or 202 (gate-specific status codes).
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "unmapped path should 404 (no gate interception, no route)"
    );
}

#[tokio::test]
async fn get_request_bypasses_gate_entirely() {
    // Non-POST methods must not be gated.
    let router = Router::new()
        .route("/content", axum::routing::get(test_handler))
        .layer(gate_client::tower_layer());

    let req = Request::builder()
        .method(Method::GET)
        .uri("/content")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn gate_verdict_header_is_present_on_allowed_response() {
    let router = build_router();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/content")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let verdict = resp.headers().get("x-gate-verdict");
    assert!(
        verdict.is_some(),
        "x-gate-verdict header should be present on an allowed response"
    );
    assert_eq!(verdict.unwrap(), "allow");
}
