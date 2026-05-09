//! Capability deriver: bundles ∩ manifest, with override.
//!
//! Tasks 11-14 fill in `scan_bundles`, `fetch_manifest_renderers`,
//! `parse_override`/`load_override`, and the `derive_capability` orchestrator.
//! This file is the stub that compiles end-to-end so the module aggregator
//! has something to re-export.

use crate::render::types::RenderCapabilityProfile;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityDeriverError {
    #[error("bundles directory unreadable: {0}")]
    BundleDirRead(String),
    #[error("manifest fetch failed: {0}")]
    ManifestFetch(String),
    #[error("override config malformed: {0}")]
    OverrideMalformed(String),
}

/// Auto-derive a render-capability claim. Honest by construction:
/// only bundles on disk whose renderer is referenced in storage's manifest
/// can appear in the claim. Override may reduce the claim but never inflate.
///
/// Tasks 11-14 implement this. The stub returns Ok(None) so the module compiles.
pub async fn derive_capability(
    _bundles_dir: &std::path::Path,
    _storage_manifest_url: &str,
    _override_path: Option<&std::path::Path>,
) -> Result<Option<RenderCapabilityProfile>, CapabilityDeriverError> {
    Ok(None)
}
