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
    /// Storage's unified SyncGate verdict (`syncPaused` on `/p2p/status`):
    /// true when the storage sidecar is holding sync for ANY reason (operator
    /// pause, wifi-only on cellular, import/bulk backpressure, drain backlog).
    /// None until the cached storage status supplies the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_paused: Option<bool>,
    /// Reasons currently holding sync (kebab-case, from storage
    /// `/p2p/status.syncReasons`). None when the cached status lacks the
    /// field; empty exactly when `syncPaused` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_reasons: Option<Vec<String>>,
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
            sync_paused: None,
            sync_reasons: None,
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
    /// How many roles the discovery service has actually mapped
    /// (`zome_configs.len()`).
    ///
    /// The load-bearing number behind `connected`. A socket to the conductor is
    /// not a usable conductor: with an empty role map every zome call answers
    /// `No zome config found for role 'imagodei'. Available: []` and every
    /// hosted registration fails HTTP 500 AGENT_KEY_ERROR. Named in camelCase
    /// on purpose — it is a NEW field, and the honest name is worth more than
    /// consistency with the snake_case fields already on the wire.
    #[serde(rename = "rolesDiscovered")]
    pub roles_discovered: usize,
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
    /// This endpoint's place in THIS doorway's declared peer list:
    ///
    /// * `"primary"` — the declared rank-0 peer (`--storage-url`, else the
    ///   first `--storage-urls` entry). Its breaker, and only its breaker,
    ///   decides `shedding` / `degrading`.
    /// * `"pool"` — any other endpoint the breaker map has ever seen: a
    ///   lower-ranked steward peer, a failover target, a drill victim. Its
    ///   circuit is reported in full but never demotes this doorway.
    /// * `"undeclared"` — this doorway declares NO storage peer, so there is
    ///   no basis to scope: every endpoint counts, exactly as before.
    ///
    /// ADDITIVE field — nothing that read this payload before loses a field.
    pub role: &'static str,
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
    /// How many roles the conductor discovery service has mapped.
    ///
    /// `Some(0)` is the CONDUCTOR-BLIND regime: the doorway holds a socket it
    /// cannot use, so every zome call answers `No zome config found for role
    /// 'imagodei'. Available: []` and every hosted registration fails HTTP 500
    /// AGENT_KEY_ERROR. That is a serving failure by any honest definition, and
    /// on 2026-08-21 nothing carried a bad status code while it was true.
    ///
    /// `Option` for the same reason `freshness` is: the breaker-only
    /// [`ServingHealth::observe`] form has no `AppState` to count roles from.
    /// The 503 predicate in `serving_check` reads THIS field, so what the body
    /// shows and what the status code means cannot drift apart.
    #[serde(rename = "rolesDiscovered", skip_serializing_if = "Option::is_none")]
    pub roles_discovered: Option<usize>,
    /// True when the warmup finished a pass having produced NOTHING, so this
    /// doorway is serving an empty projection while it waits to re-warm.
    ///
    /// A doorway in this state answers every content route from an empty
    /// projection: `/health/startup` reads `projection.content: 0` and
    /// `servedBundleHeads: []`, and `/` pays a full 10s shell-fetch budget for a
    /// bundle nothing ever stocked before shedding. It is not serving, and until
    /// now nothing said so — `completed: true` made it look warm.
    ///
    /// `Option` for the same reason `freshness` is: the breaker-only
    /// [`ServingHealth::observe`] form has no `AppState` to read warmup from.
    #[serde(rename = "warmupEmpty", skip_serializing_if = "Option::is_none")]
    pub warmup_empty: Option<bool>,
}

impl ServingHealth {
    /// Read the live breaker map. Uses `snapshot()`, which is non-mutating —
    /// observing health must never consume a half-open trial.
    ///
    /// `primary_endpoint` is this doorway's DECLARED rank-0 storage peer
    /// (`Args::declared_primary_storage_peer` — the same list
    /// `RouteRegistry::declare_peer_priority` is fed). It SCOPES the two
    /// verdicts:
    ///
    /// WHY SCOPED. `shedding`/`degrading` used to be `any()` over the whole
    /// breaker map, which is every endpoint this doorway has EVER called —
    /// pool peers included. So an open circuit on a pool peer (`:8091` after a
    /// failover drill) demoted the doorway to `degraded` and 503'd
    /// `/health/serving` while its declared primary was closed and answering
    /// every read. That is a sibling-classification LIE: the invariant is that
    /// siblings are honestly classifiable as serving | shedding | dead, and a
    /// doorway serving perfectly off its primary is *serving*.
    ///
    /// The full upstream list is unchanged — every pool circuit is still
    /// reported, now with a `role`, so observability WIDENS while the verdict
    /// narrows to the endpoint this doorway actually serves from.
    ///
    /// `None` (nothing declared) keeps the old pool-wide behaviour: with no
    /// declaration there is no basis to pick a primary, and going blind would
    /// be worse than over-reporting.
    pub fn observe(
        breakers: &crate::routes::UpstreamBreakers,
        primary_endpoint: Option<&str>,
    ) -> Self {
        // Normalize BOTH sides the way every breaker call site keys the map
        // (`trim_end_matches('/')`, as `http.rs` does before `would_shed`) and
        // the way `declare_peer_priority` keys its rank map. A silent key
        // mismatch here would classify EVERY endpoint as pool and the doorway
        // would never report a shed again.
        let primary = primary_endpoint.map(|p| p.trim_end_matches('/'));
        let upstreams: Vec<ServingUpstream> = breakers
            .snapshot()
            .into_iter()
            .map(|s| {
                let role = match primary {
                    None => "undeclared",
                    Some(p) if s.endpoint.trim_end_matches('/') == p => "primary",
                    Some(_) => "pool",
                };
                ServingUpstream {
                    endpoint: s.endpoint,
                    circuit: s.circuit,
                    error_streak: s.error_streak,
                    shedding: s.skipped,
                    role,
                }
            })
            .collect();
        // An endpoint counts toward the verdict when it IS the declared
        // primary, or when nothing is declared at all.
        let counts = |u: &&ServingUpstream| u.role != "pool";
        Self {
            shedding: upstreams.iter().filter(counts).any(|u| u.shedding),
            degrading: upstreams.iter().filter(counts).any(|u| u.error_streak > 0),
            upstreams,
            freshness: None,
            warmup_empty: None,
            // Breaker-only form: no AppState, so no role count. `None` is a
            // real third answer ("not observed here"), never a silent zero —
            // a zero would 503 every caller of the breaker-only form.
            roles_discovered: None,
        }
    }

    /// [`Self::observe`] plus the live freshness block — the form every route
    /// uses. Split from `observe` so the breaker-only reading stays available
    /// to callers that hold no AppState (and so the existing breaker tests keep
    /// their exact subject).
    pub fn observe_with_freshness(state: &AppState) -> Self {
        let primary = state.args.declared_primary_storage_peer();
        let mut health = Self::observe(&state.upstream_breakers, primary.as_deref());
        health.freshness = Some(crate::routes::freshness::status_block(
            state.network_stage,
            state.stage_provenance,
            &state.freshness_pantry,
        ));
        // Only a doorway that RUNS discovery can be judged conductor-blind.
        // `main` spawns the discovery task under exactly this flag, so a read
        // replica (`--projection-writer=false`, serving from shared MongoDB and
        // deliberately never dialing a conductor) reports `None` — "not observed
        // here" — and is never 503'd for an empty map it was never meant to fill.
        health.roles_discovered = if state.args.projection_writer {
            Some(state.zome_configs.len())
        } else {
            None
        };
        // Only a doorway that RUNS a warmup can be judged empty-warm; one
        // without a warmup task reports `None` ("not observed here") rather than
        // a `false` it did not earn.
        health.warmup_empty = state.warmup_state.as_ref().map(|ws| ws.is_empty_warm());
        health
    }
}

/// Projection role details
#[derive(Serialize)]
pub struct ProjectionRole {
    /// Whether this instance is the projection writer
    pub writer: bool,
}

/// The coupling rule for `conductor.connected`, as a pure function so it can be
/// exercised over every combination without standing up a real worker pool.
///
/// BOTH halves are required and NEITHER is dev-mode-optional:
/// * `workers_live` — the pool has at least one connected worker. Without it the
///   doorway has no session to call through.
/// * `roles_discovered > 0` — discovery mapped at least one role. Without it
///   every zome call fails `No zome config found for role …`.
///
/// The field is what `/health`'s own docs tell the seeder to check before
/// seeding, so it may never disagree with `connected_workers` sitting beside it.
pub(crate) fn conductor_is_connected(workers_live: bool, roles_discovered: usize) -> bool {
    workers_live && roles_discovered > 0
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

    // How many roles discovery actually mapped. This is what makes a conductor
    // USABLE — see `ConductorHealth::roles_discovered`.
    let roles_discovered = state.zome_configs.len();

    // In dev mode, conductor connection is optional: the doorway can operate as
    // a pure HTTP bridge to elohim-storage. What dev mode may NOT do is claim a
    // conductor connection it cannot use. Reported on 2026-08-21 from the local
    // mesh: `conductor.connected: true` with `connected_workers: 0` and an empty
    // role map, while every hosted registration answered HTTP 500
    // AGENT_KEY_ERROR. `connected: true` was a hardcoded dev-mode constant, so
    // the one field the seeder is told to check before seeding could not report
    // the one condition that makes seeding fail.
    //
    // The role map is the honest gate on BOTH sides: a live worker with no roles
    // serves nothing, so it is not "connected" in any sense a caller can use.
    // (`readiness_check` keeps its dev-mode "always ready" contract explicitly
    // rather than inheriting it from this flag — see there.)
    //
    // The WORKER tally is the other half, and it is NOT dev-mode-optional. A
    // dev-mode carve-out that gated on the role map alone still reported
    // `connected: true` with `connected_workers: 0` after a conductor restart
    // (roles_discovered is a cached boot artifact — it survives the pool losing
    // every session). Dev mode's real contract is "a conductor is optional",
    // which `status`/`healthy` express below; it is never a licence for
    // `conductor.connected` to disagree with `conductor.connected_workers`.
    // (Measured on the household mesh 2026-08-22 —
    // features/doorway/peer-conductor-connection-resilience.feature.)
    let conductor_connected = conductor_is_connected(actual_conductor_connected, roles_discovered);

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

    // The serving path, read from the live breaker map (non-mutating), scoped
    // to the DECLARED primary peer — a pool peer's open circuit is reported in
    // `upstreams` but never demotes this doorway.
    let serving = ServingHealth::observe(
        &state.upstream_breakers,
        args.declared_primary_storage_peer().as_deref(),
    );

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
            roles_discovered,
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

    // 503 on ANY regime that stops this doorway answering: an open circuit
    // (cannot serve), a non-zero error streak (serving, but riding upstream
    // timeouts — the 20s-page regime), or a CONDUCTOR-BLIND doorway (empty role
    // map, so every zome call and every hosted registration fails). The third
    // was added 2026-08-21: a doorway that booted before its conductor answered
    // /health with conductor.connected=true and /health/serving with 200 while
    // every registration returned HTTP 500 AGENT_KEY_ERROR — the same
    // nothing-carries-a-bad-status-code hole this endpoint exists to close.
    let conductor_blind = serving.roles_discovered == Some(0);
    // A doorway whose warmup produced nothing serves an empty projection — every
    // content route answers from a projection with no rows, and `/` burns the
    // full shell-fetch budget for a bundle that was never stocked. Added
    // 2026-08-21 alongside the honest `completed` flag: the boot-order case
    // (doorway up at 22:55, storage at ~23:07) previously answered
    // /health/serving with 200 forever.
    let warmup_empty = serving.warmup_empty == Some(true);
    let failing = serving.shedding || serving.degrading || conductor_blind || warmup_empty;
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
    } else if state.args.dev_mode {
        // Dev mode stays ALWAYS READY, unchanged. This used to be inherited from
        // `conductor.connected` being a hardcoded dev-mode `true`; now that the
        // flag reports the honest role-map state, the readiness contract has to
        // say so itself instead of riding a lie that happened to be convenient.
        // A dev doorway is a legitimate pure HTTP bridge to elohim-storage, so
        // an empty role map must not pull it out of a load balancer — it must
        // only stop it CLAIMING a conductor it cannot call.
        true
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
    let info = doorway_version_document();
    let body = serde_json::to_string(&info)
        .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

#[derive(Debug, Clone, serde::Serialize)]
struct DoorwayVersionResponse {
    #[serde(flatten)]
    build: elohim_compute::BuildInfo,
    passport: DoorwayPassport,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DoorwayPassport {
    service: DoorwayServicePassport,
    host: DoorwayHostPassport,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DoorwayServicePassport {
    #[serde(flatten)]
    build: elohim_compute::BuildInfo,
    compiled_features: Vec<String>,
    active_transports: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DoorwayHostPassport {
    kernel: String,
    release: String,
    os: String,
    arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    container_hint: Option<String>,
}

fn doorway_version_document() -> DoorwayVersionResponse {
    let build = elohim_compute::BuildInfo::new("elohim-doorway");
    DoorwayVersionResponse {
        build: build.clone(),
        passport: DoorwayPassport {
            service: DoorwayServicePassport {
                build,
                compiled_features: Vec::new(),
                // Doorway is a Track-4 projection, not a P2P participant.
                active_transports: Vec::new(),
            },
            host: doorway_host_passport(),
        },
    }
}

fn doorway_host_passport() -> DoorwayHostPassport {
    DoorwayHostPassport {
        kernel: read_host_fact("/proc/sys/kernel/ostype")
            .unwrap_or_else(|| std::env::consts::OS.to_string()),
        release: read_host_fact("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|| "unknown".to_string()),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        container_hint: doorway_container_hint(),
    }
}

fn read_host_fact(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn doorway_container_hint() -> Option<String> {
    if std::env::var_os("KUBERNETES_SERVICE_HOST").is_some() {
        return Some("kubernetes".to_string());
    }
    if std::path::Path::new("/.dockerenv").exists() {
        return Some("docker".to_string());
    }
    if std::path::Path::new("/run/.containerenv").exists() {
        return Some("container".to_string());
    }
    let cgroup = std::fs::read_to_string("/proc/1/cgroup").ok()?;
    for (needle, hint) in [
        ("kubepods", "kubernetes"),
        ("containerd", "containerd"),
        ("docker", "docker"),
        ("podman", "podman"),
    ] {
        if cgroup.contains(needle) {
            return Some(hint.to_string());
        }
    }
    None
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

    // ONE BREAKER, ONE VIEW.
    //
    // MEASURED (local mesh, 2026-08-21): doorway A shed 503
    // `{"status":"catching-up","circuit":"open","errorStreak":3}` on every
    // request while THIS endpoint reported every upstream `circuit: closed,
    // errorStreak: 0`. An operator reading /health/startup saw a healthy
    // doorway that was refusing everything.
    //
    // Both numbers were true of DIFFERENT breakers. The shed decision reads
    // `state.upstream_breakers` (storage_proxy / catching_up); this block used
    // to read `WarmStreamHealth`, a SECOND per-upstream circuit map private to
    // the warmup task, with its own threshold, its own cooldown, and a tick
    // domain of warm-up PASS COUNTERS rather than wall clock. Once warmup
    // completed, that map froze at whatever it last saw — typically all-closed,
    // forever — and went on being advertised as if it were live upstream health.
    //
    // `state.upstream_breakers` is authoritative by definition: it is the map
    // the shed decision consults, so it is the only one that can answer "is this
    // doorway refusing requests?". Both keys below now read it. The warmup
    // task's internal gate still gates the warmup loop — that is a legitimate
    // private concern — it is simply no longer ADVERTISED as the answer to a
    // question it cannot answer.
    //
    // `snapshot()` is non-mutating: observing health must never admit a
    // half-open trial.
    let authoritative_upstreams: Vec<serde_json::Value> = state
        .upstream_breakers
        .snapshot()
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "upstream": b.endpoint,
                "circuit": b.circuit,
                "errorStreak": b.error_streak,
                "recentFailures": b.recent_failures,
                // The breaker tracks no last-good timestamp yet (same honest
                // null the self-healing UpstreamView carries).
                "lastGood": serde_json::Value::Null,
                "skipped": b.skipped,
            })
        })
        .collect();

    let warmup = if let Some(ref ws) = state.warmup_state {
        serde_json::json!({
            "inProgress": ws.in_progress.load(std::sync::atomic::Ordering::Relaxed),
            "attempts": ws.attempts.load(std::sync::atomic::Ordering::Relaxed),
            "maxAttempts": ws.max_attempts.load(std::sync::atomic::Ordering::Relaxed),
            "completed": ws.completed.load(std::sync::atomic::Ordering::Relaxed),
            // A pass that ran to the end having streamed NOTHING. `completed`
            // used to be stored unconditionally, so this state was reported as
            // a successful warmup — indistinguishable from a real one, and
            // never retried. See WarmupState::completed_empty.
            "completedEmpty": ws.completed_empty.load(std::sync::atomic::Ordering::Relaxed),
            "produced": ws.produced.load(std::sync::atomic::Ordering::Relaxed),
            "lastError": ws.last_error.lock().unwrap().clone(),
            "budgetSecs": crate::projection::warm_stream::WARMUP_TOTAL_BUDGET_SECS,
            // Authoritative — see above. NOT ws.health.snapshot().
            "upstreams": authoritative_upstreams.clone(),
        })
    } else {
        serde_json::json!(null)
    };

    // The live serving answer, in the SAME shape /health and /health/serving
    // carry, from the SAME breaker map. A reader polling /health/startup during
    // recovery can now see the shed without cross-referencing another endpoint.
    let serving = ServingHealth::observe(
        &state.upstream_breakers,
        args.declared_primary_storage_peer().as_deref(),
    );

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
        // The live shed answer, same source + shape as /health and
        // /health/serving. See the ONE BREAKER, ONE VIEW note above.
        "serving": serving,
        "upstreams": authoritative_upstreams,
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

    #[test]
    fn version_passport_keeps_legacy_fields_and_adds_camel_case_blocks() {
        let expected = elohim_compute::BuildInfo::new("elohim-doorway");
        let expected_json = serde_json::to_value(&expected).unwrap();
        let actual = serde_json::to_value(doorway_version_document()).unwrap();

        for key in [
            "version",
            "commit",
            "commitFull",
            "buildTime",
            "rustcVersion",
            "service",
        ] {
            assert_eq!(actual[key], expected_json[key], "legacy field {key}");
        }
        assert_eq!(actual["service"], serde_json::json!("elohim-doorway"));
        assert!(actual["passport"]["service"].is_object());
        assert!(actual["passport"]["service"]
            .get("compiledFeatures")
            .is_some());
        assert!(actual["passport"]["service"]
            .get("activeTransports")
            .is_some());
        assert!(actual["passport"]["host"].get("arch").is_some());
        assert!(actual["passport"]["host"].get("container_hint").is_none());
    }
    use crate::config::Args;
    use clap::Parser;

    fn test_state() -> AppState {
        let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        AppState::new(args)
    }

    /// A dev-mode state — the shape every local-mesh and Che doorway runs in,
    /// and the one whose `conductor.connected` was hardcoded `true`.
    fn dev_state() -> AppState {
        let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0", "--dev-mode"]);
        AppState::new(args)
    }

    /// Populate the role map the way `DiscoveryService::discover` does.
    fn discover_role(state: &AppState, role: &str) {
        state.zome_configs.insert(
            format!("uhC0k-{role}"),
            crate::worker::ZomeCallConfig {
                dna_hash: format!("uhC0k-{role}"),
                agent_pub_key: "uhCAk-test".to_string(),
                zome_name: "content_store".to_string(),
                app_id: "elohim".to_string(),
                role_name: role.to_string(),
            },
        );
    }

    /// THE REGRESSION THIS CLOSES (local mesh, 2026-08-21). The doorways were
    /// started before the conductors. Afterwards `/health` reported
    /// `conductor.connected: true` with `connected_workers: 0` and an EMPTY
    /// role map, while every hosted registration answered HTTP 500
    /// `{"error":"Failed to get agent identity","code":"AGENT_KEY_ERROR"}`
    /// because `get_zome_config_by_role` had nothing to find
    /// (`No zome config found for role 'imagodei'. Available: []`).
    ///
    /// `connected` is the ONE field this struct's own docs tell the seeder to
    /// check before seeding, and in dev mode it was a hardcoded constant — so
    /// it could not report the single condition that makes seeding fail.
    #[test]
    fn health_does_not_claim_a_conductor_it_cannot_call() {
        let state = dev_state();
        assert_eq!(state.zome_configs.len(), 0, "role map starts empty");

        let r = build_health_response(&state);
        assert_eq!(
            r.conductor.roles_discovered, 0,
            "the honest count rides the wire"
        );
        assert!(
            !r.conductor.connected,
            "an empty role map means every zome call fails — dev mode may skip \
             the conductor, but it may not CLAIM one it cannot call"
        );
    }

    /// The other half of the contract: once discovery fills the role map, the
    /// claim comes back. Without this the fix above would just be a new lie in
    /// the opposite direction.
    #[test]
    fn a_discovered_role_map_restores_the_connected_claim() {
        let state = dev_state();
        discover_role(&state, "imagodei");

        let r = build_health_response(&state);
        assert_eq!(r.conductor.roles_discovered, 1);
        assert!(
            !r.conductor.connected,
            "a role map alone is not a session — this state has no pool, so the \
             claim stays false"
        );
        assert!(
            conductor_is_connected(true, r.conductor.roles_discovered),
            "a doorway with BOTH a live worker and a resolved role map reports a \
             usable conductor"
        );
    }

    /// The mesh regression this closes (2026-08-22,
    /// features/doorway/peer-conductor-connection-resilience.feature): a
    /// conductor restart drops every pool session, but `roles_discovered` is a
    /// cached boot artifact and survives it. The dev-mode carve-out gated on the
    /// role map ALONE, so `/health` answered `connected: true` beside
    /// `connected_workers: 0` — the two fields telling opposite stories.
    #[test]
    fn connected_never_disagrees_with_the_worker_tally() {
        assert!(
            !conductor_is_connected(false, 5),
            "no live worker means not connected, however many roles are cached"
        );
        assert!(
            !conductor_is_connected(true, 0),
            "a live worker with no roles serves nothing"
        );
        assert!(conductor_is_connected(true, 5));
        assert!(!conductor_is_connected(false, 0));
    }

    /// `/health/serving` is the status-code-bearing probe — the endpoint that
    /// exists because on 2026-08-20 NOTHING returned a bad status code while
    /// the doorway could not serve. A conductor-blind doorway is that same
    /// hole: 200 while every registration 500s.
    #[test]
    fn serving_probe_503s_while_the_role_map_is_empty() {
        let state = Arc::new(dev_state());
        let resp = serving_check(Arc::clone(&state));
        assert_eq!(
            resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "conductor-blind is a serving failure and must carry the status code"
        );
        assert!(
            resp.headers().contains_key("Retry-After"),
            "a caller told 503 must be told when to come back"
        );
    }

    /// Scoping sibling: the 503 above is caused by the EMPTY role map, not by
    /// the probe having become unconditionally red.
    #[test]
    fn serving_probe_200s_once_roles_are_discovered() {
        let state = dev_state();
        discover_role(&state, "imagodei");
        let resp = serving_check(Arc::new(state));
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// A read replica (`--projection-writer=false`) never dials a conductor —
    /// `main` spawns the discovery task only for writers. Judging it
    /// conductor-blind would 503 a doorway that is serving perfectly from the
    /// shared projection store, so the gate reports "not observed here" instead
    /// of a silent zero.
    #[test]
    fn a_read_replica_is_never_judged_conductor_blind() {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0", "--dev-mode"]);
        args.projection_writer = false;
        let state = Arc::new(AppState::new(args));
        assert_eq!(
            state.zome_configs.len(),
            0,
            "a replica never fills a role map"
        );

        let serving = ServingHealth::observe_with_freshness(&state);
        assert_eq!(
            serving.roles_discovered, None,
            "not observed, never a zero that would 503"
        );
        assert_eq!(serving_check(Arc::clone(&state)).status(), StatusCode::OK);
    }

    /// The honest `connected` flag must NOT quietly pull a dev doorway out of a
    /// load balancer. A dev doorway is a legitimate pure HTTP bridge to
    /// elohim-storage; readiness kept its "always ready" contract explicitly
    /// when `connected` stopped being a hardcoded `true`.
    #[test]
    fn dev_mode_readiness_survives_the_honest_connected_flag() {
        let state = Arc::new(dev_state());
        let resp = readiness_check(Arc::clone(&state));
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "dev readiness is unchanged by the conductor-honesty fix"
        );
    }

    /// Drive `state`'s breaker for `ep` into a shedding (Open) state.
    fn open_the_breaker(state: &AppState, ep: &str) {
        for _ in 0..crate::routes::upstream_health::UPSTREAM_CIRCUIT_FAIL_THRESHOLD {
            state.upstream_breakers.record(ep, false);
        }
    }

    /// ONE BREAKER, ONE VIEW.
    ///
    /// MEASURED (local mesh 2026-08-21): doorway A shed 503
    /// `{"status":"catching-up","circuit":"open","errorStreak":3}` on every
    /// request while `/health/startup` reported every upstream
    /// `circuit: closed, errorStreak: 0` — so an operator reading the startup
    /// surface saw a healthy doorway that was refusing everything. Both numbers
    /// were true of DIFFERENT breakers: the shed decision reads
    /// `upstream_breakers`; the startup block read `WarmStreamHealth`, the
    /// warmup task's private map, frozen at whatever it last saw.
    ///
    /// This drives the AUTHORITATIVE breaker into a shed and asserts every
    /// surface says so together.
    #[tokio::test]
    async fn startup_and_serving_report_the_same_breaker_state() {
        let state = Arc::new(test_state());
        let ep = "http://storage-a:8090";
        open_the_breaker(&state, ep);

        // The authoritative answer. `test_state()` declares no storage peer, so
        // the classification is UNDECLARED and stays pool-wide — this test's
        // subject (one breaker, one view) is unchanged by the primary scoping.
        let serving = ServingHealth::observe(&state.upstream_breakers, None);
        assert!(serving.shedding, "the breaker is open — this doorway sheds");

        let resp = startup_check(Arc::clone(&state)).await;
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .expect("startup body")
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("startup json");

        let upstreams = json["upstreams"].as_array().cloned().unwrap_or_default();
        assert_eq!(upstreams.len(), 1, "the one known upstream");
        assert_eq!(upstreams[0]["upstream"], ep);
        assert_eq!(
            upstreams[0]["circuit"], "open",
            "/health/startup MUST report the shed the serving path is actually \
             doing — this disagreement read as a healthy doorway"
        );
        assert_eq!(
            upstreams[0]["errorStreak"],
            serde_json::json!(crate::routes::upstream_health::UPSTREAM_CIRCUIT_FAIL_THRESHOLD),
            "errorStreak must match the authoritative breaker"
        );

        // The warmup block's array is the SAME binding, so it cannot diverge.
        // (This state has no warmup task, so the block is null; when a doorway
        // has one, the two are equal by construction — pinned here so a future
        // edit cannot re-point it at a private map.)
        if !json["warmup"].is_null() {
            assert_eq!(
                json["warmup"]["upstreams"], json["upstreams"],
                "the warmup block must not carry a second, disagreeing breaker view"
            );
        }

        // ...and the startup surface answers the live serving question in the
        // same shape /health and /health/serving use.
        assert_eq!(json["serving"]["shedding"], serde_json::json!(true));
        assert_eq!(
            json["serving"]["upstreams"][0]["circuit"],
            serde_json::json!("open")
        );
    }

    /// A warmup that produced nothing must not read as serving.
    ///
    /// MEASURED (local mesh 2026-08-21): doorway A booted at 22:55, storage came
    /// up at ~23:07. The warmup ran against unreachable/empty storage, stored
    /// `completed: true` unconditionally, and never re-ran — so
    /// `/health/startup` showed a completed warmup beside `projection.content:
    /// 0` and `servedBundleHeads: []`, and `/health/serving` answered 200 while
    /// the doorway served an empty projection until someone restarted it.
    #[test]
    fn an_empty_warmup_is_not_serving() {
        use crate::projection::warm_stream::WarmupState;
        use std::sync::atomic::Ordering::Relaxed;

        let mut state = dev_state();
        discover_role(&state, "imagodei");
        let ws = Arc::new(WarmupState::new());
        // The pass ran to the end and produced NOTHING.
        ws.in_progress.store(false, Relaxed);
        ws.completed_empty.store(true, Relaxed);
        ws.completed.store(false, Relaxed);
        state.warmup_state = Some(Arc::clone(&ws));
        let state = Arc::new(state);

        assert!(ws.is_empty_warm(), "a produced-nothing pass is empty-warm");

        let serving = ServingHealth::observe_with_freshness(&state);
        assert_eq!(serving.warmup_empty, Some(true));
        assert_eq!(
            serving_check(Arc::clone(&state)).status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a doorway serving an empty projection is NOT serving — this 200'd \
             for the whole boot-order window"
        );
    }

    #[test]
    fn a_productive_warmup_serves() {
        use crate::projection::warm_stream::WarmupState;
        use std::sync::atomic::Ordering::Relaxed;

        let mut state = dev_state();
        discover_role(&state, "imagodei");
        let ws = Arc::new(WarmupState::new());
        ws.in_progress.store(false, Relaxed);
        ws.produced.store(1_234, Relaxed);
        ws.completed.store(true, Relaxed);
        ws.completed_empty.store(false, Relaxed);
        state.warmup_state = Some(ws);
        let state = Arc::new(state);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert_eq!(serving.warmup_empty, Some(false));
        assert_eq!(serving_check(Arc::clone(&state)).status(), StatusCode::OK);
    }

    /// A doorway with no warmup task at all must not be 503'd for an empty
    /// warmup it was never asked to run — `None` is a real third answer.
    #[test]
    fn no_warmup_task_is_not_judged_empty() {
        let state = dev_state();
        discover_role(&state, "imagodei");
        let state = Arc::new(state);
        let serving = ServingHealth::observe_with_freshness(&state);
        assert_eq!(serving.warmup_empty, None);
        assert_eq!(serving_check(state).status(), StatusCode::OK);
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
        // This test is about the BREAKER regimes; give it a discovered role so
        // the conductor-blind gate (added 2026-08-21) is not what it measures.
        discover_role(&state, "imagodei");
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
            sync_paused: Some(true),
            sync_reasons: Some(vec!["drain-backlog".to_string()]),
            observed_age_ms: Some(4_000),
            stale: false,
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["caughtUp"], serde_json::json!(true));
        assert_eq!(json["converged"], serde_json::json!(false));
        assert_eq!(json["observedAgeMs"], serde_json::json!(4_000));
        assert_eq!(json["stale"], serde_json::json!(false));
        assert_eq!(json["syncPaused"], serde_json::json!(true));
        assert_eq!(json["syncReasons"], serde_json::json!(["drain-backlog"]));
    }

    #[test]
    fn health_emits_a_stale_p2p_block_when_no_snapshot_is_cached() {
        let state = test_state();
        let response = build_health_response(&state);
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["p2p"]["stale"], serde_json::json!(true));
        assert!(json["p2p"].get("observedAgeMs").is_none());
    }

    // ---------------------------------------------------------------------
    // HONEST SIBLING CLASSIFICATION (habits: doorway-failover, red a).
    //
    // `shedding`/`degrading` were `any()` over the WHOLE breaker map — every
    // endpoint this doorway had ever called. A failover drill that opened a
    // POOL peer's circuit (`:8091`) therefore demoted a doorway whose declared
    // primary was closed and answering: `/health` said `degraded`,
    // `/health/serving` said 503, and a sibling reading that could not tell a
    // shedding doorway from a serving one. These pin the scoping.
    // ---------------------------------------------------------------------

    const PRIMARY_EP: &str = "http://primary:8090";
    const POOL_EP: &str = "http://pool:8091";

    /// A state whose DECLARED primary is [`PRIMARY_EP`] (rank 0), with one
    /// lower-ranked pool peer — the shape every alpha doorway runs in.
    fn pooled_state() -> AppState {
        let args = Args::parse_from([
            "doorway",
            "--listen",
            "127.0.0.1:0",
            "--dev-mode",
            "--storage-url",
            PRIMARY_EP,
            "--storage-urls",
            POOL_EP,
        ]);
        AppState::new(args)
    }

    fn upstream_of<'a>(serving: &'a ServingHealth, endpoint: &str) -> &'a ServingUpstream {
        serving
            .upstreams
            .iter()
            .find(|u| u.endpoint == endpoint)
            .unwrap_or_else(|| panic!("{endpoint} must still be reported"))
    }

    #[test]
    fn a_pool_peers_open_circuit_never_demotes_the_doorway() {
        let state = pooled_state();
        discover_role(&state, "imagodei");
        open_the_breaker(&state, POOL_EP);
        // The primary has answered — a closed circuit with a clean streak.
        state.upstream_breakers.record(PRIMARY_EP, true);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert!(
            !serving.shedding,
            "the DECLARED primary is closed — this doorway is serving, not shedding"
        );
        assert!(
            !serving.degrading,
            "a pool peer's error streak is not this doorway's serving regime"
        );

        // Observability WIDENS: the pool peer's open circuit is still reported,
        // now labelled.
        let pool = upstream_of(&serving, POOL_EP);
        assert_eq!(pool.circuit, "open", "the pool circuit is reported in full");
        assert!(
            pool.shedding,
            "that endpoint IS shedding — say so per-upstream"
        );
        assert_eq!(pool.role, "pool");
        assert_eq!(upstream_of(&serving, PRIMARY_EP).role, "primary");

        assert_eq!(
            serving_check(Arc::new(state)).status(),
            StatusCode::OK,
            "a doorway serving off a healthy primary must not 503 for a pool drill"
        );
    }

    #[test]
    fn the_declared_primary_shedding_still_sheds_the_doorway() {
        let state = pooled_state();
        discover_role(&state, "imagodei");
        open_the_breaker(&state, PRIMARY_EP);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert!(
            serving.shedding,
            "the primary is open — this doorway cannot serve, and must say so"
        );
        assert_eq!(upstream_of(&serving, PRIMARY_EP).role, "primary");
        assert_eq!(
            serving_check(Arc::new(state)).status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn the_declared_primarys_error_streak_still_reads_degrading() {
        let state = pooled_state();
        discover_role(&state, "imagodei");
        // ONE failure: below `UPSTREAM_CIRCUIT_FAIL_THRESHOLD`, so the circuit
        // stays CLOSED — the blind, expensive regime `degrading` exists for.
        state.upstream_breakers.record(PRIMARY_EP, false);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert!(!serving.shedding, "still closed: not shedding");
        assert!(
            serving.degrading,
            "a non-zero streak on the PRIMARY is the 20-second-page regime"
        );
        assert_eq!(
            serving_check(Arc::new(state)).status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn an_unranked_endpoint_counts_as_pool_never_as_primary() {
        let state = pooled_state();
        discover_role(&state, "imagodei");
        // A peer the registry discovered at runtime — in the breaker map, in
        // NO declaration. Failing open, it must not speak for the doorway.
        let stranger = "http://discovered-later:8090";
        open_the_breaker(&state, stranger);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert_eq!(upstream_of(&serving, stranger).role, "pool");
        assert!(
            !serving.shedding,
            "an unranked endpoint is pool, not primary"
        );
        assert!(!serving.degrading);
    }

    #[test]
    fn a_doorway_with_no_declared_peer_keeps_the_pool_wide_verdict() {
        // No `--storage-url`, no `--storage-urls`: there is no basis to pick a
        // primary, and going blind would be worse than over-reporting. This is
        // the pre-scoping behaviour, kept deliberately.
        let state = test_state();
        open_the_breaker(&state, POOL_EP);

        let serving = ServingHealth::observe_with_freshness(&state);
        assert!(serving.shedding, "undeclared: every endpoint still counts");
        assert_eq!(upstream_of(&serving, POOL_EP).role, "undeclared");
    }

    #[test]
    fn the_primary_lookup_normalizes_the_trailing_slash() {
        // The breaker map is keyed by the URL the proxy called with; the
        // declaration may carry a trailing slash. A key mismatch here would
        // silently classify EVERYTHING as pool and the doorway would never
        // report a shed again.
        let breakers = crate::routes::UpstreamBreakers::default();
        for _ in 0..crate::routes::upstream_health::UPSTREAM_CIRCUIT_FAIL_THRESHOLD {
            breakers.record(PRIMARY_EP, false);
        }
        let serving = ServingHealth::observe(&breakers, Some("http://primary:8090/"));
        assert_eq!(upstream_of(&serving, PRIMARY_EP).role, "primary");
        assert!(serving.shedding, "trailing slash must not lose the primary");
    }
}
