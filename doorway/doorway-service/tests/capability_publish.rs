//! Integration tests for `/admin/capability` endpoint and AppState wiring.
//!
//! These tests construct an `AppState` directly (same pattern as
//! `tests/dashboard_topology.rs`) and call `handle_admin_capability` without
//! an HTTP round-trip.  The crate uses hyper + a hand-rolled dispatch, not an
//! axum Router, so the standard `tower::ServiceExt::oneshot` approach does not
//! apply here.

use std::sync::Arc;

use clap::Parser as _;
use doorway::{
    config::Args,
    render::types::{BundleEntry, RenderCapabilityProfile, RendererKind},
    routes::handle_admin_capability,
    server::AppState,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `AppState` with the given `render_capability` value.
fn state_with_capability(capability: Option<RenderCapabilityProfile>) -> Arc<AppState> {
    let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
    let mut state = AppState::new(args);
    state.render_capability = capability;
    Arc::new(state)
}

// ---------------------------------------------------------------------------
// Test 1 — profile is serialised and returned with status 200
// ---------------------------------------------------------------------------

/// When `render_capability` is `Some`, the handler must return 200 with a JSON
/// body that round-trips through `RenderCapabilityProfile`.
#[tokio::test]
async fn admin_capability_returns_layered_profile() {
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into(), "doorway-hosted".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };

    let state = state_with_capability(Some(profile));
    let response = handle_admin_capability(state).await;

    assert_eq!(
        response.status(),
        200,
        "handler must return 200 for a populated profile"
    );

    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let returned: RenderCapabilityProfile =
        serde_json::from_slice(&bytes).expect("body must be valid RenderCapabilityProfile JSON");

    assert_eq!(
        returned.bundles[0].name, "lamad-app",
        "bundle name must survive round-trip"
    );
    assert!(
        returned.auth_modes.contains(&"doorway-hosted".to_string()),
        "auth_modes must include 'doorway-hosted'"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — None capability is returned as JSON null with status 200
// ---------------------------------------------------------------------------

/// When `render_capability` is `None` the handler must return 200 with the
/// literal JSON body `null` (not an empty object, not a 404).
#[tokio::test]
async fn admin_capability_returns_null_when_unset() {
    let state = state_with_capability(None);
    let response = handle_admin_capability(state).await;

    assert_eq!(
        response.status(),
        200,
        "handler must return 200 even when capability is None"
    );

    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let body_str = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(
        body_str.trim(),
        "null",
        "body must be JSON null when render_capability is None"
    );
}
