//! Cross-stack integration test for the SSR capability flow.
//!
//! Verifies the contract between doorway (publishes via `/admin/capability`)
//! and elohim-storage (pulls via `DOORWAY_CAPABILITY_URL` and layers into
//! `PeerStatusView` via `build_peer_status_view`).
//!
//! Spec: genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md
//! Plan: genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md

use elohim_storage::{
    build_peer_status_view, load_render_capability_from_url, BundleEntry, RendererKind,
};
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

/// Serialize env-var mutations across tests in this file. The same env var
/// (`DOORWAY_CAPABILITY_URL`) is read by `load_render_capability_from_url`,
/// so two parallel tests can leak state into each other (memory:
/// feedback_env_var_test_flakiness).
static DOORWAY_CAP_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fake_peer_status_row(peer_id: &str) -> elohim_storage::db::peer_statuses::PeerStatusRow {
    elohim_storage::db::peer_statuses::PeerStatusRow {
        peer_id: peer_id.to_string(),
        status: "online".to_string(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: Some("home-nuc".into()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: format!("anchor-{peer_id}"),
        updated_at: 1_700_000_000_000_000,
    }
}

#[tokio::test]
async fn storage_fetches_capability_and_layers_it_into_view() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "bundles": [{ "name": "lamad-app", "version": "1.0.3", "renderer": "angular-ssr" }],
            "renderers": ["angular-ssr"],
            "authModes": ["anonymous", "doorway-hosted"],
            "maxConcurrentRenders": 4
        })))
        .mount(&server)
        .await;

    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let cap = load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    let cap = cap.expect("capability loaded");

    let row = fake_peer_status_row("peer-z");
    let view = build_peer_status_view(row, None, Some(&cap), None);
    let rc = view.render_capability.expect("render_capability layered");
    assert_eq!(rc.bundles.len(), 1);
    assert_eq!(rc.bundles[0].name, "lamad-app");
    assert_eq!(rc.bundles[0].renderer, RendererKind::AngularSsr);
    assert!(rc.auth_modes.contains(&"doorway-hosted".to_string()));
    assert!(rc.auth_modes.contains(&"anonymous".to_string()));
    assert_eq!(rc.max_concurrent_renders, 4);
}

#[tokio::test]
async fn storage_returns_null_capability_when_doorway_unreachable() {
    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Port 1 is reserved (TCPMUX) and almost always closed → connection refused.
    std::env::set_var("DOORWAY_CAPABILITY_URL", "http://127.0.0.1:1/never");
    let cap = load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(
        cap.is_none(),
        "unreachable doorway must produce null capability"
    );
}

#[tokio::test]
async fn storage_returns_null_capability_when_doorway_returns_5xx() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let cap = load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(cap.is_none());
}

#[tokio::test]
async fn storage_returns_null_capability_when_doorway_returns_garbage() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_string("definitely not json"))
        .mount(&server)
        .await;

    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let cap = load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(
        cap.is_none(),
        "unparseable body must produce null capability"
    );
}

#[tokio::test]
async fn view_with_no_capability_emits_null_render_capability() {
    let row = fake_peer_status_row("peer-no-render");
    let view = build_peer_status_view(row, None, None, None);
    assert!(view.render_capability.is_none());
    assert!(view.extensions.is_none());
}

#[tokio::test]
async fn view_serializes_render_capability_with_camel_case_when_present() {
    use elohim_storage::RenderCapabilityProfile;

    let cap = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 2,
        memory_budget_mib: None,
    };
    let row = fake_peer_status_row("peer-render");
    let view = build_peer_status_view(row, None, Some(&cap), None);
    let json: serde_json::Value = serde_json::to_value(&view).unwrap();
    // Spec contract: camelCase keys, kebab-case enum values, anonymous always present.
    assert!(json.get("renderCapability").is_some());
    let rc = &json["renderCapability"];
    assert_eq!(rc["bundles"][0]["renderer"], "angular-ssr");
    assert_eq!(rc["maxConcurrentRenders"], 2);
    assert_eq!(rc["authModes"][0], "anonymous");
    // memoryBudgetMib was None and skip_serializing_if = "Option::is_none"
    assert!(rc.get("memoryBudgetMib").is_none());
}
