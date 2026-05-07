//! Smoke test: doorway dispatches a manifest-declared SSR route through
//! the in-process renderer (Task 13 — manifest-driven SSR dispatch).
//!
//! The `/render-test/*` hardcoded arm was removed in Task 13 and replaced by
//! registry-driven dispatch: any route declared in elohim-storage's manifest
//! with `render = "angular-ssr"` is served through the in-process Renderer.
//!
//! # Running
//!
//! Stand up doorway with the fixture bundle AND elohim-storage with the
//! manifest declaring `/lamad/concept/{id}` with `render = "angular-ssr"`,
//! then:
//!
//! ```bash
//! SSR_BUNDLE_PATH=/path/to/elohim-render/fixtures/angular-fixture-bundle.mjs \
//!     cargo run --release -- --listen 0.0.0.0:8888 --dev-mode \
//!         --storage-url http://localhost:8090 &
//! RUSTFLAGS="" cargo test --test ssr_smoke -- --ignored
//! ```

#[tokio::test]
#[ignore = "requires running doorway with SSR_BUNDLE_PATH set to fixture, plus storage with manifest"]
async fn manifest_driven_ssr_route_returns_rendered_html() {
    let body = reqwest::get("http://localhost:8888/lamad/concept/test-fixture")
        .await
        .expect("connect to doorway at localhost:8888")
        .text()
        .await
        .expect("read response body");

    // The fixture renders the url it receives from renderApplication.
    // /lamad/concept/test-fixture is passed through to the renderer as-is.
    assert!(
        body.contains("fixture rendered"),
        "body did not contain expected fixture output: {}",
        body
    );
    assert!(
        body.contains("ngh="),
        "no hydration markers in body — Angular SSR did not run: {}",
        body
    );
}
