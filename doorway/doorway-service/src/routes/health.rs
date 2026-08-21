//! Health check endpoints
//!
//! Provides Kubernetes-style health probes:
//! - /health, /healthz - Liveness probe (is the service running?)
//! - /ready, /readyz - Readiness probe (is the service ready for traffic?)
//!
//! Liveness probes return 200 if doorway is running, regardless of conductor status.
//! Readiness probes return 200 only if at least one worker is connected to conductor,
//! UNLESS dev_mode is enabled (conductor connection is optional in dev mode).
//!
//! Dev mode: Conductor connection is optional. Doorway can operate as a pure
//! HTTP bridge to elohim-storage without direct conductor access. The conductor
//! path is only needed for web-based development (Eclipse Che) scenarios.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

use crate::server::AppState;

/// Health response for seeder compatibility and doorway-picker UI
///
/// The seeder expects this format:
/// - healthy: boolean - overall health status
/// - version: string - service version
/// - cacheEnabled: boolean - whether projection cache is available
/// - cache.enabled: boolean - backwards compat for seeder's data.cache?.enabled
/// - conductor.connected: boolean - whether conductor is available (seeder should check this!)
///
/// The doorway-picker UI expects:
/// - status: 'online' | 'degraded' | 'offline' | 'maintenance'
/// - registrationOpen: boolean - whether new users can register
#[derive(Serialize)]
pub struct HealthResponse {
    /// Overall health status (true if service is running)
    pub healthy: bool,
    /// Doorway status for UI display: 'online', 'degraded', 'offline', 'maintenance'
    pub status: &'static str,
    /// Whether new user registration is open
    #[serde(rename = "registrationOpen")]
    pub registration_open: bool,
    /// Service version
    pub version: &'static str,
    /// Uptime in seconds
    pub uptime: u64,
    /// Whether cache/projection is enabled (top-level for new clients)
    #[serde(rename = "cacheEnabled")]
    pub cache_enabled: bool,
    /// Cache status object (backwards compat for seeder: data.cache?.enabled)
    pub cache: CacheStatus,
    /// Current timestamp
    pub timestamp: String,
    /// Operating mode
    pub mode: String,
    /// Node identifier
    pub node_id: String,
    /// Conductor connection status - IMPORTANT: seeder should check conductor.connected!
    pub conductor: ConductorHealth,
    /// Projection role (writer or reader)
    pub projection: ProjectionRole,
    /// P2P network status (from elohim-storage sidecar). This block is always
    /// present so callers can distinguish unavailable cache data (`stale: true`)
    /// from a missing field.
    pub p2p: P2PHealth,
    /// Backing-aware DHT-participation identity (Tier C — peer-discovery spec).
    #[serde(rename = "dhtBacking")]
    pub dht_backing: DhtBacking,
    /// Whether the SERVING path can actually answer — the per-upstream circuit
    /// breakers this doorway proxies through. See [`ServingHealth`].
    pub serving: ServingHealth,
    /// Whether zome discovery has completed (zome_configs populated)
    #[serde(rename = "discoveryComplete")]
    pub discovery_complete: bool,
    /// Error message if conductor not connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// P2P network health from elohim-storage sidecar
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct P2PHealth {
    /// Whether P2P networking is enabled
    pub enabled: bool,
    /// Number of connected P2P peers
    pub peer_count: usize,
    /// Local peer ID
    pub peer_id: Option<String>,
    /// Projection-reconcile caught-up flag (from storage projectionReconcile).
    /// None when storage's reconcile task is not spawned (block is null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caught_up: Option<bool>,
    /// True only when the reconcile sweep has neither pending nor exhausted
    /// gaps. None until the storage sidecar ships this additive field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converged: Option<bool>,
    /// Count of anchors that diverged during reconcile (from projectionReconcile).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergent_anchor: Option<usize>,
    /// Milliseconds since doorway last observed the storage `/p2p/status`.
    /// Absent when there is no cached snapshot or the lock is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_age_ms: Option<u64>,
    /// Whether this P2P status is too old to treat as current. Always present:
    /// an unavailable snapshot is stale rather than silently omitted.
    pub stale: bool,
}

impl Default for P2PHealth {
    fn default() -> Self {
        Self {
            enabled: false,
            peer_count: 0,
            peer_id: None,
            caught_up: None,
            converged: None,
            divergent_anchor: None,
            observed_age_ms: None,
            stale: true,
        }
    }
}

/// One cached storage P2P status plus the instant at which doorway observed it.
/// The timestamp stays doorway-local; `/health` derives the age at read time.
#[derive(Clone)]
pub struct P2PHealthSnapshot {
    health: P2PHealth,
    observed_at: Instant,
}

impl P2PHealthSnapshot {
    pub fn new(health: P2PHealth) -> Self {
        Self {
            health,
            observed_at: Instant::now(),
        }
    }

    pub fn health(&self) -> &P2PHealth {
        &self.health
    }

    fn with_observed_age(&self) -> P2PHealth {
        let observed_age_ms =
            u64::try_from(self.observed_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        P2PHealth {
            observed_age_ms: Some(observed_age_ms),
            stale: is_stale(Some(observed_age_ms)),
            ..self.health.clone()
        }
    }
}

/// Two sync rounds at the 60s default cadence. A snapshot older than this
/// describes a fleet state that has had two full opportunities to change.
pub const P2P_HEALTH_STALE_AFTER_MS: u64 = 120_000;

pub fn is_stale(observed_age_ms: Option<u64>) -> bool {
    match observed_age_ms {
        Some(age) => age > P2P_HEALTH_STALE_AFTER_MS,
        None => true,
    }
}

/// Backing-aware DHT-participation identity (Tier C — peer-discovery
/// fractal-federation spec, 2026-07-09 §6).
///
/// "Green" trust is DHT-membership-blind without the discovery *backing*: two
/// doorways serving identical DNA hashes read identical yet sit on partitioned
/// DHTs when their bootstrap table or signal relay differs (the adam/matthew
/// diagnosis — identical hashes, different backing). The endpoint URL alone
/// would LIE (two endpoints on one shared backing are coherent), so this block
/// reports the *backing* the doorway can honestly see: the bootstrap store
/// backend + shared table id, and the signal relay id. Correlate with storage
/// `/health` `dhtParticipation.dnaHashes` for the full `dnaHash + backing`
/// identity.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DhtBacking {
    /// Kitsune bootstrap endpoint (necessary but insufficient — two endpoints
    /// on ONE shared table are coherent, so URL alone cannot prove partition).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_url: Option<String>,
    /// Bootstrap store backend as the LIVE store reports it: `"mongo"` = shared
    /// table (genesis-pair islanding healed — F-BOOTSTRAP); `"mem"` = per-pod,
    /// unshared (a partition signal). `None` when bootstrap is disabled here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_backend: Option<&'static str>,
    /// Shared bootstrap table id (mongo DB name, e.g. `elohim-bootstrap`),
    /// surfaced only when the backend is `"mongo"`. Two doorways sharing this
    /// table discover each other's peers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_table: Option<String>,
    /// WebRTC signal relay URL — the relay id. Two doorways with different
    /// `signalUrl` complete WebRTC on SEPARATE relays, so peers that DISCOVER
    /// each other still cannot handshake → partitioned (spec §2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_url: Option<String>,
    /// Whether the signal relay has a shared backend. TRUE iff the mongo
    /// SignalBus is active (selected when BOOTSTRAP_MONGODB_DB is set) —
    /// sibling relays then bridge frames cross-pod. FALSE = per-pod
    /// in-memory only, and relay identity is the URL alone (spec §2).
    pub signal_shared: bool,
}

/// Read the shared bootstrap-table id (mongo DB name) once. This mirrors the
/// value `BootstrapStore::new` selected the mongo backend against at startup;
/// cached in a `OnceLock` to keep the env read off the `/health` hot path.
fn bootstrap_table_id() -> Option<&'static str> {
    static TABLE_ID: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    TABLE_ID
        .get_or_init(|| std::env::var("BOOTSTRAP_MONGODB_DB").ok())
        .as_deref()
}

/// Cache status for backwards compatibility
#[derive(Serialize)]
pub struct CacheStatus {
    /// Whether cache is enabled
    pub enabled: bool,
}

/// Conductor connection health details
#[derive(Serialize)]
pub struct ConductorHealth {
    /// Whether conductor is connected - seeder MUST check this before seeding!
    pub connected: bool,
    /// Number of connected workers
    pub connected_workers: usize,
    /// Total number of workers
    pub total_workers: usize,
    /// Number of conductors in the pool
    pub pool_size: usize,
    /// Per-conductor pools with at least one connected worker
    pub pools_healthy: usize,
    /// Total per-conductor pools (one per conductor in CONDUCTOR_URLS)
    pub pools_total: usize,
}

/// One upstream's breaker as the serving path sees it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServingUpstream {
    /// Storage endpoint this doorway proxies to.
    pub endpoint: String,
    /// "closed" | "half-open" | "open".
    pub circuit: &'static str,
    /// Consecutive recorded failures.
    pub error_streak: u32,
    /// True when this endpoint is shedding calls right now.
    pub shedding: bool,
}

/// Can this doorway actually SERVE?
///
/// WHY THIS BLOCK EXISTS. `healthy` was a hardcoded `true` ("the service is
/// running") and `status` was `"online"` whenever the conductor was connected
/// — and in DEV_MODE `conductor_connected` is forced true regardless. Both
/// background health pollers, meanwhile, reach storage on their OWN private
/// reqwest clients, which are breaker-blind. So on 2026-08-20 elohim.host
/// reported `healthy: true, status: "online"` continuously while every
/// `/db/content/*` read returned 503 `cause=upstream`: nothing in the health
/// response had ever consulted the serving path. This block does.
///
/// It is deliberately BODY-ONLY. `/health` on :8080 is simultaneously the
/// startup, readiness AND liveness probe for the doorway container
/// (`genesis/orchestrator/manifests/doorway/alpha{,-b}.yaml`), so demoting its
/// HTTP status when an upstream sheds would CrashLoop the pod and, via
/// readiness, pull a doorway that can still serve warm shells out of the
/// Service — turning a degraded peer into a dead one, on both doorways at once
/// (they flap independently but overlap). The status-code-bearing signal lives
/// at `/health/serving`, which nothing in k8s probes.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServingHealth {
    /// True when ANY upstream is shedding — the doorway cannot serve that peer
    /// at all. Cheap and fast: a shed costs single-digit milliseconds.
    pub shedding: bool,
    /// True when ANY upstream has a non-zero error streak — the SLOW regime.
    ///
    /// This exists because `shedding` alone is blind for exactly
    /// `UPSTREAM_CIRCUIT_FAIL_THRESHOLD - 1` (= 2) failures, and that blind
    /// window is the expensive one. While the circuit is still Closed each
    /// request rides the full upstream budget — `EPR_DISPATCH_TIMEOUT_SECS`
    /// twice over on `/`, measured at 20.4s — and only the third failure opens
    /// the circuit, after which sheds cost ~5ms. So a shedding-only signal is
    /// honest precisely when the failure is cheap and blind precisely when a
    /// person is staring at a 20-second page load. A non-zero streak means at
    /// least one real upstream timeout just happened; any recorded success
    /// resets it to zero, so a healthy doorway reads 0.
    pub degrading: bool,
    /// Per-upstream breaker state.
    pub upstreams: Vec<ServingUpstream>,
    /// The live freshness policy this doorway is serving under — declared
    /// stage, its provenance, the per-class requirement, and the last-good
    /// pantry's occupancy against its declared ceiling.
    ///
    /// `Option` because `ServingHealth::observe` (the breaker-only form) has no
    /// AppState to read a stage or a pantry from; `observe_with_freshness` is
    /// the form the routes use. C7: the rows are DERIVED from
    /// `seam_contracts::freshness::requirement`, not restated beside it, so
    /// what this advertises cannot drift from what the proxy decides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<crate::routes::freshness::FreshnessStatus>,
}

impl ServingHealth {
    /// Read the live breaker map. Uses `snapshot()`, which is non-mutating —
    /// observing health must never consume a half-open trial.
    pub fn observe(breakers: &crate::routes::UpstreamBreakers) -> Self {
        let upstreams: Vec<ServingUpstream> = breakers
            .snapshot()
            .into_iter()
            .map(|s| ServingUpstream {
                endpoint: s.endpoint,
                circuit: s.circuit,
                error_streak: s.error_streak,
                shedding: s.skipped,
            })
            .collect();
        Self {
            shedding: upstreams.iter().any(|u| u.shedding),
            degrading: upstreams.iter().any(|u| u.error_streak > 0),
            upstreams,
            freshness: None,
        }
    }

    /// [`Self::observe`] plus the live freshness block — the form every route
    /// uses. Split from `observe` so the breaker-only reading stays available
    /// to callers that hold no AppState (and so the existing breaker tests keep
    /// their exact subject).
    pub fn observe_with_freshness(state: &AppState) -> Self {
        let mut health = Self::observe(&state.upstream_breakers);
        health.freshness = Some(crate::routes::freshness::status_block(
            state.network_stage,
            state.stage_provenance,
            &state.freshness_pantry,
        ));
        health
    }
}

/// Projection role details
#[derive(Serialize)]
pub struct ProjectionRole {
    /// Whether this instance is the projection writer
    pub writer: bool,
}

/// Build health response with current state
fn build_health_response(state: &AppState) -> HealthResponse {
    let args = &state.args;

    // Check worker pool health
    let (actual_conductor_connected, connected_workers, total_workers) = match &state.pool {
        Some(pool) => {
            let connected = pool.connected_count();
            let total = pool.worker_count();
            (pool.is_healthy(), connected, total)
        }
        None => {
            // No pool - can't verify conductor status
            (false, 0, 0)
        }
    };

    // Conductor pool size from registry
    let pool_size = state
        .conductor_registry
        .as_ref()
        .map(|r| r.conductor_count())
        .unwrap_or(0);

    // Per-conductor pool health from router
    let (pools_healthy, pools_total) = state
        .conductor_router
        .as_ref()
        .map(|r| (r.pools().healthy_count(), r.pools().total_count()))
        .unwrap_or((0, 0));

    // In dev mode, conductor connection is optional
    // Doorway can operate as pure HTTP bridge to elohim-storage
    let conductor_connected = if args.dev_mode {
        // Dev mode: always report healthy for conductor
        // (actual status still shown in connected_workers/total_workers)
        true
    } else {
        actual_conductor_connected
    };

    // Check if projection/cache is enabled
    let cache_enabled = state.projection.is_some();

    // Include conductor status info in error field if not connected (but not blocking in dev mode)
    let error = if !actual_conductor_connected && !args.dev_mode {
        Some(format!(
            "No workers connected to conductor ({connected_workers}/{total_workers} workers) - seeding will fail"
        ))
    } else if !actual_conductor_connected && args.dev_mode {
        Some(format!(
            "Dev mode: conductor not connected ({connected_workers}/{total_workers} workers) - using elohim-storage path"
        ))
    } else {
        None
    };

    // Determine status for doorway-picker UI
    // - 'online': fully operational
    // - 'degraded': running but conductor not connected (limited functionality)
    // - 'offline': would return early if service truly down
    // - 'maintenance': reserved for planned maintenance
    let status = if conductor_connected || args.dev_mode {
        "online"
    } else {
        "degraded"
    };

    // Registration is always open for now (can be made configurable via args later)
    let registration_open = true;

    // Calculate uptime in seconds since AppState creation (process start)
    let uptime = (chrono::Utc::now() - state.started_at).num_seconds().max(0) as u64;

    // Read cached P2P health (non-blocking — uses try_read to avoid stalling health checks)
    let p2p = match state.p2p_health.try_read() {
        Ok(guard) => guard
            .as_ref()
            .map(P2PHealthSnapshot::with_observed_age)
            .unwrap_or_default(),
        Err(_) => P2PHealth::default(),
    };

    // Backing-aware DHT-participation identity (Tier C). `bootstrap_backend` is
    // read from the LIVE store (authoritative shared-vs-per-pod signal); the
    // table id is surfaced only when that backend is the shared mongo store.
    let bootstrap_backend = state.bootstrap.as_ref().map(|b| b.k2().backend_name());
    // Signal-shared is read from the LIVE signal bus (D2): "mongo" backend = the
    // shared cross-relay bus is active (a domain's relays forward to each other);
    // "mem" / absent = a per-pod bus (no cross-relay forwarding). This is the
    // signal analog of bootstrap_backend and flips to shared when D2 is configured.
    let signal_shared = state
        .signal
        .as_ref()
        .map(|s| s.bus().backend_name() == "mongo")
        .unwrap_or(false);
    let dht_backing = DhtBacking {
        bootstrap_url: args.bootstrap_url.clone(),
        bootstrap_backend,
        bootstrap_table: match bootstrap_backend {
            Some("mongo") => bootstrap_table_id().map(str::to_string),
            _ => None,
        },
        signal_url: args.signal_url.clone(),
        signal_shared,
    };

    // The serving path, read from the live breaker map (non-mutating).
    let serving = ServingHealth::observe(&state.upstream_breakers);

    HealthResponse {
        // `healthy` used to be a hardcoded `true` meaning "the process is up".
        // That is what let elohim.host report green through a total /db outage
        // on 2026-08-20. It now means "the process is up AND its serving path
        // can answer". The HTTP status is deliberately unchanged (see
        // `ServingHealth` — this endpoint is the k8s liveness probe).
        healthy: !serving.shedding && !serving.degrading,
        status: if serving.shedding || serving.degrading {
            "degraded"
        } else {
            status
        },
        registration_open,
        version: env!("CARGO_PKG_VERSION"),
        uptime,
        cache_enabled,
        cache: CacheStatus {
            enabled: cache_enabled,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
        mode: if args.dev_mode {
            "development".to_string()
        } else {
            "production".to_string()
        },
        node_id: args.node_id.to_string(),
        conductor: ConductorHealth {
            connected: conductor_connected,
            connected_workers,
            total_workers,
            pool_size,
            pools_healthy,
            pools_total,
        },
        projection: ProjectionRole {
            writer: args.projection_writer,
        },
        p2p,
        dht_backing,
        serving,
        discovery_complete: *state.discovery_ready.borrow(),
        error,
    }
}

/// Handle liveness probe (/health, /healthz)
///
/// Returns 200 OK if doorway service is running.
/// The response body includes conductor status for informational purposes.
/// Callers that need to verify conductor connectivity should check `conductor.connected`.
pub fn health_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = build_health_response(&state);

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":true,"error":"Serialization failed"}"#.to_string());

    // Liveness probe: always return 200 if service is running
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle the serving probe (`/health/serving`) — the status-code-bearing
/// answer to "can this doorway actually serve right now?".
///
/// 200 when every upstream breaker is closed; **503 + Retry-After** when any is
/// shedding, with the same `ServingHealth` body `/health` carries.
///
/// This is the endpoint an alert, an a2o scenario, or an operator should watch.
/// It is deliberately NOT any Kubernetes probe: `/health` is simultaneously the
/// startup, readiness and liveness probe for the doorway container, so a
/// serving-aware status code there would CrashLoop the pod (liveness) and pull
/// a doorway that can still serve warm shells out of the Service (readiness).
/// Splitting the signal from the probe is the whole point — the gap this closes
/// is that on 2026-08-20 NOTHING returned a bad status code while elohim.host
/// 503'd every content read for hours.
pub fn serving_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let serving = ServingHealth::observe_with_freshness(&state);
    let body = serde_json::to_string(&serving)
        .unwrap_or_else(|_| r#"{"shedding":true,"degrading":true,"upstreams":[]}"#.to_string());

    // 503 on EITHER regime: an open circuit (cannot serve) or a non-zero error
    // streak (serving, but riding upstream timeouts — the 20s-page regime).
    let failing = serving.shedding || serving.degrading;
    let mut builder = Response::builder()
        .header("Content-Type", "application/json")
        .status(if failing {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        });
    if failing {
        builder = builder.header(
            "Retry-After",
            crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS.to_string(),
        );
    }
    builder.body(Full::new(Bytes::from(body))).unwrap()
}

/// Handle readiness probe (/ready, /readyz)
///
/// Returns 200 OK only if doorway can accept traffic.
/// In production: requires conductor connection.
/// In dev mode: conductor is optional (doorway bridges to elohim-storage).
/// Use this endpoint for load balancer health checks and seeder pre-flight checks.
pub fn readiness_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = build_health_response(&state);

    // Readiness depends on role:
    // - Writer instances: need conductor connection (to write projections)
    // - Reader instances: only need MongoDB (conductor is optional)
    // - Dev mode: always ready (conductor.connected is forced true)
    let is_ready = if !state.args.projection_writer {
        // Read replicas are ready if they have a projection store (MongoDB)
        state.projection.is_some() || state.args.dev_mode
    } else {
        response.conductor.connected
    };

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":false,"error":"Serialization failed"}"#.to_string());

    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle version endpoint (/version)
///
/// Returns build information for deployment verification.
/// The orchestrator uses this to verify deployments match expected commits.
pub fn version_info() -> Response<Full<Bytes>> {
    let info = elohim_compute::BuildInfo::new("elohim-doorway");
    let body = serde_json::to_string(&info)
        .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle startup progress endpoint (/health/startup)
///
/// Returns JSON showing startup progress for the bootstrap page to poll.
/// Reports identity, storage, projection, and root-app readiness.
pub async fn startup_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let args = &state.args;

    // identity.did — derive from doorway_id
    let did = if let Some(ref id) = args.doorway_id {
        if id.contains('.') {
            format!("did:web:{id}")
        } else {
            let domain = id.replace('-', ".");
            format!("did:web:{domain}")
        }
    } else {
        format!("did:web:{}", args.node_id)
    };

    // storage
    let storage_ready = args.storage_url.is_some();

    // projection counts from hot cache
    let (content, humans, relationships) = state
        .projection
        .as_ref()
        .map(|p| p.count_by_type())
        .unwrap_or((0, 0, 0));
    let projection_ready = content + humans + relationships > 0;

    // root projection — whichever EPR commitment declares urlPath="/" is the root.
    // ROOT_APP_SLUG is gone; the router consults live projection data from storage.
    let root_projection = state.epr_router.dispatch("/").map(|p| {
        serde_json::json!({
            "eprId": p.epr_id,
            "urlPath": p.url_path,
            "ready": true,
        })
    });

    let warmup = if let Some(ref ws) = state.warmup_state {
        serde_json::json!({
            "inProgress": ws.in_progress.load(std::sync::atomic::Ordering::Relaxed),
            "attempts": ws.attempts.load(std::sync::atomic::Ordering::Relaxed),
            "maxAttempts": ws.max_attempts.load(std::sync::atomic::Ordering::Relaxed),
            "completed": ws.completed.load(std::sync::atomic::Ordering::Relaxed),
            "lastError": ws.last_error.lock().unwrap().clone(),
            "budgetSecs": crate::projection::warm_stream::WARMUP_TOTAL_BUDGET_SECS,
            "upstreams": ws.health.snapshot(),
        })
    } else {
        serde_json::json!(null)
    };

    // Served SSR bundle-head attestations (Track-4 T4-1). Each entry:
    // {slug, serverBlobHash, materializedAt (rfc3339), status (current|stale|
    // refreshing|failed)} plus declaredServerBlobHash when a reconcile check has
    // observed it. Empty on a non-SSR doorway. A parallel CI/probe consumes this
    // exact camelCase shape — see render::registry::BundleHead::to_json.
    let served_bundle_heads = state.renderer_registry.heads_json();

    let body = serde_json::json!({
        "identity": {
            "ready": true,
            "did": did,
        },
        "storage": {
            "ready": storage_ready,
        },
        "projection": {
            "ready": projection_ready,
            "content": content,
            "humans": humans,
            "relationships": relationships,
        },
        "rootProjection": root_projection,
        "warmup": warmup,
        "servedBundleHeads": served_bundle_heads,
    })
    .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Args;
    use clap::Parser;

    fn test_state() -> AppState {
        let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        AppState::new(args)
    }

    /// Drive `state`'s breaker for `ep` into a shedding (Open) state.
    fn open_the_breaker(state: &AppState, ep: &str) {
        for _ in 0..crate::routes::upstream_health::UPSTREAM_CIRCUIT_FAIL_THRESHOLD {
            state.upstream_breakers.record(ep, false);
        }
    }

    #[test]
    fn health_is_green_when_nothing_is_shedding() {
        let state = test_state();
        let r = build_health_response(&state);
        assert!(r.healthy);
        assert!(!r.serving.shedding);
        assert!(r.serving.upstreams.is_empty(), "no upstream seen yet");
    }

    #[test]
    fn health_stops_reporting_green_while_the_serving_path_sheds() {
        // THE REGRESSION THIS CLOSES. On 2026-08-20 elohim.host answered
        // /health with healthy=true, status="online" while every
        // /db/content/* read returned 503 cause=upstream for hours, because
        // `healthy` was a hardcoded `true` and nothing consulted the breakers.
        let state = test_state();
        let ep = "http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090";
        open_the_breaker(&state, ep);

        let r = build_health_response(&state);
        assert!(
            !r.healthy,
            "a doorway whose only upstream is shedding is NOT healthy"
        );
        assert_eq!(r.status, "degraded");
        assert!(r.serving.shedding);

        let up = &r.serving.upstreams;
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].endpoint, ep);
        assert_eq!(up[0].circuit, "open");
        assert!(up[0].shedding);
        assert_eq!(
            up[0].error_streak,
            crate::routes::upstream_health::UPSTREAM_CIRCUIT_FAIL_THRESHOLD
        );

        // ...and it is visible in the wire body, camelCase.
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["healthy"], serde_json::json!(false));
        assert_eq!(json["serving"]["shedding"], serde_json::json!(true));
        assert_eq!(json["serving"]["upstreams"][0]["errorStreak"], 3);
    }

    #[test]
    fn health_is_demoted_in_the_slow_regime_before_the_circuit_ever_opens() {
        // THE BLIND WINDOW. `shedding` alone only becomes true on the THIRD
        // consecutive failure. The first two ride the full upstream budget —
        // that is the 10s/20s page load a person actually experiences — and a
        // shedding-only signal reads green through exactly that window. One
        // recorded failure is enough to stop claiming health.
        let state = test_state();
        let ep = "http://peer:8090";
        state.upstream_breakers.record(ep, false);

        let r = build_health_response(&state);
        assert!(!r.serving.shedding, "one failure does not open the circuit");
        assert!(r.serving.degrading, "but it IS the slow regime");
        assert!(!r.healthy, "and health must not read green through it");
        assert_eq!(r.status, "degraded");

        // A recorded success clears it.
        state.upstream_breakers.record(ep, true);
        let r = build_health_response(&state);
        assert!(!r.serving.degrading);
        assert!(r.healthy);
    }

    #[test]
    fn health_keeps_returning_200_while_shedding() {
        // THE SAFETY RAIL. /health on :8080 is simultaneously the startup,
        // readiness AND liveness probe for the doorway container. If this
        // endpoint ever answers non-2xx on a degraded upstream, kubelet
        // CrashLoops the pod and readiness pulls it from the Service — an
        // upstream blip would become a doorway outage, on a pair that flaps
        // independently but overlaps. The honest signal goes in the BODY here
        // and in the STATUS CODE at /health/serving.
        let state = Arc::new(test_state());
        open_the_breaker(&state, "http://peer:8090");
        let resp = health_check(Arc::clone(&state));
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "liveness must not flip on a shedding upstream"
        );
    }

    #[test]
    fn serving_probe_carries_the_status_code_health_cannot() {
        let state = Arc::new(test_state());
        let r = serving_check(Arc::clone(&state));
        assert_eq!(r.status(), StatusCode::OK, "nothing shedding yet");

        // A single failure — still Closed, but already the slow regime.
        state.upstream_breakers.record("http://peer:8090", false);
        assert_eq!(
            serving_check(Arc::clone(&state)).status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "the slow regime is a serving failure even before the circuit opens"
        );

        open_the_breaker(&state, "http://peer:8090");
        let r = serving_check(Arc::clone(&state));
        assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            r.headers().get("Retry-After").unwrap(),
            &crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS.to_string()
        );
    }

    #[test]
    fn uptime_is_relative_to_process_start_not_epoch() {
        let state = test_state();
        let response = build_health_response(&state);
        // A freshly created AppState has near-zero uptime. The old bug
        // reported seconds-since-UNIX-epoch here (~1.7 billion).
        assert!(
            response.uptime < 60,
            "uptime should be seconds since start, got {}",
            response.uptime
        );
    }

    #[test]
    fn dht_backing_reports_signal_unshared_and_omits_absent_urls() {
        // Tier C: default args carry no bootstrap_url/signal_url and test_state
        // wires no bootstrap store — the backing block must still be present,
        // honestly reporting the signal relay as unshared (per-pod SBD relay).
        let state = test_state();
        let response = build_health_response(&state);
        let json = serde_json::to_value(&response).unwrap();
        let backing = &json["dhtBacking"];
        assert!(backing.is_object(), "dhtBacking must always be present");
        assert_eq!(
            backing["signalShared"],
            serde_json::json!(false),
            "signal relay has no shared backend today (spec §2)"
        );
        // Absent optional backing facts are omitted, never fabricated.
        assert!(backing.get("bootstrapUrl").is_none());
        assert!(backing.get("signalUrl").is_none());
        assert!(backing.get("bootstrapTable").is_none());
    }

    #[test]
    fn liveness_returns_200_even_without_conductor() {
        let state = Arc::new(test_state());
        let resp = health_check(state);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn startup_warmup_block_includes_upstreams_and_budget() {
        use crate::projection::warm_stream::WarmupState;
        let ws = WarmupState::new();
        ws.health.record_outcome("http://peer-a:8090", false, 0);
        let upstreams = serde_json::to_value(ws.health.snapshot()).unwrap();
        let block = serde_json::json!({
            "inProgress": false,
            "upstreams": upstreams,
            "budgetSecs": 75u64,
        });
        let s = block.to_string();
        assert!(s.contains("\"upstreams\""));
        assert!(s.contains("\"errorStreak\""));
        assert!(s.contains("\"budgetSecs\":75"));
    }

    #[test]
    fn p2p_health_carries_reconcile_caught_up_and_divergent_anchor() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 2,
            peer_id: Some("p".to_string()),
            caught_up: Some(true),
            divergent_anchor: Some(0),
            ..Default::default()
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["caughtUp"], serde_json::json!(true));
        assert_eq!(json["divergentAnchor"], serde_json::json!(0));
    }

    #[test]
    fn p2p_health_omits_reconcile_fields_when_absent() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 0,
            peer_id: None,
            caught_up: None,
            divergent_anchor: None,
            ..Default::default()
        };
        let json = serde_json::to_value(&h).unwrap();
        assert!(
            json.get("caughtUp").is_none(),
            "caughtUp must be omitted when None"
        );
        assert!(json.get("divergentAnchor").is_none());
    }

    #[test]
    fn a_cached_p2p_snapshot_older_than_two_rounds_is_marked_stale() {
        // The cache stays (health must not block); its age makes a stale
        // snapshot distinguishable from a current storage status response.
        assert!(!is_stale(Some(59_000)), "inside one round: fresh");
        assert!(!is_stale(Some(119_999)), "inside two rounds: fresh");
        assert!(is_stale(Some(120_001)), "past two rounds: stale");
        assert!(is_stale(None), "no snapshot is stale, never fresh");
    }

    #[test]
    fn p2p_health_carries_age_staleness_and_convergence() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 2,
            peer_id: Some("p".to_string()),
            caught_up: Some(true),
            converged: Some(false),
            divergent_anchor: Some(1_860),
            observed_age_ms: Some(4_000),
            stale: false,
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["caughtUp"], serde_json::json!(true));
        assert_eq!(json["converged"], serde_json::json!(false));
        assert_eq!(json["observedAgeMs"], serde_json::json!(4_000));
        assert_eq!(json["stale"], serde_json::json!(false));
    }

    #[test]
    fn health_emits_a_stale_p2p_block_when_no_snapshot_is_cached() {
        let state = test_state();
        let response = build_health_response(&state);
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["p2p"]["stale"], serde_json::json!(true));
        assert!(json["p2p"].get("observedAgeMs").is_none());
    }
}
