//! Caching layer for Doorway
//!
//! Provides in-memory LRU caching for Holochain zome calls to reduce
//! conductor load and improve response times.
//!
//! ## Cache Rule Discovery
//!
//! DNAs can optionally implement `__doorway_cache_rules` to declare caching needs.
//! If not implemented, Doorway uses convention-based defaults:
//! - `get_*` and `list_*` functions are cached (5 min TTL, auth required)
//! - Other functions are not cached
//!
//! See [`rules`] module for the protocol specification.
//!
//! ## Content Resolution
//!
//! The [`resolution`] module provides tiered content source routing:
//! - Projection (MongoDB cache) → Conductor (DHT) → External (URLs)
//! - Automatic fallback when sources are unavailable
//! - Learning from successful resolutions
//!
//! ## Write Buffer
//!
//! The [`write_buffer`] module provides batched conductor writes:
//! - Priority queues (High → Normal → Bulk)
//! - Deduplication and retry logic
//! - Backpressure signaling

pub mod access_control;
pub mod keys;
pub mod reach_aware_serving;
pub mod resolution;
pub mod rules;
pub mod store;
pub mod tiered;
pub mod write_buffer;

pub use access_control::{can_serve_at_reach, geographic_distance, prioritize_sources, CustodianSource, RequesterContext};
pub use keys::CacheKey;
pub use reach_aware_serving::{create_reach_aware_cache_key, extract_reach_from_response, should_serve_response, extract_requester_context};
pub use resolution::{DoorwayResolver, ResolutionResult, ResolutionStats};
pub use rules::{CacheRule, CacheRuleStore, DefaultRules, DnaRules, CACHE_RULES_FN};
pub use store::{CacheEntry, ContentCache};
pub use tiered::{
    BlobMetadata, CaptionMetadata, TieredBlobCache, TieredCacheConfig, TieredCacheStats,
    TierStats, VariantMetadata, spawn_tiered_cleanup_task,
};
pub use write_buffer::{DoorwayWriteBuffer, WriteBufferConfig, FlushResult, WritePriority, WriteOpType};

use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// TTL for content by ID (immutable content)
    pub content_ttl: Duration,
    /// TTL for content lists and aggregates
    pub list_ttl: Duration,
    /// TTL for user-specific data
    pub user_ttl: Duration,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            content_ttl: Duration::from_secs(3600),      // 1 hour
            list_ttl: Duration::from_secs(300),          // 5 minutes
            user_ttl: Duration::from_secs(60),           // 1 minute
            cleanup_interval: Duration::from_secs(60),   // Run cleanup every minute
        }
    }
}

impl CacheConfig {
    /// Create config from environment or defaults
    pub fn from_env() -> Self {
        let max_entries = std::env::var("CACHE_MAX_ENTRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10_000);

        let content_ttl_secs = std::env::var("CACHE_CONTENT_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);

        let list_ttl_secs = std::env::var("CACHE_LIST_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        let user_ttl_secs = std::env::var("CACHE_USER_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        Self {
            max_entries,
            content_ttl: Duration::from_secs(content_ttl_secs),
            list_ttl: Duration::from_secs(list_ttl_secs),
            user_ttl: Duration::from_secs(user_ttl_secs),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 10_000);
        assert_eq!(config.content_ttl, Duration::from_secs(3600));
        assert_eq!(config.list_ttl, Duration::from_secs(300));
        assert_eq!(config.user_ttl, Duration::from_secs(60));
    }
}
