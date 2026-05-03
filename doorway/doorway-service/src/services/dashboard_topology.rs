//! Dashboard topology service for the Doorway operator panel.
//!
//! Aggregates storage steward rows, federation peer rows, projection coverage,
//! and public surface health into a single [`DoorwayDashboardView`] payload.
//!
//! ## Phase 4 stubs
//!
//! Several fields are stubbed in this phase:
//! - `storage_stewards` — always empty; Phase 5 wires [`RouteRegistry`] lookup.
//! - `federation_peers.online` — always `true` (cached = known).
//! - `federation_peers.direction` — always [`FederationDirection::OutboundOnly`].
//! - `federation_peers.shared_cid_count` — always `0`.
//! - `projection_coverage.projected_cid_count` / `known_cid_count` — both use
//!   `cache_stats.entries`; real projection-cache split is future work.
//! - `projection_coverage.projection_lag_ms_avg` — always `0`.
//! - `public_surface` — DNS/TLS probes are stub values; Phase 5 wires real probes.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::cache::store::CacheStats;
use crate::services::federation::{get_cached_peers, PeerCache};
use crate::services::RouteRegistry;

// =============================================================================
// View types (inline — no crate::views::dashboard module exists yet)
// Snake_case never leaves the Rust boundary; all view structs use camelCase.
// =============================================================================

/// Top-level dashboard payload for a doorway operator.
///
/// Schema: `elohim/sdk/schemas/v1/views/doorway-dashboard-view.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayDashboardView {
    /// Public hostname of this doorway.
    pub doorway_hostname: String,
    /// Per-storage-steward rows for peers registered with this doorway.
    pub storage_stewards: Vec<DashboardSteward>,
    /// Per-doorway federation rows.
    pub federation_peers: Vec<DashboardFederationPeer>,
    /// Projection cache and lag aggregate for this doorway.
    pub projection_coverage: ProjectionCoverage,
    /// DNS/TLS/reachability surface for the doorway hostname.
    pub public_surface: PublicSurfaceState,
}

/// Per-storage-steward row in a doorway dashboard.
///
/// Schema: `elohim/sdk/schemas/v1/views/dashboard-steward.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSteward {
    /// Peer identifier (agent pub key or node id).
    pub peer_id: String,
    /// Device archetype string.
    ///
    /// TODO(Phase 5): replace with the DeviceArchetype enum once it is published
    /// in a doorway-reachable crate. Field is currently unreachable because
    /// `storage_stewards` is empty in Phase 4.
    pub archetype: String,
    /// Human-readable display name, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Whether the steward is currently online.
    pub online: bool,
    /// Count of CIDs this steward currently hosts.
    pub hosting_count: u64,
    /// Approximate libp2p Kad hop distance from the doorway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hint: Option<u32>,
}

/// Per-doorway federation row.
///
/// Schema: `elohim/sdk/schemas/v1/views/dashboard-federation-peer.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFederationPeer {
    /// Public hostname of the federated doorway.
    pub doorway_hostname: String,
    /// Whether the peer is currently online.
    pub online: bool,
    /// Direction of byte flow across this federation edge.
    pub direction: FederationDirection,
    /// Count of CIDs both doorways project.
    pub shared_cid_count: u64,
}

/// Direction of federation byte flow.
///
/// Matches schema enum literals: `bidirectional` / `outbound_only` / `inbound_only`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationDirection {
    Bidirectional,
    OutboundOnly,
    InboundOnly,
}

/// Projection cache and lag aggregate for a doorway.
///
/// Schema: `elohim/sdk/schemas/v1/views/projection-coverage.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionCoverage {
    /// Number of CIDs the doorway is currently projecting.
    pub projected_cid_count: u64,
    /// Total CIDs the doorway is aware of (projected + known but unprojected).
    pub known_cid_count: u64,
    /// Trailing 24h cache hit ratio (0.0–1.0).
    /// Derived from `CacheStats::hit_rate()` (which returns 0–100) divided by 100.
    pub cache_hit_rate_24h: f64,
    /// Average ms between source observation and projection ack over last 24h.
    /// Phase 4 stub — always 0 until projection lag telemetry is wired.
    pub projection_lag_ms_avg: u64,
}

/// DNS/TLS/reachability surface for a doorway hostname.
///
/// Schema: `elohim/sdk/schemas/v1/views/public-surface-state.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceState {
    /// Whether DNS resolves for this hostname.
    pub dns_resolves: bool,
    /// Resolved A/AAAA target, when DNS resolves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_target: Option<String>,
    /// Whether the TLS certificate is valid.
    pub tls_valid: bool,
    /// Days until TLS cert expires (negative if already expired).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_expires_in_days: Option<i32>,
    /// True when an external probe can reach the doorway.
    pub public_reachable: bool,
}

// =============================================================================
// Error type
// =============================================================================

/// Errors produced by the dashboard topology service.
#[derive(Debug, Error)]
pub enum DashboardError {
    /// Returned when federation peer enumeration fails.
    /// TODO(Phase 5): constructed once `collect_federation_peers` performs
    /// liveness/direction probes that can fail.
    #[error("federation error: {0}")]
    Federation(String),

    /// Returned when projection-coverage metrics cannot be read.
    /// TODO(Phase 5): constructed once projection cache exposes its own
    /// hits/misses split distinct from `CacheStats`.
    #[error("metrics error: {0}")]
    Metrics(String),
}

// =============================================================================
// Service
// =============================================================================

/// Aggregates topology data into [`DoorwayDashboardView`].
///
/// Dependencies are injected at construction; `build_view()` is the only public
/// method. This keeps the service independently testable without spinning up a
/// full doorway runtime.
pub struct DashboardTopologyService {
    /// Public hostname of this doorway, surfaced verbatim in the view.
    doorway_hostname: String,
    /// Live cache stats — read once per `build_view()` call.
    cache_stats: Arc<RwLock<CacheStats>>,
    /// Known federation peers, populated by the federation background task.
    peer_cache: PeerCache,
    /// Route registry — kept for Phase 5 steward lookup; not called yet.
    #[allow(dead_code)]
    route_registry: Arc<RouteRegistry>,
}

impl DashboardTopologyService {
    /// Construct the service with injected dependencies.
    pub fn new(
        cache_stats: Arc<RwLock<CacheStats>>,
        peer_cache: PeerCache,
        route_registry: Arc<RouteRegistry>,
    ) -> Self {
        Self {
            doorway_hostname: derive_doorway_hostname(),
            cache_stats,
            peer_cache,
            route_registry,
        }
    }

    /// Construct the service with an explicit hostname (useful in tests or when
    /// the hostname is known at construction time).
    pub fn with_hostname(
        hostname: impl Into<String>,
        cache_stats: Arc<RwLock<CacheStats>>,
        peer_cache: PeerCache,
        route_registry: Arc<RouteRegistry>,
    ) -> Self {
        Self {
            doorway_hostname: hostname.into(),
            cache_stats,
            peer_cache,
            route_registry,
        }
    }

    /// Build the full dashboard view.
    ///
    /// Reads from the live peer cache and cache stats snapshot; never blocks on
    /// network calls in Phase 4.
    pub async fn build_view(&self) -> Result<DoorwayDashboardView, DashboardError> {
        let storage_stewards = self.collect_storage_stewards().await?;
        let federation_peers = self.collect_federation_peers().await?;
        let projection_coverage = self.build_projection_coverage().await?;
        let public_surface = build_public_surface(&self.doorway_hostname);

        Ok(DoorwayDashboardView {
            doorway_hostname: self.doorway_hostname.clone(),
            storage_stewards,
            federation_peers,
            projection_coverage,
            public_surface,
        })
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Phase 4 stub — returns empty vec.
    ///
    /// TODO(Phase 5): wire [`RouteRegistry`] to enumerate registered storage
    /// stewards, join with imagodei DHT for archetype + display name, and query
    /// rea_commitments for hosting counts.
    async fn collect_storage_stewards(&self) -> Result<Vec<DashboardSteward>, DashboardError> {
        Ok(vec![])
    }

    /// Builds federation peer rows from the cached [`PeerDoorway`] list.
    async fn collect_federation_peers(
        &self,
    ) -> Result<Vec<DashboardFederationPeer>, DashboardError> {
        let peers = get_cached_peers(&self.peer_cache).await;

        let rows = peers
            .into_iter()
            .map(|peer| {
                let doorway_hostname = parse_hostname_from_url(&peer.url);
                DashboardFederationPeer {
                    doorway_hostname,
                    // Phase 4 stub: having a cached entry counts as "known online".
                    online: true,
                    // Phase 4 stub: direction requires bidirectional handshake; defer to Phase 5.
                    direction: FederationDirection::OutboundOnly,
                    // Phase 4 stub: shared CID count requires cross-peer inventory comparison.
                    // TODO(Phase 5): compute from intersection of projection manifests.
                    shared_cid_count: 0,
                }
            })
            .collect();

        Ok(rows)
    }

    /// Builds projection coverage metrics from the live cache stats snapshot.
    async fn build_projection_coverage(&self) -> Result<ProjectionCoverage, DashboardError> {
        let stats = self.cache_stats.read().await.clone();

        // Phase 4 stub: projected_cid_count and known_cid_count both use
        // cache entries until the real projection-cache split is tracked.
        // TODO(Phase 5): separate "projected" (served from cache) vs "known"
        // (seen in DHT signals but not yet locally cached).
        let cid_count = stats.entries as u64;

        // CacheStats::hit_rate() returns 0-100; schema requires 0-1 ratio.
        let cache_hit_rate_24h = stats.hit_rate() / 100.0;

        Ok(ProjectionCoverage {
            projected_cid_count: cid_count,
            known_cid_count: cid_count,
            cache_hit_rate_24h,
            // Phase 4 stub: lag telemetry not yet wired.
            projection_lag_ms_avg: 0,
        })
    }
}

// =============================================================================
// Free functions
// =============================================================================

/// Build the public surface state for the given hostname.
///
/// Phase 4 stub: all values are hardcoded placeholders. Phase 5 wires real
/// DNS resolution, TLS certificate probes, and external reachability checks.
pub fn build_public_surface(hostname: &str) -> PublicSurfaceState {
    let _ = hostname; // consumed by Phase 5 probes
    PublicSurfaceState {
        // Phase 4 stub: assume DNS resolves for any configured hostname.
        dns_resolves: true, // Phase 4 stub: no probe; assume the configured hostname is resolvable.
        dns_target: Some("127.0.0.1".into()),
        // Phase 4 stub: TLS probe not wired; assume invalid until probed.
        tls_valid: false,
        tls_expires_in_days: None,
        // Phase 4 stub: reachability probe not wired.
        public_reachable: true, // Phase 4 stub: no external probe yet.
    }
}

/// Map a direction string (e.g., from a config or wire message) to
/// [`FederationDirection`].
///
/// Unrecognised strings default to [`FederationDirection::OutboundOnly`].
pub fn parse_direction(s: &str) -> FederationDirection {
    match s {
        "bidirectional" => FederationDirection::Bidirectional,
        "inbound_only" => FederationDirection::InboundOnly,
        _ => FederationDirection::OutboundOnly,
    }
}

/// Extract a displayable hostname from a URL string without pulling in the
/// `url` crate.
///
/// Strips the scheme prefix, then takes everything before the first `/`.
/// Examples:
/// - `"https://parish.elohim.host"` → `"parish.elohim.host"`
/// - `"https://parish.elohim.host/path"` → `"parish.elohim.host"`
/// - `"parish.elohim.host"` → `"parish.elohim.host"`
///
/// Best-effort host extraction from a peer URL. Trims scheme + path; does NOT
/// strip ports or handle IPv6 brackets. Peer URLs are expected to be
/// well-formed and host-only or host:port for display.
pub fn parse_hostname_from_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

/// Derive the doorway hostname from the `DOORWAY_HOSTNAME` env var, falling
/// back to `"localhost"` when not set.
fn derive_doorway_hostname() -> String {
    std::env::var("DOORWAY_HOSTNAME").unwrap_or_else(|_| "localhost".into())
}
