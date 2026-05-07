use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, Renderer, Result,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

struct EmptyFetcher;

#[async_trait]
impl DataFetcher for EmptyFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{}".to_vec(),
            content_hash: None,
        })
    }
}

#[tokio::test]
async fn angular_renderer_returns_rendered_html() {
    let bundle: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "fixtures",
        "angular-fixture-bundle.mjs",
    ]
    .iter()
    .collect();
    let renderer = AngularRenderer::new(bundle).expect("renderer init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/lamad/concept/test".into(),
        data_fetcher: Arc::new(EmptyFetcher),
        limits: RenderLimits::default(),
    };
    let out = renderer.render(ctx).await.expect("render");
    assert_eq!(out.status, 200);
    assert!(out.html.contains("fixture rendered /lamad/concept/test"));
    assert!(
        out.html.contains("ngh="),
        "hydration markers missing: {}",
        out.html
    );
}

#[tokio::test]
#[ignore = "requires built elohim-app SSR bundle"]
async fn angular_renderer_with_real_bundle() {
    // Path to the real elohim-app SSR bundle. Skip if not built.
    let bundle: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "app",
        "elohim-app",
        "dist",
        "elohim-app",
        "server",
        "main.server.mjs",
    ]
    .iter()
    .collect();

    if !bundle.exists() {
        eprintln!(
            "SSR bundle not built — skipping (run `cd app/elohim-app && pnpm exec ng build`)"
        );
        return;
    }

    let renderer = AngularRenderer::new(bundle).expect("renderer init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/".into(),
        data_fetcher: Arc::new(EmptyFetcher),
        limits: RenderLimits {
            wall_time_ms: 120_000,
            memory_mb: 512,
            max_fetches: 32,
            max_output_bytes: 5 * 1024 * 1024,
        },
    };
    let out = renderer.render(ctx).await.expect("render real bundle");
    assert_eq!(out.status, 200);
    // The real Angular bundle should at least produce <html>...</html>
    assert!(
        out.html.contains("<html"),
        "no <html> tag in output: {}",
        &out.html[..out.html.len().min(500)]
    );
}
