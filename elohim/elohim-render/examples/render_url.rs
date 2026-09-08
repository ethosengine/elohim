//! Render a URL against a local Angular SSR server bundle and print the result.
//!
//! The developer-eyes rig for the SSR seam: run the SAME render the doorway
//! runs in production, locally, and see the actual HTML + trace — so "what did
//! the renderer actually emit?" is answerable from evidence, not theory.
//!
//! ```bash
//! RUSTFLAGS="" cargo run --example render_url -- <main.server.mjs> <url>
//! # e.g.
//! RUSTFLAGS="" cargo run --example render_url -- /tmp/bundle/main.server.mjs /
//! ```
//!
//! Fetches are stubbed to fail fast (matches a render with no reachable peer —
//! the `fetches=0` production signature). Swap in a real fetcher if a
//! content-bearing render needs reproducing.
//! Set `ELOHIM_RENDER_FETCH_BASE=http://127.0.0.1:8888` to read from an owned
//! local mesh instead. `ELOHIM_RENDER_EXPECT_TEXT` requires an application-owned
//! marker in the emitted HTML, so a loading page cannot pass as the intended
//! content. This checks SSR output; it does not prove browser hydration.

use std::sync::Arc;

use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, FetcherTrust, RenderContext,
    RenderLimits, RenderSpec, Renderer,
};

/// Fetcher that fails fast — reproduces a render with no reachable peer.
struct FailFetcher;

/// Explicit local-mesh diagnostic mode. Never forwards bundle credentials or
/// follows redirects; all reads are confined to the operator-selected loopback.
struct LoopbackFetcher {
    base: url::Url,
    client: reqwest::Client,
}

#[async_trait]
impl DataFetcher for LoopbackFetcher {
    fn trust_scope(&self) -> FetcherTrust {
        FetcherTrust::Ambient
    }

    async fn fetch(&self, req: FetchRequest) -> elohim_render::Result<FetchResponse> {
        let error = |e: String| elohim_render::RenderError::DataFetch(e);
        if req.method != "GET" {
            return Err(error("render_url loopback mode allows GET only".into()));
        }
        let original = url::Url::parse(&req.url).map_err(|e| error(e.to_string()))?;
        let mut target = self.base.clone();
        target.set_path(original.path());
        target.set_query(original.query());
        eprintln!("[fetch -> LOOPBACK] {target}");
        let response = self
            .client
            .get(target)
            .send()
            .await
            .map_err(|e| error(e.to_string()))?;
        if !response.status().is_success() {
            return Err(error(format!(
                "loopback returned HTTP {} for {}",
                response.status(),
                req.url
            )));
        }
        Ok(FetchResponse {
            status: response.status().as_u16(),
            headers: response
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect(),
            body: response
                .bytes()
                .await
                .map_err(|e| error(e.to_string()))?
                .to_vec(),
            content_hash: None,
        })
    }
}

#[async_trait]
impl DataFetcher for FailFetcher {
    fn trust_scope(&self) -> FetcherTrust {
        FetcherTrust::Ambient
    }

    async fn fetch(&self, req: FetchRequest) -> elohim_render::Result<FetchResponse> {
        eprintln!("[fetch → FAIL] {} {}", req.method, req.url);
        Err(elohim_render::RenderError::DataFetch(format!(
            "render_url stub fetcher (url: {})",
            req.url
        )))
    }
}

#[tokio::main]
async fn main() {
    // Surface console.* (routed through tracing at target elohim_render::js_console)
    // and any Rust-side tracing::debug! diagnostics. Defaults to the console
    // target only; override with RUST_LOG for more.
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "elohim_render::js_console=info,elohim_render=info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_target(false)
        .init();

    let mut args = std::env::args().skip(1);
    let bundle = args
        .next()
        .expect("usage: render_url <main.server.mjs> <url>");
    let url = args
        .next()
        .expect("usage: render_url <main.server.mjs> <url>");

    let fetcher: Arc<dyn DataFetcher> = match std::env::var("ELOHIM_RENDER_FETCH_BASE") {
        Ok(base) => {
            let base = url::Url::parse(&base).expect("valid loopback fetch base");
            assert!(
                base.scheme() == "http"
                    && base.username().is_empty()
                    && base.password().is_none()
                    && matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
                "fetch base must be HTTP loopback"
            );
            Arc::new(LoopbackFetcher {
                base,
                client: reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .expect("HTTP client"),
            })
        }
        Err(_) => Arc::new(FailFetcher),
    };

    let renderer = AngularRenderer::new(bundle.clone().into(), Arc::clone(&fetcher))
        .expect("renderer init failed");

    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: url.clone(),
        data_fetcher: fetcher,
        limits: RenderLimits {
            wall_time_ms: std::env::var("ELOHIM_RENDER_WALL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60_000),
            ..Default::default()
        },
    };

    match renderer.render(ctx).await {
        Ok(out) => {
            eprintln!("=== TRACE ===");
            eprintln!("terminal: {}", out.trace.terminal.as_str());
            eprintln!("fetches:  {}", out.trace.fetches.len());
            eprintln!("wall_ms:  {}", out.trace.wall_ms);
            eprintln!("html len: {}", out.html.len());
            eprintln!(
                "compose anchors: <app-root>={} </app-root>={}",
                out.html.matches("<app-root").count(),
                out.html.matches("</app-root>").count()
            );
            eprintln!("=== HTML ===");
            println!("{}", out.html);
            if matches!(
                out.trace.terminal.as_str(),
                "errored" | "stalled" | "timed-out"
            ) {
                eprintln!("RENDER REFUSED: terminal {}", out.trace.terminal.as_str());
                std::process::exit(1);
            }
            if let Ok(expected) = std::env::var("ELOHIM_RENDER_EXPECT_TEXT") {
                if expected.is_empty() || !out.html.contains(&expected) {
                    eprintln!("RENDER REFUSED: expected content marker absent: {expected:?}");
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            eprintln!("RENDER ERROR: {e}");
            std::process::exit(1);
        }
    }
}
