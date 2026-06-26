//! elohim-render — JS execution runtime for server-side rendering.
//!
//! Embeds V8 via deno_core. Exposes a `Renderer` trait that frameworks
//! plug into. Doorway and elohim-storage both consume this crate.
//!
//! See `genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md`.

pub mod angular;
pub mod bootstrap;
pub mod chrome;
pub mod data_fetcher;
pub mod error;
pub mod renderer;
pub mod runtime;
pub(crate) mod shim;
pub mod stats;
pub mod traced_fetcher;
pub mod types;

pub use angular::AngularRenderer;
pub use bootstrap::{materialize_server_bundle, BundleSource};
pub use chrome::{base_palette, BasePalette, ColorScheme, Theme, ThemeTokens};
pub use data_fetcher::{DataFetcher, FetchRequest, FetchResponse};
pub use error::{RenderError, Result};
pub use renderer::{EchoRenderer, Renderer};
pub use stats::{RenderTraceSnapshot, RenderTraceStats};
pub use traced_fetcher::{FetchLog, TracingFetcher, DEFAULT_SOFT_BUDGET_MS};
pub use types::{
    ContentRef, FetchEvent, FetchOutcome, RenderContext, RenderLimits, RenderOutput, RenderSpec,
    RenderTerminal, RenderTrace,
};
