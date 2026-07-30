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

use std::sync::Arc;

use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, FetcherTrust, RenderContext,
    RenderLimits, RenderSpec, Renderer,
};

/// Fetcher that fails fast — reproduces a render with no reachable peer.
struct FailFetcher;

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

    let fetcher: Arc<dyn DataFetcher> = Arc::new(FailFetcher);

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
        }
        Err(e) => {
            eprintln!("RENDER ERROR: {e}");
            std::process::exit(1);
        }
    }
}
