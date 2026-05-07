//! Storage's direct-to-peer SSR endpoint.
//!
//! Requires storage running with `--features ssr` and seeded content.
//!
//! Run the live integration test manually:
//!
//! ```bash
//! SSR_BUNDLE_PATH=/path/to/server.mjs \
//!   RUSTFLAGS='--cfg getrandom_backend="custom"' \
//!   cargo test --features ssr --test ssr_direct -- --ignored
//! ```

// =============================================================================
// Live integration test (requires running storage instance)
// =============================================================================

#[cfg(feature = "ssr")]
#[tokio::test]
#[ignore = "requires running storage with --features ssr and SSR_BUNDLE_PATH set"]
async fn storage_spa_endpoint_returns_rendered_html() {
    let body = reqwest::get("http://localhost:8090/spa/lamad/concept/test-fixture")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        body.contains("<title>") || body.contains("fixture"),
        "body did not contain expected SSR content: {}",
        body
    );
    assert!(body.contains("ngh="), "no hydration markers in: {}", body);
}

// =============================================================================
// Compile-time guard: default (no ssr) profile must not pull in V8
// =============================================================================

#[cfg(not(feature = "ssr"))]
#[test]
fn ssr_disabled_compiles_without_renderer() {
    // No-op: this test exists to ensure the crate compiles cleanly when the
    // ssr feature is OFF.  The default build profile must NOT pull in
    // V8 / deno_core dependencies.
}
