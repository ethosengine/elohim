//! Capability derivation, override layering, and exposure for the doorway's
//! SSR runtime. The deriver scans on-disk bundles, intersects with elohim-
//! storage's manifest of SSR-eligible routes, and produces a
//! `RenderCapabilityProfile` that doorway exposes at `/admin/capability`.
//!
//! The profile is the source of truth for "what this doorway claims it can
//! render" — auto-honest by construction (only what's on disk + in the
//! manifest can be claimed), with operator override able to reduce.
//!
//! Spec: genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md
//! Plan: genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md

pub mod breaker;
pub mod bundle_heads;
pub mod capability;
pub mod coherence;
pub mod registry;
pub mod types;
pub mod warm_shell;

pub use breaker::SsrRenderBreaker;

pub use capability::{
    derive_capability, fetch_compute_budget, CapabilityDeriverError, ComputeBudget,
};
pub use types::{BundleEntry, RenderCapabilityProfile, RendererKind};

/// Conservative default SSR render concurrency for a doorway that loaded an SSR
/// renderer but derived NO `RenderCapabilityProfile` (no `SSR_BUNDLES_DIR` /
/// capability URL — the live alpha state: `SSR_BUNDLE_PATH` set, capability URL
/// unset). Each cold V8 render holds a 60s wall budget; on a cpu:1 pod an
/// unbounded landing hot path is an outage vector. 2 bounds concurrent isolates;
/// overflow sheds to the projected-bundle fallback (today's serving behavior),
/// so the bound degrades safely.
pub const DEFAULT_SSR_RENDER_PERMITS: usize = 2;

/// Size the SSR render semaphore, given the derived capability (if any) and
/// whether an SSR renderer is loaded. A derived capability bounds concurrency to
/// its `max_concurrent_renders` (unchanged behavior). No capability but a
/// renderer present → `DEFAULT_SSR_RENDER_PERMITS` (bounds the un-gated landing
/// hot path). No renderer → `None` (nothing to render, no limiter). Pure, so the
/// sizing decision is unit-testable without a runtime.
pub fn ssr_semaphore_permits(
    capability: Option<&RenderCapabilityProfile>,
    renderer_present: bool,
) -> Option<usize> {
    match capability {
        Some(c) => Some(c.max_concurrent_renders as usize),
        None if renderer_present => Some(DEFAULT_SSR_RENDER_PERMITS),
        None => None,
    }
}

/// Most degenerate fetches named on one render-trace warn line. A render that
/// stalls on more than this is already legible from the count.
const DEGENERATE_FETCHES_NAMED: usize = 5;

/// Name the fetches that made a render degenerate: every `Stalled` or `Errored`
/// event on the trace, as `METHOD url (outcome)`, joined with `"; "`. `None`
/// when every fetch arrived, so a healthy render's URLs never reach the log.
///
/// `RenderTrace` records each fetch's URL, and the header, Loki line and
/// `/admin/render-stats` expose only counts. With counts alone, the 2026-09-12
/// and 2026-09-28 alpha cold-window firings of `render-degenerate` showed that
/// one of 29 fetches never settled but not which one (see
/// `genesis/data/timeline/backlog/self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall.md`).
pub fn degenerate_fetch_summary(trace: &elohim_render::RenderTrace) -> Option<String> {
    use elohim_render::FetchOutcome;
    let named: Vec<String> = trace
        .fetches
        .iter()
        .filter_map(|f| {
            let outcome = match &f.outcome {
                FetchOutcome::Arrived { .. } => return None,
                FetchOutcome::Stalled { waited_ms } => format!("stalled {waited_ms}ms"),
                FetchOutcome::Errored { duration_ms, .. } => format!("errored {duration_ms}ms"),
            };
            Some(format!("{} {} ({outcome})", f.method, f.url))
        })
        .collect();
    if named.is_empty() {
        return None;
    }
    let total = named.len();
    let mut line = named
        .into_iter()
        .take(DEGENERATE_FETCHES_NAMED)
        .collect::<Vec<_>>()
        .join("; ");
    if total > DEGENERATE_FETCHES_NAMED {
        line.push_str(&format!("; +{} more", total - DEGENERATE_FETCHES_NAMED));
    }
    Some(line)
}

#[cfg(test)]
mod degenerate_fetch_summary_tests {
    use super::degenerate_fetch_summary;
    use elohim_render::{FetchEvent, FetchOutcome, RenderTrace};

    fn event(url: &str, outcome: FetchOutcome) -> FetchEvent {
        FetchEvent {
            url: url.to_string(),
            method: "GET".to_string(),
            offset_ms: 0,
            outcome,
        }
    }

    fn arrived(url: &str) -> FetchEvent {
        event(
            url,
            FetchOutcome::Arrived {
                status: 200,
                bytes: 10,
                duration_ms: 3,
                empty: false,
            },
        )
    }

    #[test]
    fn healthy_render_names_nothing() {
        let trace = RenderTrace {
            fetches: vec![arrived("/db/a"), arrived("/db/b")],
            ..Default::default()
        };
        assert_eq!(degenerate_fetch_summary(&trace), None);
    }

    #[test]
    fn names_the_one_stalled_fetch_among_arrivals() {
        // The alpha cold-window shape: 28 arrive fast, exactly one never settles.
        let mut fetches: Vec<FetchEvent> = (0..28).map(|i| arrived(&format!("/db/{i}"))).collect();
        fetches.push(event(
            "/db/content/slow",
            FetchOutcome::Stalled { waited_ms: 1200 },
        ));
        let trace = RenderTrace {
            fetches,
            ..Default::default()
        };
        assert_eq!(
            degenerate_fetch_summary(&trace).as_deref(),
            Some("GET /db/content/slow (stalled 1200ms)")
        );
    }

    #[test]
    fn names_errored_fetches_and_caps_the_list() {
        let fetches: Vec<FetchEvent> = (0..7)
            .map(|i| {
                event(
                    &format!("/db/{i}"),
                    FetchOutcome::Errored {
                        message: "boom".to_string(),
                        duration_ms: 4,
                    },
                )
            })
            .collect();
        let trace = RenderTrace {
            fetches,
            ..Default::default()
        };
        let line = degenerate_fetch_summary(&trace).expect("errored fetches are named");
        assert!(line.starts_with("GET /db/0 (errored 4ms); GET /db/1 (errored 4ms)"));
        assert!(line.contains("GET /db/4 (errored 4ms)"));
        assert!(!line.contains("/db/5 "));
        assert!(line.ends_with("; +2 more"));
    }
}

#[cfg(test)]
mod semaphore_sizing_tests {
    use super::types::RenderCapabilityProfile;
    use super::{ssr_semaphore_permits, DEFAULT_SSR_RENDER_PERMITS};

    fn profile(max_concurrent_renders: u32) -> RenderCapabilityProfile {
        RenderCapabilityProfile {
            bundles: vec![],
            renderers: vec![],
            auth_modes: vec![],
            max_concurrent_renders,
            memory_budget_mib: None,
        }
    }

    #[test]
    fn capability_present_sizes_to_max_concurrent_renders() {
        let cap = profile(7);
        assert_eq!(ssr_semaphore_permits(Some(&cap), true), Some(7));
        // renderer_present is irrelevant once a capability was derived.
        assert_eq!(ssr_semaphore_permits(Some(&cap), false), Some(7));
    }

    #[test]
    fn capability_absent_renderer_present_uses_conservative_default() {
        // The live alpha state: SSR_BUNDLE_PATH set, no capability URL. Must bound
        // the landing hot path rather than leave V8 renders unlimited on cpu:1.
        assert_eq!(
            ssr_semaphore_permits(None, true),
            Some(DEFAULT_SSR_RENDER_PERMITS)
        );
        assert_eq!(DEFAULT_SSR_RENDER_PERMITS, 2);
    }

    #[test]
    fn no_capability_no_renderer_has_no_limiter() {
        assert_eq!(ssr_semaphore_permits(None, false), None);
    }
}
