//! Verifies that consecutive SSR requests for the same URL with unchanged content
//! hit the cache (HIT) on the second request, with the first marked MISS.
//!
//! Requires: doorway running with SSR_BUNDLE_PATH set + storage with content fixture.

#[tokio::test]
#[ignore = "requires live doorway + storage"]
async fn render_cache_serves_second_request_from_cache() {
    let url = "http://localhost:8888/lamad/concept/test-fixture";

    let client = reqwest::Client::new();
    let r1 = client.get(url).send().await.unwrap();
    let header1 = r1.headers().get("x-render-cache").cloned();
    let _body1 = r1.text().await.unwrap();

    let r2 = client.get(url).send().await.unwrap();
    let header2 = r2.headers().get("x-render-cache").cloned();

    assert_eq!(
        header1.as_ref().map(|v| v.to_str().unwrap()),
        Some("MISS"),
        "first request must be a cache MISS"
    );
    assert_eq!(
        header2.as_ref().map(|v| v.to_str().unwrap()),
        Some("HIT"),
        "second request must be a cache HIT"
    );
}
