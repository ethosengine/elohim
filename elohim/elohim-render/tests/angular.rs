use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, RenderTerminal, Renderer, Result,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

/// An upstream that never settles within a tight soft budget.
struct SlowFetcher {
    delay_ms: u64,
}

#[async_trait]
impl DataFetcher for SlowFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{\"late\":true}".to_vec(),
            content_hash: None,
        })
    }
}

/// Records its identity into a shared log on every fetch — proves WHICH fetcher
/// the render actually used (the per-request one or the construction-time one).
struct MarkerFetcher {
    marker: &'static str,
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl DataFetcher for MarkerFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        self.calls.lock().unwrap().push(self.marker.to_string());
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{\"ok\":true}".to_vec(),
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
async fn render_trace_classifies_a_stalled_upstream_as_stalled() {
    // The keystone fault-injection: a slow upstream + a tight soft budget. The
    // render must COMPLETE (the soft-deadline rejects the fetch fast, the fixture
    // catches it and emits a shell) and self-classify as `stalled` — never hang
    // to the wall-time. Deterministic, no deploy required.
    let renderer = AngularRenderer::with_soft_budget(
        trace_fixture_bundle(),
        Arc::new(SlowFetcher { delay_ms: 10_000 }),
        40,
    )
    .expect("init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/slow".into(),
        data_fetcher: Arc::new(SlowFetcher { delay_ms: 10_000 }),
        limits: RenderLimits::default(),
    };
    let out = renderer
        .render(ctx)
        .await
        .expect("render completes despite the stall");

    assert_eq!(
        out.trace.terminal,
        RenderTerminal::Stalled,
        "a never-settling upstream must classify as stalled; trace: {:?}",
        out.trace.fetches
    );
}

#[tokio::test]
async fn render_uses_the_per_request_ctx_data_fetcher_not_the_construction_one() {
    // The SSR auth contract: doorway builds a per-request fetcher carrying the
    // user's session credential and passes it in ctx.data_fetcher. The render MUST
    // use it — otherwise authenticated SSR fetches as anonymous (reach-aware
    // content falls back to public). Guards against the renderer ignoring
    // ctx.data_fetcher in favour of its construction-time fetcher.
    let construction_calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let request_calls = Arc::new(Mutex::new(Vec::<String>::new()));

    let renderer = AngularRenderer::new(
        trace_fixture_bundle(),
        Arc::new(MarkerFetcher {
            marker: "construction",
            calls: construction_calls.clone(),
        }),
    )
    .expect("init");

    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/auth-content".into(),
        data_fetcher: Arc::new(MarkerFetcher {
            marker: "request",
            calls: request_calls.clone(),
        }),
        limits: RenderLimits::default(),
    };
    renderer.render(ctx).await.expect("render");

    assert!(
        !request_calls.lock().unwrap().is_empty(),
        "the per-request ctx.data_fetcher must be used (the fixture issues a fetch)"
    );
    assert!(
        construction_calls.lock().unwrap().is_empty(),
        "the construction-time fetcher must NOT be used when ctx provides one — \
         using it is the SSR-as-anonymous auth hole"
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
