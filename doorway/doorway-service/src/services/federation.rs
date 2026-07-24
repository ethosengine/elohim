//! Federation Service
//!
//! Core federation engine for doorway-to-doorway cooperation:
//! - DHT registration via infrastructure DNA
//! - Periodic heartbeat reporting
//! - Cross-doorway content fetching via DID resolution
//! - Runtime-mutable peer URL list for admin-managed federation topology
//!
//! This is the capstone of the 5-stage agency model, enabling community
//! doorway stewards to register, publish, and participate in the network.
//!
//! # Doorway Lifecycle & Steward Failure (Future Work)
//!
//! Doorway operators are **custodians, not owners**. Hosted humans own their
//! identity (Holochain agent keys) and must never be stranded by steward
//! failure. The following scenarios must be handled:
//!
//! - **Steward death/incapacitation**: Dead-man's switch — if heartbeats stop
//!   for N days, federation peers treat the doorway as implicitly draining.
//! - **Capture/compromise**: Federation peers can flag a doorway as compromised,
//!   triggering automatic migration warnings to hosted humans.
//! - **Voluntary shutdown (drain protocol)**: Steward signals "shuttering" to
//!   the federation. Peer doorways absorb hosted humans. Humans who haven't
//!   exported keys get escalating urgency warnings with a deadline.
//! - **Admin capability constraints**: Admin APIs must never allow actions that
//!   strand humans without recourse. All admin mutations are runtime-only
//!   (reset on restart) as a safety measure.
//!
//! The human-scale P2P network (not the federation itself) is the ground truth.
//! Doorways serve humans; humans do not belong to doorways.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::config::Args;
use crate::server::AppState;
use crate::services::did_resolver::DIDResolver;
use crate::services::zome_caller::ZomeCaller;

// =============================================================================
// Configuration
// =============================================================================

/// Federation configuration — derived from CLI args
#[derive(Debug, Clone)]
pub struct FederationConfig {
    /// Whether federation is enabled (requires doorway_id + doorway_url)
    pub enabled: bool,
    /// Unique doorway identifier (e.g., "alpha-elohim-host")
    pub doorway_id: String,
    /// Public URL of this doorway (e.g., "https://alpha.elohim.host")
    pub doorway_url: String,
    /// Pkarr-shaped, signed candidate addresses advertised by this doorway.
    pub endpoints: Vec<infrastructure_types::DoorwayEndpoint>,
    /// Geographic region for routing
    pub region: Option<String>,
    /// Heartbeat interval in seconds (default: 60)
    pub heartbeat_interval_secs: u64,
    /// Role name in the hApp for infrastructure DNA
    pub infrastructure_role: String,
    /// Zome name within infrastructure DNA
    pub zome_name: String,
}

impl FederationConfig {
    /// Build federation config from CLI args.
    /// Returns None if doorway_id or doorway_url not configured (federation disabled).
    pub fn from_args(args: &Args) -> Option<Self> {
        let doorway_id = args.doorway_id.as_ref()?;
        let doorway_url = args.doorway_url.as_ref()?;
        let ttl_secs = args.doorway_endpoint_ttl_secs;
        let mut endpoints = vec![infrastructure_types::DoorwayEndpoint {
            service: "gateway".to_string(),
            url: doorway_url.clone(),
            priority: 0,
            ttl_secs,
        }];
        for (index, url) in args.doorway_urls.iter().enumerate() {
            if url != doorway_url && !endpoints.iter().any(|endpoint| endpoint.url == *url) {
                endpoints.push(infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: url.clone(),
                    priority: u16::try_from(index + 1).unwrap_or(u16::MAX),
                    ttl_secs,
                });
            }
        }
        if let Some(url) = &args.bootstrap_url {
            endpoints.push(infrastructure_types::DoorwayEndpoint {
                service: "bootstrap".to_string(),
                url: url.clone(),
                priority: 0,
                ttl_secs,
            });
        }
        if let Some(url) = &args.signal_url {
            endpoints.push(infrastructure_types::DoorwayEndpoint {
                service: "signal".to_string(),
                url: url.clone(),
                priority: 0,
                ttl_secs,
            });
        }

        Some(Self {
            enabled: true,
            doorway_id: doorway_id.clone(),
            doorway_url: doorway_url.clone(),
            endpoints,
            region: args.region.clone(),
            heartbeat_interval_secs: 60,
            infrastructure_role: "infrastructure".to_string(),
            zome_name: "infrastructure".to_string(),
        })
    }
}

// =============================================================================
// Zome Input/Output Types (shared with infrastructure coordinator zome)
// =============================================================================

pub use infrastructure_types::{
    DoorwayOutput, DoorwayRegistration, FindPublishersInput, RecordHealthAttestationInput,
    RegisterDoorwayInput,
};

/// Federation error types
#[derive(Debug)]
pub enum FederationError {
    /// Zome call failed
    ZomeCallFailed(String),
    /// DID resolution failed
    DIDResolutionFailed(String),
    /// No publishers found for content
    NoPublishers(String),
    /// Remote doorway fetch failed
    RemoteFetchFailed(String),
}

impl std::fmt::Display for FederationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FederationError::ZomeCallFailed(e) => write!(f, "Zome call failed: {e}"),
            FederationError::DIDResolutionFailed(e) => write!(f, "DID resolution failed: {e}"),
            FederationError::NoPublishers(h) => write!(f, "No publishers for content: {h}"),
            FederationError::RemoteFetchFailed(e) => write!(f, "Remote fetch failed: {e}"),
        }
    }
}

// =============================================================================
// Registration
// =============================================================================

/// Register this doorway in the infrastructure DNA's DHT.
///
/// Called at startup. On "already exists" error, falls back to update_doorway
/// for idempotent restarts.
pub async fn register_doorway_in_dht(
    config: &FederationConfig,
    zome_caller: &ZomeCaller,
    capabilities: Vec<String>,
) -> Result<(), String> {
    let caps_json = serde_json::to_string(&capabilities)
        .map_err(|e| format!("Failed to serialize capabilities: {e}"))?;

    let input = RegisterDoorwayInput {
        id: config.doorway_id.clone(),
        url: config.doorway_url.clone(),
        identity_root: None,
        endpoints: config.endpoints.clone(),
        capabilities_json: caps_json,
        reach: "public".to_string(),
        region: config.region.clone(),
        bandwidth_mbps: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    info!(
        doorway_id = %config.doorway_id,
        doorway_url = %config.doorway_url,
        "Registering doorway in infrastructure DHT"
    );

    match zome_caller
        .call::<RegisterDoorwayInput, DoorwayOutput>(
            &config.infrastructure_role,
            &config.zome_name,
            "register_doorway",
            &input,
        )
        .await
    {
        Ok(_output) => {
            info!(
                doorway_id = %config.doorway_id,
                "Doorway registered in DHT successfully"
            );
            Ok(())
        }
        Err(e) if e.contains("already exists") => {
            // Doorway already registered — update instead (idempotent startup)
            info!(
                doorway_id = %config.doorway_id,
                "Doorway already registered, updating..."
            );
            match zome_caller
                .call::<RegisterDoorwayInput, DoorwayOutput>(
                    &config.infrastructure_role,
                    &config.zome_name,
                    "update_doorway",
                    &input,
                )
                .await
            {
                Ok(_) => {
                    info!("Doorway registration updated successfully");
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to update doorway registration: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            warn!("Failed to register doorway in DHT: {}", e);
            Err(e)
        }
    }
}

// =============================================================================
// Heartbeat
// =============================================================================

/// Spawn periodic peer-health-probe task (every heartbeat_interval_secs).
///
/// On every Nth tick it probes cached federation peers' `/health` endpoints and
/// records peer-witnessed health attestations via the infrastructure zome.
/// Logs warnings on failure but does not crash.
///
/// NOTE: This task no longer reports a *self*-heartbeat to the conductor. The
/// `record_heartbeat` coordinator fn was retired from the infrastructure DNA
/// (observation-event-layer spec `2026-05-11-observation-event-layer-design.md`
/// §10 Stage 6) — doorway self-liveness now flows through
/// `infrastructure:doorway-heartbeat` observations on the Track 2 observation
/// substrate (not yet wired here; Stages 4/7 of that spec). Calling the removed
/// fn every interval failed unconditionally, and a zome-call `Err` clears the
/// conductor connection (`zome_caller::call_zome`), so the dead call drove a
/// reconnect-churn loop that wedged the gateway under load (2026-06-13 freeze).
/// The peer-health-probe path below uses `record_health_attestation`, which
/// still exists (bridges to elohim `issue_attestation`), so it is retained.
pub fn spawn_heartbeat_task(
    config: FederationConfig,
    zome_caller: Arc<ZomeCaller>,
    state: Arc<AppState>,
) -> JoinHandle<()> {
    let interval = std::time::Duration::from_secs(config.heartbeat_interval_secs);

    tokio::spawn(async move {
        info!(
            doorway_id = %config.doorway_id,
            interval_secs = config.heartbeat_interval_secs,
            "Federation peer-health-probe task started"
        );

        let mut probe_counter: u32 = 0;
        let probe_interval: u32 = 5; // Every 5th interval (~5 minutes)
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        loop {
            tokio::time::sleep(interval).await;

            // Peer health probing — every Nth tick
            probe_counter += 1;
            if probe_counter >= probe_interval {
                probe_counter = 0;

                let cached_peers = get_cached_peers(&state.peer_cache).await;
                for peer in &cached_peers {
                    let probe_start = std::time::Instant::now();
                    let health_url = format!("{}/health", peer.url.trim_end_matches('/'));

                    let (observed_status, response_time_ms, conductor_healthy) =
                        match http_client.get(&health_url).send().await {
                            Ok(resp) => {
                                let elapsed =
                                    probe_start.elapsed().as_millis().min(u32::MAX as u128) as u32;
                                if resp.status().is_success() {
                                    let conductor_ok =
                                        resp.json::<serde_json::Value>().await.ok().and_then(|v| {
                                            v.get("conductor")?.get("connected")?.as_bool()
                                        });
                                    let status = if conductor_ok == Some(true) {
                                        "online"
                                    } else {
                                        "degraded"
                                    };
                                    (status.to_string(), Some(elapsed), conductor_ok)
                                } else {
                                    ("degraded".to_string(), Some(elapsed), None)
                                }
                            }
                            Err(_) => ("unreachable".to_string(), None, None),
                        };

                    let attestation_input = RecordHealthAttestationInput {
                        attestor_doorway_id: config.doorway_id.clone(),
                        subject_doorway_id: peer.id.clone(),
                        observed_status: observed_status.clone(),
                        response_time_ms,
                        conductor_healthy,
                    };

                    match rmp_serde::to_vec(&attestation_input) {
                        Ok(payload) => {
                            match zome_caller
                                .call_zome(
                                    &config.infrastructure_role,
                                    &config.zome_name,
                                    "record_health_attestation",
                                    payload,
                                )
                                .await
                            {
                                Ok(_) => {
                                    debug!(
                                        peer = %peer.id,
                                        status = %observed_status,
                                        "Health attestation recorded"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        peer = %peer.id,
                                        error = %e,
                                        "Failed to record health attestation"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to serialize attestation input");
                        }
                    }
                }
            }
        }
    })
}

// =============================================================================
// Cross-Doorway Content Fetch
// =============================================================================

/// Fetch content from a remote doorway via DHT publisher discovery + DID resolution.
///
/// Called by DeliveryRelay as final fallback tier when local storage returns 404.
///
/// Flow:
/// 1. Query infrastructure DHT for publishers of this content hash
/// 2. For each publisher, extract doorway URL from ContentServer endpoints
/// 3. Resolve doorway's DID document via DIDResolver
/// 4. Extract ElohimBlobStore service endpoint
/// 5. Fetch blob via HTTP GET
/// 6. Add X-Federation-Hop header to prevent infinite loops
pub async fn fetch_from_remote_doorway(
    content_hash: &str,
    zome_caller: &ZomeCaller,
    did_resolver: &DIDResolver,
    config: &FederationConfig,
) -> Result<Vec<u8>, FederationError> {
    debug!(
        content_hash = %content_hash,
        "Searching for remote publishers via infrastructure DHT"
    );

    // Step 1: Find publishers in DHT
    let input = FindPublishersInput {
        content_hash: content_hash.to_string(),
        capability: Some("blob".to_string()),
        prefer_region: config.region.clone(),
        limit: Some(5),
        online_only: Some(true),
    };

    let publishers_result = zome_caller
        .call_zome(
            &config.infrastructure_role,
            &config.zome_name,
            "find_publishers",
            rmp_serde::to_vec(&input)
                .map_err(|e| FederationError::ZomeCallFailed(e.to_string()))?,
        )
        .await
        .map_err(FederationError::ZomeCallFailed)?;

    // Parse publishers response
    #[derive(Deserialize)]
    struct FindPublishersOutput {
        #[allow(dead_code)]
        content_hash: String,
        publishers: Vec<PublisherInfo>,
    }
    #[derive(Deserialize)]
    struct PublisherInfo {
        #[allow(dead_code)]
        action_hash: Vec<u8>,
        server: ServerInfo,
    }
    #[derive(Deserialize)]
    struct ServerInfo {
        endpoints: Vec<EndpointInfo>,
        #[allow(dead_code)]
        online: bool,
        #[allow(dead_code)]
        priority: u8,
        #[allow(dead_code)]
        region: Option<String>,
    }
    #[derive(Deserialize)]
    struct EndpointInfo {
        url: String,
        #[allow(dead_code)]
        protocol: String,
    }

    let output: FindPublishersOutput = rmp_serde::from_slice(&publishers_result)
        .map_err(|e| FederationError::ZomeCallFailed(format!("Failed to parse publishers: {e}")))?;

    if output.publishers.is_empty() {
        return Err(FederationError::NoPublishers(content_hash.to_string()));
    }

    // Step 2-5: Try each publisher
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    for publisher in &output.publishers {
        for endpoint in &publisher.server.endpoints {
            // Try direct endpoint fetch first
            let blob_url = format!("{}/{}", endpoint.url.trim_end_matches('/'), content_hash);

            debug!(
                blob_url = %blob_url,
                "Attempting federation fetch from remote publisher"
            );

            match http_client
                .get(&blob_url)
                .header("X-Federation-Hop", "1")
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(bytes) => {
                        info!(
                            content_hash = %content_hash,
                            source = %endpoint.url,
                            size = bytes.len(),
                            "Federation fetch successful"
                        );
                        return Ok(bytes.to_vec());
                    }
                    Err(e) => {
                        warn!("Failed to read response body from {}: {}", blob_url, e);
                        continue;
                    }
                },
                Ok(resp) => {
                    debug!(
                        status = %resp.status(),
                        url = %blob_url,
                        "Remote publisher returned non-success"
                    );
                    continue;
                }
                Err(e) => {
                    debug!(
                        error = %e,
                        url = %blob_url,
                        "Failed to reach remote publisher"
                    );
                    continue;
                }
            }
        }

        // If direct endpoint failed, try DID resolution for the doorway URL
        // Extract domain from the first endpoint URL to construct a DID
        if let Some(endpoint) = publisher.server.endpoints.first() {
            if let Some(domain) = extract_domain_from_url(&endpoint.url) {
                let did = format!("did:web:{domain}");
                match did_resolver.resolve(&did).await {
                    Ok(doc) => {
                        // Find ElohimBlobStore service endpoint
                        if let Some(blob_service) = doc
                            .service
                            .iter()
                            .find(|s| s.service_type == "ElohimBlobStore")
                        {
                            let blob_url = format!(
                                "{}/{}",
                                blob_service.service_endpoint.trim_end_matches('/'),
                                content_hash
                            );

                            match http_client
                                .get(&blob_url)
                                .header("X-Federation-Hop", "1")
                                .send()
                                .await
                            {
                                Ok(resp) if resp.status().is_success() => {
                                    if let Ok(bytes) = resp.bytes().await {
                                        info!(
                                            content_hash = %content_hash,
                                            source = %blob_url,
                                            "Federation fetch via DID resolution successful"
                                        );
                                        return Ok(bytes.to_vec());
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                    Err(e) => {
                        debug!(did = %did, error = ?e, "DID resolution failed for remote publisher");
                    }
                }
            }
        }
    }

    Err(FederationError::RemoteFetchFailed(format!(
        "All {} publishers failed for content {}",
        output.publishers.len(),
        content_hash
    )))
}

/// Extract domain from a URL (e.g., "https://alpha.elohim.host/store" -> "alpha.elohim.host")
fn extract_domain_from_url(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let domain = without_scheme.split('/').next()?.split(':').next()?;

    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

// =============================================================================
// Doorway List (for federation routes)
// =============================================================================

/// Get all registered doorways from the DHT.
/// Used by the /api/v1/federation/doorways endpoint.
///
/// DHT-native federation discovery: enumerates every registered doorway via the
/// infrastructure zome's `get_all_doorways` list-all anchor. This rides the
/// conductor DHT gossip plane (WAN-NAT-correct) — a registration written through
/// any doorway's conductor gossips to every peer's DHT, so each doorway sees the
/// whole federation, not just itself. The FEDERATION_PEERS HTTP loop is a bootstrap
/// fallback; this is the canonical path.
pub async fn get_all_doorways(
    zome_caller: &ZomeCaller,
    config: &FederationConfig,
) -> Result<Vec<DoorwayRegistration>, String> {
    match zome_caller
        .call::<(), Vec<DoorwayOutput>>(
            &config.infrastructure_role,
            &config.zome_name,
            "get_all_doorways",
            &(),
        )
        .await
    {
        Ok(outputs) => Ok(outputs.into_iter().map(|o| o.doorway).collect()),
        Err(e) => {
            warn!("Failed to query doorways from DHT: {}", e);
            Err(e)
        }
    }
}

// =============================================================================
// Peer Discovery (HTTP-based federation)
// =============================================================================

/// Cached peer doorway info from HTTP federation queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDoorway {
    pub id: String,
    pub url: String,
    pub region: Option<String>,
    pub capabilities: Vec<String>,
    pub source_peer: String,
}

/// Shared cache of known peer doorways, refreshed periodically
pub type PeerCache = Arc<tokio::sync::RwLock<Vec<PeerDoorway>>>;

/// Create a new empty peer cache
pub fn new_peer_cache() -> PeerCache {
    Arc::new(tokio::sync::RwLock::new(Vec::new()))
}

// =============================================================================
// Cross-Edge Coherence Probe (F-COHERENCE)
// =============================================================================

/// Probe results from sibling doorways' `/api/v1/federation/coherence`.
///
/// Cat-C node-local read-model (transport-only here; the DATA type +
/// comparison logic live in `crate::routes::coherence`). Refreshed inside the
/// existing peer-discovery loop — observability-only, NEVER reconciled.
pub type PeerCoherenceCache =
    Arc<tokio::sync::RwLock<Vec<crate::routes::coherence::PeerCoherence>>>;

/// Create a new empty peer-coherence cache.
pub fn new_peer_coherence_cache() -> PeerCoherenceCache {
    Arc::new(tokio::sync::RwLock::new(Vec::new()))
}

/// Fetch one peer's `CoherenceManifest`, returning a TRI-STATE
/// `(reachable, Option<CoherenceManifest>)` so a 200-with-garbage-body is
/// distinguished from a down / not-yet-deployed peer (fix 2):
///
///   - transport error, 404, or ANY non-200 (incl. 5xx during a rolling deploy)
///     → `(false, None)`: down or not-yet-deployed; never fires a false
///     divergence alarm while a sibling is still deploying.
///   - HTTP 200 but the body fails to deserialize → `(true, None)`: a peer
///     serving a 200 garbage/incompatible body IS a real (mixed-version)
///     divergence to surface, NOT "still deploying".
///   - HTTP 200 + a parseable manifest → `(true, Some(manifest))`.
///
/// The 5xx → `(false, None)` mapping is deliberate: a 503 during a rolling
/// deploy mapping to `reachable=true` would manufacture exactly the deploy-noise
/// WARN this feature exists to avoid. ONLY `200 && body-parse-fails` is the
/// reachable-but-divergent case.
///
/// NO per-request timeout here — the `coherence_client` already carries a
/// client-level 5s timeout (fix 10: one timeout, not two).
///
/// (X-COH-DEF: retry cadence DEFERS to `elohim_compute::backoff::jittered` when
/// present — until then the existing loop interval is the cadence.)
async fn fetch_peer_coherence(
    client: &reqwest::Client,
    peer_url: &str,
) -> (bool, Option<crate::routes::coherence::CoherenceManifest>) {
    let url = format!(
        "{}/api/v1/federation/coherence",
        peer_url.trim_end_matches('/')
    );
    match client.get(&url).send().await {
        // 200 OK → reachable; body may or may not deserialize.
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(manifest) => (true, Some(manifest)),
            // 200 but the body is garbage/incompatible → reachable, no manifest.
            // This IS a divergence (mixed version), so reachable=true is correct.
            Err(_) => (true, None),
        },
        // Any non-200 (404 not-yet-deployed, 5xx rolling deploy) → not reachable.
        _ => (false, None),
    }
}

/// Probe each known peer's coherence manifest CONCURRENTLY, compare each to OUR
/// self-manifest, store the per-peer verdicts in `cache`, and emit a structured
/// WARN alarm ONLY for peers whose divergence is NEW or CHANGED since the prior
/// tick (edge-triggered — fix 5).
///
/// Fix 1 — concurrency: the per-peer probes run via `join_all`, so a tick's
/// latency is ~max(timeout), not the sum of N serial 5s probes. This keeps the
/// 60s discovery cadence (which also feeds `PeerCache` / `/p2p-peers`) from
/// drifting when several siblings are slow.
///
/// Fix 6 — empty `peer_url`s are skipped (an empty url builds a relative URL
/// reqwest rejects, swallowed as `(false, None)` forever).
///
/// Fix 5 — the WARN is edge-triggered: we read the PRIOR cache snapshot before
/// overwriting it and warn only on newly/changed-divergent peers
/// (`divergences_to_warn`). This kills the per-tick log-spam AND makes
/// `coherence_cache` a real READER, not a write-only dead store.
///
/// `self_manifest` is supplied by the caller each tick (the caller gen-gates the
/// recompute). `peers` is a snapshot of the federation peer cache — `(id, url)`
/// pairs to probe.
pub async fn refresh_coherence(
    self_manifest: &crate::routes::coherence::CoherenceManifest,
    peers: &[(String, String)],
    client: &reqwest::Client,
    cache: &PeerCoherenceCache,
) {
    use crate::routes::coherence::{compare_to_peer, divergences_to_warn};

    // Fix 1 + 6: probe every non-empty-url peer CONCURRENTLY; each future does
    // fetch + compare and yields its `PeerCoherence` verdict.
    let probes = peers
        .iter()
        .filter(|(_, peer_url)| !peer_url.trim().is_empty())
        .map(|(peer_id, peer_url)| async move {
            let (reachable, peer_manifest) = fetch_peer_coherence(client, peer_url).await;
            compare_to_peer(self_manifest, peer_id, reachable, peer_manifest.as_ref())
        });
    let results: Vec<crate::routes::coherence::PeerCoherence> =
        futures::future::join_all(probes).await;

    // Fix 5 — edge-triggered divergence ALARM. Read the PRIOR verdicts BEFORE
    // overwriting the cache, and warn only on the newly/changed-divergent subset.
    // Make the degraded state LOUD (FallbackOutcome precedent: the EPR-router
    // degraded state hid for days at DEBUG). Naming BOTH doorway_ids + the build
    // SHAs lets the operator instantly tell content-skew (digests differ, builds
    // equal) from deploy-skew (builds differ — the actual "two EPR heads" symptom).
    let prior = cache.read().await.clone();
    for pc in divergences_to_warn(&prior, &results) {
        warn!(
            self_doorway = %self_manifest.doorway_id,
            peer_doorway = %pc.doorway_id,
            self_digest = %self_manifest.digest,
            peer_digest = ?pc.digest,
            self_build = ?self_manifest.build_id,
            peer_build = ?pc.build_id,
            divergent_paths = ?pc.divergent_paths,
            "CROSS-EDGE EPR HEAD DIVERGENCE — two doorways serving different heads"
        );
    }

    let mut cache_write = cache.write().await;
    *cache_write = results;
}

/// Shared mutable list of federation peer URLs.
/// Seeded from FEDERATION_PEERS env var, mutable via admin API at runtime.
pub type PeerUrlList = Arc<tokio::sync::RwLock<Vec<String>>>;

/// Create a new peer URL list from initial seeds
pub fn new_peer_url_list(initial: Vec<String>) -> PeerUrlList {
    Arc::new(tokio::sync::RwLock::new(initial))
}

/// Add a peer URL to the list. Returns true if added (not a duplicate).
pub async fn add_peer_url(list: &PeerUrlList, url: String) -> bool {
    let mut urls = list.write().await;
    if urls.contains(&url) {
        false
    } else {
        urls.push(url);
        true
    }
}

/// Remove a peer URL from the list. Returns true if removed.
pub async fn remove_peer_url(list: &PeerUrlList, url: &str) -> bool {
    let mut urls = list.write().await;
    let len_before = urls.len();
    urls.retain(|u| u != url);
    urls.len() < len_before
}

/// Get a snapshot of the current peer URL list.
pub async fn get_peer_urls(list: &PeerUrlList) -> Vec<String> {
    list.read().await.clone()
}

/// Fetch federation info from a single peer doorway.
/// Queries `{peer_url}/api/v1/federation/doorways` and returns parsed doorways.
async fn fetch_single_peer(client: &reqwest::Client, peer_url: &str) -> Vec<PeerDoorway> {
    let url = format!(
        "{}/api/v1/federation/doorways",
        peer_url.trim_end_matches('/')
    );

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // Parse the FederationDoorwaysResponse shape
            #[derive(Deserialize)]
            struct PeerResponse {
                doorways: Vec<PeerEntry>,
            }
            #[derive(Deserialize)]
            struct PeerEntry {
                id: String,
                url: String,
                region: Option<String>,
                capabilities: Vec<String>,
                #[allow(dead_code)]
                status: Option<String>,
            }

            match resp.json::<PeerResponse>().await {
                Ok(parsed) => parsed
                    .doorways
                    .into_iter()
                    .map(|d| PeerDoorway {
                        id: d.id,
                        url: d.url,
                        region: d.region,
                        capabilities: d.capabilities,
                        source_peer: peer_url.to_string(),
                    })
                    .collect(),
                Err(e) => {
                    warn!(peer = %peer_url, error = %e, "Failed to parse peer federation response");
                    Vec::new()
                }
            }
        }
        Ok(resp) => {
            debug!(peer = %peer_url, status = %resp.status(), "Peer federation query returned non-success");
            Vec::new()
        }
        Err(e) => {
            debug!(peer = %peer_url, error = %e, "Failed to reach peer doorway");
            Vec::new()
        }
    }
}

/// Refresh the peer cache by querying all configured peers.
/// Deduplicates by doorway id — if multiple peers report the same doorway,
/// the first occurrence wins.
pub async fn refresh_peer_cache(peer_urls: &[String], self_id: Option<&str>, cache: &PeerCache) {
    if peer_urls.is_empty() {
        // Fix 12: no configured peers → clear the cache rather than stranding a
        // stale set. A stale entry left here would be a phantom peer the
        // coherence probe alarms against even after federation was emptied.
        let mut cache_write = cache.write().await;
        cache_write.clear();
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut all_peers = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Exclude self from results
    if let Some(id) = self_id {
        seen_ids.insert(id.to_string());
    }

    for peer_url in peer_urls {
        let discovered = fetch_single_peer(&client, peer_url).await;
        for peer in discovered {
            if seen_ids.insert(peer.id.clone()) {
                all_peers.push(peer);
            }
        }
    }

    let count = all_peers.len();
    {
        let mut cache_write = cache.write().await;
        *cache_write = all_peers;
    }

    if count > 0 {
        info!(
            peers = count,
            sources = peer_urls.len(),
            "Federation peer cache refreshed"
        );
    }
}

/// Spawn a background task that periodically refreshes the peer cache.
/// Initial fetch happens after `initial_delay`, then every `interval`.
/// Reads peer URLs from the shared mutable list on each iteration.
///
/// Also runs the F-COHERENCE cross-edge probe on each tick (additive): it
/// recomputes THIS edge's self-manifest from `epr_router` and compares it
/// against every discovered peer's coherence manifest, storing the verdicts in
/// `coherence_cache` and alarming on divergence. Detection-only — never
/// reconciles.
#[allow(clippy::too_many_arguments)]
pub fn spawn_peer_discovery_task(
    peer_urls: PeerUrlList,
    self_id: Option<String>,
    cache: PeerCache,
    epr_router: Arc<crate::projection::EprRouter>,
    coherence_cache: PeerCoherenceCache,
    initial_delay: std::time::Duration,
    interval: std::time::Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Wait for startup to settle (peer doorways may still be booting)
        tokio::time::sleep(initial_delay).await;

        // BuildInfo is process-constant — construct once, reuse the commit SHA
        // for every per-tick self-manifest.
        let build = elohim_compute::BuildInfo::new("elohim-doorway");
        // Fix 3: cross-edge coherence is meaningless without a self-identity. If
        // `doorway_id` is unknown, the self-exclusion in `refresh_peer_cache`
        // (keyed on the real id) can't drop a self-echo, so the doorway would
        // coherence-probe ITSELF. `Some(id)` only when we have a real id (never
        // the "unknown" fallback).
        let coherence_self_id: Option<String> = match self_id.as_deref() {
            Some(id) if !id.is_empty() && id != "unknown" => Some(id.to_string()),
            _ => None,
        };
        let coherence_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        // Fix 8 — generation-gate cache: last `(generation, CoherenceManifest)`.
        // The self-manifest is only re-minted when the router generation changed.
        let mut cached_self_manifest: Option<(u64, crate::routes::coherence::CoherenceManifest)> =
            None;

        loop {
            // ALWAYS refresh the peer cache — it feeds /p2p-peers and the 60s
            // discovery cadence, independent of coherence. (`self_id`, including
            // the "unknown" fallback, is still used for self-exclusion here.)
            let urls = peer_urls.read().await.clone();
            refresh_peer_cache(&urls, self_id.as_deref(), &cache).await;

            // F-COHERENCE cross-edge probe — runs only when we have a real
            // self-identity (fix 3).
            if let Some(coherence_id) = coherence_self_id.as_deref() {
                // Fix 8: recompute the self-manifest only when the generation moved.
                let gen = epr_router.generation();
                if crate::routes::coherence::should_recompute_self_manifest(
                    cached_self_manifest.as_ref(),
                    gen,
                ) {
                    let minted = crate::routes::coherence::router_fingerprint(
                        epr_router.as_ref(),
                        coherence_id,
                        Some(&build.commit),
                    );
                    cached_self_manifest = Some((gen, minted));
                }
                // Safe: just set above when None.
                let self_manifest = &cached_self_manifest.as_ref().unwrap().1;

                // Fix 4: before the EPR router populates (boot / pre-first
                // projection) the self-manifest has zero heads → its digest is the
                // empty-set CID, which mismatches any populated peer → a spurious
                // divergence WARN. Skip the probe/compare/WARN entirely this tick.
                if !self_manifest.heads.is_empty() {
                    let peers: Vec<(String, String)> = get_cached_peers(&cache)
                        .await
                        .into_iter()
                        .map(|p| (p.id, p.url))
                        .collect();
                    refresh_coherence(self_manifest, &peers, &coherence_client, &coherence_cache)
                        .await;
                }
            }

            tokio::time::sleep(interval).await;
        }
    })
}

/// Get the current list of known peers from the cache
pub async fn get_cached_peers(cache: &PeerCache) -> Vec<PeerDoorway> {
    cache.read().await.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_federation_config_from_args_none() {
        let args = Args::parse_from(["doorway"]);
        let config = FederationConfig::from_args(&args);
        assert!(
            config.is_none(),
            "Should be None when doorway_id/url not set"
        );
    }

    #[test]
    fn test_federation_config_from_args_some() {
        let args = Args::parse_from([
            "doorway",
            "--doorway-id",
            "alpha-elohim-host",
            "--doorway-url",
            "https://alpha.elohim.host",
            "--doorway-urls",
            "https://alpha-backup.example,https://alpha.elohim.host",
            "--doorway-endpoint-ttl-secs",
            "120",
            "--bootstrap-url",
            "https://bootstrap.elohim.host",
            "--signal-url",
            "wss://signal.elohim.host",
            "--region",
            "us-west",
        ]);
        let config = FederationConfig::from_args(&args).unwrap();
        assert_eq!(config.doorway_id, "alpha-elohim-host");
        assert_eq!(config.doorway_url, "https://alpha.elohim.host");
        assert_eq!(config.region, Some("us-west".to_string()));
        assert_eq!(config.heartbeat_interval_secs, 60);
        assert_eq!(
            config.endpoints,
            vec![
                infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: "https://alpha.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "gateway".to_string(),
                    url: "https://alpha-backup.example".to_string(),
                    priority: 1,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "bootstrap".to_string(),
                    url: "https://bootstrap.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
                infrastructure_types::DoorwayEndpoint {
                    service: "signal".to_string(),
                    url: "wss://signal.elohim.host".to_string(),
                    priority: 0,
                    ttl_secs: 120,
                },
            ]
        );
    }

    #[test]
    fn test_extract_domain_from_url() {
        assert_eq!(
            extract_domain_from_url("https://alpha.elohim.host/store"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain_from_url("http://localhost:8080/api"),
            Some("localhost".to_string())
        );
        assert_eq!(extract_domain_from_url(""), None);
    }

    #[test]
    fn test_register_doorway_input_serialization() {
        let input = RegisterDoorwayInput {
            id: "test-doorway".to_string(),
            url: "https://test.elohim.host".to_string(),
            identity_root: None,
            endpoints: vec![infrastructure_types::DoorwayEndpoint {
                service: "gateway".to_string(),
                url: "https://test.elohim.host".to_string(),
                priority: 0,
                ttl_secs: 300,
            }],
            capabilities_json: r#"["gateway","bootstrap"]"#.to_string(),
            reach: "public".to_string(),
            region: Some("us-west".to_string()),
            bandwidth_mbps: None,
            version: "0.1.0".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: RegisterDoorwayInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-doorway");
    }

    // ── F-COHERENCE: tri-state fetch + edge-trigger + empty-clear ─────────────
    mod coherence_probe {
        use super::super::{
            fetch_peer_coherence, new_peer_cache, new_peer_coherence_cache, refresh_coherence,
            refresh_peer_cache, PeerDoorway,
        };
        use crate::projection::EprRouter;
        use crate::routes::coherence::{router_fingerprint, CoherenceManifest, EprHeadFingerprint};
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        /// Build this edge's self-manifest from a head set (real digest path).
        fn self_manifest(heads: &[(&str, &str)]) -> CoherenceManifest {
            let router = EprRouter::new();
            router.replace_all(
                heads
                    .iter()
                    .map(|(p, e)| crate::routes::coherence::sample_projection(p, e))
                    .collect(),
            );
            router_fingerprint(&router, "alpha", Some("build-alpha"))
        }

        /// Serialize a CoherenceManifest body for a mock peer to return.
        fn manifest_body(id: &str, heads: &[(&str, &str)]) -> CoherenceManifest {
            let mut hv: Vec<EprHeadFingerprint> = heads
                .iter()
                .map(|(p, e)| EprHeadFingerprint {
                    url_path: (*p).to_string(),
                    epr_id: (*e).to_string(),
                })
                .collect();
            let digest = crate::routes::coherence::mint_head_set_digest(&mut hv);
            CoherenceManifest {
                doorway_id: id.to_string(),
                generation: 1,
                heads: hv,
                digest,
                build_id: Some("build-peer".to_string()),
            }
        }

        async fn mount_coherence(server: &MockServer, body: &CoherenceManifest) {
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(server)
                .await;
        }

        #[tokio::test]
        async fn fetch_404_is_unreachable() {
            let server = MockServer::start().await;
            // No mock mounted → 404 on the coherence path.
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(!reachable, "404 → not reachable (not-yet-deployed)");
            assert!(manifest.is_none());
        }

        #[tokio::test]
        async fn fetch_transport_error_is_unreachable() {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(200))
                .build()
                .unwrap();
            // Dead port → transport error.
            let (reachable, manifest) =
                fetch_peer_coherence(&client, "http://127.0.0.1:1/never").await;
            assert!(!reachable);
            assert!(manifest.is_none());
        }

        #[tokio::test]
        async fn fetch_200_garbage_body_is_reachable_no_manifest() {
            // Fix 2: a peer that answers 200 with an undeserializable body is a
            // reachable divergence (reachable=true, manifest=None), NOT "down".
            let server = MockServer::start().await;
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(reachable, "200 garbage body IS reachable (a divergence)");
            assert!(manifest.is_none(), "garbage body yields no manifest");
        }

        #[tokio::test]
        async fn fetch_200_5xx_is_unreachable() {
            // A 5xx (rolling-deploy state) must map to not-reachable so it never
            // manufactures deploy-noise WARNs.
            let server = MockServer::start().await;
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/api/v1/federation/coherence"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let (reachable, _m) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(!reachable, "503 → not reachable (deploy state, no alarm)");
        }

        #[tokio::test]
        async fn fetch_200_valid_manifest_is_reachable_with_manifest() {
            let server = MockServer::start().await;
            mount_coherence(&server, &manifest_body("apex", &[("/lamad", "A")])).await;
            let client = reqwest::Client::new();
            let (reachable, manifest) = fetch_peer_coherence(&client, &server.uri()).await;
            assert!(reachable);
            let m = manifest.expect("valid manifest parses");
            assert_eq!(m.doorway_id, "apex");
        }

        #[tokio::test]
        async fn refresh_coherence_writes_verdicts_into_cache() {
            // The cache must become a real reader/writer: a divergent peer lands
            // a verdict with reachable=true, agrees=false.
            let server = MockServer::start().await;
            mount_coherence(&server, &manifest_body("apex", &[("/lamad", "DIFFERENT")])).await;
            let me = self_manifest(&[("/lamad", "MINE")]);
            let cache = new_peer_coherence_cache();
            let client = reqwest::Client::new();
            let peers = vec![("apex".to_string(), server.uri())];

            refresh_coherence(&me, &peers, &client, &cache).await;

            let stored = cache.read().await.clone();
            assert_eq!(stored.len(), 1);
            assert!(stored[0].reachable);
            assert!(!stored[0].agrees, "divergent peer → !agrees");
            // Fix 7: labeled from the manifest's self-report.
            assert_eq!(stored[0].doorway_id, "apex");
        }

        #[tokio::test]
        async fn refresh_coherence_skips_empty_peer_url() {
            // Fix 6: an empty url must be skipped, not produce a dead verdict.
            let me = self_manifest(&[("/lamad", "MINE")]);
            let cache = new_peer_coherence_cache();
            let client = reqwest::Client::new();
            let peers = vec![("ghost".to_string(), "".to_string())];

            refresh_coherence(&me, &peers, &client, &cache).await;

            let stored = cache.read().await.clone();
            assert!(stored.is_empty(), "empty-url peer must not yield a verdict");
        }

        #[tokio::test]
        async fn refresh_peer_cache_clears_on_empty_urls() {
            // Fix 12: an empty peer-url list clears any stale cache entry.
            let cache = new_peer_cache();
            // Seed a stale peer directly.
            cache.write().await.push(PeerDoorway {
                id: "stale".into(),
                url: "https://stale.example".into(),
                region: None,
                capabilities: vec![],
                source_peer: "seed".into(),
            });
            assert_eq!(cache.read().await.len(), 1);

            refresh_peer_cache(&[], Some("alpha"), &cache).await;
            assert!(
                cache.read().await.is_empty(),
                "empty peer-url list must clear the stale cache"
            );
        }
    }
}
