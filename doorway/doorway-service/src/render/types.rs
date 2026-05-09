//! Rust mirrors of the protocol's render-capability profile types.
//! These match `elohim-storage::RenderCapabilityProfile` etc. on the wire
//! so storage can deserialize doorway's `/admin/capability` response directly.
//!
//! Source of truth for the wire shape:
//! `elohim/sdk/schemas/v1/views/render-capability-profile.schema.json`

use serde::{Deserialize, Serialize};

/// Renderer kind a bundle targets. Mirrors the protocol's renderer-kind enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererKind {
    AngularSsr,
    ReactRsc,
    VueSsr,
    SvelteSsr,
    LitSsr,
    StaticHtml,
}

/// One bundle a doorway carries (mirrors `bundles[]` items in the profile schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleEntry {
    pub name: String,
    pub version: String,
    pub renderer: RendererKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// Tier-1 render capability profile. View-layer Category C operational state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderCapabilityProfile {
    pub bundles: Vec<BundleEntry>,
    pub renderers: Vec<RendererKind>,
    pub auth_modes: Vec<String>,
    pub max_concurrent_renders: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_budget_mib: Option<u32>,
}
