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

use holochain_client::{
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
    ConductorApiError, ZomeCallTarget,
};
use holochain_types::prelude::ExternIO;
use serde::{de::DeserializeOwned, Serialize};
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
    admin_addr: String,
    app_addr: String,
    installed_app_id: String,
    /// The connected AppWebsocket (lazily initialized)
    app_ws: RwLock<Option<AppWebsocket>>,
    /// Lock to prevent concurrent connection attempts
    connecting: Mutex<()>,
}

impl ZomeCaller {
    /// Create a new ZomeCaller.
    ///
    /// `admin_url` and `app_url` can be either `ws://host:port` URLs or `host:port` addresses.
    pub fn new(admin_url: &str, app_url: &str, installed_app_id: &str) -> Self {
        info!(
            admin_url = %admin_url,
            app_url = %app_url,
            installed_app_id = %installed_app_id,
            "ZomeCaller created"
        );
        Self {
            admin_addr: strip_ws_scheme(admin_url),
            app_addr: strip_ws_scheme(app_url),
            installed_app_id: installed_app_id.to_string(),
            app_ws: RwLock::new(None),
            connecting: Mutex::new(()),
        }
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

        info!("ZomeCaller connecting to conductor");

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
        let reconnect_budget = deadline * 5 + Duration::from_secs(1);

        let connect_body = async {
            // Step 1: Connect to admin interface.
            // Hard-bounded: an unresolvable conductor host (cluster NXDOMAIN) must
            // resolve to an Err within `deadline`, never hang the caller forever.
            // Resolve via tokio's ASYNC resolver first (off the worker pool), then
            // connect with an already-resolved SocketAddr so holochain_client's
            // synchronous std::net getaddrinfo never parks a worker (alpha doorway
            // crashloop RCA 2026-06-15; see crate::conductor::resolve_host_port).
            // The per-step `timeout` below can only bound an async resolve, not a
            // blocking syscall — so this also makes that timeout effective.
            let admin_socket = crate::conductor::resolve_host_port(&self.admin_addr).await?;
            let admin_ws = tokio::time::timeout(
                deadline,
                AdminWebsocket::connect(admin_socket, Some(String::from("doorway-zome-caller"))),
            )
            .await
            .map_err(|_| {
                format!(
                    "Admin connect timed out after {}ms ({})",
                    deadline.as_millis(),
                    self.admin_addr
                )
            })?
            .map_err(|e| format!("Admin connect failed: {e}"))?;

            info!("ZomeCaller connected to admin at {}", self.admin_addr);

            // Step 2: Find cell_id from app info
            let apps = tokio::time::timeout(deadline, admin_ws.list_apps(None))
                .await
                .map_err(|_| format!("list_apps timed out after {}ms", deadline.as_millis()))?
                .map_err(|e| format!("list_apps failed: {e}"))?;

            let app_info = apps
                .iter()
                .find(|a| a.installed_app_id == self.installed_app_id)
                .ok_or_else(|| format!("App '{}' not found", self.installed_app_id))?;

            // Step 3: Authorize signing credentials for ALL provisioned cells
            let signer = ClientAgentSigner::default();
            let mut cell_count = 0u32;

            for (role_name, cells) in &app_info.cell_info {
                for cell in cells {
                    if let holochain_client::CellInfo::Provisioned(p) = cell {
                        let credentials = tokio::time::timeout(
                        deadline,
                        admin_ws.authorize_signing_credentials(
                            AuthorizeSigningCredentialsPayload {
                                cell_id: p.cell_id.clone(),
                                functions: None,
                            },
                        ),
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
                app_id = %self.installed_app_id,
                cells = cell_count,
                "Signing credentials authorized for all cells"
            );

            // Step 4: Issue app auth token
            let token = tokio::time::timeout(
                deadline,
                admin_ws.issue_app_auth_token(
                    holochain_client::IssueAppAuthenticationTokenPayload {
                        installed_app_id: self.installed_app_id.clone(),
                        expiry_seconds: 3600,
                        single_use: false,
                    },
                ),
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
            let app_socket = crate::conductor::resolve_host_port(&self.app_addr).await?;
            let app_ws = tokio::time::timeout(
                deadline,
                AppWebsocket::connect(app_socket, token.token, signer_arc, None),
            )
            .await
            .map_err(|_| {
                format!(
                    "App WebSocket connect timed out after {}ms ({})",
                    deadline.as_millis(),
                    self.app_addr
                )
            })?
            .map_err(|e| format!("App WebSocket connect failed: {e}"))?;

            info!(
                "ZomeCaller connected to app interface at {} with signing",
                self.app_addr
            );

            // Store connection
            {
                let mut ws = self.app_ws.write().await;
                *ws = Some(app_ws);
            }

            Ok::<(), String>(())
        };

        // Enforce the overall budget. On elapse, return promptly so `_lock`
        // drops and queued callers stop serializing behind a doomed reconnect.
        match tokio::time::timeout(reconnect_budget, connect_body).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                warn!(
                    budget_ms = reconnect_budget.as_millis(),
                    "ZomeCaller reconnect exceeded full budget; releasing connecting lock"
                );
                Err(format!(
                    "reconnect exceeded full budget after {}ms",
                    reconnect_budget.as_millis()
                ))
            }
        }
    }

    /// Call a zome function with raw bytes payload, return raw bytes.
    ///
    /// The AppWebsocket handles signing automatically.
    pub async fn call_zome(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        self.ensure_connected().await?;

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
            let app_ws = ws.as_ref().ok_or("Not connected")?;
            tokio::time::timeout(
                deadline,
                app_ws.call_zome(
                    ZomeCallTarget::RoleName(role_name.into()),
                    zome_name.into(),
                    fn_name.into(),
                    ExternIO::from(payload),
                ),
            )
            .await
        };

        // Unwrap the timeout layer: Elapsed → a synthetic conductor error that
        // triggers the same connection-reset path as a real call failure.
        let result = match result {
            Ok(inner) => inner,
            Err(_elapsed) => {
                warn!(
                    role_name = %role_name,
                    fn_name = %fn_name,
                    timeout_ms = deadline.as_millis(),
                    "Zome call timed out, clearing connection"
                );
                let mut ws = self.app_ws.write().await;
                *ws = None;
                return Err(format!(
                    "Zome call timed out after {}ms ({role_name}/{zome_name}/{fn_name})",
                    deadline.as_millis()
                ));
            }
        };

        match result {
            Ok(extern_io) => {
                debug!(
                    result_len = extern_io.as_bytes().len(),
                    "ZomeCaller zome call succeeded"
                );
                Ok(extern_io.into_vec())
            }
            Err(e) => {
                // Connection-churn root fix (2026-06-13 iteration 3): only a
                // genuinely-dead socket warrants a reconnect. A zome-returned
                // application error (the conductor answered) leaves the socket
                // healthy — clearing it here is the churn that wedged the
                // gateway under load. Split warn! so Loki distinguishes the two.
                if is_transport_error(&e) {
                    warn!(
                        role_name = %role_name,
                        fn_name = %fn_name,
                        "Zome call transport failure, clearing connection: {e}"
                    );
                    let mut ws = self.app_ws.write().await;
                    *ws = None;
                } else {
                    warn!(
                        role_name = %role_name,
                        fn_name = %fn_name,
                        "Zome call returned application error, keeping connection: {e}"
                    );
                }
                Err(format!("Zome call failed: {e}"))
            }
        }
    }

    /// Typed wrapper: serialize input with MessagePack, deserialize output.
    pub async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        input: &I,
    ) -> Result<O, String> {
        let payload = rmp_serde::to_vec_named(input)
            .map_err(|e| format!("Failed to serialize input: {e}"))?;

        let response_bytes = self
            .call_zome(role_name, zome_name, fn_name, payload)
            .await?;

        rmp_serde::from_slice(&response_bytes).map_err(|e| {
            let structure =
                match rmpv::decode::read_value(&mut std::io::Cursor::new(&response_bytes)) {
                    Ok(val) => format!("{val:?}"),
                    Err(decode_err) => {
                        format!("<raw {} bytes, decode err: {decode_err}>", response_bytes.len())
                    }
                };
            format!(
                "Failed to deserialize response: {e} | response_bytes({} bytes) structure: {structure}",
                response_bytes.len()
            )
        })
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        self.app_ws.read().await.is_some()
    }
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
        let budget = deadline * 5 + Duration::from_secs(1);
        // Strictly greater than the sum of the per-step deadlines, so a legitimate
        // (slow but progressing) reconnect is never falsely budget-killed…
        assert!(budget > deadline * 5);
        // …yet finite — never "no timeout", the bug class this whole module guards.
        assert_eq!(budget, Duration::from_millis(51_000));
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
