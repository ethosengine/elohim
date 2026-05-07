//! Smoke test: doorway hardcoded /render-test route returns rendered HTML.
//!
//! Uses the angular-fixture-bundle so we don't need a real Angular build.
//!
//! # Running
//!
//! Stand up doorway with the fixture bundle on port 8888, then:
//!
//! ```bash
//! SSR_BUNDLE_PATH=/path/to/elohim-render/fixtures/angular-fixture-bundle.mjs \
//!     cargo run --release -- --listen 0.0.0.0:8888 --dev-mode &
//! RUSTFLAGS="" cargo test --test ssr_smoke -- --ignored
//! ```
//!
//! # What the fixture produces
//!
//! The fixture bundle strips the `/render-test/` prefix before calling
//! `renderApplication`.  A request to `/render-test/foo` passes `/foo` to the
//! renderer, so the output contains `fixture rendered /foo` (not the full
//! request path).  The `ngh=` attribute is the Angular hydration marker the
//! fixture always emits.

#[tokio::test]
#[ignore = "requires running doorway with SSR_BUNDLE_PATH set to fixture"]
async fn render_test_route_returns_rendered_html() {
    let body = reqwest::get("http://localhost:8888/render-test/foo")
        .await
        .expect("connect to doorway at localhost:8888")
        .text()
        .await
        .expect("read response body");
    // The fixture renders the url passed to renderApplication ("/foo"),
    // not the full doorway request path ("/render-test/foo").
    assert!(
        body.contains("fixture rendered /foo"),
        "body did not contain expected fixture output: {}",
        body
    );
    assert!(
        body.contains("ngh="),
        "no hydration markers in body: {}",
        body
    );
}
