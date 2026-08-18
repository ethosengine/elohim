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
pub mod capability;
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
