use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, RenderTerminal, Renderer, Result,
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

/// Returns a non-empty JSON body — a healthy data arrival.
struct ContentFetcher;

#[async_trait]
impl DataFetcher for ContentFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{\"title\":\"Micah 6:8\"}".to_vec(),
            content_hash: None,
        })
    }
}

/// Returns an empty JSON array — a truthful "no content" (the EprRouter-empties shape).
struct EmptyArrayFetcher;

#[async_trait]
impl DataFetcher for EmptyArrayFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"[]".to_vec(),
            content_hash: None,
        })
    }
}

fn trace_fixture_bundle() -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "fixtures",
        "trace-fixture-bundle.mjs",
    ]
    .iter()
    .collect()
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
    let renderer = AngularRenderer::new(bundle, Arc::new(EmptyFetcher)).expect("renderer init");
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
async fn render_trace_records_a_fetch_and_classifies_rendered() {
    let renderer =
        AngularRenderer::new(trace_fixture_bundle(), Arc::new(ContentFetcher)).expect("init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/test".into(),
        data_fetcher: Arc::new(ContentFetcher),
        limits: RenderLimits::default(),
    };
    let out = renderer.render(ctx).await.expect("render");

    assert!(
        !out.trace.fetches.is_empty(),
        "the data fetch must appear in the render trace, got {:?}",
        out.trace.fetches
    );
    assert_eq!(
        out.trace.terminal,
        RenderTerminal::Rendered,
        "a healthy data arrival classifies as rendered"
    );
}

#[tokio::test]
async fn render_trace_classifies_empty_upstream_as_rendered_empty() {
    // The whole point: a truthful empty (`[]`) is distinguishable from a stall.
    let renderer =
        AngularRenderer::new(trace_fixture_bundle(), Arc::new(EmptyArrayFetcher)).expect("init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/empty".into(),
        data_fetcher: Arc::new(EmptyArrayFetcher),
        limits: RenderLimits::default(),
    };
    let out = renderer.render(ctx).await.expect("render");

    assert_eq!(
        out.trace.terminal,
        RenderTerminal::RenderedEmpty,
        "an empty-array upstream is rendered-empty, not stalled; trace: {:?}",
        out.trace.fetches
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

    let renderer = AngularRenderer::new(bundle, Arc::new(EmptyFetcher)).expect("renderer init");
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
