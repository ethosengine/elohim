//! Delivery Relay - CDN-style content delivery assistance
//!
//! This module provides CDN-like functionality that COMPLEMENTS (not replaces)
//! the agent-side `holochain-cache-core` and `elohim-storage`.
//!
//! ## Architecture Position
//!
//! ```text
//! External Clients
//!       │
//!       ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │  DOORWAY (DeliveryRelay)                                    │
//! │  - Request coalescing (dedupe concurrent requests)          │
//! │  - Shard caching (hot Reed-Solomon shards)                  │
//! │  - Geographic routing hints                                 │
//! │  - CDN-style read caching                                   │
//! │                                                             │
//! │  DOES NOT: Write batching (that's agent's WriteBuffer)      │
//! └─────────────────────────────────────────────────────────────┘
//!       │
//!       ▼ (proxies to / caches from)
//! ┌─────────────────────────────────────────────────────────────┐
//! │  AGENT (holochain-cache-core + elohim-storage)              │
//! │  - Primary blob storage                                     │
//! │  - WriteBuffer for conductor protection                     │
//! │  - Reed-Solomon encoding/decoding                           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Difference from WriteBuffer
//!
//! - **WriteBuffer** (agent-side): Batches WRITES to protect conductor
//! - **DeliveryRelay** (doorway): Optimizes READS for external clients
//!
//! ## Features
//!
//! ### Request Coalescing
//!
//! When multiple clients request the same blob simultaneously, doorway
//! makes only ONE request to the agent and broadcasts the result to all
//! waiting clients. This prevents thundering herd on popular content.
//!
//! ### Shard Caching
//!
//! For Reed-Solomon encoded content, doorway can cache hot shards to
//! reduce load on elohim-storage nodes. Shards are immutable (hash-addressed)
//! so caching is safe.
//!
//! ### Geographic Routing
//!
//! Doorway can hint clients toward geographically closer elohim-storage
//! nodes for direct P2P connections, reducing latency.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for delivery relay
#[derive(Debug, Clone)]
pub struct DeliveryRelayConfig {
    /// Maximum concurrent coalesced requests
    pub max_coalesced_requests: usize,
    /// Timeout for coalesced request to complete
    pub coalesce_timeout: Duration,
    /// Maximum shard cache size in bytes
    pub shard_cache_max_bytes: u64,
    /// TTL for cached shards (they're immutable, so can be long)
    pub shard_cache_ttl: Duration,
    /// Whether to enable geographic routing hints
    pub enable_geo_routing: bool,
}

impl Default for DeliveryRelayConfig {
    fn default() -> Self {
        Self {
            max_coalesced_requests: 1000,
            coalesce_timeout: Duration::from_secs(30),
            shard_cache_max_bytes: 1024 * 1024 * 1024, // 1 GB
            shard_cache_ttl: Duration::from_secs(86400), // 24 hours
            enable_geo_routing: true,
        }
    }
}

// =============================================================================
// Coalesced Request
// =============================================================================

/// A request that may be coalesced with other concurrent requests
#[derive(Debug)]
pub struct CoalescedRequest {
    /// Content hash being requested
    pub hash: String,
    /// When the request started
    pub started_at: Instant,
    /// Number of clients waiting for this request
    pub waiting_count: usize,
}

/// Internal state for in-flight coalesced requests
struct InFlightRequest {
    /// Broadcast channel to notify waiters
    sender: broadcast::Sender<Result<Vec<u8>, String>>,
    /// When request started
    started_at: Instant,
}

// =============================================================================
// Delivery Relay
// =============================================================================

/// CDN-style delivery relay for doorway
///
/// Complements agent-side caching with:
/// - Request coalescing for concurrent requests
/// - Shard caching for Reed-Solomon content
/// - Geographic routing hints
pub struct DeliveryRelay {
    config: DeliveryRelayConfig,
    /// In-flight requests being coalesced (hash -> broadcast channel)
    in_flight: RwLock<HashMap<String, InFlightRequest>>,
    /// Simple shard cache (hash -> (data, cached_at))
    /// TODO: Replace with proper LRU from holochain-cache-core
    shard_cache: RwLock<HashMap<String, (Vec<u8>, Instant)>>,
    /// Total cached bytes
    cached_bytes: RwLock<u64>,
}

impl DeliveryRelay {
    /// Create a new delivery relay
    pub fn new(config: DeliveryRelayConfig) -> Self {
        info!(
            max_coalesced = config.max_coalesced_requests,
            shard_cache_mb = config.shard_cache_max_bytes / (1024 * 1024),
            "DeliveryRelay initialized"
        );

        Self {
            config,
            in_flight: RwLock::new(HashMap::new()),
            shard_cache: RwLock::new(HashMap::new()),
            cached_bytes: RwLock::new(0),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(DeliveryRelayConfig::default())
    }

    // =========================================================================
    // Request Coalescing
    // =========================================================================

    /// Check if a request for this hash is already in flight.
    /// If so, return a receiver to wait for the result.
    /// If not, return None (caller should make the request).
    pub async fn try_coalesce(&self, hash: &str) -> Option<broadcast::Receiver<Result<Vec<u8>, String>>> {
        let in_flight = self.in_flight.read().await;
        in_flight.get(hash).map(|req| req.sender.subscribe())
    }

    /// Register a new in-flight request for coalescing.
    /// Returns a sender to broadcast the result when complete.
    pub async fn register_in_flight(&self, hash: &str) -> Option<broadcast::Sender<Result<Vec<u8>, String>>> {
        let mut in_flight = self.in_flight.write().await;

        // Check if already registered (race condition)
        if in_flight.contains_key(hash) {
            return None;
        }

        // Check limit
        if in_flight.len() >= self.config.max_coalesced_requests {
            debug!(hash = hash, "Coalescing limit reached, not registering");
            return None;
        }

        let (sender, _) = broadcast::channel(1);
        in_flight.insert(hash.to_string(), InFlightRequest {
            sender: sender.clone(),
            started_at: Instant::now(),
        });

        Some(sender)
    }

    /// Complete an in-flight request, broadcasting result to all waiters.
    pub async fn complete_request(&self, hash: &str, result: Result<Vec<u8>, String>) {
        let mut in_flight = self.in_flight.write().await;
        if let Some(req) = in_flight.remove(hash) {
            let waiting = req.sender.receiver_count();
            let duration_ms = req.started_at.elapsed().as_millis();

            debug!(
                hash = hash,
                waiting = waiting,
                duration_ms = duration_ms,
                success = result.is_ok(),
                "Completing coalesced request"
            );

            // Broadcast result (ignore send errors - receivers may have dropped)
            let _ = req.sender.send(result);
        }
    }

    /// Get current in-flight request count
    pub async fn in_flight_count(&self) -> usize {
        self.in_flight.read().await.len()
    }

    // =========================================================================
    // Shard Caching
    // =========================================================================

    /// Get a cached shard by hash
    pub async fn get_shard(&self, hash: &str) -> Option<Vec<u8>> {
        let cache = self.shard_cache.read().await;
        if let Some((data, cached_at)) = cache.get(hash) {
            // Check TTL
            if cached_at.elapsed() < self.config.shard_cache_ttl {
                return Some(data.clone());
            }
        }
        None
    }

    /// Cache a shard (if space available)
    pub async fn cache_shard(&self, hash: &str, data: Vec<u8>) {
        let size = data.len() as u64;

        // Check if we have space
        let current = *self.cached_bytes.read().await;
        if current + size > self.config.shard_cache_max_bytes {
            // TODO: Evict old entries instead of refusing
            debug!(
                hash = hash,
                size = size,
                current = current,
                max = self.config.shard_cache_max_bytes,
                "Shard cache full, not caching"
            );
            return;
        }

        // Cache the shard
        let mut cache = self.shard_cache.write().await;
        let mut bytes = self.cached_bytes.write().await;

        // Remove old entry if exists
        if let Some((old_data, _)) = cache.remove(hash) {
            *bytes -= old_data.len() as u64;
        }

        cache.insert(hash.to_string(), (data, Instant::now()));
        *bytes += size;

        debug!(hash = hash, size = size, total_cached = *bytes, "Shard cached");
    }

    /// Get shard cache statistics
    pub async fn shard_cache_stats(&self) -> ShardCacheStats {
        let cache = self.shard_cache.read().await;
        let bytes = *self.cached_bytes.read().await;

        ShardCacheStats {
            entry_count: cache.len(),
            total_bytes: bytes,
            max_bytes: self.config.shard_cache_max_bytes,
        }
    }

    // =========================================================================
    // Geographic Routing (Stub)
    // =========================================================================

    /// Get routing hints for a blob hash.
    ///
    /// Returns list of elohim-storage endpoints ordered by proximity to the
    /// requester. Clients can use these for direct P2P connections.
    ///
    /// TODO: Implement actual geo-routing based on:
    /// - Requester IP geolocation
    /// - Known elohim-storage node locations
    /// - Network latency measurements
    pub async fn get_routing_hints(&self, _hash: &str, _requester_ip: Option<&str>) -> Vec<RoutingHint> {
        if !self.config.enable_geo_routing {
            return vec![];
        }

        // TODO: Implement geographic routing
        // For now, return empty (clients fall back to doorway)
        vec![]
    }

    // =========================================================================
    // Cleanup
    // =========================================================================

    /// Clean up expired entries
    pub async fn cleanup(&self) {
        // Clean expired shards
        let mut cache = self.shard_cache.write().await;
        let mut bytes = self.cached_bytes.write().await;

        let ttl = self.config.shard_cache_ttl;
        let before = cache.len();

        cache.retain(|_, (data, cached_at)| {
            if cached_at.elapsed() >= ttl {
                *bytes -= data.len() as u64;
                false
            } else {
                true
            }
        });

        let evicted = before - cache.len();
        if evicted > 0 {
            info!(evicted = evicted, remaining = cache.len(), "Shard cache cleanup");
        }

        // Clean timed-out in-flight requests
        drop(cache);
        drop(bytes);

        let mut in_flight = self.in_flight.write().await;
        let timeout = self.config.coalesce_timeout;

        in_flight.retain(|hash, req| {
            if req.started_at.elapsed() >= timeout {
                debug!(hash = hash, "In-flight request timed out");
                let _ = req.sender.send(Err("Request timed out".into()));
                false
            } else {
                true
            }
        });
    }
}

// =============================================================================
// Statistics
// =============================================================================

/// Shard cache statistics
#[derive(Debug, Clone)]
pub struct ShardCacheStats {
    pub entry_count: usize,
    pub total_bytes: u64,
    pub max_bytes: u64,
}

/// Routing hint for direct P2P connection
#[derive(Debug, Clone)]
pub struct RoutingHint {
    /// Endpoint URL for elohim-storage node
    pub endpoint: String,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u32,
    /// Geographic region
    pub region: Option<String>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_coalescing() {
        let relay = DeliveryRelay::with_defaults();
        let hash = "sha256-abc123";

        // First request registers
        let sender = relay.register_in_flight(hash).await;
        assert!(sender.is_some());

        // Second request coalesces
        let receiver = relay.try_coalesce(hash).await;
        assert!(receiver.is_some());

        // Complete the request
        let data = vec![1, 2, 3, 4];
        relay.complete_request(hash, Ok(data.clone())).await;

        // Receiver should get the data
        let mut rx = receiver.unwrap();
        let result = rx.recv().await.unwrap();
        assert_eq!(result.unwrap(), data);

        // Request should be removed
        assert_eq!(relay.in_flight_count().await, 0);
    }

    #[tokio::test]
    async fn test_shard_caching() {
        let relay = DeliveryRelay::with_defaults();
        let hash = "sha256-abc123";
        let data = vec![1, 2, 3, 4, 5];

        // Initially not cached
        assert!(relay.get_shard(hash).await.is_none());

        // Cache the shard
        relay.cache_shard(hash, data.clone()).await;

        // Now cached
        let cached = relay.get_shard(hash).await;
        assert_eq!(cached, Some(data));

        // Check stats
        let stats = relay.shard_cache_stats().await;
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.total_bytes, 5);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config = DeliveryRelayConfig {
            shard_cache_ttl: Duration::from_millis(10),
            ..Default::default()
        };
        let relay = DeliveryRelay::new(config);

        // Cache a shard
        relay.cache_shard("hash1", vec![1, 2, 3]).await;
        assert_eq!(relay.shard_cache_stats().await.entry_count, 1);

        // Wait for TTL
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Cleanup should remove it
        relay.cleanup().await;
        assert_eq!(relay.shard_cache_stats().await.entry_count, 0);
    }
}
