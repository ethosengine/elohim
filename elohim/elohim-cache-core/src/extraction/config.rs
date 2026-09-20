//! The extraction cache's CONFIG — plain serde data, no runtime.
//!
//! Split out of `cache.rs` (2026-09-19) so it can be read without the `native`
//! feature. The cache itself needs tokio and stays gated; its configuration is
//! a field of the storage node's boot `Config`, which lives in
//! `elohim-settings` — a crate that is runtime-free by construction and whose
//! lockfile boundary test denies `tokio`, so it cannot enable `native` to reach
//! this struct.
//!
//! Moved verbatim; the public path `elohim_cache_core::extraction::
//! ExtractionCacheConfig` is unchanged, and a `native` build sees exactly the
//! same type it always did.
//!
//! See `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md` §4 row 3.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the extraction cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionCacheConfig {
    /// Whether the extraction cache is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Maximum total cache size in bytes
    #[serde(default = "default_budget")]
    pub budget_bytes: u64,
    /// Time-to-live in seconds for cached extractions
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
    /// Directory for cached extractions
    #[serde(default)]
    pub cache_dir: PathBuf,
}

fn default_enabled() -> bool {
    true
}
fn default_budget() -> u64 {
    512 * 1024 * 1024
} // 512 MB
fn default_ttl() -> u64 {
    3600
} // 1 hour

impl Default for ExtractionCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            budget_bytes: default_budget(),
            ttl_secs: default_ttl(),
            cache_dir: PathBuf::new(), // Must be set by caller
        }
    }
}
