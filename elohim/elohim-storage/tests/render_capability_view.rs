//! Cross-stack integration test for the SSR capability flow.
//!
//! Verifies the contract between doorway (publishes via `/admin/capability`)
//! and elohim-storage (pulls via `DOORWAY_CAPABILITY_URL` and layers into
//! `PeerStatusView` via `build_peer_status_view`).
//!
//! Spec: genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md
//! Plan: genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md

// Each `#[tokio::test]` constructs its own current-thread runtime, so the
// shared std::sync::Mutex serializes ACROSS those runtimes (across OS threads),
// not within a single async runtime. The deadlock risk that motivates the
// `await_holding_lock` lint (one task awaiting while another contends for the
// same lock on the same runtime) does not apply here. Switching to
// `tokio::sync::Mutex` would be wrong: that's a task-level mutex and would
// not protect against cross-runtime env-var contention.
#![allow(clippy::await_holding_lock)]

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

/// Cross-stack contract: doorway's `fetch_compute_budget` extracts CPU fields
/// at three JSON pointer paths. This test asserts that a SheafaDashboardStateView
/// assembled from `current_compute_metrics()` + the env-driven operator limits
/// actually carries values at those paths, so the doorway-side pointer access
/// will succeed against a real storage payload.
#[test]
fn storage_compute_view_satisfies_doorway_pointer_paths() {
    use elohim_storage::api::compute::{
        current_allocation_block, current_ceiling_limit, current_compute_metrics,
    };

    let compute_metrics = current_compute_metrics();
    let ceiling = current_ceiling_limit();
    let allocation = current_allocation_block();

    // Probes are live on every CI host: at least 1 CPU core present.
    assert!(
        compute_metrics.cpu_total_cores >= 1,
        "cpu_count probe must succeed on test host"
    );

    // Build the minimal subset of SheafaDashboardStateView's wire shape that
    // doorway's fetch_compute_budget cares about. We project the views via
    // serde_json::to_value to test the actual camelCase output, not the Rust
    // field names.
    let computed = serde_json::to_value(&compute_metrics).unwrap();
    let ceiling_v = serde_json::to_value(&ceiling).unwrap();
    let allocation_v = serde_json::to_value(&allocation).unwrap();
    let body = serde_json::json!({
        "computeMetrics": computed,
        "constitutionalLimits": { "ceilingLimit": ceiling_v },
        "allocations": { "allocationBlocks": [allocation_v] },
    });

    // Three pointer paths must resolve — these are the same paths used by
    // `doorway/render::fetch_compute_budget`. Drift here would silently
    // collapse the budget to None and SSR concurrency would fall back to
    // DEFAULT_MAX_CONCURRENT.
    assert!(
        body.pointer("/computeMetrics/cpuTotalCores").is_some(),
        "/computeMetrics/cpuTotalCores must be present (doorway extracts cpu_total_cores)"
    );
    assert!(
        body.pointer("/constitutionalLimits/ceilingLimit/computeMaxCores")
            .is_some(),
        "/constitutionalLimits/ceilingLimit/computeMaxCores must be present (doorway extracts ceiling_max_cores)"
    );
    assert!(
        body.pointer("/allocations/allocationBlocks/0/cpuCores")
            .is_some(),
        "/allocations/allocationBlocks/0/cpuCores must be present (doorway extracts allocation_cpu_cores)"
    );

    // Round-trip the live cpu probe through JSON to confirm the integer
    // doorway reads is the integer the probe reported.
    let cpu = body
        .pointer("/computeMetrics/cpuTotalCores")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert_eq!(cpu as u32, compute_metrics.cpu_total_cores);
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
