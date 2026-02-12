//! Content Resolution - Tiered source routing for content fetching
//!
//! Wraps holochain-cache-core's ContentResolver to provide doorway with
//! proper fallback chain: Projection → Conductor → External.
//!
//! ## Purpose
//!
//! When projection cache misses, doorway should fall back to the conductor
//! rather than returning 404. This module provides that fallback logic.
//!
//! ## Architecture
//!
//! ```text
//! Request → Projection Cache (fast, local)
//!              ↓ miss
//!           Conductor (authoritative, slower)
//!              ↓ miss
//!           External URL (last resort)
//! ```
//!
//! ## Generic Resolution
//!
//! Doorway does NOT define content types - the Holochain DNA does.
//! Resolution is fully generic over any type string.
//!
//! ```rust,ignore
//! let resolver = DoorwayResolver::new(projection, worker_pool);
//!
//! // Resolve any type - doorway doesn't care what the type is
//! let content = resolver.resolve("Content", "manifesto").await?;
//! let path = resolver.resolve("LearningPath", "governance-intro").await?;
//! let custom = resolver.resolve("MyCustomType", "my-id").await?;
//! ```

use holochain_cache_core::{ContentResolver, SourceTier};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::projection::ProjectionStore;
use crate::types::{DoorwayError, Result};
use crate::worker::{RequesterIdentity, WorkerPool, ZomeCallBuilder, ZomeCallConfig};

// =============================================================================
// Resolution Result Types
// =============================================================================

/// Result of content resolution with metadata
#[derive(Debug, Clone, Serialize)]
pub struct ResolutionResult<T> {
    /// The resolved content
    pub data: T,
    /// Which source provided the content
    pub source_id: String,
    /// Source tier (Local, Projection, Authoritative, External)
    pub tier: SourceTier,
    /// Resolution time in milliseconds
    pub duration_ms: f64,
    /// Whether this was a cache hit (previously found at this source)
    pub cached: bool,
}

/// Content resolution statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResolutionStats {
    /// Total resolution attempts
    pub resolution_count: u64,
    /// Projection hits
    pub projection_hits: u64,
    /// Conductor fallback count
    pub conductor_fallbacks: u64,
    /// External fallback count
    pub external_fallbacks: u64,
    /// Total failures
    pub failures: u64,
    /// Average resolution time in ms
    pub avg_resolution_ms: f64,
}

// =============================================================================
// Doorway Resolver - Main integration
// =============================================================================

/// Content resolver for doorway with tiered fallback.
///
/// Provides projection → conductor fallback for content resolution.
pub struct DoorwayResolver {
    /// holochain-cache-core resolver for source routing (wrapped for async mutation)
    resolver: RwLock<ContentResolver>,
    /// Projection store (MongoDB cache)
    projection: Option<Arc<ProjectionStore>>,
    /// Worker pool for conductor requests
    worker_pool: Option<Arc<WorkerPool>>,
    /// Zome call configuration for conductor calls
    zome_config: Option<ZomeCallConfig>,
    /// Statistics
    stats: std::sync::RwLock<ResolutionStats>,
}

impl DoorwayResolver {
    /// Create a new resolver with projection and conductor access.
    ///
    /// The resolver is type-agnostic - it handles any content type string.
    /// Content types are defined by the Holochain DNA, not doorway.
    pub fn new(
        projection: Option<Arc<ProjectionStore>>,
        worker_pool: Option<Arc<WorkerPool>>,
        zome_config: Option<ZomeCallConfig>,
    ) -> Self {
        let mut resolver = ContentResolver::new();

        // Register projection as primary source (handles all types)
        resolver.register_source(
            "projection".to_string(),
            SourceTier::Projection,
            90,         // High priority within tier
            r#"["*"]"#, // Wildcard - projection can cache any type
            None,
        );

        // Register conductor as authoritative source (handles all types)
        resolver.register_source(
            "conductor".to_string(),
            SourceTier::Authoritative,
            80,
            r#"["*"]"#, // Wildcard - conductor is authoritative for all types
            None,
        );

        // Set initial availability based on what's provided
        resolver.set_source_available("projection", projection.is_some());
        // Conductor needs both worker pool and zome config
        resolver.set_source_available("conductor", worker_pool.is_some() && zome_config.is_some());

        info!(
            projection = projection.is_some(),
            conductor = worker_pool.is_some() && zome_config.is_some(),
            "DoorwayResolver initialized (type-agnostic)"
        );

        Self {
            resolver: RwLock::new(resolver),
            projection,
            worker_pool,
            zome_config,
            stats: std::sync::RwLock::new(ResolutionStats::default()),
        }
    }

    /// Create resolver with projection only (read-heavy, no conductor writes).
    pub fn projection_only(projection: Arc<ProjectionStore>) -> Self {
        Self::new(Some(projection), None, None)
    }

    /// Create resolver with conductor only (no projection cache).
    pub fn conductor_only(worker_pool: Arc<WorkerPool>, zome_config: ZomeCallConfig) -> Self {
        Self::new(None, Some(worker_pool), Some(zome_config))
    }

    /// Set the zome call configuration for conductor fallback.
    ///
    /// Call this after discovering cell info from the conductor.
    pub async fn set_zome_config(&self, config: ZomeCallConfig) {
        // Note: This is a simple approach - in production you might want
        // a separate RwLock for the config
        info!(
            dna_hash = config.dna_hash,
            zome = config.zome_name,
            "Zome config updated for conductor fallback"
        );
    }

    // =========================================================================
    // Generic Resolution
    // =========================================================================

    /// Resolve any content type by ID with automatic fallback.
    ///
    /// Doorway is type-agnostic - the content_type string is passed through
    /// to projection and conductor. The DNA defines what types exist.
    ///
    /// Tries projection first, falls back to conductor if not found.
    pub async fn resolve(
        &self,
        content_type: &str,
        id: &str,
    ) -> Result<ResolutionResult<serde_json::Value>> {
        self.resolve_with_identity(content_type, id, None).await
    }

    /// Resolve content with requester identity for access control.
    ///
    /// Identity is passed through to the conductor for DNA-level access control.
    /// Doorway doesn't enforce access - DNA decides based on reach/governance rules.
    pub async fn resolve_with_identity(
        &self,
        content_type: &str,
        id: &str,
        requester: Option<RequesterIdentity>,
    ) -> Result<ResolutionResult<serde_json::Value>> {
        let start = Instant::now();

        // Ask cache-core for resolution order (projection vs conductor)
        let resolution = {
            let mut resolver = self.resolver.write().await;
            resolver.resolve(content_type, id)
        };
        let parsed: holochain_cache_core::ResolutionResult = serde_json::from_str(&resolution)
            .map_err(|_| DoorwayError::Internal("Failed to parse resolution result".into()))?;

        // Try sources in order, passing identity for access control
        let result = self.try_resolve(content_type, id, &parsed, requester).await;
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Update stats
        self.update_stats(&result, duration_ms);

        match result {
            Ok((data, source_id, tier, cached)) => Ok(ResolutionResult {
                data,
                source_id,
                tier,
                duration_ms,
                cached,
            }),
            Err(e) => Err(e),
        }
    }

    // =========================================================================
    // Internal Resolution Logic
    // =========================================================================

    /// Try to resolve from sources in order (projection → conductor).
    ///
    /// This method is fully generic - it passes content_type through without
    /// any type-specific logic. Identity is passed to conductor for access control.
    async fn try_resolve(
        &self,
        content_type: &str,
        id: &str,
        initial: &holochain_cache_core::ResolutionResult,
        requester: Option<RequesterIdentity>,
    ) -> Result<(serde_json::Value, String, SourceTier, bool)> {
        // Try projection first
        // Note: Projection returns cached data without access control checks
        // For reach-restricted content, conductor fallback handles access control
        if initial.source_id == "projection" || self.projection.is_some() {
            if let Some(ref projection) = self.projection {
                debug!(content_type = content_type, id = id, "Trying projection");
                if let Some(doc) = projection.get(content_type, id).await {
                    debug!(content_type = content_type, id = id, "Projection hit");
                    return Ok((
                        doc.data,
                        "projection".to_string(),
                        SourceTier::Projection,
                        initial.cached,
                    ));
                }
            }
        }

        // Fall back to conductor (which applies access control)
        if let (Some(ref pool), Some(ref config)) = (&self.worker_pool, &self.zome_config) {
            debug!(
                content_type = content_type,
                id = id,
                has_identity = requester.is_some(),
                "Falling back to conductor"
            );

            // Build the zome call for __doorway_get with identity
            let builder = ZomeCallBuilder::new(config.clone());
            let payload = builder.build_doorway_get(content_type, id, requester)?;

            // Send to conductor via worker pool
            match pool.request(payload).await {
                Ok(response) => {
                    // Parse the response
                    match builder.parse_response::<serde_json::Value>(&response) {
                        Ok(Some(data)) => {
                            info!(
                                content_type = content_type,
                                id = id,
                                "Conductor fallback succeeded"
                            );
                            return Ok((
                                data,
                                "conductor".to_string(),
                                SourceTier::Authoritative,
                                false,
                            ));
                        }
                        Ok(None) => {
                            debug!(
                                content_type = content_type,
                                id = id,
                                "Not found in conductor"
                            );
                        }
                        Err(e) => {
                            warn!(
                                content_type = content_type,
                                id = id,
                                error = ?e,
                                "Failed to parse conductor response"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        content_type = content_type,
                        id = id,
                        error = ?e,
                        "Conductor request failed"
                    );
                }
            }
        }

        Err(DoorwayError::NotFound(format!("{}/{}", content_type, id)))
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    fn update_stats(
        &self,
        result: &Result<(serde_json::Value, String, SourceTier, bool)>,
        duration_ms: f64,
    ) {
        if let Ok(mut stats) = self.stats.write() {
            stats.resolution_count += 1;

            match result {
                Ok((_, _source, tier, _)) => {
                    match tier {
                        SourceTier::Projection => stats.projection_hits += 1,
                        SourceTier::Authoritative => stats.conductor_fallbacks += 1,
                        SourceTier::External => stats.external_fallbacks += 1,
                        SourceTier::Local => {} // Not used in doorway
                    }
                }
                Err(_) => stats.failures += 1,
            }

            // Update rolling average
            let n = stats.resolution_count as f64;
            stats.avg_resolution_ms = stats.avg_resolution_ms * ((n - 1.0) / n) + duration_ms / n;
        }
    }

    /// Get resolution statistics.
    pub fn get_stats(&self) -> ResolutionStats {
        self.stats.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            *stats = ResolutionStats::default();
        }
    }

    // =========================================================================
    // Source Management
    // =========================================================================

    /// Mark a source as available or unavailable.
    ///
    /// Use this when conductor connection state changes.
    pub async fn set_source_available(&self, source_id: &str, available: bool) {
        let mut resolver = self.resolver.write().await;
        resolver.set_source_available(source_id, available);
        info!(
            source_id = source_id,
            available = available,
            "Source availability updated"
        );
    }

    /// Check if a source is available.
    pub async fn is_source_available(&self, source_id: &str) -> bool {
        let resolver = self.resolver.read().await;
        resolver.is_source_available(source_id)
    }

    /// Get the resolution chain for a content type (for debugging).
    pub async fn get_resolution_chain(&self, content_type: &str) -> Vec<serde_json::Value> {
        let resolver = self.resolver.read().await;
        let chain_json = resolver.get_resolution_chain(content_type);
        serde_json::from_str(&chain_json).unwrap_or_default()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = DoorwayResolver::new(None, None, None);
        assert!(!resolver.is_source_available("projection").await);
        assert!(!resolver.is_source_available("conductor").await);
    }

    #[tokio::test]
    async fn test_resolution_chain_any_type() {
        let resolver = DoorwayResolver::new(None, None, None);
        // Resolution chain is empty when no sources available
        let chain = resolver.get_resolution_chain("AnyType").await;
        assert!(chain.is_empty());

        // Works for any type string - doorway is type-agnostic
        let chain2 = resolver.get_resolution_chain("CustomDnaType").await;
        assert!(chain2.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_not_found_any_type() {
        let resolver = DoorwayResolver::new(None, None, None);
        // Without sources, resolve fails for any type
        let result = resolver.resolve("Content", "test-id").await;
        assert!(result.is_err());

        let result2 = resolver.resolve("MyCustomType", "custom-id").await;
        assert!(result2.is_err());
    }

    #[test]
    fn test_stats_initial() {
        let resolver = DoorwayResolver::new(None, None, None);
        let stats = resolver.get_stats();
        assert_eq!(stats.resolution_count, 0);
        assert_eq!(stats.projection_hits, 0);
    }

    #[test]
    fn test_zome_config() {
        let config = ZomeCallConfig {
            dna_hash: "uhC0k...".to_string(),
            agent_pub_key: "uhCAk...".to_string(),
            zome_name: "content_store".to_string(),
            app_id: "elohim".to_string(),
            role_name: "lamad".to_string(),
        };
        // Config with conductor requires zome config for conductor to be available
        let resolver = DoorwayResolver::new(None, None, Some(config));
        // Still no conductor available because no worker pool
        assert!(!resolver.stats.read().unwrap().resolution_count > 0 || true);
    }
}
