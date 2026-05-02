//! Integration tests for the dashboard topology service.
//!
//! These tests construct dependencies directly (no DoorwayHarness) and assert
//! against the Phase 4 stub behaviour described in the plan.

use std::sync::Arc;

use doorway::cache::store::CacheStats;
use doorway::services::dashboard_topology::DashboardTopologyService;
use doorway::services::federation::{new_peer_cache, PeerDoorway};
use doorway::services::RouteRegistry;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_cache_stats(entries: usize, hits: u64, misses: u64) -> Arc<RwLock<CacheStats>> {
    Arc::new(RwLock::new(CacheStats {
        entries,
        hits,
        misses,
        evictions: 0,
    }))
}

fn make_service(
    cache_stats: Arc<RwLock<CacheStats>>,
    peer_cache: doorway::services::federation::PeerCache,
) -> DashboardTopologyService {
    let registry = Arc::new(RouteRegistry::with_defaults());
    DashboardTopologyService::with_hostname("test.elohim.host", cache_stats, peer_cache, registry)
}

// ---------------------------------------------------------------------------
// T28 Test 1: storage stewards are always empty in Phase 4
// ---------------------------------------------------------------------------

/// Phase 4 stub: `storage_stewards` returns an empty list until Phase 5 wires
/// the RouteRegistry lookup.
#[tokio::test]
async fn dashboard_topology_reports_storage_stewards_stub() {
    let cache_stats = make_cache_stats(0, 0, 0);
    let peer_cache = new_peer_cache();
    let svc = make_service(cache_stats, peer_cache);

    let view = svc.build_view().await.unwrap();

    assert!(
        view.storage_stewards.is_empty(),
        "Phase 4 stub: storage_stewards must be empty until RouteRegistry lookup is wired"
    );
}

// ---------------------------------------------------------------------------
// T28 Test 2: federation peers are projected from the peer cache
// ---------------------------------------------------------------------------

/// When a PeerDoorway is in the cache its hostname should appear in
/// `federation_peers`.
#[tokio::test]
async fn dashboard_topology_reports_federation_peers_from_cache() {
    let cache_stats = make_cache_stats(0, 0, 0);
    let peer_cache = new_peer_cache();

    peer_cache.write().await.push(PeerDoorway {
        id: "doorway-2".into(),
        url: "https://parish.elohim.host".into(),
        region: None,
        capabilities: vec![],
        source_peer: "self".into(),
    });

    let svc = make_service(cache_stats, peer_cache);
    let view = svc.build_view().await.unwrap();

    assert!(
        view.federation_peers
            .iter()
            .any(|f| f.doorway_hostname == "parish.elohim.host"),
        "Expected 'parish.elohim.host' in federation_peers, got: {:?}",
        view.federation_peers
            .iter()
            .map(|f| &f.doorway_hostname)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// T28 Test 3: cache hit rate is projected correctly (percentage → ratio)
// ---------------------------------------------------------------------------

/// `CacheStats::hit_rate()` returns 0-100; the schema requires 0-1. The service
/// must divide by 100 before writing to `cache_hit_rate_24h`.
///
/// Seed: hits=87, misses=13 → hit_rate()=87.0 → cache_hit_rate_24h=0.87
#[tokio::test]
async fn dashboard_topology_includes_cache_hit_rate() {
    let cache_stats = make_cache_stats(10, 87, 13);
    let peer_cache = new_peer_cache();
    let svc = make_service(cache_stats, peer_cache);

    let view = svc.build_view().await.unwrap();

    let rate = view.projection_coverage.cache_hit_rate_24h;
    assert!(
        (rate - 0.87).abs() < 0.01,
        "Expected cache_hit_rate_24h ≈ 0.87, got {rate}"
    );
}
