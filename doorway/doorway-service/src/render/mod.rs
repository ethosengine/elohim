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

pub mod capability;
pub mod types;

pub use capability::{
    derive_capability, fetch_compute_budget, CapabilityDeriverError, ComputeBudget,
};
pub use types::{BundleEntry, RenderCapabilityProfile, RendererKind};
