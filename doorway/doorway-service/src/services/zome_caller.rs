//! ZomeCaller - Signed zome call client using official holochain_client
//!
//! Makes signed zome calls to the Holochain conductor via AppWebsocket.
//! Used by:
//! - Federation service (register_doorway, record_heartbeat, find_publishers)
//! - Identity management (create_human via auth routes)
//!
//! ## Auth & Signing Flow
//! 1. Connect to admin interface, list apps to find cell_id
//! 2. Authorize signing credentials for the cell
//! 3. Issue app auth token, connect AppWebsocket with signer
//! 4. All zome calls are automatically signed by the client
//!
//! ## Conductor failover (single-peer SPOF removal)
//!
//! LIVE-INCIDENT INVARIANT (2026-08-05/06 doorway-B federation outage): the
//! doorway pins this caller to ONE conductor (`--conductor-url` /
//! `CONDUCTOR_ADMIN_URL`). When that peer got sick — adam under sqlite
//! write-guard pressure, `authorize_signing_credentials timed out after 10000ms
//! for role 'infrastructure'` — `GET /api/v1/federation/doorways` timed out
//! CONTINUOUSLY for 5.5 hours. One sick peer took down a doorway API route.
//!
//! The cure has three parts, and the third is the crux:
//!
//! 1. **An ordered fallback list**, taken from the conductor pool the doorway
//!    already declares (`CONDUCTOR_URLS`) — no new config schema.
//! 2. **A primary cooldown.** Trying the sick primary first on every call would
//!    still spend the full 10s deadline per request, and the browser aborts at
//!    ~5s — so the route would stay dead. After an availability failure the
//!    primary is skipped for [`PRIMARY_UNHEALTHY_COOLDOWN`] and re-probed after
//!    (half-open), so recovery is automatic with no flap logic.
//! 3. **Signing credentials are PER-CONDUCTOR.** `authorize_signing_credentials`
//!    grants a cap on *that* conductor's cells for *that* conductor's agent; a
//!    credential from the primary is meaningless on a fallback. So every
//!    endpoint is connected through the one shared [`connect_endpoint`] routine,
//!    which builds a FRESH `ClientAgentSigner` and authorizes against that
//!    endpoint's OWN admin interface. There is no shared signer on `ZomeCaller`
//!    — that absence is load-bearing, not incidental.
//!
//! ## What may fail over, and what may NOT
//!
//! Failing over changes WHICH AGENT signs the call, because each conductor has
//! its own agent key and its own source chain. That is harmless for
//! agent-agnostic DHT reads (`get_all_doorways`, `find_publishers` — any peer
//! answers identically) and unacceptable for identity-binding writes
//! (`create_human` binds the Human to the acting agent; `register_doorway` /
//! `record_heartbeat` author the doorway's own registration).
//!
//! So failover is OPT-IN at the call site: [`ZomeCaller::call`] /
//! [`ZomeCaller::call_zome`] stay primary-pinned (unchanged semantics), and
//! [`ZomeCaller::call_failover`] / [`ZomeCaller::call_zome_failover`] are the
//! read-path variants. Do not move a write onto the failover variants without
//! first answering what a second author does to that entry's validation.

use holochain_client::{
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
    ConductorApiError, ZomeCallTarget,
};
use holochain_types::prelude::ExternIO;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Default hard deadline for any single conductor interaction (connect, list,
/// authorize, token issue, or zome call).
///
/// LIVE-INCIDENT INVARIANT (2026-06-13 doorway freeze): a conductor lookup that
/// hangs forever — `Failed to connect to conductor: Name or service not known`
/// (intermittent cluster NXDOMAIN), or a `record_heartbeat` zome call that never
/// settles — must NOT be able to wedge the caller. On a cpu=1 pod tokio runs few
/// worker threads, and one synchronously-blocked await freezes the gateway
/// (`/health` included). Bounding every conductor future here is the substrate's
/// promise that an unreachable conductor degrades to an `Err`, never to a hang.
///
/// Parameter-bearing discovery: this deadline is the per-conductor-call SLA. Too
/// low and a slow-but-healthy conductor flaps; too high and a dead conductor
/// holds a connection slot. 10s mirrors the existing `discover_existing_agents`
/// connect timeout (`main.rs`) and the SSR reqwest client timeout.
const DEFAULT_ZOME_CALL_TIMEOUT_MS: u64 = 10_000;

/// Read the conductor-call hard deadline from `DOORWAY_ZOME_CALL_TIMEOUT_MS`,
/// falling back to [`DEFAULT_ZOME_CALL_TIMEOUT_MS`]. A `0` value or an unparseable
/// value falls back to the default (never "no timeout" — that is the bug class
/// this guards against).
///
/// `pub` so the binary crate (`main.rs::discover_existing_agents`) can bound its
/// startup `list_apps` probe with the same SLA — a conductor that accepts the
/// connection then stalls must not wedge boot for the full startup budget.
pub fn zome_call_timeout() -> Duration {
    let ms = std::env::var("DOORWAY_ZOME_CALL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_ZOME_CALL_TIMEOUT_MS);
    Duration::from_millis(ms)
}

/// How long the primary conductor is skipped after an availability failure.
///
/// Parameter-bearing discovery (2026-08-05 doorway-B outage): the naive "always
/// try primary first" failover is a NO-OP for a browser client. The primary
/// deadline is 10s; the SPA aborts at ~4.95s. So every request would still be
/// user-visibly dead while the primary is sick, even with a healthy fallback
/// standing by. The cooldown is what converts failover from theory into a served
/// response — for requests INSIDE a cooldown window. Boundary requests still pay
/// the primary's price: the first request to hit a freshly-degraded primary eats
/// the full primary deadline before the cooldown starts, and after each cooldown
/// expiry one request eats the half-open re-probe. Expect intermittent
/// client-visible timeouts at those boundaries, not zero.
///
/// 60s is chosen so a transient conductor stall costs at most one cooled minute
/// of primary-affinity (the fallback answers reads identically), while a peer
/// that has genuinely recovered is re-probed promptly. Deliberately a const, not
/// an env var: it is an availability invariant, not an operator tuning knob.
const PRIMARY_UNHEALTHY_COOLDOWN: Duration = Duration::from_secs(60);

/// Why a conductor call failed, at the only granularity failover cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFailureClass {
    /// The conductor could not answer — unreachable, unresponsive, dead socket,
    /// or any step of the connect/authorize handshake timing out. ANOTHER
    /// conductor may well be able to answer, so this is the failover trigger.
    Unavailable,
    /// The conductor DID answer, with an error (validation, missing function,
    /// wire error). Every peer in the pool would answer the same way, so failing
    /// over just relocates the identical error and doubles the load. Never fail
    /// over on this class.
    Application,
}

/// A conductor failure carrying both its class (for routing) and its message
/// (for the caller). The public API still returns `String`, so downstream
/// string matching — e.g. federation's `e.contains("already exists")` — keeps
/// working byte-identically.
#[derive(Debug, Clone)]
struct CallError {
    class: CallFailureClass,
    message: String,
}

impl CallError {
    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            class: CallFailureClass::Unavailable,
            message: message.into(),
        }
    }
}

/// One conductor's pair of WebSocket interfaces.
///
/// The admin address is what makes per-conductor signing credentials possible:
/// `authorize_signing_credentials` must be issued against the SAME conductor
/// that will later verify the signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorEndpoint {
    pub admin_addr: String,
    pub app_addr: String,
}

impl ConductorEndpoint {
    /// Build from an admin URL and an app URL (either may carry a `ws://` scheme).
    pub fn new(admin_url: &str, app_url: &str) -> Self {
        Self {
            admin_addr: strip_ws_scheme(admin_url),
            app_addr: strip_ws_scheme(app_url),
        }
    }

    /// Build from an app URL alone, deriving the admin interface by the
    /// established port-minus-one convention (8445 app → 8444 admin).
    ///
    /// This is the same derivation `main.rs::discover_existing_agents` already
    /// applies to `CONDUCTOR_URLS` entries, so the fallback list needs no new
    /// configuration — each fallback gets its OWN admin interface, which is
    /// exactly what per-conductor credential authorization requires.
    pub fn from_app_url(app_url: &str) -> Self {
        Self::new(&crate::derive_admin_url_from_app(app_url), app_url)
    }
}

/// Build the ordered fallback list for a primary endpoint from the doorway's
/// declared conductor pool.
///
/// Drops the primary (it is tried first by construction) and any duplicates,
/// preserving pool order so the operator's declared preference is honored.
pub fn build_fallback_endpoints(
    primary: &ConductorEndpoint,
    pool_app_urls: &[String],
) -> Vec<ConductorEndpoint> {
    let mut out: Vec<ConductorEndpoint> = Vec::new();
    for url in pool_app_urls {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let ep = ConductorEndpoint::from_app_url(url);
        if ep.app_addr == primary.app_addr {
            continue;
        }
        if out.iter().any(|e| e.app_addr == ep.app_addr) {
            continue;
        }
        out.push(ep);
    }
    out
}

/// Classify a [`ConductorApiError`] as a TRANSPORT failure (the socket is
/// genuinely dead → reconnect) versus an APPLICATION-level error (the conductor
/// answered → the socket is healthy → keep the connection).
///
/// LIVE-INCIDENT INVARIANT (2026-06-13 doorway freeze, iteration 3 — the
/// connection-churn root fix): `call_zome` used to clear the connection
/// (`*ws = None` → reconnect) on ANY `Err`. So any periodic conductor call that
/// returned a zome-level `Err` — `find_publishers`/`register_doorway`/
/// `record_health_attestation` validation failures, a "function doesn't exist"
/// during a coordinator-zome version skew — forced a needless reconnect. That
/// reconnect churn (NXDOMAIN / WebSocket-reset cycle seen in Loki) piled SSR
/// conductor-fetches onto the bounded worker pool and wedged the gateway under
/// load. The cure: clear ONLY on a dead socket.
///
/// Structured as default-KEEP: only the genuinely-transport variants return
/// `true`. The two misclassification directions are asymmetric — clearing on an
/// application error is exactly the churn bug we are removing (expensive and
/// self-reinforcing under load); keeping on a transport error self-corrects,
/// because a truly dead socket returns `WebsocketError`/`IoError` again on the
/// very next call and clears then. Bias toward keep, so a future-added
/// `ConductorApiError` variant defaults to no-churn.
///
/// - TRANSPORT (clear): `WebsocketError` (socket-level failure / reset),
///   `IoError` (connection reset/refused, IO failure). The call TIMEOUT
///   (`tokio::time::timeout` elapse) is also transport, but it is an
///   `Elapsed` — not a `ConductorApiError` — so it is handled in its own arm in
///   `call_zome` and never routes through here.
/// - APPLICATION (keep): `ExternalApiWireError(_)` — the conductor responded
///   with a wire-level error (`RibosomeError`/`InternalError`/validation/
///   "function doesn't exist"/auth), so the socket is healthy. Also
///   `CellNotFound`, `AppNotFound`, `SignZomeCallError`, `FreshNonceError` —
///   local/pre-call errors where the socket is fine.
fn is_transport_error(e: &ConductorApiError) -> bool {
    matches!(
        e,
        ConductorApiError::WebsocketError(_) | ConductorApiError::IoError(_)
    )
}

/// Generic zome call client with signed requests.
///
/// Follows the same pattern as elohim-storage's HcClient — uses the official
/// holochain_client::AppWebsocket which handles request signing automatically.
pub struct ZomeCaller {
    /// The pinned primary conductor. Every call tries this first (unless it is
    /// in availability cooldown), and identity-binding writes NEVER leave it.
    primary: ConductorEndpoint,
    /// Ordered fallback conductors, primary excluded and deduped. Empty for a
    /// single-conductor caller, in which case behavior is byte-identical to the
    /// pre-failover implementation.
    fallbacks: Vec<ConductorEndpoint>,
    installed_app_id: String,
    /// The connected AppWebsocket for [`Self::primary`] (lazily initialized)
    app_ws: RwLock<Option<AppWebsocket>>,
    /// Lock to prevent concurrent connection attempts
    connecting: Mutex<()>,
    /// Lazily-established fallback connection, tagged with WHICH fallback it is.
    ///
    /// Cached deliberately: re-running the connect handshake per call would
    /// re-issue `authorize_signing_credentials` — a conductor-side cap-grant
    /// write — on every federation poll for the whole duration of a primary
    /// outage, converting one sick peer into load on a healthy one.
    fallback_ws: RwLock<Option<(usize, AppWebsocket)>>,
    /// Lock to prevent concurrent fallback connection attempts
    fallback_connecting: Mutex<()>,
    /// Which fallback to try first (round-robins away from a failing one).
    fallback_idx: AtomicUsize,
    /// While set and in the future, the primary is skipped. See
    /// [`PRIMARY_UNHEALTHY_COOLDOWN`].
    primary_unhealthy_until: RwLock<Option<tokio::time::Instant>>,
}

impl ZomeCaller {
    /// Create a single-conductor ZomeCaller (no failover).
    ///
    /// `admin_url` and `app_url` can be either `ws://host:port` URLs or `host:port` addresses.
    ///
    /// Used for the deliberately-pinned temporary callers (hosted registration
    /// on a specific operator conductor), where routing to a different peer
    /// would be a correctness bug rather than a resilience win.
    pub fn new(admin_url: &str, app_url: &str, installed_app_id: &str) -> Self {
        Self::with_fallbacks(admin_url, app_url, installed_app_id, &[])
    }

    /// Create a ZomeCaller with an ordered fallback conductor pool.
    ///
    /// `pool_app_urls` is the doorway's already-declared conductor pool
    /// (`CONDUCTOR_URLS`); the primary is filtered out of it automatically, and
    /// each remaining entry derives its own admin interface so signing
    /// credentials are authorized per-conductor.
    pub fn with_fallbacks(
        admin_url: &str,
        app_url: &str,
        installed_app_id: &str,
        pool_app_urls: &[String],
    ) -> Self {
        let primary = ConductorEndpoint::new(admin_url, app_url);
        let fallbacks = build_fallback_endpoints(&primary, pool_app_urls);
        info!(
            admin_url = %admin_url,
            app_url = %app_url,
            installed_app_id = %installed_app_id,
            fallback_conductors = fallbacks.len(),
            "ZomeCaller created"
        );
        for (i, ep) in fallbacks.iter().enumerate() {
            debug!(index = i, app = %ep.app_addr, admin = %ep.admin_addr, "ZomeCaller fallback conductor");
        }
        Self {
            primary,
            fallbacks,
            installed_app_id: installed_app_id.to_string(),
            app_ws: RwLock::new(None),
            connecting: Mutex::new(()),
            fallback_ws: RwLock::new(None),
            fallback_connecting: Mutex::new(()),
            fallback_idx: AtomicUsize::new(0),
            primary_unhealthy_until: RwLock::new(None),
        }
    }

    /// Number of configured fallback conductors (0 = no failover available).
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.len()
    }

    /// Get or create the AppWebsocket connection with signing credentials.
    async fn ensure_connected(&self) -> Result<(), String> {
        // Fast path: already connected
        {
            let ws = self.app_ws.read().await;
            if ws.is_some() {
                return Ok(());
            }
        }

        // Slow path: connect with signing credentials
        let _lock = self.connecting.lock().await;

        // Double-check after acquiring lock
        {
            let ws = self.app_ws.read().await;
            if ws.is_some() {
                return Ok(());
            }
        }

        info!(conductor = %self.primary.app_addr, "ZomeCaller connecting to conductor");

        let deadline = zome_call_timeout();

        // Overall reconnect budget. The slow path runs up to 5 sequential
        // conductor steps, each bounded by `deadline` — so the worst case is
        // ~5×deadline (≈50s at the 10s default) while the `connecting` mutex is
        // held. Under a full cluster NXDOMAIN every step times out in sequence,
        // and concurrent callers serialize behind this mutex for the whole
        // stretch. Bounding the entire body (additively to the per-step
        // timeouts) guarantees the mutex releases promptly so queued callers get
        // a fast `Err` instead of parking for the full ~50s. `_lock` lives in the
        // outer scope, so any early return drops it immediately; `app_ws` is only
        // written at the very end, so a mid-reconnect elapse leaves it cleanly
        // `None` for the next caller.
        let reconnect_budget = reconnect_budget(deadline);

        // Enforce the overall budget. On elapse, return promptly so `_lock`
        // drops and queued callers stop serializing behind a doomed reconnect.
        let connected = match tokio::time::timeout(
            reconnect_budget,
            connect_endpoint(&self.installed_app_id, &self.primary, deadline),
        )
        .await
        {
            Ok(inner) => inner?,
            Err(_elapsed) => {
                warn!(
                    budget_ms = reconnect_budget.as_millis(),
                    "ZomeCaller reconnect exceeded full budget; releasing connecting lock"
                );
                return Err(format!(
                    "reconnect exceeded full budget after {}ms",
                    reconnect_budget.as_millis()
                ));
            }
        };

        let mut ws = self.app_ws.write().await;
        *ws = Some(connected);
        Ok(())
    }

    /// Ensure the cached fallback connection targets `idx`, connecting if not.
    ///
    /// Each fallback gets its own [`connect_endpoint`] run — i.e. its own
    /// `ClientAgentSigner` and its own `authorize_signing_credentials` against
    /// its own admin interface. A credential minted on the primary is worthless
    /// here, and nothing in this path reuses one.
    async fn ensure_fallback_connected(
        &self,
        idx: usize,
        deadline: Duration,
    ) -> Result<(), String> {
        {
            let ws = self.fallback_ws.read().await;
            if matches!(ws.as_ref(), Some((i, _)) if *i == idx) {
                return Ok(());
            }
        }

        let _lock = self.fallback_connecting.lock().await;

        {
            let ws = self.fallback_ws.read().await;
            if matches!(ws.as_ref(), Some((i, _)) if *i == idx) {
                return Ok(());
            }
        }

        let endpoint = self
            .fallbacks
            .get(idx)
            .ok_or_else(|| format!("fallback index {idx} out of range"))?;

        info!(
            conductor = %endpoint.app_addr,
            index = idx,
            "ZomeCaller connecting to FALLBACK conductor (primary unavailable)"
        );

        let budget = reconnect_budget(deadline);
        let connected = match tokio::time::timeout(
            budget,
            connect_endpoint(&self.installed_app_id, endpoint, deadline),
        )
        .await
        {
            Ok(inner) => inner?,
            Err(_elapsed) => {
                warn!(
                    budget_ms = budget.as_millis(),
                    conductor = %endpoint.app_addr,
                    "Fallback reconnect exceeded full budget; releasing connecting lock"
                );
                return Err(format!(
                    "fallback reconnect exceeded full budget after {}ms",
                    budget.as_millis()
                ));
            }
        };

        let mut ws = self.fallback_ws.write().await;
        *ws = Some((idx, connected));
        Ok(())
    }

    /// Call a zome function on the PINNED PRIMARY conductor.
    ///
    /// No failover. This is the correct entry point for identity-binding writes
    /// (`create_human`, `register_doorway`, `record_heartbeat`) where changing
    /// the signing agent would change what the entry means. For agent-agnostic
    /// DHT reads use [`Self::call_zome_failover`].
    ///
    /// The AppWebsocket handles signing automatically.
    pub async fn call_zome(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        self.call_zome_primary(role_name, zome_name, fn_name, payload)
            .await
            .map_err(|e| e.message)
    }

    /// Primary-pinned call, retaining the failure classification for routing.
    async fn call_zome_primary(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, CallError> {
        // Every connect/authorize/token failure is an availability failure: the
        // conductor could not be brought to a usable state. Another peer may
        // still answer, so this class is what triggers failover.
        self.ensure_connected()
            .await
            .map_err(CallError::unavailable)?;

        debug!(
            role_name = %role_name,
            zome_name = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            "ZomeCaller making signed zome call"
        );

        let deadline = zome_call_timeout();

        // Hold read lock only for the call, then release.
        // The zome call is hard-bounded: a conductor that accepts the connection
        // but never answers (the `record_heartbeat` hang seen in the live freeze)
        // must surface as a timeout Err, not park the calling task forever.
        let result = {
            let ws = self.app_ws.read().await;
            let app_ws = ws
                .as_ref()
                .ok_or_else(|| CallError::unavailable("Not connected"))?;
            call_on_ws(app_ws, role_name, zome_name, fn_name, payload, deadline).await
        };

        match result {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                // Connection-churn root fix (2026-06-13 iteration 3): only a
                // genuinely-dead socket warrants a reconnect. A zome-returned
                // application error (the conductor answered) leaves the socket
                // healthy — clearing it here is the churn that wedged the
                // gateway under load. Split warn! so Loki distinguishes the two.
                if e.class == CallFailureClass::Unavailable {
                    warn!(
                        role_name = %role_name,
                        fn_name = %fn_name,
                        "Zome call unavailable, clearing connection: {}", e.message
                    );
                    let mut ws = self.app_ws.write().await;
                    *ws = None;
                } else {
                    warn!(
                        role_name = %role_name,
                        fn_name = %fn_name,
                        "Zome call returned application error, keeping connection: {}", e.message
                    );
                }
                Err(e)
            }
        }
    }

    /// Call a zome function, failing over to another configured conductor when
    /// the primary cannot answer.
    ///
    /// ONLY for agent-agnostic DHT reads — see the module header. A failed-over
    /// call is signed by a DIFFERENT agent, which is meaningless for a read and
    /// semantically wrong for a write.
    ///
    /// Routing:
    /// - No fallbacks configured → identical to [`Self::call_zome`].
    /// - Primary in availability cooldown → skipped entirely (this is what makes
    ///   the failover visible to a client that aborts at ~5s).
    /// - Primary answers with an application error → returned as-is; every peer
    ///   would answer the same, so failing over would only relocate the error.
    pub async fn call_zome_failover(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        if self.fallbacks.is_empty() {
            return self.call_zome(role_name, zome_name, fn_name, payload).await;
        }

        let mut primary_error: Option<CallError> = None;

        if !self.primary_in_cooldown().await {
            match self
                .call_zome_primary(role_name, zome_name, fn_name, payload.clone())
                .await
            {
                Ok(bytes) => {
                    self.clear_primary_cooldown().await;
                    return Ok(bytes);
                }
                Err(e) if e.class == CallFailureClass::Application => return Err(e.message),
                Err(e) => {
                    warn!(
                        conductor = %self.primary.app_addr,
                        role_name = %role_name,
                        fn_name = %fn_name,
                        cooldown_s = PRIMARY_UNHEALTHY_COOLDOWN.as_secs(),
                        "Primary conductor unavailable, failing over: {}", e.message
                    );
                    self.mark_primary_unhealthy().await;
                    primary_error = Some(e);
                }
            }
        } else {
            debug!(
                conductor = %self.primary.app_addr,
                "Primary conductor in availability cooldown, routing straight to fallback"
            );
        }

        match self
            .call_zome_via_fallback(role_name, zome_name, fn_name, payload)
            .await
        {
            Ok(bytes) => Ok(bytes),
            Err(fallback_error) => Err(match primary_error {
                Some(pe) => format!(
                    "{}; every fallback conductor also failed: {}",
                    pe.message, fallback_error.message
                ),
                None => format!(
                    "primary conductor in availability cooldown; every fallback conductor also failed: {}",
                    fallback_error.message
                ),
            }),
        }
    }

    /// Try each configured fallback conductor in turn, starting where the last
    /// failure left off.
    ///
    /// Stops early on an application error — the conductor answered, and every
    /// other peer would answer identically.
    async fn call_zome_via_fallback(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, CallError> {
        let deadline = zome_call_timeout();
        let count = self.fallbacks.len();
        let mut last: Option<CallError> = None;

        for _ in 0..count {
            let idx = self.fallback_idx.load(Ordering::Relaxed) % count;

            if let Err(e) = self.ensure_fallback_connected(idx, deadline).await {
                warn!(
                    conductor = %self.fallbacks[idx].app_addr,
                    "Fallback conductor connect failed: {e}"
                );
                self.advance_fallback(idx, count);
                last = Some(CallError::unavailable(e));
                continue;
            }

            let result = {
                let ws = self.fallback_ws.read().await;
                match ws.as_ref() {
                    Some((i, app_ws)) if *i == idx => {
                        call_on_ws(
                            app_ws,
                            role_name,
                            zome_name,
                            fn_name,
                            payload.clone(),
                            deadline,
                        )
                        .await
                    }
                    // A concurrent caller rotated the slot out from under us.
                    // Treat as unavailable and try the next endpoint.
                    _ => Err(CallError::unavailable("fallback connection was replaced")),
                }
            };

            match result {
                Ok(bytes) => {
                    info!(
                        conductor = %self.fallbacks[idx].app_addr,
                        role_name = %role_name,
                        fn_name = %fn_name,
                        "Served from FALLBACK conductor (primary unavailable)"
                    );
                    return Ok(bytes);
                }
                Err(e) if e.class == CallFailureClass::Application => return Err(e),
                Err(e) => {
                    warn!(
                        conductor = %self.fallbacks[idx].app_addr,
                        "Fallback conductor unavailable: {}", e.message
                    );
                    {
                        let mut ws = self.fallback_ws.write().await;
                        if matches!(ws.as_ref(), Some((i, _)) if *i == idx) {
                            *ws = None;
                        }
                    }
                    self.advance_fallback(idx, count);
                    last = Some(e);
                }
            }
        }

        Err(last.unwrap_or_else(|| CallError::unavailable("no fallback conductors configured")))
    }

    fn advance_fallback(&self, idx: usize, count: usize) {
        self.fallback_idx
            .store((idx + 1) % count, Ordering::Relaxed);
    }

    /// Whether the primary is currently being skipped for availability reasons.
    pub async fn primary_in_cooldown(&self) -> bool {
        match *self.primary_unhealthy_until.read().await {
            Some(until) => tokio::time::Instant::now() < until,
            None => false,
        }
    }

    async fn mark_primary_unhealthy(&self) {
        let mut guard = self.primary_unhealthy_until.write().await;
        *guard = Some(tokio::time::Instant::now() + PRIMARY_UNHEALTHY_COOLDOWN);
    }

    async fn clear_primary_cooldown(&self) {
        let mut guard = self.primary_unhealthy_until.write().await;
        if guard.is_some() {
            info!(
                conductor = %self.primary.app_addr,
                "Primary conductor healthy again, clearing availability cooldown"
            );
            *guard = None;
        }
    }

    /// Typed wrapper: serialize input with MessagePack, deserialize output.
    ///
    /// Primary-pinned — see [`Self::call_zome`].
    pub async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        input: &I,
    ) -> Result<O, String> {
        let payload = encode_input(input)?;
        let response_bytes = self
            .call_zome(role_name, zome_name, fn_name, payload)
            .await?;
        decode_response(&response_bytes)
    }

    /// Typed wrapper with conductor failover — see [`Self::call_zome_failover`].
    ///
    /// ONLY for agent-agnostic DHT reads.
    pub async fn call_failover<I: Serialize, O: DeserializeOwned>(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        input: &I,
    ) -> Result<O, String> {
        let payload = encode_input(input)?;
        let response_bytes = self
            .call_zome_failover(role_name, zome_name, fn_name, payload)
            .await?;
        decode_response(&response_bytes)
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        self.app_ws.read().await.is_some()
    }
}

/// Overall budget for one endpoint's connect handshake.
///
/// The slow path runs up to 5 sequential conductor steps, each bounded by
/// `deadline` — so the worst case is ~5×deadline (≈50s at the 10s default)
/// while a `connecting` mutex is held. Bounding the entire body (additively to
/// the per-step timeouts) guarantees the mutex releases promptly so queued
/// callers get a fast `Err` instead of parking for the full stretch.
fn reconnect_budget(deadline: Duration) -> Duration {
    deadline * 5 + Duration::from_secs(1)
}

/// Connect to ONE conductor and return a signing-capable AppWebsocket.
///
/// THE PER-CONDUCTOR CREDENTIAL CRUX. Signing credentials are scoped to a
/// conductor's cells and agent: `authorize_signing_credentials` issues a cap
/// grant on THAT conductor, and only THAT conductor will verify the resulting
/// signature. So this routine builds a FRESH [`ClientAgentSigner`] and
/// authorizes against `endpoint.admin_addr` every time it runs — a fallback
/// conductor never borrows the primary's credentials, because there is nothing
/// to borrow: the signer is a local, per-connection value that dies with the
/// socket. Both the primary and every fallback go through this one function so
/// the invariant cannot drift apart between the two paths.
async fn connect_endpoint(
    installed_app_id: &str,
    endpoint: &ConductorEndpoint,
    deadline: Duration,
) -> Result<AppWebsocket, String> {
    // Step 1: Connect to admin interface.
    // Hard-bounded: an unresolvable conductor host (cluster NXDOMAIN) must
    // resolve to an Err within `deadline`, never hang the caller forever.
    // Resolve via tokio's ASYNC resolver first (off the worker pool), then
    // connect with an already-resolved SocketAddr so holochain_client's
    // synchronous std::net getaddrinfo never parks a worker (alpha doorway
    // crashloop RCA 2026-06-15; see crate::conductor::resolve_host_port).
    // The per-step `timeout` below can only bound an async resolve, not a
    // blocking syscall — so this also makes that timeout effective.
    let admin_socket = crate::conductor::resolve_host_port(&endpoint.admin_addr).await?;
    let admin_ws = tokio::time::timeout(
        deadline,
        AdminWebsocket::connect(admin_socket, Some(String::from("doorway-zome-caller"))),
    )
    .await
    .map_err(|_| {
        format!(
            "Admin connect timed out after {}ms ({})",
            deadline.as_millis(),
            endpoint.admin_addr
        )
    })?
    .map_err(|e| format!("Admin connect failed: {e}"))?;

    info!("ZomeCaller connected to admin at {}", endpoint.admin_addr);

    // Step 2: Find cell_id from app info
    let apps = tokio::time::timeout(deadline, admin_ws.list_apps(None))
        .await
        .map_err(|_| format!("list_apps timed out after {}ms", deadline.as_millis()))?
        .map_err(|e| format!("list_apps failed: {e}"))?;

    let app_info = apps
        .iter()
        .find(|a| a.installed_app_id == installed_app_id)
        .ok_or_else(|| format!("App '{installed_app_id}' not found"))?;

    // Step 3: Authorize signing credentials for ALL provisioned cells OF THIS
    // CONDUCTOR. A fresh signer per connection — never shared across endpoints.
    let signer = ClientAgentSigner::default();
    let mut cell_count = 0u32;

    for (role_name, cells) in &app_info.cell_info {
        for cell in cells {
            if let holochain_client::CellInfo::Provisioned(p) = cell {
                let credentials = tokio::time::timeout(
                    deadline,
                    admin_ws.authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                        cell_id: p.cell_id.clone(),
                        functions: None,
                    }),
                )
                .await
                .map_err(|_| {
                    format!(
                        "authorize_signing_credentials timed out after {}ms for role '{role_name}'",
                        deadline.as_millis()
                    )
                })?
                .map_err(|e| {
                    format!("authorize_signing_credentials failed for role '{role_name}': {e}")
                })?;

                signer.add_credentials(p.cell_id.clone(), credentials);
                cell_count += 1;
                debug!(role = %role_name, "Authorized signing for cell");
            }
        }
    }

    if cell_count == 0 {
        return Err("No provisioned cells found".to_string());
    }

    info!(
        app_id = %installed_app_id,
        conductor = %endpoint.admin_addr,
        cells = cell_count,
        "Signing credentials authorized for all cells"
    );

    // Step 4: Issue app auth token
    let token = tokio::time::timeout(
        deadline,
        admin_ws.issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
            installed_app_id: installed_app_id.to_string(),
            expiry_seconds: 3600,
            single_use: false,
        }),
    )
    .await
    .map_err(|_| {
        format!(
            "issue_app_auth_token timed out after {}ms",
            deadline.as_millis()
        )
    })?
    .map_err(|e| format!("issue_app_auth_token failed: {e}"))?;

    // Step 5: Connect AppWebsocket with signer
    let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);
    let app_socket = crate::conductor::resolve_host_port(&endpoint.app_addr).await?;
    let app_ws = tokio::time::timeout(
        deadline,
        AppWebsocket::connect(app_socket, token.token, signer_arc, None),
    )
    .await
    .map_err(|_| {
        format!(
            "App WebSocket connect timed out after {}ms ({})",
            deadline.as_millis(),
            endpoint.app_addr
        )
    })?
    .map_err(|e| format!("App WebSocket connect failed: {e}"))?;

    info!(
        "ZomeCaller connected to app interface at {} with signing",
        endpoint.app_addr
    );

    Ok(app_ws)
}

/// Issue one bounded zome call over an established socket, classifying the
/// failure so the caller can decide whether another conductor could do better.
async fn call_on_ws(
    app_ws: &AppWebsocket,
    role_name: &str,
    zome_name: &str,
    fn_name: &str,
    payload: Vec<u8>,
    deadline: Duration,
) -> Result<Vec<u8>, CallError> {
    let result = tokio::time::timeout(
        deadline,
        app_ws.call_zome(
            ZomeCallTarget::RoleName(role_name.into()),
            zome_name.into(),
            fn_name.into(),
            ExternIO::from(payload),
        ),
    )
    .await;

    match result {
        // The conductor accepted the connection but never answered (the
        // `record_heartbeat` hang seen in the live freeze). Availability class.
        Err(_elapsed) => Err(CallError::unavailable(format!(
            "Zome call timed out after {}ms ({role_name}/{zome_name}/{fn_name})",
            deadline.as_millis()
        ))),
        Ok(Ok(extern_io)) => {
            debug!(
                result_len = extern_io.as_bytes().len(),
                "ZomeCaller zome call succeeded"
            );
            Ok(extern_io.into_vec())
        }
        Ok(Err(e)) => Err(CallError {
            class: if is_transport_error(&e) {
                CallFailureClass::Unavailable
            } else {
                CallFailureClass::Application
            },
            message: format!("Zome call failed: {e}"),
        }),
    }
}

fn encode_input<I: Serialize>(input: &I) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(input).map_err(|e| format!("Failed to serialize input: {e}"))
}

fn decode_response<O: DeserializeOwned>(response_bytes: &[u8]) -> Result<O, String> {
    rmp_serde::from_slice(response_bytes).map_err(|e| {
        let structure = match rmpv::decode::read_value(&mut std::io::Cursor::new(response_bytes)) {
            Ok(val) => format!("{val:?}"),
            Err(decode_err) => {
                format!(
                    "<raw {} bytes, decode err: {decode_err}>",
                    response_bytes.len()
                )
            }
        };
        format!(
            "Failed to deserialize response: {e} | response_bytes({} bytes) structure: {structure}",
            response_bytes.len()
        )
    })
}

/// Strip `ws://` or `wss://` scheme from a URL, returning just `host:port`.
///
/// `ToSocketAddrs` (used by holochain_client) needs `host:port`, not a URL.
fn strip_ws_scheme(url: &str) -> String {
    url.trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ws_scheme() {
        assert_eq!(strip_ws_scheme("ws://localhost:8445"), "localhost:8445");
        assert_eq!(strip_ws_scheme("wss://host:443"), "host:443");
        assert_eq!(strip_ws_scheme("localhost:8445"), "localhost:8445");
        assert_eq!(
            strip_ws_scheme("ws://elohim-matthew-alpha-0.headless:8445"),
            "elohim-matthew-alpha-0.headless:8445"
        );
    }

    // ── transport-vs-application error class (connection-churn root fix) ─────

    /// The load-bearing invariant for the 2026-06-13 doorway freeze, iteration 3
    /// (the connection-churn root fix): a ZOME-RETURNED application error must
    /// NOT clear the connection (the conductor answered → the socket is healthy),
    /// while a genuine TRANSPORT failure MUST. `call_zome`'s `Err(e)` arm gates
    /// the `*ws = None` reset on `is_transport_error`, so this directly pins the
    /// classification that decides clear-vs-keep.
    ///
    /// (The TIMEOUT case — the third transport class — is an `Elapsed`, not a
    /// `ConductorApiError`, so it has its own arm in `call_zome` and is exercised
    /// by `a_conductor_call_that_never_returns_errors_within_the_deadline` above.)
    #[test]
    fn application_errors_keep_the_connection() {
        use holochain_conductor_api::ExternalApiWireError;

        // "function that doesn't exist" — the named case from the incident.
        // A coordinator-zome version skew surfaces this as a RibosomeError; the
        // conductor answered, so the socket is healthy → KEEP.
        assert!(
            !is_transport_error(&ConductorApiError::ExternalApiWireError(
                ExternalApiWireError::RibosomeError(
                    "Zome function find_publishers doesn't exist".to_string()
                )
            )),
            "a zome-returned RibosomeError (function doesn't exist) must NOT clear the connection"
        );

        // A validation / internal error from the guest — still an answer.
        assert!(
            !is_transport_error(&ConductorApiError::ExternalApiWireError(
                ExternalApiWireError::InternalError("validation failed".to_string())
            )),
            "a zome-returned validation/internal error must NOT clear the connection"
        );

        // Local/pre-call routing errors: the socket is fine → KEEP.
        assert!(
            !is_transport_error(&ConductorApiError::CellNotFound),
            "CellNotFound is local routing, not a dead socket → keep"
        );
        assert!(
            !is_transport_error(&ConductorApiError::AppNotFound),
            "AppNotFound is local routing, not a dead socket → keep"
        );
        assert!(
            !is_transport_error(&ConductorApiError::SignZomeCallError(
                "no provenance".to_string()
            )),
            "a pre-call signing error means the socket was never used → keep"
        );
    }

    /// The other half of the invariant: a genuine transport failure (the socket
    /// is dead) MUST clear the connection so the next call reconnects.
    #[test]
    fn transport_errors_clear_the_connection() {
        use std::io::{Error as IoError, ErrorKind};

        // Connection reset — the canonical dead-socket signal.
        assert!(
            is_transport_error(&ConductorApiError::IoError(IoError::new(
                ErrorKind::ConnectionReset,
                "connection reset by peer"
            ))),
            "an IO connection-reset must clear the connection"
        );
        // Connection refused — conductor went away.
        assert!(
            is_transport_error(&ConductorApiError::IoError(IoError::new(
                ErrorKind::ConnectionRefused,
                "connection refused"
            ))),
            "an IO connection-refused must clear the connection"
        );
        // A generic IO error is still transport-class.
        assert!(
            is_transport_error(&ConductorApiError::IoError(IoError::new(
                ErrorKind::BrokenPipe,
                "broken pipe"
            ))),
            "a broken-pipe IO error must clear the connection"
        );
    }

    // ── conductor-call hard deadline (live-freeze guard) ────────────────────

    #[test]
    fn zome_call_timeout_defaults_when_unset() {
        // A guard at default: the deadline is never "no timeout".
        // (Env var intentionally not set here; another test owns the override
        // case behind a serial lock to avoid set_var cross-test bleed.)
        std::env::remove_var("DOORWAY_ZOME_CALL_TIMEOUT_MS");
        assert_eq!(
            zome_call_timeout(),
            Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS)
        );
    }

    /// The load-bearing invariant for the 2026-06-13 doorway freeze: a conductor
    /// future that NEVER settles must resolve to an `Err` within the bounded
    /// deadline — it must not hang the calling task forever (which, on the
    /// cpu-limited pod, froze the whole gateway).
    ///
    /// This exercises the exact `tokio::time::timeout(deadline, <future>)` shape
    /// that `ensure_connected` and `call_zome` now wrap every conductor await in.
    /// A real `ZomeCaller` needs a live conductor, so we model the never-settling
    /// conductor call with a future that never completes and assert the timeout
    /// fires within a small bound.
    #[tokio::test(start_paused = true)]
    async fn a_conductor_call_that_never_returns_errors_within_the_deadline() {
        let deadline = Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS);
        // A future that never resolves — models a conductor that accepts the
        // connection but never answers (the `record_heartbeat` hang).
        let never_settles = std::future::pending::<Result<(), String>>();

        let result = tokio::time::timeout(deadline, never_settles).await;

        assert!(
            result.is_err(),
            "a never-settling conductor future must resolve to a timeout Err, not hang"
        );
    }

    #[test]
    fn zero_or_garbage_timeout_falls_back_to_default_never_unbounded() {
        // A misconfigured `0` or unparseable value must NEVER mean "no timeout"
        // — that is the precise bug class this guard exists to prevent.
        // Serialized via a process-local lock so the temporary set_var cannot
        // bleed into a parallel test (the env-flake class).
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().unwrap();

        std::env::set_var("DOORWAY_ZOME_CALL_TIMEOUT_MS", "0");
        assert_eq!(
            zome_call_timeout(),
            Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS),
            "0 must fall back to the default, not disable the timeout"
        );

        std::env::set_var("DOORWAY_ZOME_CALL_TIMEOUT_MS", "not-a-number");
        assert_eq!(
            zome_call_timeout(),
            Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS),
            "unparseable must fall back to the default"
        );

        std::env::set_var("DOORWAY_ZOME_CALL_TIMEOUT_MS", "3000");
        assert_eq!(
            zome_call_timeout(),
            Duration::from_millis(3000),
            "a valid override is honored"
        );

        std::env::remove_var("DOORWAY_ZOME_CALL_TIMEOUT_MS");
    }

    // ── reconnect budget bounds the connecting-mutex hold (HIGH freeze guard) ─

    /// The overall reconnect budget is `deadline * 5 + 1s`: the slow path runs up
    /// to 5 sequential conductor steps, each bounded by `deadline`. This pins the
    /// formula so a future per-step addition can't silently outrun the budget.
    #[test]
    fn reconnect_budget_covers_all_five_sequential_steps() {
        let deadline = Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS);
        let budget = reconnect_budget(deadline);
        // Strictly greater than the sum of the per-step deadlines, so a legitimate
        // (slow but progressing) reconnect is never falsely budget-killed…
        assert!(budget > deadline * 5);
        // …yet finite — never "no timeout", the bug class this whole module guards.
        assert_eq!(budget, Duration::from_millis(51_000));
    }

    // ── conductor failover: single-peer SPOF removal (2026-08-06) ───────────

    fn primary() -> ConductorEndpoint {
        ConductorEndpoint::new("ws://elohim-adam-alpha:4444", "ws://elohim-adam-alpha:4445")
    }

    /// The doorway declares its conductor pool ONCE (`CONDUCTOR_URLS`). The
    /// fallback list is derived from it — no new config schema — with the
    /// primary filtered out (it is tried first by construction) and duplicates
    /// dropped, preserving the operator's declared order.
    #[test]
    fn fallbacks_come_from_the_declared_pool_minus_the_primary() {
        let pool = vec![
            // The primary itself, as the pool always contains it.
            "ws://elohim-adam-alpha:4445".to_string(),
            "ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445".to_string(),
            "ws://elohim-jessica-alpha-0.elohim-jessica-alpha-headless:8445".to_string(),
            // A duplicate and an empty entry — both must be ignored.
            "ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445".to_string(),
            "  ".to_string(),
        ];

        let fallbacks = build_fallback_endpoints(&primary(), &pool);

        assert_eq!(
            fallbacks.len(),
            2,
            "primary and duplicate must be filtered out of the fallback list"
        );
        assert_eq!(
            fallbacks[0].app_addr,
            "elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445"
        );
        assert_eq!(
            fallbacks[1].app_addr, "elohim-jessica-alpha-0.elohim-jessica-alpha-headless:8445",
            "pool order is the operator's declared preference and must be preserved"
        );
    }

    /// THE CRUX. Signing credentials are per-conductor: a cap grant issued by
    /// one conductor is worthless on another. So each fallback must carry its
    /// OWN admin interface — on its own host — for `connect_endpoint` to
    /// authorize against. A fallback that borrowed the primary's admin address
    /// would authorize on the sick peer and then sign calls the healthy peer
    /// rejects, which is exactly the bug this test forbids.
    #[test]
    fn each_fallback_authorizes_against_its_own_admin_interface() {
        let pool = vec![
            "ws://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8445".to_string(),
            "ws://elohim-james-alpha-0.elohim-james-alpha-headless:8445".to_string(),
        ];
        let primary = primary();
        let fallbacks = build_fallback_endpoints(&primary, &pool);

        for ep in &fallbacks {
            let admin_host = ep.admin_addr.rsplit_once(':').unwrap().0;
            let app_host = ep.app_addr.rsplit_once(':').unwrap().0;
            assert_eq!(
                admin_host, app_host,
                "a fallback's admin interface must live on its OWN host — \
                 credentials authorized elsewhere are unusable"
            );
            assert_ne!(
                ep.admin_addr, primary.admin_addr,
                "a fallback must never reuse the primary's admin interface"
            );
            // Established socat convention: admin = app - 1 (8445 app → 8444 admin).
            assert_eq!(ep.admin_addr, format!("{admin_host}:8444"));
        }
    }

    /// An application error is answered identically by every peer in the pool.
    /// Failing over on it would relocate the same error and double the load, so
    /// the class split — not merely "did the call fail" — is what gates failover.
    #[test]
    fn only_availability_failures_are_failover_worthy() {
        use holochain_conductor_api::ExternalApiWireError;
        use std::io::{Error as IoError, ErrorKind};

        // Availability: the conductor could not answer.
        assert!(is_transport_error(&ConductorApiError::IoError(
            IoError::new(ErrorKind::ConnectionRefused, "connection refused")
        )));
        // Application: the conductor DID answer.
        assert!(
            !is_transport_error(&ConductorApiError::ExternalApiWireError(
                ExternalApiWireError::InternalError("validation failed".to_string())
            )),
            "a zome-returned error must not trigger failover — every peer answers the same"
        );
        assert_eq!(
            CallError::unavailable("x").class,
            CallFailureClass::Unavailable
        );
    }

    /// A caller with no fallbacks configured (the deliberately-pinned temporary
    /// callers, and any single-conductor deployment) must behave EXACTLY as
    /// before: `call_zome_failover` delegates straight to the primary path.
    #[test]
    fn a_single_conductor_caller_has_no_failover_surface() {
        let caller = ZomeCaller::new("ws://localhost:4444", "ws://localhost:4445", "elohim");
        assert_eq!(caller.fallback_count(), 0);

        // Even when handed a pool consisting only of itself.
        let caller = ZomeCaller::with_fallbacks(
            "ws://localhost:4444",
            "ws://localhost:4445",
            "elohim",
            &["ws://localhost:4445".to_string()],
        );
        assert_eq!(
            caller.fallback_count(),
            0,
            "the primary must not become its own fallback"
        );
    }

    /// The parameter-bearing discovery from the live outage: "always try the
    /// primary first" is a NO-OP for a browser. The primary deadline is 10s and
    /// the SPA aborts at ~4.95s, so without a cooldown the route stays dead for
    /// the whole outage even with a healthy fallback standing by. After one
    /// availability failure the primary must be SKIPPED, and re-probed once the
    /// cooldown expires (half-open, no flap logic).
    #[tokio::test(start_paused = true)]
    async fn an_unhealthy_primary_is_skipped_then_re_probed() {
        let caller = ZomeCaller::with_fallbacks(
            "ws://elohim-adam-alpha:4444",
            "ws://elohim-adam-alpha:4445",
            "elohim",
            &["ws://elohim-matthew-alpha-0.headless:8445".to_string()],
        );
        assert_eq!(caller.fallback_count(), 1);

        assert!(
            !caller.primary_in_cooldown().await,
            "a fresh caller prefers its pinned primary"
        );

        caller.mark_primary_unhealthy().await;
        assert!(
            caller.primary_in_cooldown().await,
            "after an availability failure the sick primary must be skipped, \
             not re-tried at 10s per request"
        );

        // Just before expiry: still skipped.
        tokio::time::advance(PRIMARY_UNHEALTHY_COOLDOWN - Duration::from_secs(1)).await;
        assert!(caller.primary_in_cooldown().await);

        // Past expiry: re-probed automatically (half-open).
        tokio::time::advance(Duration::from_secs(2)).await;
        assert!(
            !caller.primary_in_cooldown().await,
            "a recovered primary must be re-probed without operator action"
        );
    }

    /// A primary that starts answering again clears the cooldown, so affinity
    /// returns to the doorway's co-located premise conductor on its own.
    #[tokio::test(start_paused = true)]
    async fn a_recovered_primary_reclaims_affinity() {
        let caller = ZomeCaller::with_fallbacks(
            "ws://elohim-adam-alpha:4444",
            "ws://elohim-adam-alpha:4445",
            "elohim",
            &["ws://elohim-matthew-alpha-0.headless:8445".to_string()],
        );

        caller.mark_primary_unhealthy().await;
        assert!(caller.primary_in_cooldown().await);

        caller.clear_primary_cooldown().await;
        assert!(
            !caller.primary_in_cooldown().await,
            "a successful primary call must restore primary affinity immediately"
        );
    }

    /// Failing fallbacks rotate rather than re-hammering the same sick peer,
    /// and the rotation wraps so every configured conductor gets a turn.
    #[test]
    fn a_failing_fallback_rotates_to_the_next_conductor() {
        let caller = ZomeCaller::with_fallbacks(
            "ws://elohim-adam-alpha:4444",
            "ws://elohim-adam-alpha:4445",
            "elohim",
            &[
                "ws://a-0.headless:8445".to_string(),
                "ws://b-0.headless:8445".to_string(),
                "ws://c-0.headless:8445".to_string(),
            ],
        );
        let count = caller.fallback_count();
        assert_eq!(count, 3);

        assert_eq!(caller.fallback_idx.load(Ordering::Relaxed), 0);
        caller.advance_fallback(0, count);
        assert_eq!(caller.fallback_idx.load(Ordering::Relaxed), 1);
        caller.advance_fallback(1, count);
        assert_eq!(caller.fallback_idx.load(Ordering::Relaxed), 2);
        caller.advance_fallback(2, count);
        assert_eq!(
            caller.fallback_idx.load(Ordering::Relaxed),
            0,
            "rotation must wrap so no conductor is starved"
        );
    }

    /// End-to-end routing proof without a live conductor: point the primary AND
    /// the fallback at closed ports. The call must
    ///   (a) attempt the primary,
    ///   (b) attempt the fallback with its OWN connect handshake,
    ///   (c) surface a combined error rather than hanging, and
    ///   (d) leave the primary in cooldown so the NEXT request does not stall on
    ///       the sick peer — which is the actual SPOF removal.
    #[tokio::test]
    async fn a_dead_primary_routes_to_the_fallback_and_then_stays_off_the_primary() {
        // Port 1 on loopback is closed → ECONNREFUSED immediately, so this test
        // exercises the routing without waiting on any timeout.
        let caller = ZomeCaller::with_fallbacks(
            "ws://127.0.0.1:1",
            "ws://127.0.0.1:2",
            "elohim",
            &["ws://127.0.0.1:3".to_string()],
        );
        assert_eq!(caller.fallback_count(), 1);

        let result = tokio::time::timeout(
            Duration::from_secs(20),
            caller.call_zome_failover(
                "infrastructure",
                "infrastructure",
                "get_all_doorways",
                vec![],
            ),
        )
        .await
        .expect("failover must not hang the calling task");

        let err = result.expect_err("both conductors are closed ports");
        assert!(
            err.contains("every fallback conductor also failed"),
            "the error must name BOTH attempts so an operator can see the failover ran: {err}"
        );

        assert!(
            caller.primary_in_cooldown().await,
            "one sick primary must not be re-tried on every subsequent request — \
             that is what kept GET /api/v1/federation/doorways dead for 5.5h"
        );
    }

    /// The load-bearing fix #2 invariant: when every reconnect step hangs (full
    /// NXDOMAIN), the overall budget timeout must fire and RELEASE the `connecting`
    /// mutex promptly, so a concurrent caller queued behind it gets a fast `Err`
    /// instead of parking for the full ~50s.
    ///
    /// We model `ensure_connected`'s slow path exactly: a task acquires the
    /// `connecting` mutex, then runs `timeout(budget, <never-settling reconnect>)`
    /// while holding it. A second task tries to acquire the same mutex. Under a
    /// paused clock we advance past the budget and assert (a) the held task's
    /// reconnect resolved to a timeout `Err`, and (b) the second caller then
    /// acquires the lock — i.e. the budget bounds the mutex hold.
    #[tokio::test(start_paused = true)]
    async fn reconnect_budget_releases_the_connecting_mutex() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let deadline = Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS);
        let budget = deadline * 5 + Duration::from_secs(1);

        // The exact `connecting: Mutex<()>` shape ZomeCaller uses.
        let connecting: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

        // Task A: holds the lock across the budget-bounded never-settling reconnect.
        let lock_a = Arc::clone(&connecting);
        let holder = tokio::spawn(async move {
            let _lock = lock_a.lock().await;
            // Models the slow-path body: every conductor step hangs forever.
            let never_settles = std::future::pending::<Result<(), String>>();
            tokio::time::timeout(budget, never_settles).await
            // `_lock` drops here on return — the prompt release fix #2 guarantees.
        });

        // Give task A a tick to acquire the lock before task B contends.
        tokio::task::yield_now().await;

        // Task B: a concurrent caller queued behind the same mutex.
        let lock_b = Arc::clone(&connecting);
        let waiter = tokio::spawn(async move {
            let _lock = lock_b.lock().await;
        });

        // Advance the paused clock just past the budget. The held task's reconnect
        // must elapse to an `Err`, releasing the mutex…
        tokio::time::advance(budget + Duration::from_millis(1)).await;

        let holder_result = holder.await.expect("holder task panicked");
        assert!(
            holder_result.is_err(),
            "a full-NXDOMAIN reconnect must elapse to a budget Err, not hang"
        );

        // …so the queued caller acquires the lock promptly (well within the
        // budget again), rather than parking for the full ~50s a second time.
        let waiter_done = tokio::time::timeout(budget, waiter).await;
        assert!(
            waiter_done.is_ok(),
            "the connecting mutex must release promptly so queued callers proceed"
        );
    }
}
