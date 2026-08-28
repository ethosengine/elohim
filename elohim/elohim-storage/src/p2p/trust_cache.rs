//! Per-connection trust context cache.
//!
//! Stores verified trust contexts keyed by libp2p PeerId. Populated by
//! the trust handshake, queried by check_reach_authorization for fast-path
//! ambient authorization.

use libp2p::PeerId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::trust_verification::VerifiedTrustContext;

/// Thread-safe peer trust cache
#[derive(Clone)]
pub struct PeerTrustCache {
    inner: Arc<RwLock<HashMap<PeerId, VerifiedTrustContext>>>,
}

impl PeerTrustCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert or replace a peer's verified trust context.
    pub async fn insert(&self, peer_id: PeerId, ctx: VerifiedTrustContext) {
        self.inner.write().await.insert(peer_id, ctx);
    }

    /// Get a peer's trust context if it exists and is not expired.
    pub async fn get(&self, peer_id: &PeerId) -> Option<VerifiedTrustContext> {
        let cache = self.inner.read().await;
        let ctx = cache.get(peer_id)?;
        if ctx.verified_at.elapsed() < ctx.ttl {
            Some(ctx.clone())
        } else {
            None
        }
    }

    /// Try to get a peer's trust context without blocking (for sync fast-path).
    /// Returns None if lock is contended or entry missing/expired.
    pub fn try_get(&self, peer_id: &PeerId) -> Option<VerifiedTrustContext> {
        let cache = self.inner.try_read().ok()?;
        let ctx = cache.get(peer_id)?;
        if ctx.verified_at.elapsed() < ctx.ttl {
            Some(ctx.clone())
        } else {
            None
        }
    }

    /// Remove a peer's trust context (on disconnect).
    pub async fn remove(&self, peer_id: &PeerId) {
        self.inner.write().await.remove(peer_id);
    }

    /// Evict all expired entries.
    pub async fn evict_expired(&self) {
        let mut cache = self.inner.write().await;
        cache.retain(|_, ctx| ctx.verified_at.elapsed() < ctx.ttl);
    }

    /// Try to get a trust context by agent pubkey (for use from check_reach_authorization
    /// which doesn't have the PeerId).
    pub fn try_get_by_agent(&self, agent_pubkey: Option<&str>) -> Option<VerifiedTrustContext> {
        let agent = agent_pubkey?;
        let cache = self.inner.try_read().ok()?;
        cache
            .values()
            .find(|ctx| ctx.agent_pubkey == agent && ctx.verified_at.elapsed() < ctx.ttl)
            .cloned()
    }

    /// Number of cached peers (for observability).
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Whether the cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Bounded-cardinality peer classification for metrics labels — never the
    /// raw peer id (see the `SYNC_REQUEST_OUTCOMES` doc comment in
    /// `metrics.rs` for why: an unbounded-cardinality bomb on a real mesh).
    /// Reads the cached verified trust context's `reach_ceiling`, clamped to
    /// the [`crate::generated_enums::CORE_REACH_LEVELS`] vocabulary (the same
    /// closed set `inc_projection_reconcile_reach_narrowed` clamps
    /// wire-derived reach against — an unrecognised string, from a future
    /// build's new tier or a bug, lands on `"other"` rather than minting its
    /// own label) or `"unverified"` when no context is cached for the peer
    /// (never handshook, or the entry expired). Cache-read only — `try_get`'s
    /// non-blocking `try_read`, no network, no conductor call — so it is safe
    /// to call from a hot swarm-event-loop path.
    pub fn peer_class_for(&self, peer: &PeerId) -> &'static str {
        match self.try_get(peer) {
            Some(ctx) => sanitize_reach_ceiling(&ctx.reach_ceiling),
            None => "unverified",
        }
    }
}

/// Clamp a `reach_ceiling` string to the closed
/// [`crate::generated_enums::CORE_REACH_LEVELS`] vocabulary, mapping anything
/// outside it to `"other"`. Returns a `'static` reference into the generated
/// vocabulary array (never the caller's owned string), so the result is
/// always safe to use as a bounded-cardinality metrics label.
fn sanitize_reach_ceiling(reach_ceiling: &str) -> &'static str {
    crate::generated_enums::CORE_REACH_LEVELS
        .iter()
        .find(|&&tier| tier == reach_ceiling)
        .copied()
        .unwrap_or("other")
}

impl Default for PeerTrustCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_verification::VerifiedTrustContext;
    use std::time::{Duration, Instant};

    fn make_context(ceiling: &str, ttl_secs: u64) -> VerifiedTrustContext {
        VerifiedTrustContext {
            agent_pubkey: "uhCAk_test".to_string(),
            agent_verified: true,
            reach_ceiling: ceiling.to_string(),
            verified_memberships: vec![],
            verified_relationships: vec![],
            verified_attestations: vec![],
            verified_stewardship: vec![],
            verified_at: Instant::now(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    #[tokio::test]
    async fn insert_and_get() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        cache.insert(peer, make_context("trusted", 3600)).await;
        let ctx = cache.get(&peer).await.unwrap();
        assert_eq!(ctx.reach_ceiling, "trusted");
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_peer() {
        let cache = PeerTrustCache::new();
        assert!(cache.get(&PeerId::random()).await.is_none());
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        cache.insert(peer, make_context("trusted", 0)).await;
        assert!(cache.get(&peer).await.is_none());
    }

    #[tokio::test]
    async fn remove_clears_entry() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        cache.insert(peer, make_context("trusted", 3600)).await;
        cache.remove(&peer).await;
        assert!(cache.get(&peer).await.is_none());
    }

    #[tokio::test]
    async fn evict_expired_cleans_stale_entries() {
        let cache = PeerTrustCache::new();
        let fresh = PeerId::random();
        let stale = PeerId::random();
        cache.insert(fresh, make_context("trusted", 3600)).await;
        cache.insert(stale, make_context("community", 0)).await;
        cache.evict_expired().await;
        assert_eq!(cache.len().await, 1);
        assert!(cache.get(&fresh).await.is_some());
        assert!(cache.get(&stale).await.is_none());
    }

    #[test]
    fn try_get_returns_valid_entry() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        // Use blocking runtime for sync try_get test
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(cache.insert(peer, make_context("community", 3600)));
        let ctx = cache.try_get(&peer).unwrap();
        assert_eq!(ctx.reach_ceiling, "community");
    }

    #[test]
    fn try_get_returns_none_for_missing() {
        let cache = PeerTrustCache::new();
        assert!(cache.try_get(&PeerId::random()).is_none());
    }

    #[test]
    fn peer_class_for_cached_context_returns_its_reach_ceiling() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(cache.insert(peer, make_context("trusted", 3600)));
        assert_eq!(cache.peer_class_for(&peer), "trusted");
    }

    #[test]
    fn peer_class_for_unknown_reach_ceiling_string_clamps_to_other() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        let rt = tokio::runtime::Runtime::new().unwrap();
        // A reach_ceiling value outside CORE_REACH_LEVELS — a future tier, or
        // a bug — must never mint its own metrics label.
        rt.block_on(cache.insert(peer, make_context("some-future-reach-tier", 3600)));
        assert_eq!(cache.peer_class_for(&peer), "other");
    }

    #[test]
    fn peer_class_for_no_cached_context_returns_unverified() {
        let cache = PeerTrustCache::new();
        assert_eq!(cache.peer_class_for(&PeerId::random()), "unverified");
    }

    #[test]
    fn sanitize_reach_ceiling_covers_the_full_canonical_vocabulary() {
        for tier in crate::generated_enums::CORE_REACH_LEVELS {
            assert_eq!(sanitize_reach_ceiling(tier), *tier);
        }
        assert_eq!(sanitize_reach_ceiling("not-a-real-tier"), "other");
    }
}
