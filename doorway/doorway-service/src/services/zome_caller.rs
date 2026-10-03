//! ZomeCaller - Signed zome call client using official holochain_client
//!
//! Makes signed zome calls to the Holochain conductor via AppWebsocket.
//! Used by:
//! - Federation service (register_doorway, record_heartbeat, find_publishers)
//! - Identity management (create_human via auth routes)
//!
//! ## Auth & Signing Flow
//! 1. Connect to admin interface, list apps to find each role's cell_id
//! 2. Issue app auth token, connect AppWebsocket with the cached signer
//! 3. On a call, make sure the ONE role being called has signing credentials
//!    (reuse, or mint once) — see [`RoleCredentials::ensure`]
//! 4. The zome call is signed by the client
//!
//! Connecting authors nothing. A credential is a `CapGrant` on the conductor's
//! chain, and minting one runs zome init plus two write transactions that the
//! conductor does not cancel when we stop waiting. Authorizing every role cell
//! at connect made a call on one cell pay for five and fail if any one of them
//! was slow (hosted registration's `create_human` on `imagodei`, 2026-10-03).
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
//!    which authorizes against that endpoint's OWN admin interface. There is
//!    still no signer field on `ZomeCaller` — that absence is load-bearing, not
//!    incidental: a signer reachable from the caller is a signer that can be
//!    reused across endpoints, which is the bug this invariant forbids.
//!
//!    What changed (2026-09-20): the per-connection signer is no longer FRESH.
//!    `authorize_signing_credentials` COMMITS a `CapGrant`, nothing revokes
//!    one, and the conductor re-reads every grant on every zome call — so a
//!    fresh signer per connect made reconnect churn (lines below) an unbounded
//!    minter on exactly the two peers the public doorways front. Credentials
//!    now come from
//!    [`crate::services::signing_credentials`], a process-lifetime cache keyed
//!    by (conductor admin address, installed app id) → per-cell credentials.
//!    The per-conductor invariant is PRESERVED by that key: nothing is ever
//!    shared across endpoints, and a fallback still authorizes on its own admin
//!    interface the first time it is used. Nothing is written to disk; a
//!    restart re-mints once per cell.
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
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, CellId, ClientAgentSigner,
    ConductorApiError, SigningCredentials, ZomeCallTarget,
};
use holochain_types::prelude::ExternIO;

use crate::conductor::typed_admin::{classify_request_error, FaultClass};
use crate::services::signing_credentials::{
    cache as credential_cache, endpoint_key, looks_like_a_rejected_cap_grant, mint_cooldown,
    CredentialDecision, MintError, SigningCredentialCache,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
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

/// Expiry for the token minted in [`connect_endpoint`]. Class M
/// (mint-and-consume): the token is used exactly once, in the
/// `AppWebsocket::connect()` call immediately below its issuance, and is
/// never stored — only the resulting authenticated `AppWebsocket` is cached
/// on `ZomeCaller`. A reconnect calls `connect_endpoint` again, minting a
/// fresh token, so a single-use token never blocks reconnection. Generous on
/// purpose (see `conductor/typed_app.rs`): a connect that outlives the expiry
/// fails auth and loops, which a stalled conductor cannot afford.
const MINT_AND_CONSUME_EXPIRY_SECS: u64 = 300;

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
    /// The live connection to [`Self::primary`] (lazily initialized)
    app_ws: RwLock<Option<Arc<Connection>>>,
    /// Lock to prevent concurrent connection attempts
    connecting: Mutex<()>,
    /// Lazily-established fallback connection, tagged with WHICH fallback it is.
    ///
    /// Cached deliberately: re-running the connect handshake per call would
    /// re-issue `authorize_signing_credentials` — a conductor-side cap-grant
    /// write — on every federation poll for the whole duration of a primary
    /// outage, converting one sick peer into load on a healthy one.
    fallback_ws: RwLock<Option<(usize, Arc<Connection>)>>,
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

        // Overall reconnect budget, sized for 5 sequential conductor steps each
        // bounded by `deadline` (~5×deadline, ≈50s at the 10s default) while
        // the `connecting` mutex is held. The slow path now runs four (admin
        // connect, list_apps, token, app connect) — the credential mint moved
        // to the call path — and the budget is deliberately left unchanged.
        // Under a full cluster NXDOMAIN every step times out in sequence,
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
        *ws = Some(Arc::new(connected));
        Ok(())
    }

    /// Ensure the cached fallback connection targets `idx`, connecting if not.
    ///
    /// Each fallback gets its own [`connect_endpoint`] run — i.e. its own
    /// `ClientAgentSigner`, and (on the first call to a role) its own
    /// `authorize_signing_credentials` against its own admin interface. A
    /// credential minted on the primary is worthless here, and nothing in this
    /// path reuses one.
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
        *ws = Some((idx, Arc::new(connected)));
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
        // Every connect/token failure is an availability failure: the conductor
        // could not be brought to a usable state. Another peer may still
        // answer, so this class is what triggers failover. (A failed credential
        // mint for the role being called is the same class — see
        // `call_on_connection`.)
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
        //
        // The read guard is held only long enough to CLONE the connection's
        // `Arc`. A credential mint and the call both run without it: tokio's
        // RwLock is write-preferring, so a writer queued behind a read guard
        // held for the length of a mint would stall every later reader too.
        let connection = self
            .app_ws
            .read()
            .await
            .clone()
            .ok_or_else(|| CallError::unavailable("Not connected"))?;
        let result = call_on_connection(
            &connection,
            role_name,
            zome_name,
            fn_name,
            payload,
            deadline,
        )
        .await;

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
                    self.clear_primary_if_current(&connection).await;
                } else if self.heal_if_cap_grant_rejected(&self.primary, role_name, &e.message) {
                    // The one sanctioned clear on an application error — see
                    // `heal_if_cap_grant_rejected`. Bounded to once per
                    // (conductor, role) per process, so it cannot churn.
                    self.clear_primary_if_current(&connection).await;
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

            // As on the primary: clone the `Arc` under the guard, call without it.
            let fallback = match self.fallback_ws.read().await.as_ref() {
                Some((i, connection)) if *i == idx => Some(Arc::clone(connection)),
                _ => None,
            };
            let result = match fallback {
                Some(connection) => {
                    call_on_connection(
                        &connection,
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
                None => Err(CallError::unavailable("fallback connection was replaced")),
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
                Err(e) if e.class == CallFailureClass::Application => {
                    // Same bounded heal as the primary path: a cap-grant-shaped
                    // refusal on a fallback drops THAT fallback's socket once so
                    // the next use re-mints on ITS own admin interface. The
                    // error still surfaces — we never retry here.
                    if self.heal_if_cap_grant_rejected(&self.fallbacks[idx], role_name, &e.message)
                    {
                        let mut ws = self.fallback_ws.write().await;
                        if matches!(ws.as_ref(), Some((i, _)) if *i == idx) {
                            *ws = None;
                        }
                    }
                    return Err(e);
                }
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

    /// Drop the primary connection — but only if it is still the one this call
    /// used. With the read guard no longer held across the call, another caller
    /// may already have replaced it with a healthy connection.
    async fn clear_primary_if_current(&self, used: &Arc<Connection>) {
        let mut ws = self.app_ws.write().await;
        if ws
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, used))
        {
            *ws = None;
        }
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

    /// A zome call was refused. If the refusal is CAP-GRANT-SHAPED, ask the
    /// process credential cache for exactly ONE replacement credential for this
    /// (conductor, role), and report whether the socket must be dropped so the
    /// next call reconnects and re-mints.
    ///
    /// # Why this is the one sanctioned exception to "application errors keep
    /// the connection"
    ///
    /// The churn fix (2026-06-13 iteration 3) says a conductor that ANSWERED
    /// has a healthy socket, so an application error must not clear it. An
    /// unauthorized answer is an application error by that rule — and it is
    /// also the ONLY place a vanished `CapGrant` can be observed (a conductor
    /// reinstalled, or a database restored from an older snapshot under the
    /// same `CellId`; the cached credential parses fine and reuse looks correct
    /// right up to the call). So this arm clears the socket — but only when the
    /// cache ACCEPTS the heal, which it does at most once per (conductor, role)
    /// per process. A second rejection returns `false`, the socket is kept, and
    /// the error surfaces unchanged. Bounded by construction: it cannot loop,
    /// and it cannot re-create the per-connect minting it exists to replace.
    fn heal_if_cap_grant_rejected(
        &self,
        endpoint: &ConductorEndpoint,
        role_name: &str,
        message: &str,
    ) -> bool {
        if !looks_like_a_rejected_cap_grant(message) {
            return false;
        }
        let key = endpoint_key(&endpoint.admin_addr, &self.installed_app_id);
        if credential_cache().mark_for_remint(&key, role_name) {
            warn!(
                conductor = %endpoint.admin_addr,
                role_name = %role_name,
                "Conductor rejected our cached signing credentials — discarding them ONCE so \
                 the next connect mints a fresh grant: {message}"
            );
            true
        } else {
            warn!(
                conductor = %endpoint.admin_addr,
                role_name = %role_name,
                "Conductor rejected our signing credentials AGAIN for this role — not \
                 re-minting; the fault is not a stale credential: {message}"
            );
            false
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
/// Sized for five sequential conductor steps, each bounded by `deadline`
/// (≈50s at the 10s default) while a `connecting` mutex is held. The connect
/// now runs four — the credential mint moved to the call path — and the budget
/// is deliberately unchanged. Bounding the entire body (additively to the
/// per-step timeouts) guarantees the mutex releases promptly so queued callers
/// get a fast `Err` instead of parking for the full stretch.
fn reconnect_budget(deadline: Duration) -> Duration {
    deadline * 5 + Duration::from_secs(1)
}

/// The signing credentials of ONE connection, minted per role on first use.
///
/// Holds what a mint needs and nothing a mint has done: which cell each role
/// resolves to, the process-cache signer for this (conductor, app), and which
/// roles this connection has already settled. No socket lives here, so the
/// "one call, one grant" behaviour is testable without a conductor.
struct RoleCredentials {
    installed_app_id: String,
    /// The conductor admin address, for log lines.
    admin_addr: String,
    /// The process-cache key for this (conductor, app).
    ep_key: String,
    /// The process-cache signer for `ep_key` — the SAME `Arc` the
    /// `AppWebsocket` signs with, so a credential added here after the socket
    /// connected is seen by the very next call (`AppWebsocket::call_zome` reads
    /// the signer per call).
    signer: Arc<ClientAgentSigner>,
    /// Role name → that role's provisioned cell on this conductor.
    cells: HashMap<String, CellId>,
    /// Roles this connection has already settled, so a steady-state call takes
    /// no lease and logs nothing. Dies with the connection — which is what the
    /// bounded heal relies on: it drops the socket, the next connection starts
    /// with this set empty, and `ensure` consumes the heal mark.
    ready: std::sync::Mutex<HashSet<String>>,
}

impl RoleCredentials {
    fn new(
        installed_app_id: &str,
        admin_addr: &str,
        signer: Arc<ClientAgentSigner>,
        cells: HashMap<String, CellId>,
    ) -> Self {
        Self {
            installed_app_id: installed_app_id.to_string(),
            admin_addr: admin_addr.to_string(),
            ep_key: endpoint_key(admin_addr, installed_app_id),
            signer,
            cells,
            ready: std::sync::Mutex::new(HashSet::new()),
        }
    }

    fn is_ready(&self, role_name: &str) -> bool {
        self.ready
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(role_name)
    }

    /// Make sure the ONE role a call targets can be signed for — reuse the
    /// process-cached credential, or mint it once.
    ///
    /// MINT ONLY FOR THE CELL BEING CALLED. This is what a connect used to do
    /// for every role at once. `mint` is the conductor round trip; it is
    /// invoked at most once and only when nothing is cached for this role.
    ///
    /// A role this app does not have is not an error HERE: the call goes ahead
    /// and the client answers `CellNotFound`, exactly as it did when the
    /// connect minted every role.
    async fn ensure<F, Fut>(
        &self,
        cache: &SigningCredentialCache,
        role_name: &str,
        deadline: Duration,
        mint: F,
    ) -> Result<(), String>
    where
        F: FnOnce(CellId) -> Fut,
        Fut: Future<Output = Result<SigningCredentials, MintError>>,
    {
        let Some(cell_id) = self.cells.get(role_name) else {
            return Ok(());
        };
        if self.is_ready(role_name) {
            return Ok(());
        }

        let ensured = cache
            .ensure(
                &self.signer,
                &self.ep_key,
                role_name,
                cell_id,
                worst_case_mint(deadline),
                mint_cooldown(),
                || mint(cell_id.clone()),
            )
            .await?;

        self.ready
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role_name.to_string());

        // One line per (connection, role), with the same fields the per-connect
        // line carried: `minted` is the number of CapGrants this authored (0 or
        // 1), `reused` the number it avoided.
        let minted = u32::from(ensured.decision == CredentialDecision::Mint);
        info!(
            app_id = %self.installed_app_id,
            conductor = %self.admin_addr,
            role = %role_name,
            cells = 1u32,
            minted,
            reused = 1 - minted,
            "Signing credentials ready"
        );
        Ok(())
    }
}

/// The longest one credential mint can take: an authorize over the kept admin
/// socket, then — if that socket was dead — a redial (one DNS resolve and one
/// connect) and a second authorize, each bounded by `deadline`.
///
/// This is what a lease WAITER is given. It used to be given `deadline` alone,
/// which is shorter than the mint it was waiting on, so waiters fell through
/// the lease and minted in parallel with the holder. No deadline is changed:
/// the same per-step bound, added up.
fn worst_case_mint(deadline: Duration) -> Duration {
    deadline * 3 + crate::conductor::dns_resolve_timeout()
}

/// One established connection to ONE conductor.
struct Connection {
    app_ws: AppWebsocket,
    /// The admin socket the connect ran over, kept so the first call to a role
    /// can mint that role's credential without a second handshake.
    ///
    /// On a long-lived caller it can be dead by the time a NEW role is first
    /// called. If the mint reports the request was never written (the socket
    /// was already torn down), [`mint_role_credential`] dials a fresh admin
    /// socket, leaves it here, and mints over that — the call does not fail.
    admin_ws: std::sync::Mutex<AdminWebsocket>,
    /// Where to dial a replacement admin socket.
    admin_addr: String,
    roles: RoleCredentials,
}

impl Connection {
    fn admin(&self) -> AdminWebsocket {
        self.admin_ws
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// Why minting one role's credential failed.
#[derive(Debug)]
enum MintFailure {
    /// The admin client returned an error — typed, so it can be classed.
    Conductor(ConductorApiError),
    /// The authorize outran the per-call deadline.
    TimedOut,
    /// A replacement admin socket could not be dialled.
    Redial(String),
}

/// Did this mint fail WITHOUT its request ever being written?
///
/// The typed test hosted provisioning uses ([`FaultClass::NotSent`]):
/// `holochain_websocket` answers `Close("No connection")` when the socket was
/// already torn down before the write — exactly what a kept-but-idle admin
/// socket looks like once its peer has closed it.
fn mint_never_written(failure: &MintFailure) -> bool {
    matches!(
        failure,
        MintFailure::Conductor(error) if classify_request_error(error) == FaultClass::NotSent
    )
}

/// Run `first`; if it failed without its request ever being written, run
/// `redial_and_retry` ONCE instead of surfacing the failure.
///
/// Nothing reached the conductor in that case, so the retry cannot author a
/// second `CapGrant`. Any other failure — a request that was written and then
/// lost, a timeout, a refusal — is returned unchanged and never retried: its
/// grant may exist.
async fn retry_if_never_written<T, E, First, Retry, RetryFut>(
    first: First,
    never_written: impl Fn(&E) -> bool,
    redial_and_retry: Retry,
) -> Result<T, E>
where
    First: Future<Output = Result<T, E>>,
    Retry: FnOnce() -> RetryFut,
    RetryFut: Future<Output = Result<T, E>>,
{
    match first.await {
        Err(failure) if never_written(&failure) => redial_and_retry().await,
        outcome => outcome,
    }
}

/// One `authorize_signing_credentials`, bounded by `deadline`.
async fn authorize_once(
    admin: &AdminWebsocket,
    cell_id: CellId,
    deadline: Duration,
) -> Result<SigningCredentials, MintFailure> {
    match tokio::time::timeout(
        deadline,
        admin.authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
            cell_id,
            functions: None,
        }),
    )
    .await
    {
        Err(_elapsed) => Err(MintFailure::TimedOut),
        Ok(Err(error)) => Err(MintFailure::Conductor(error)),
        Ok(Ok(credentials)) => Ok(credentials),
    }
}

/// Mint ONE role's signing credential over the connection's admin socket,
/// replacing that socket first if it turns out to be dead.
///
/// Each conductor step is bounded by `deadline`, the same per-call deadline as
/// everywhere else in this module; none is extended.
///
/// The error says whether a grant may have been authored anyway
/// ([`MintError::outcome_unknown`]): a timeout, or a request written to a
/// socket that then dropped. The conductor does not cancel either.
async fn mint_role_credential(
    connection: &Connection,
    role_name: &str,
    cell_id: CellId,
    deadline: Duration,
) -> Result<SigningCredentials, MintError> {
    let kept = connection.admin();
    let minted = retry_if_never_written(
        authorize_once(&kept, cell_id.clone(), deadline),
        mint_never_written,
        || async {
            warn!(
                conductor = %connection.admin_addr,
                role = %role_name,
                "Kept admin socket is dead — redialling it to mint this role's credential"
            );
            let fresh = redial_admin(&connection.admin_addr, deadline)
                .await
                .map_err(MintFailure::Redial)?;
            *connection
                .admin_ws
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = fresh.clone();
            authorize_once(&fresh, cell_id.clone(), deadline).await
        },
    )
    .await;

    match minted {
        Ok(credentials) => {
            info!(role = %role_name, "Authorized signing for cell (one CapGrant authored)");
            Ok(credentials)
        }
        Err(failure) => Err(mint_error(role_name, deadline, failure)),
    }
}

/// Render a mint failure, and say whether a grant may exist regardless.
fn mint_error(role_name: &str, deadline: Duration, failure: MintFailure) -> MintError {
    match failure {
        MintFailure::TimedOut => MintError::abandoned(format!(
            "authorize_signing_credentials timed out after {}ms for role '{role_name}'",
            deadline.as_millis()
        )),
        MintFailure::Conductor(e) => {
            let message =
                format!("authorize_signing_credentials failed for role '{role_name}': {e}");
            match classify_request_error(&e) {
                FaultClass::SentOutcomeUnknown => MintError::abandoned(message),
                FaultClass::NotSent | FaultClass::Refused => MintError::refused(message),
            }
        }
        // The replacement socket could not be dialled: nothing was sent on it.
        MintFailure::Redial(e) => MintError::refused(e),
    }
}

/// Dial a fresh admin socket for an existing connection.
async fn redial_admin(admin_addr: &str, deadline: Duration) -> Result<AdminWebsocket, String> {
    let admin_socket = crate::conductor::resolve_host_port(admin_addr).await?;
    tokio::time::timeout(
        deadline,
        AdminWebsocket::connect(admin_socket, Some(String::from("doorway-zome-caller"))),
    )
    .await
    .map_err(|_| {
        format!(
            "Admin connect timed out after {}ms ({admin_addr})",
            deadline.as_millis()
        )
    })?
    .map_err(|e| format!("Admin connect failed: {e}"))
}

/// Connect to ONE conductor and return a signing-capable connection.
///
/// THE PER-CONDUCTOR CREDENTIAL CRUX. Signing credentials are scoped to a
/// conductor's cells and agent: `authorize_signing_credentials` issues a cap
/// grant on THAT conductor, and only THAT conductor will verify the resulting
/// signature. So the signer comes from the process cache under THIS endpoint's
/// (admin address, app) key, and every mint runs against `endpoint.admin_addr`
/// — a fallback conductor never borrows the primary's credentials. Both the
/// primary and every fallback go through this one function so the invariant
/// cannot drift apart between the two paths.
///
/// AUTHORS NOTHING. The connect resolves which cell each role maps to and
/// stops; a role's credential is minted by [`call_on_connection`] the first
/// time that role is called.
async fn connect_endpoint(
    installed_app_id: &str,
    endpoint: &ConductorEndpoint,
    deadline: Duration,
) -> Result<Connection, String> {
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

    // Step 2: Find each role's cell_id from app info
    let apps = tokio::time::timeout(deadline, admin_ws.list_apps(None))
        .await
        .map_err(|_| format!("list_apps timed out after {}ms", deadline.as_millis()))?
        .map_err(|e| format!("list_apps failed: {e}"))?;

    let app_info = apps
        .iter()
        .find(|a| a.installed_app_id == installed_app_id)
        .ok_or_else(|| format!("App '{installed_app_id}' not found"))?;

    // Which cell each role resolves to — a READ. No credential is authorized
    // here: `authorize_signing_credentials` is a chain write (a `CapGrant` that
    // is never revoked, behind a zome init), and a call on one role must not
    // pay for the other four.
    let cells = provisioned_role_cells(
        app_info
            .cell_info
            .iter()
            .map(|(role_name, cells)| (role_name.as_str(), cells.as_slice())),
    );

    if cells.is_empty() {
        return Err("No provisioned cells found".to_string());
    }

    // The process cache holds one signer per (conductor, app) — the same
    // `ClientAgentSigner` a previous connection populated — so a reconnect
    // finds every role it already minted and authorizes NOTHING.
    let roles = RoleCredentials::new(
        installed_app_id,
        &endpoint.admin_addr,
        credential_cache().signer_for(&endpoint_key(&endpoint.admin_addr, installed_app_id)),
        cells,
    );

    // Step 3: Issue app auth token
    let token = tokio::time::timeout(
        deadline,
        admin_ws.issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
            installed_app_id: installed_app_id.to_string(),
            expiry_seconds: MINT_AND_CONSUME_EXPIRY_SECS,
            single_use: true,
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

    // Step 4: Connect AppWebsocket with the cached signer.
    //
    // `ClientAgentSigner` is internally `Arc<RwLock<HashMap<CellId, _>>>`, so
    // handing the SAME signer to a second socket for the same conductor is
    // safe and is precisely what makes a reconnect free — and what lets a
    // credential minted AFTER this connect be used by this socket.
    let app_socket = crate::conductor::resolve_host_port(&endpoint.app_addr).await?;
    let app_ws = tokio::time::timeout(
        deadline,
        AppWebsocket::connect(app_socket, token.token, roles.signer.clone(), None),
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
        app_id = %installed_app_id,
        roles = roles.cells.len(),
        "ZomeCaller connected to app interface at {} with signing (credentials are \
         minted per role on first call)",
        endpoint.app_addr
    );

    Ok(Connection {
        app_ws,
        admin_ws: std::sync::Mutex::new(admin_ws),
        admin_addr: endpoint.admin_addr.clone(),
        roles,
    })
}

/// Role name → provisioned cell, from an app's cell listing.
///
/// Only `Provisioned` cells are callable by role name; clone and stem cells are
/// skipped, as they always were.
fn provisioned_role_cells<'a>(
    cell_info: impl Iterator<Item = (&'a str, &'a [holochain_client::CellInfo])>,
) -> HashMap<String, CellId> {
    let mut out = HashMap::new();
    for (role_name, cells) in cell_info {
        for cell in cells {
            if let holochain_client::CellInfo::Provisioned(p) = cell {
                out.insert(role_name.to_string(), p.cell_id.clone());
                break;
            }
        }
    }
    out
}

/// One zome call over an established connection: settle the signing credential
/// for the ONE role being called, then issue the call.
///
/// A mint that fails or outruns `deadline` is an availability failure, the
/// same class it was when it ran inside the connect: the conductor could not
/// be brought to a usable state for this call, another peer may still answer,
/// and the caller drops the connection.
async fn call_on_connection(
    connection: &Connection,
    role_name: &str,
    zome_name: &str,
    fn_name: &str,
    payload: Vec<u8>,
    deadline: Duration,
) -> Result<Vec<u8>, CallError> {
    connection
        .roles
        .ensure(credential_cache(), role_name, deadline, |cell_id| {
            mint_role_credential(connection, role_name, cell_id, deadline)
        })
        .await
        .map_err(CallError::unavailable)?;

    call_on_ws(
        &connection.app_ws,
        role_name,
        zome_name,
        fn_name,
        payload,
        deadline,
    )
    .await
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

    // ── cap-grant minting is bounded (2026-09-20) ──────────────────────────

    /// THE INVARIANT this module's credential reuse exists for: a reconnect to
    /// the SAME conductor must author no new `CapGrant`.
    ///
    /// `connect_endpoint` needs a live conductor, so this pins the layer the
    /// decision actually lives in: the process cache hands back the same signer
    /// for the same (conductor, app) key, and an already-credentialed cell
    /// decides `Reuse` — the branch that skips
    /// `authorize_signing_credentials` entirely.
    #[test]
    fn a_reconnect_to_the_same_conductor_reuses_one_signer() {
        use crate::services::signing_credentials::{
            cache as credential_cache, endpoint_key, SigningCredentialCache,
        };

        // The live path: the process cache, keyed exactly as connect_endpoint keys it.
        let key = endpoint_key("elohim-adam-alpha:4444", "elohim");
        let first = credential_cache().signer_for(&key);
        let second = credential_cache().signer_for(&key);
        assert!(
            std::sync::Arc::ptr_eq(&first, &second),
            "the second connect must reuse the first connect's credentials — a fresh \
             signer here is a fresh CapGrant per cell, on a chain nothing prunes"
        );

        // An untouched cell of a fresh cache is the only thing that mints.
        let fresh = SigningCredentialCache::new();
        let signer = fresh.signer_for(&key);
        assert_eq!(
            fresh.decide(&signer, &key, "infrastructure", &test_cell(7)),
            crate::services::signing_credentials::CredentialDecision::Mint
        );
    }

    fn test_cell(seed: u8) -> holochain_client::CellId {
        use holo_hash::DnaHash;
        use holochain_client::AgentPubKey;
        holochain_client::CellId::new(
            DnaHash::from_raw_36(vec![seed; 36]),
            AgentPubKey::from_raw_36(vec![seed.wrapping_add(1); 36]),
        )
    }

    /// HEAL ONCE, THEN SURFACE. A cap-grant-shaped refusal buys exactly one
    /// replacement credential for that (conductor, role); the next one does not,
    /// so the error reaches the caller instead of minting again. Never a loop.
    #[test]
    fn a_cap_grant_rejection_heals_once_then_surfaces() {
        // A unique admin address so this test cannot collide with another test
        // in the same process over the shared cache.
        let caller = ZomeCaller::new(
            "ws://heal-once-test:4444",
            "ws://heal-once-test:4445",
            "elohim-heal-once-test",
        );
        let endpoint =
            ConductorEndpoint::new("ws://heal-once-test:4444", "ws://heal-once-test:4445");

        assert!(
            caller.heal_if_cap_grant_rejected(
                &endpoint,
                "infrastructure",
                "Zome call failed: Unauthorized"
            ),
            "the first cap-grant rejection must discard the cached credential once"
        );
        for _ in 0..5 {
            assert!(
                !caller.heal_if_cap_grant_rejected(
                    &endpoint,
                    "infrastructure",
                    "Zome call failed: Unauthorized"
                ),
                "a repeated rejection must NOT mint again — it must surface"
            );
        }
    }

    /// An ordinary application error must never trigger the heal. If it did,
    /// every validation failure would author a `CapGrant` — the growth this
    /// change removes, wearing a different hat.
    #[test]
    fn an_ordinary_application_error_never_mints() {
        let caller = ZomeCaller::new(
            "ws://no-heal-test:4444",
            "ws://no-heal-test:4445",
            "elohim-no-heal-test",
        );
        let endpoint = ConductorEndpoint::new("ws://no-heal-test:4444", "ws://no-heal-test:4445");

        for message in [
            "Zome call failed: Zome function find_publishers doesn't exist",
            "Zome call failed: validation failed",
            "Zome call timed out after 10000ms",
        ] {
            assert!(
                !caller.heal_if_cap_grant_rejected(&endpoint, "imagodei", message),
                "must not be read as a stale grant: {message}"
            );
        }
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

    // ── mint only for the cell being called (2026-10-03) ───────────────────
    //
    // A hosted registration calls `create_human` on `imagodei` alone. The
    // connect used to author a `CapGrant` — a zome init and two write
    // transactions the conductor never cancels — on every one of the app's five
    // role cells first, and abort on the first slow one. These drive the
    // production `RoleCredentials::ensure` (and through it the production
    // `SigningCredentialCache::ensure`) with a counting stub in place of
    // `authorize_signing_credentials`.

    use crate::services::signing_credentials::SigningCredentialCache as Cache;
    use holochain_client::AgentSigner;
    use std::sync::atomic::AtomicU32;
    use tokio::sync::Notify;

    const FIVE_ROLES: [&str; 5] = [
        "lamad",
        "imagodei",
        "infrastructure",
        "node_registry",
        "mishpat",
    ];

    const NO_WAIT_EXPECTED: Duration = Duration::from_secs(30);

    fn role_cell(role: &str) -> holochain_client::CellId {
        let index = FIVE_ROLES
            .iter()
            .position(|r| *r == role)
            .expect("a role of the five-role app");
        test_cell(40 + index as u8)
    }

    /// One connection's view of a five-role app, signing through `cache`'s
    /// signer for that (conductor, app) — exactly how `connect_endpoint` builds
    /// it. Two calls with the same `admin_addr` model two connections (a
    /// reconnect, or two temporary callers) to one hosted app.
    fn five_role_connection(cache: &Cache, admin_addr: &str) -> RoleCredentials {
        let app = "elohim-conductor-0-ab12cd";
        RoleCredentials::new(
            app,
            admin_addr,
            cache.signer_for(&endpoint_key(admin_addr, app)),
            FIVE_ROLES
                .iter()
                .map(|role| (role.to_string(), role_cell(role)))
                .collect(),
        )
    }

    fn fake_credentials(seed: u8) -> SigningCredentials {
        use holochain_client::AgentPubKey;
        use holochain_zome_types::prelude::CapSecret;
        let cap_secret: CapSecret = [seed; 64]
            .as_slice()
            .try_into()
            .expect("64-byte cap secret");
        SigningCredentials {
            signing_agent_key: AgentPubKey::from_raw_36(vec![seed; 36]),
            keypair: ed25519_dalek::SigningKey::from_bytes(&[seed.max(1); 32]),
            cap_secret,
        }
    }

    /// Stands in for `authorize_signing_credentials`: every invocation is one
    /// `CapGrant` authored on the conductor.
    async fn counted_mint(grants: &AtomicU32) -> Result<SigningCredentials, MintError> {
        grants.fetch_add(1, Ordering::SeqCst);
        Ok(fake_credentials(7))
    }

    fn credentialed_roles(roles: &RoleCredentials) -> Vec<&'static str> {
        FIVE_ROLES
            .iter()
            .copied()
            .filter(|role| roles.signer.get_provenance(&role_cell(role)).is_some())
            .collect()
    }

    /// THE CHANGE. A call to one role of a five-role app authors exactly ONE
    /// grant — for that role's cell — and leaves the other four untouched.
    #[tokio::test]
    async fn a_call_to_one_role_of_a_five_role_app_authors_exactly_one_grant() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "one-role-test:4444");
        let grants = AtomicU32::new(0);

        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_cell| {
                counted_mint(&grants)
            })
            .await
            .expect("the mint succeeds");

        assert_eq!(
            grants.load(Ordering::SeqCst),
            1,
            "one call on one role must author ONE CapGrant, not one per role"
        );
        assert_eq!(
            credentialed_roles(&connection),
            vec!["imagodei"],
            "only the role that was called may hold a credential"
        );

        // …and the second call on that role authors nothing.
        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_cell| {
                counted_mint(&grants)
            })
            .await
            .expect("reuse cannot fail");
        assert_eq!(grants.load(Ordering::SeqCst), 1);
    }

    /// The mint is handed the cell of the role being called — not the first
    /// role's, not every role's.
    #[tokio::test]
    async fn the_mint_is_asked_for_the_called_roles_own_cell() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "own-cell-test:4444");
        let asked: std::sync::Mutex<Vec<holochain_client::CellId>> =
            std::sync::Mutex::new(Vec::new());

        connection
            .ensure(&cache, "mishpat", NO_WAIT_EXPECTED, |cell| {
                asked.lock().unwrap().push(cell);
                async { Ok(fake_credentials(3)) }
            })
            .await
            .unwrap();

        assert_eq!(*asked.lock().unwrap(), vec![role_cell("mishpat")]);
    }

    /// The other roles are not abandoned — each is minted on ITS first use,
    /// exactly once, through the same path. And a later connection to the same
    /// conductor+app (a reconnect, or the next temporary caller) reuses both.
    #[tokio::test]
    async fn a_later_call_to_another_role_authors_exactly_one_more() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "second-role-test:4444");
        let grants = AtomicU32::new(0);

        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await
            .unwrap();
        assert_eq!(grants.load(Ordering::SeqCst), 1);

        connection
            .ensure(&cache, "lamad", NO_WAIT_EXPECTED, |_| counted_mint(&grants))
            .await
            .unwrap();
        assert_eq!(
            grants.load(Ordering::SeqCst),
            2,
            "the second role's first call authors exactly one more grant"
        );
        let mut held = credentialed_roles(&connection);
        held.sort_unstable();
        assert_eq!(held, vec!["imagodei", "lamad"]);

        // A NEW connection to the same conductor+app: the once-per-(conductor,
        // app, role)-per-process ceiling still holds across connections.
        let reconnected = five_role_connection(&cache, "second-role-test:4444");
        for role in ["imagodei", "lamad"] {
            reconnected
                .ensure(&cache, role, NO_WAIT_EXPECTED, |_| counted_mint(&grants))
                .await
                .unwrap();
        }
        assert_eq!(
            grants.load(Ordering::SeqCst),
            2,
            "a reconnect must author nothing for a role this process already minted"
        );
    }

    /// THE RACE, at the call path. Two connections to one hosted app (two
    /// temporary callers, or a primary call racing the half-open re-probe) make
    /// their FIRST call to the same role at once. Exactly one grant is authored;
    /// the second waits on the cell lease, re-decides, and reuses.
    ///
    /// Genuinely concurrent: the first mint PARKS inside the authorize window
    /// until the test releases it, so both calls are provably in flight.
    #[tokio::test]
    async fn two_concurrent_first_calls_to_one_role_author_one_grant() {
        let cache = Arc::new(Cache::new());
        let first = Arc::new(five_role_connection(&cache, "race-test:4444"));
        let second = Arc::new(five_role_connection(&cache, "race-test:4444"));
        let grants = Arc::new(AtomicU32::new(0));
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());

        let a = {
            let (cache, connection, grants) =
                (Arc::clone(&cache), Arc::clone(&first), Arc::clone(&grants));
            let (entered, release) = (Arc::clone(&entered), Arc::clone(&release));
            tokio::spawn(async move {
                connection
                    .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| async {
                        grants.fetch_add(1, Ordering::SeqCst);
                        entered.notify_one();
                        release.notified().await;
                        Ok(fake_credentials(1))
                    })
                    .await
            })
        };

        // A is provably INSIDE the authorize window, holding the cell lease.
        entered.notified().await;
        assert_eq!(grants.load(Ordering::SeqCst), 1);

        let b = {
            let (cache, connection, grants) =
                (Arc::clone(&cache), Arc::clone(&second), Arc::clone(&grants));
            tokio::spawn(async move {
                connection
                    .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| async {
                        grants.fetch_add(1, Ordering::SeqCst);
                        Ok(fake_credentials(2))
                    })
                    .await
            })
        };

        // B must be parked on the lease, not authorizing.
        tokio::task::yield_now().await;
        assert_eq!(
            grants.load(Ordering::SeqCst),
            1,
            "the second call entered the authorize window while the first held the cell"
        );

        // `notify_one` stores a permit, so the release cannot be lost to
        // scheduling order.
        release.notify_one();
        a.await.unwrap().expect("the first call mints");
        b.await.unwrap().expect("the second call reuses");

        assert_eq!(
            grants.load(Ordering::SeqCst),
            1,
            "exactly ONE CapGrant may be authored for one role, however many first calls race"
        );
        assert_eq!(cache.live_leases(), 0, "the lease map must be reclaimed");
    }

    /// A failed mint settles nothing: the role is not marked ready, nothing is
    /// cached, and the next call mints. (And the error is the caller's to see —
    /// `call_on_connection` classes it `Unavailable`, as a failed connect was.)
    #[tokio::test]
    async fn a_failed_mint_leaves_the_role_unsettled_for_the_next_call() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "failed-mint-test:4444");
        let grants = AtomicU32::new(0);

        let refused = connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| async {
                Err(MintError::refused(
                    "authorize_signing_credentials failed for role 'imagodei': refused",
                ))
            })
            .await;
        assert!(refused.is_err());
        assert!(!connection.is_ready("imagodei"));
        assert!(credentialed_roles(&connection).is_empty());

        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await
            .expect("the next call must be allowed to mint");
        assert_eq!(grants.load(Ordering::SeqCst), 1);
    }

    /// A role the app does not have authors nothing and is not an error at this
    /// layer — the call proceeds and the client reports `CellNotFound`, as it
    /// did when the connect minted every role.
    #[tokio::test]
    async fn a_role_the_app_does_not_have_authors_nothing() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "unknown-role-test:4444");
        let grants = AtomicU32::new(0);

        connection
            .ensure(&cache, "no-such-role", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await
            .expect("an unknown role is the client's error to report, not the mint's");

        assert_eq!(grants.load(Ordering::SeqCst), 0);
        assert!(credentialed_roles(&connection).is_empty());
    }

    /// THE HEAL STILL WORKS. A cap-grant-shaped refusal marks the role and the
    /// caller drops the connection; the NEXT connection's first call to that
    /// role must author exactly one replacement — and only for that role.
    #[tokio::test]
    async fn a_heal_mark_buys_one_replacement_on_the_next_connection() {
        let cache = Cache::new();
        let admin = "heal-on-call-test:4444";
        let connection = five_role_connection(&cache, admin);
        let grants = AtomicU32::new(0);

        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await
            .unwrap();
        assert_eq!(grants.load(Ordering::SeqCst), 1);

        assert!(cache.mark_for_remint(&connection.ep_key, "imagodei"));
        drop(connection); // what clearing the socket does

        let next = five_role_connection(&cache, admin);
        next.ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
            counted_mint(&grants)
        })
        .await
        .unwrap();
        assert_eq!(
            grants.load(Ordering::SeqCst),
            2,
            "the heal must mint exactly one replacement"
        );

        next.ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
            counted_mint(&grants)
        })
        .await
        .unwrap();
        assert_eq!(
            grants.load(Ordering::SeqCst),
            2,
            "…and the mark is consumed"
        );
    }

    // ── the kept admin socket (lazy mints on a long-lived caller) ───────────

    fn never_written() -> MintFailure {
        MintFailure::Conductor(ConductorApiError::WebsocketError(
            holochain_websocket::WebsocketError::Close("No connection".to_string()),
        ))
    }

    /// A long-lived caller's first use of a new role finds its kept admin
    /// socket already torn down. Nothing was written, so the admin socket is
    /// redialled and the mint goes ahead — the call does not fail.
    #[tokio::test]
    async fn a_dead_kept_admin_socket_is_redialled_instead_of_failing_the_call() {
        let redials = AtomicU32::new(0);
        let outcome = retry_if_never_written(
            async { Err::<u8, _>(never_written()) },
            mint_never_written,
            || async {
                redials.fetch_add(1, Ordering::SeqCst);
                Ok(7u8)
            },
        )
        .await;

        assert_eq!(outcome.expect("the retry over a fresh socket succeeds"), 7);
        assert_eq!(redials.load(Ordering::SeqCst), 1);
    }

    /// A mint whose request WAS written is never retried: the grant may exist,
    /// and a second authorize would author a second one.
    #[tokio::test]
    async fn a_mint_that_was_written_is_never_retried() {
        use std::io::{Error as IoError, ErrorKind};
        for written in [
            MintFailure::Conductor(ConductorApiError::WebsocketError(
                holochain_websocket::WebsocketError::Close("ConnectionClosed".to_string()),
            )),
            MintFailure::Conductor(ConductorApiError::IoError(IoError::new(
                ErrorKind::BrokenPipe,
                "broken pipe",
            ))),
            MintFailure::Conductor(ConductorApiError::ExternalApiWireError(
                holochain_conductor_api::ExternalApiWireError::InternalError(
                    "source chain head has moved".to_string(),
                ),
            )),
            MintFailure::TimedOut,
            MintFailure::Redial("Admin connect failed".to_string()),
        ] {
            let redials = AtomicU32::new(0);
            let rendered = format!("{written:?}");
            let outcome = retry_if_never_written(
                async { Err::<u8, _>(written) },
                mint_never_written,
                || async {
                    redials.fetch_add(1, Ordering::SeqCst);
                    Ok(7u8)
                },
            )
            .await;

            assert!(outcome.is_err(), "must surface unchanged: {rendered}");
            assert_eq!(redials.load(Ordering::SeqCst), 0, "{rendered}");
        }
    }

    /// The common case: a live socket mints and nothing is redialled.
    #[tokio::test]
    async fn a_live_admin_socket_is_not_redialled() {
        let redials = AtomicU32::new(0);
        let outcome = retry_if_never_written(
            async { Ok::<u8, MintFailure>(1) },
            mint_never_written,
            || async {
                redials.fetch_add(1, Ordering::SeqCst);
                Ok(2u8)
            },
        )
        .await;

        assert_eq!(outcome.unwrap(), 1);
        assert_eq!(redials.load(Ordering::SeqCst), 0);
    }

    /// One redial, not a loop: if the fresh socket also fails, that surfaces.
    #[tokio::test]
    async fn the_redial_is_attempted_once() {
        let redials = AtomicU32::new(0);
        let outcome = retry_if_never_written(
            async { Err::<u8, _>(never_written()) },
            mint_never_written,
            || async {
                redials.fetch_add(1, Ordering::SeqCst);
                Err(never_written())
            },
        )
        .await;

        assert!(outcome.is_err());
        assert_eq!(redials.load(Ordering::SeqCst), 1);
    }

    // ── a mint the doorway stopped waiting for (bounded, not cured) ─────────

    /// A lease waiter must still be waiting when the slowest possible mint
    /// finishes: authorize, redial (resolve + connect), authorize again.
    #[test]
    fn the_lease_wait_covers_the_worst_case_mint() {
        let deadline = Duration::from_millis(DEFAULT_ZOME_CALL_TIMEOUT_MS);
        let wait = worst_case_mint(deadline);
        assert_eq!(
            wait,
            deadline + crate::conductor::dns_resolve_timeout() + deadline + deadline
        );
        assert!(
            wait > deadline,
            "a waiter given only one deadline falls through and mints beside the holder"
        );
    }

    /// Which mint failures may have authored a grant anyway.
    #[test]
    fn a_timed_out_or_dropped_mint_is_abandoned_and_a_refused_one_is_not() {
        let deadline = Duration::from_secs(10);
        assert!(mint_error("imagodei", deadline, MintFailure::TimedOut).outcome_unknown);
        assert!(
            mint_error(
                "imagodei",
                deadline,
                MintFailure::Conductor(ConductorApiError::WebsocketError(
                    holochain_websocket::WebsocketError::Close("ConnectionClosed".to_string()),
                )),
            )
            .outcome_unknown
        );
        for nothing_authored in [
            never_written(),
            MintFailure::Redial("Admin connect failed".to_string()),
            MintFailure::Conductor(ConductorApiError::ExternalApiWireError(
                holochain_conductor_api::ExternalApiWireError::InternalError(
                    "source chain head has moved".to_string(),
                ),
            )),
        ] {
            let rendered = format!("{nothing_authored:?}");
            assert!(
                !mint_error("imagodei", deadline, nothing_authored).outcome_unknown,
                "{rendered}"
            );
        }
        assert_eq!(
            mint_error("imagodei", deadline, MintFailure::TimedOut).message,
            "authorize_signing_credentials timed out after 10000ms for role 'imagodei'"
        );
    }

    /// End to end at the call path: a mint that times out is counted, and the
    /// next call to that role fails honestly instead of authoring a second
    /// grant.
    #[tokio::test(start_paused = true)]
    async fn a_timed_out_mint_is_not_repeated_by_the_next_call() {
        let cache = Cache::new();
        let connection = five_role_connection(&cache, "abandoned-mint-test:4444");
        let grants = AtomicU32::new(0);

        let first = connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| async {
                grants.fetch_add(1, Ordering::SeqCst);
                Err(MintError::abandoned("timed out"))
            })
            .await;
        assert!(first.is_err());

        let second = connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await;
        assert!(second.unwrap_err().contains("cooling down"));
        assert_eq!(grants.load(Ordering::SeqCst), 1);

        tokio::time::advance(mint_cooldown() + Duration::from_secs(1)).await;
        connection
            .ensure(&cache, "imagodei", NO_WAIT_EXPECTED, |_| {
                counted_mint(&grants)
            })
            .await
            .expect("after the cool-down, one mint");
        assert_eq!(grants.load(Ordering::SeqCst), 2);
    }
}
