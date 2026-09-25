//! Holochain Client Wrapper
//!
//! Uses the official holochain_client crate for proper signed zome calls.
//! Holochain 0.6+ requires all zome calls to be signed with nonce, expires_at,
//! and ed25519 signature. This module handles:
//!
//! 1. Connecting to admin and app websockets
//! 2. Authorizing signing credentials via admin API
//! 3. Making signed zome calls that the conductor will accept
//!
//! ## Usage
//!
//! ```ignore
//! let client = HcClient::connect(HcClientConfig {
//!     admin_url: "localhost:4444".to_string(),
//!     app_url: "localhost:4445".to_string(),
//!     app_id: "elohim".to_string(),
//!     role: Some("lamad".to_string()),
//! }).await?;
//! let result = client.call_zome("content_store", "process_import_chunk", payload).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use holochain_client::{
    AdminWebsocket, AllowedOrigins, AppWebsocket, CellId, ClientAgentSigner, ConductorApiError,
    ExternIO, ZomeCallTarget,
};

use crate::conductor_admission::AdmissionClass;
use crate::conductor_bridge_health::{
    observe_role_app_status, observe_role_zome_error, record_role_success,
};
use crate::error::StorageError;

fn is_correlated_head_record_call(zome_name: &str, fn_name: &str) -> bool {
    if zome_name != "content_store" {
        return false;
    }
    tracing::Span::current().metadata().is_some_and(|metadata| {
        matches!(
            (fn_name, metadata.name()),
            ("get_record_for_action", "head_record_http")
                | ("resolve_canonical_election", "candidate_head_http")
                | ("get_record_for_action", "candidate_head_http")
                | ("validate_carried_head_record", "candidate_head_http")
        )
    })
}

/// Preserve the conductor client's typed transport deadline before the error
/// is flattened into [`StorageError`]. String matching here would conflate
/// in-wasm/domain messages containing "timeout" with the websocket deadline.
fn is_websocket_timeout(error: &ConductorApiError) -> bool {
    matches!(
        error,
        ConductorApiError::WebsocketError(holochain_websocket::WebsocketError::Timeout(_))
    )
}

/// Observe exactly one admitted websocket attempt while its typed result is
/// still available. Admission, retries, credential healing and backoff remain
/// outside this window.
async fn observe_conductor_attempt<T, F>(
    zome_name: &str,
    fn_name: &str,
    class: &'static str,
    attempt: F,
) -> Result<T, ConductorApiError>
where
    F: Future<Output = Result<T, ConductorApiError>>,
{
    let metrics = crate::metrics::ConductorCallMetricsGuard::start(zome_name, fn_name, class);
    let diagnostic = crate::diagnostics::ConductorAttempt::start(zome_name, fn_name, class);
    let result = attempt.await;
    if result.as_ref().is_err_and(is_websocket_timeout) {
        crate::metrics::inc_conductor_call_timeout(zome_name, fn_name, class);
    }
    let success = result.is_ok();
    metrics.finish(success);
    diagnostic.finish(success);
    result
}

/// What the admission gate observed about one zome call.
///
/// `admission_wait` is the queueing delay for LOCAL capacity; `rtt` is the
/// round-trip once dispatched. Their sum is the caller's wall-clock.
#[derive(Debug, Clone, Copy)]
pub struct ZomeCallTiming {
    /// How long the call queued at the capacity gate before dispatch.
    pub admission_wait: Duration,
    /// Observed round-trip, measured from dispatch (gate wait excluded).
    pub rtt: Duration,
}

/// Conductor health information for backpressure decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorHealth {
    /// Storage information
    pub storage: Option<StorageHealth>,
    /// Network statistics
    pub network: Option<NetworkHealth>,
    /// Raw responses for debugging (JSON strings)
    pub raw_storage: Option<String>,
    pub raw_network_stats: Option<String>,
    pub raw_network_metrics: Option<String>,
}

/// Storage health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Total bytes used by entries
    pub bytes_used: u64,
    /// Number of entries
    pub entry_count: u64,
}

/// Network health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    /// Number of connected peers
    pub peer_count: Option<u64>,
    /// Any additional stats we can extract
    pub details: String,
}

/// Configuration for HcClient
#[derive(Debug, Clone)]
pub struct HcClientConfig {
    /// Admin websocket URL (e.g., "ws://localhost:4444")
    pub admin_url: String,
    /// App websocket URL (e.g., "ws://localhost:4445")
    pub app_url: String,
    /// Installed app ID
    pub app_id: String,
    /// Cell target: provisioned role ("lamad") or enabled clone name/id ("lamad.fixtures").
    pub role: Option<String>,
}

/// Mint (or reuse) signing credentials for `cell` through the post-close write
/// fence.
///
/// # Why every authorize in this crate goes through one function
///
/// `AdminWebsocket::authorize_signing_credentials` generates a keypair and then
/// calls `grant_zome_call_capability`, which **commits a `CapGrant` entry on the
/// cell's source chain**. On a chain the network has seen closed, that one
/// action is warranted by every neighbour and turned into a `Timestamp::max()`
/// cell block that holochain 0.7 cannot lift — the household mesh partitioned
/// itself exactly this way on 2026-09-05 (Task 30, `6968baf1e`).
///
/// The fence gives three outcomes:
///
/// * credentials already persisted for this cell → **reused, nothing authored**
///   (this is what makes a `storage-restart` write-free);
/// * cell recorded closed and no persisted credentials → **refused by name**,
///   nothing reaches the conductor and this role stays unconnected (503),
///   which is recoverable where a block is not;
/// * otherwise → minted once and persisted, so the next restart takes the
///   first branch.
///
/// On a build that never armed a fence ([`crate::closed_chain_fence::fence`]
/// returns `None` — library consumers, unit tests) this is byte-for-byte the
/// pre-Task-32 call.
async fn authorize_signing_credentials_fenced(
    admin_ws: &AdminWebsocket,
    cell_id: &CellId,
    label: Option<&str>,
) -> Result<holochain_client::SigningCredentials, StorageError> {
    let label = label.unwrap_or("default");
    // A CapGrant is a chain write on the ADMIN socket, so it takes the same
    // per-cell lock every zome write takes — otherwise a mint during a running
    // sweep moves the head under a gated writer, which then reports an external
    // co-author it does not have. See `chain_write_gate::grant_capability_serialized`.
    crate::chain_write_gate::grant_capability_serialized(cell_id, || async {
        match crate::closed_chain_fence::fence() {
            Some(fence) => fence
                .authorize(admin_ws, cell_id, label)
                .await
                .map_err(|e| StorageError::Connection(e.to_string())),
            None => admin_ws
                .authorize_signing_credentials(
                    holochain_client::AuthorizeSigningCredentialsPayload {
                        cell_id: cell_id.clone(),
                        functions: None,
                    },
                )
                .await
                .map_err(|e| {
                    StorageError::Connection(format!(
                        "authorize_signing_credentials ({label}) failed: {e}"
                    ))
                }),
        }
    })
    .await
}

/// Substrings that mark a zome-call failure as "the conductor does not honour
/// our capability grant", as opposed to any other failure.
///
/// Deliberately narrow. A false positive costs one extra `CapGrant` on an open
/// chain (and only once per cell per process — the fence bounds it); a list
/// wide enough to catch a transport error would spend that grant on every
/// conductor hiccup. `signature` is NOT here for exactly that reason: it
/// appears in transport and serialization failures too.
/// Expiry for the token minted in [`HcClient::connect`]. Class M
/// (mint-and-consume): the token is used exactly once, in the
/// `AppWebsocket::connect()` call immediately below its issuance, and is
/// never stored — `HcClient` only holds the resulting authenticated
/// `AppWebsocket`. A reconnect goes through `HcClientRegistry`, which calls
/// `HcClient::connect` fresh each time, minting a new token, so a single-use
/// token never blocks reconnection. `single_use: true` is the posture; the
/// expiry only bounds an UNUSED token. It is generous on purpose (not
/// Holochain's 30s default): a connect that outlives it fails auth and
/// reconnects, and each reconnect here re-runs `authorize_signing_credentials`
/// — a cap-grant mint, the row class whose scan is the fleet's dominant
/// zome-call cost.
const MINT_AND_CONSUME_EXPIRY_SECS: u64 = 300;

const CAP_GRANT_REJECTION_MARKERS: &[&str] = &[
    "unauthorized",
    "capability",
    "cap grant",
    "capgrant",
    "cap secret",
    "capsecret",
];

/// Does this zome-call error say our persisted grant is no longer honoured?
fn looks_like_a_rejected_cap_grant(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    CAP_GRANT_REJECTION_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// A zome call was refused in a way that says our persisted `CapGrant` is gone.
/// Ask the fence to discard it, ONCE, so the next connect mints a fresh one.
///
/// # Why the heal is driven from the error path and not from connect
///
/// A credential file can be perfectly well-formed while the grant it names has
/// vanished from the chain — a conductor database restored from an older
/// snapshot under the same `CellId`. Nothing at connect time can see that:
/// the file parses, the key is valid, reuse is correct as far as the fence
/// knows. The first zome call is the only place the truth appears.
///
/// The fence itself enforces the two bounds that make this safe — never on a
/// closed cell, and at most once per cell per process — so this function stays
/// a classifier and a call.
fn heal_stale_signing_credentials(cell_id: &CellId, message: &str) {
    if !looks_like_a_rejected_cap_grant(message) {
        return;
    }
    let Some(fence) = crate::closed_chain_fence::fence() else {
        return;
    };
    fence.discard_stale_credentials(cell_id, message);
}

/// Refuse a zome call that would author on a closed chain, before it reaches
/// the admission gate or the websocket.
///
/// `Ok(())` means "not refused" — the cell is open, no fence is armed, or the
/// function is one of [`crate::closed_chain_fence::CLOSED_CHAIN_READ_FNS`],
/// which a closed chain still serves.
fn refuse_write_on_closed_chain(
    cell_id: &CellId,
    zome_name: &str,
    fn_name: &str,
) -> Result<(), StorageError> {
    let Some(fence) = crate::closed_chain_fence::fence() else {
        return Ok(());
    };
    match fence.refuse_zome_call(cell_id, zome_name, fn_name) {
        Some(reason) => {
            warn!(zome = %zome_name, fn_name = %fn_name, "{reason}");
            Err(StorageError::Conductor(reason))
        }
        None => Ok(()),
    }
}

/// Holochain client with proper signing support
#[allow(dead_code)]
pub struct HcClient {
    config: HcClientConfig,
    /// The admin + app websockets and the signer ONE mint produced — held
    /// behind a swappable slot, never as plain fields. See [`HcConnection`]:
    /// a conductor restart kills these sockets, and the registry's re-mint
    /// must be able to replace them under every `Arc<HcClient>` already
    /// handed out, not only under its own slot.
    conn: ConnectionSlot<HcConnection>,
    /// The cell ID for zome calls
    cell_id: CellId,
    /// The mishpat role's cell, when the installed happ provisions one.
    /// Every zome call routed through `call_zome` targets `cell_id` (the
    /// configured role, "lamad" in production) — so mishpat coordinator calls
    /// (`create_commitment`, `get_commitment`, state links) MUST target this
    /// cell instead: on a live conductor they otherwise die with
    /// `ZomeNotFound: mishpat`, because the lamad DNA has no mishpat zome.
    /// `None` when the happ has no mishpat role (minimal local-dev bundles) —
    /// `call_zome_mishpat` then returns a legible NotFound instead of the
    /// misleading ZomeNotFound.
    mishpat_cell_id: Option<CellId>,
    /// The imagodei role's cell, when the installed happ provisions one.
    ///
    /// Same routing hazard as [`Self::mishpat_cell_id`]: the `imagodei` zome
    /// (identity, qahal collectives + memberships) lives in the imagodei DNA, so
    /// an `imagodei` zome call routed through the default `call_zome` targets the
    /// lamad cell and dies `ZomeNotFound: imagodei` — AND fails signing, because
    /// the signer is per-cell. `call_zome_imagodei` is the correct route.
    /// `None` when the happ has no imagodei role (minimal local-dev bundles).
    imagodei_cell_id: Option<CellId>,
}

/// The part of an [`HcClient`] that one MINT produces and one conductor
/// restart destroys: both websockets and the signer whose credentials the app
/// websocket was authenticated with.
///
/// WHY THIS IS SEPARATE FROM THE CLIENT (2026-09-25). `HcClient` is handed out
/// as `Arc<HcClient>`, and most long-lived consumers — the feedback projector,
/// head adoption, reanchor backfill, projection reconcile, provide — clone that
/// `Arc` ONCE at boot and keep it for the life of the process. The bridge
/// supervisor's re-mint used to build a brand-new `HcClient` and store it in
/// the registry slot only, so after `just mesh conductors-restart` the
/// supervisor logged "RE-MINTED … zome path live" about ITS handle while every
/// boot-captured clone went on dialing the closed socket — 292 consecutive
/// `Websocket closed: No connection` failures on jessica, 376 on james, until
/// storage itself was restarted. Keeping the sockets in a slot lets the re-mint
/// swap them in place under every holder at once.
///
/// `generation` names the mint. A transport failure is reported against the
/// generation that produced it, so a failure from a socket that has already
/// been replaced can never re-arm a re-mint of its successor.
#[derive(Clone)]
pub(crate) struct HcConnection {
    admin_ws: AdminWebsocket,
    app_ws: AppWebsocket,
    /// Held so the credentials outlive every clone of the app websocket that
    /// signs with them.
    #[allow(dead_code)]
    signer: Arc<ClientAgentSigner>,
    generation: u64,
}

/// Process-wide source of [`HcConnection::generation`]. Monotonic, so "older
/// than the current mint" is a plain comparison.
static NEXT_CONNECTION_GENERATION: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

fn next_connection_generation() -> u64 {
    NEXT_CONNECTION_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A value every holder of the enclosing `Arc` reads fresh on each use, and
/// that one writer can replace for all of them at once.
///
/// Reads CLONE out and release the lock immediately, so no guard is ever held
/// across an `.await` and a swap never waits on an in-flight zome call (that
/// call finishes — or fails — on the socket it started on).
pub(crate) struct ConnectionSlot<T: Clone> {
    inner: std::sync::RwLock<T>,
}

impl<T: Clone> ConnectionSlot<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            inner: std::sync::RwLock::new(value),
        }
    }

    /// The current value, cloned out.
    pub(crate) fn current(&self) -> T {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Replace the value for every holder.
    pub(crate) fn replace(&self, value: T) {
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = value;
    }
}

/// Why a freshly minted connection may NOT be adopted by an existing client,
/// or `None` when it may.
///
/// Adoption swaps SOCKETS, never cell identity: every holder of the old client
/// keeps using its `cell_id`, `mishpat_cell_id` and `imagodei_cell_id`, and the
/// fresh signer only carries credentials for the fresh client's cells. So the
/// two must name the same app and exactly the same cells. A reinstall that
/// minted a new DNA hash or agent key is refused here, and the registry falls
/// back to storing the fresh client as a separate handle.
pub(crate) fn adoption_refusal(
    old: (&str, &CellId, Option<&CellId>, Option<&CellId>),
    new: (&str, &CellId, Option<&CellId>, Option<&CellId>),
) -> Option<String> {
    if old.0 != new.0 {
        return Some(format!("app id changed ({} -> {})", old.0, new.0));
    }
    if old.1 != new.1 {
        return Some(format!("role cell changed ({} -> {})", old.1, new.1));
    }
    if old.2 != new.2 {
        return Some("mishpat cell changed".to_string());
    }
    if old.3 != new.3 {
        return Some("imagodei cell changed".to_string());
    }
    None
}

/// The one door to the admission gate for a zome call: acquire the permit, then
/// count the call by zome, FUNCTION and class. Every `call_zome` path goes
/// through here so a dispatched call cannot escape attribution — the gate's own
/// series are keyed by zome only, which says a node is busy but not with what.
/// A shed returns before the count: nothing was dispatched.
async fn admit(
    class: AdmissionClass,
    zome_name: &str,
    fn_name: &str,
) -> Result<crate::conductor_admission::AdmissionPermit, StorageError> {
    let permit = crate::conductor_admission::admission()
        .acquire(class, zome_name)
        .await?;
    crate::metrics::inc_conductor_call(zome_name, fn_name, class.label());
    Ok(permit)
}

/// The `imagodei` role's name, as the installed hApp spells it.
pub const IMAGODEI_ROLE: &str = "imagodei";

/// The `mishpat` role's name, as the installed hApp spells it.
pub const MISHPAT_ROLE: &str = "mishpat";

/// Does a failed call's text belong in the health verdict of the mint that is
/// CURRENT now?
///
/// A transport-closed failure describes ONE socket. When the call rode mint
/// `call_generation` and the handle has since adopted a newer mint, that socket
/// is the corpse the re-mint replaced: folding its death into the verdict would
/// mark the fresh, answering mint dead until something else happened to speak
/// over it. Every other failure (a domain refusal, `CellDisabled`, a shed) is a
/// statement about the conductor or the cell, not the socket, and always folds.
pub(crate) fn failure_describes_current_mint(
    msg: &str,
    call_generation: u64,
    current_generation: u64,
) -> bool {
    !(crate::conductor_bridge_health::is_transport_dead(msg)
        && call_generation < current_generation)
}

/// Whose TRANSPORT verdict one answered `app_info` probe on `probing_role`'s
/// client vouches for.
///
/// The probe's own role, always. And, when the prober is the
/// [`crate::hc_client_registry::CROSS_CELL_DRIVER_ROLE`], every cross-cell role
/// too: `mishpat` has no socket of its own — its calls ride a supervised role's
/// app websocket, so a transport failure filed against it was that socket's
/// failure, and it has no probe of its own to cancel it. Nobody else vouches
/// for a role that HAS its own supervisor: two probers answering for one role
/// over two sockets would alternate its verdict, the flap class the per-role
/// observer exists to remove.
pub(crate) fn roles_a_probe_vouches_for(probing_role: &str) -> Vec<&str> {
    let mut roles = vec![probing_role];
    if probing_role == crate::hc_client_registry::CROSS_CELL_DRIVER_ROLE {
        roles.extend(crate::hc_client_registry::CROSS_CELL_ROLES);
    }
    roles
}

/// Which ROLE a zome call against `target` is an observation ABOUT.
///
/// THE WHOLE OF F4's FIX, as a pure function. Every observation — a success, a
/// `CellDisabled`, any other failure — must be filed against the role that owns
/// the cell the call TARGETED, never against the calling client's configured
/// role. Filing by the client's role produced two symmetrical lies:
///
/// * a mishpat `CellDisabled` marked **lamad** not-running, so `/health` named
///   the wrong role and `enable_app` laddered against a role that was fine; and
/// * an ordinary **lamad** success then cleared that episode and its ladder,
///   while the mishpat cell was still refusing every call — which defeats the
///   per-role isolation the whole registry exists for, in both directions.
///
/// `mishpat` is checked before `imagodei` only because the two are disjoint
/// cells and an order had to be chosen. A client whose OWN cell is the imagodei
/// cell resolves to the same `"imagodei"` either way, which is why the answer is
/// consistent no matter which client asks.
pub(crate) fn target_role_for_cell<'a>(
    configured_role: &'a str,
    mishpat_cell_id: Option<&CellId>,
    imagodei_cell_id: Option<&CellId>,
    target: &CellId,
) -> &'a str {
    if mishpat_cell_id == Some(target) {
        return MISHPAT_ROLE;
    }
    if imagodei_cell_id == Some(target) {
        return IMAGODEI_ROLE;
    }
    configured_role
}

/// F4 — the attribution decision, tested at the seam every call path shares.
///
/// The reviewer's note is the reason this module exists: the isolated-role test
/// in `conductor_bridge_health` pins the FOLDS but does not exercise this
/// WIRING, and the wiring is where the bug was. The three `HcClient` methods
/// cannot be driven without a conductor, so the decision they all share is
/// lifted into [`target_role_for_cell`] and driven directly, with synthetic
/// `CellId`s standing in for the three cells `connect` resolves.
#[cfg(test)]
mod target_attribution_tests {
    use super::*;
    use holochain_types::prelude::{AgentPubKey, DnaHash};

    /// A distinct, valid `CellId` per `tag`. The DNA hash is what distinguishes
    /// the cells in production (one agent, five DNAs), so only it varies.
    fn cell(tag: u8) -> CellId {
        CellId::new(
            DnaHash::from_raw_32(vec![tag; 32]),
            AgentPubKey::from_raw_32(vec![0xAA; 32]),
        )
    }

    #[test]
    fn a_mishpat_failure_is_recorded_against_mishpat_not_the_client_role() {
        // THE DEFECT. The production client is configured `lamad` and reaches
        // the mishpat cell through `call_zome_mishpat`. Every observation about
        // that call — `CellDisabled` above all — used to be filed under
        // `lamad`, so `/health` named a role that was fine and the enable
        // ladder climbed against it.
        let lamad = cell(1);
        let mishpat = cell(2);
        let imagodei = cell(3);

        assert_eq!(
            target_role_for_cell("lamad", Some(&mishpat), Some(&imagodei), &mishpat),
            MISHPAT_ROLE,
            "a call that targeted the mishpat cell is an observation about mishpat"
        );
        assert_eq!(
            target_role_for_cell("lamad", Some(&mishpat), Some(&imagodei), &imagodei),
            IMAGODEI_ROLE,
            "and a call that targeted the imagodei cell is an observation about imagodei"
        );
        assert_eq!(
            target_role_for_cell("lamad", Some(&mishpat), Some(&imagodei), &lamad),
            "lamad",
            "the client's own cell is the one case where the configured role IS the target"
        );
    }

    #[test]
    fn a_client_that_provisions_no_cross_cell_role_attributes_to_itself() {
        // A minimal local-dev bundle resolves neither cross-cell CellId, so
        // every call this client can make targets its own cell.
        let own = cell(1);
        assert_eq!(
            target_role_for_cell("infrastructure", None, None, &own),
            "infrastructure"
        );
        // ...and an unrelated cell it somehow met is NOT silently claimed by a
        // cross-cell role it does not hold.
        assert_eq!(
            target_role_for_cell("infrastructure", None, None, &cell(9)),
            "infrastructure"
        );
    }

    #[test]
    fn the_imagodei_clients_own_cell_resolves_consistently_from_either_direction() {
        // Two clients can reach the imagodei cell: the `imagodei`-role client
        // through `call_zome`, and any other role's client through
        // `call_zome_imagodei`. Both must file under the SAME key or the two
        // routes would keep two half-episodes for one cell.
        let imagodei = cell(3);
        assert_eq!(
            target_role_for_cell(IMAGODEI_ROLE, None, Some(&imagodei), &imagodei),
            IMAGODEI_ROLE
        );
        assert_eq!(
            target_role_for_cell("lamad", None, Some(&imagodei), &imagodei),
            IMAGODEI_ROLE
        );
    }

    /// Every role this function can name must be a role the health layer
    /// observes, or an episode would be opened against a key nothing aggregates,
    /// probes, or renders — a fold into a black hole.
    #[test]
    fn every_attributable_cross_cell_role_is_observed() {
        for role in [MISHPAT_ROLE, IMAGODEI_ROLE] {
            assert!(
                crate::hc_client_registry::OBSERVED_ROLES.contains(&role),
                "'{role}' can receive an observation but is not observed"
            );
        }
    }
}

impl HcClient {
    /// This client's configured role, for keying the per-role zome-path
    /// observer (see [`crate::conductor_bridge_health::RoleBridgeHealth`]).
    /// Falls back to `"default"` for a client with no role configured (none
    /// of today's production call sites leave `role` unset, but the fallback
    /// keeps this total rather than panicking on a hypothetical one).
    ///
    /// This is the role of the client's OWN cell. It is the right key ONLY for
    /// a call that targeted that cell; a cross-cell call must key on
    /// [`target_role_for_cell`] instead.
    pub(crate) fn role_key(&self) -> &str {
        self.config.role.as_deref().unwrap_or("default")
    }

    /// Which role owns `target` on this client. See [`target_role_for_cell`].
    fn target_role_of(&self, target: &CellId) -> &str {
        target_role_for_cell(
            self.role_key(),
            self.mishpat_cell_id.as_ref(),
            self.imagodei_cell_id.as_ref(),
            target,
        )
    }

    /// Map a `holochain_client` zome-call failure onto a [`StorageError`] AND
    /// fold it into this client's ROLE-scoped zome-path observer (which
    /// re-derives the process-wide aggregate — see
    /// [`crate::conductor_bridge_health::observe_role_zome_error`]).
    ///
    /// Every zome-call site funnels through here so the observation cannot be
    /// forgotten at one of them: a bridge that dies on the mishpat cell is
    /// exactly as dead as one that dies on lamad, and `/health` must say so
    /// either way — including WHICH role, now that the observer is per-role.
    /// Classification (transport-dead vs the conductor answering "no" vs an
    /// admission shed that never left this process) lives in
    /// [`crate::conductor_bridge_health::classify_zome_error`], NOT here.
    fn zome_call_failed(&self, generation: u64, e: impl std::fmt::Display) -> StorageError {
        self.zome_call_failed_on(&self.cell_id, generation, e)
    }

    /// [`Self::zome_call_failed`] for a call that targeted a cell OTHER than the
    /// configured role's (the mishpat and imagodei arms), so the credential heal
    /// below blames the right chain — AND so the health observation does.
    ///
    /// The role is derived from `cell_id` rather than taken from the caller,
    /// deliberately: every call path already hands this the cell it targeted, so
    /// deriving here means no call site can forget to name the target and none
    /// of them has to change to get the attribution right.
    ///
    /// `generation` is the mint whose socket carried the call. A transport-dead
    /// failure is reported to the bridge supervisor against THIS CLIENT's role
    /// (the socket is the client's, whichever cell the call targeted), so the
    /// supervisor re-mints on zome-call evidence instead of waiting for its own
    /// next ping — and a failure from an already-replaced socket is ignored.
    fn zome_call_failed_on(
        &self,
        cell_id: &CellId,
        generation: u64,
        e: impl std::fmt::Display,
    ) -> StorageError {
        let msg = format!("Zome call failed: {}", e);
        // A socket the re-mint already replaced says nothing about the mint
        // callers dial now (see [`failure_describes_current_mint`]).
        if failure_describes_current_mint(&msg, generation, self.connection_generation()) {
            observe_role_zome_error(self.target_role_of(cell_id), &msg);
        }
        heal_stale_signing_credentials(cell_id, &msg);
        if crate::conductor_bridge_health::is_transport_dead(&msg) {
            crate::hc_client_registry::note_transport_closed(self.role_key(), generation);
        }
        StorageError::Conductor(msg)
    }

    /// The sockets this call should use, read fresh — see [`HcConnection`].
    fn connection(&self) -> HcConnection {
        self.conn.current()
    }

    /// Which mint this client's sockets currently come from.
    pub(crate) fn connection_generation(&self) -> u64 {
        self.conn.current().generation
    }

    /// Take over `fresh`'s sockets IN PLACE, so every `Arc` of `self` already
    /// handed out dials them from its next call on. Returns the adopted
    /// generation, or why adoption is refused (see [`adoption_refusal`]).
    pub(crate) fn adopt_connection_from(&self, fresh: &HcClient) -> Result<u64, String> {
        if let Some(why) = adoption_refusal(
            (
                &self.config.app_id,
                &self.cell_id,
                self.mishpat_cell_id.as_ref(),
                self.imagodei_cell_id.as_ref(),
            ),
            (
                &fresh.config.app_id,
                &fresh.cell_id,
                fresh.mishpat_cell_id.as_ref(),
                fresh.imagodei_cell_id.as_ref(),
            ),
        ) {
            return Err(why);
        }
        let adopted = fresh.connection();
        let generation = adopted.generation;
        self.conn.replace(adopted);
        Ok(generation)
    }

    /// Strip ws:// or wss:// prefix from URL to get socket address
    /// holochain_client expects "host:port" format, not "ws://host:port"
    fn to_socket_addr(url: &str) -> String {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("wss://") {
            rest.to_string()
        } else if let Some(rest) = url.strip_prefix("ws://") {
            rest.to_string()
        } else {
            url.to_string()
        }
    }

    /// Connect to Holochain conductor and set up signing credentials
    pub async fn connect(config: HcClientConfig) -> Result<Self, StorageError> {
        // Convert URLs to socket addresses (strip ws:// prefix)
        let admin_addr = Self::to_socket_addr(&config.admin_url);
        let app_addr = Self::to_socket_addr(&config.app_url);

        info!(
            admin_addr = %admin_addr,
            app_addr = %app_addr,
            app_id = %config.app_id,
            "Connecting to Holochain conductor with signing support"
        );

        // Connect to admin interface (socket_addr, origin)
        let admin_ws = AdminWebsocket::connect(&admin_addr, None)
            .await
            .map_err(|e| StorageError::Connection(format!("Admin connect failed: {}", e)))?;

        info!("Connected to admin interface");

        // Ensure an app interface exists on the expected port.
        // The embedded conductor (tauri-plugin-holochain) uses random ports,
        // so we attach one on our expected port via admin API.
        let app_port: u16 = app_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4445);
        match admin_ws
            .attach_app_interface(app_port, None, AllowedOrigins::Any, None)
            .await
        {
            Ok(port) => info!(port, "Attached app interface"),
            Err(e) => {
                // May already be attached from a previous call — continue
                warn!(
                    "attach_app_interface on port {}: {} (may already exist)",
                    app_port, e
                );
            }
        }

        // Get app info to find cell
        let apps = admin_ws
            .list_apps(None)
            .await
            .map_err(|e| StorageError::Connection(format!("list_apps failed: {}", e)))?;

        let app_info = apps
            .iter()
            .find(|a| a.installed_app_id == config.app_id)
            .ok_or_else(|| StorageError::NotFound(format!("App '{}' not found", config.app_id)))?;

        // Find the cell for the specified role
        let cell_id = if let Some(role) = &config.role {
            let (role_name, _) = crate::cell_discovery::split_cell_target(role)?;
            let cells = app_info.cell_info.get(role_name).ok_or_else(|| {
                StorageError::NotFound(format!("Cell target '{role}' not found; no fallback"))
            })?;
            crate::cell_discovery::select_target_cell(cells, role)?
        } else {
            // Use first available cell
            app_info
                .cell_info
                .values()
                .next()
                .and_then(|cells| cells.first())
                .and_then(|cell| match cell {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
                .ok_or_else(|| StorageError::NotFound("No cells found".to_string()))?
        };

        info!(
            dna_hash = %hex::encode(&cell_id.dna_hash().get_raw_39()[..8]),
            "Found cell"
        );

        // Resolve the mishpat role's cell when the happ provisions one, so
        // governance zome calls target the DNA that actually carries the
        // mishpat zome (targeting the lamad cell dies with ZomeNotFound).
        let mishpat_cell_id = app_info
            .cell_info
            .get("mishpat")
            .and_then(|cells| cells.first())
            .and_then(|cell| match cell {
                holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                _ => None,
            });
        match &mishpat_cell_id {
            Some(mc) => info!(
                dna_hash = %hex::encode(&mc.dna_hash().get_raw_39()[..8]),
                "Found mishpat cell"
            ),
            None => info!("No mishpat role in installed happ — mishpat zome calls unavailable"),
        }

        // Resolve the imagodei role's cell for the same reason: the imagodei
        // zome (qahal collectives/memberships, identity) lives in the imagodei
        // DNA, so routing those calls at the lamad cell dies ZomeNotFound.
        let imagodei_cell_id = app_info
            .cell_info
            .get("imagodei")
            .and_then(|cells| cells.first())
            .and_then(|cell| match cell {
                holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                _ => None,
            });
        match &imagodei_cell_id {
            Some(ic) => info!(
                dna_hash = %hex::encode(&ic.dna_hash().get_raw_39()[..8]),
                "Found imagodei cell"
            ),
            None => info!("No imagodei role in installed happ — imagodei zome calls unavailable"),
        }

        // Create signing credentials
        let signer = ClientAgentSigner::default();

        // Signing credentials for this cell.
        //
        // **This is a CHAIN WRITE, not a handshake** — `authorize_signing_credentials`
        // commits a `CapGrant` on the cell's source chain. Task 30 measured what
        // that costs on a chain the network has seen closed: every neighbour
        // warrants the author and turns the warrant into a permanent cell block
        // holochain 0.7 cannot lift. So it goes through the post-close fence
        // (Task 32), which reuses persisted credentials wherever they exist and
        // refuses a mint on a closed cell BY NAME rather than authoring it.
        let credentials =
            authorize_signing_credentials_fenced(&admin_ws, &cell_id, config.role.as_deref())
                .await?;

        // Add credentials to signer
        signer.add_credentials(cell_id.clone(), credentials);

        // Authorize the mishpat cell too — the signer is per-cell, so without
        // this a role-targeted mishpat call fails at signing, not at dispatch.
        if let Some(mc) = &mishpat_cell_id {
            let mishpat_credentials =
                authorize_signing_credentials_fenced(&admin_ws, mc, Some("mishpat")).await?;
            signer.add_credentials(mc.clone(), mishpat_credentials);
        }

        // Same for imagodei — without per-cell credentials a role-targeted
        // imagodei call fails at signing rather than at dispatch.
        if let Some(ic) = &imagodei_cell_id {
            let imagodei_credentials =
                authorize_signing_credentials_fenced(&admin_ws, ic, Some("imagodei")).await?;
            signer.add_credentials(ic.clone(), imagodei_credentials);
        }
        info!("Signing credentials authorized");

        // Get app auth token
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: config.app_id.clone(),
                expiry_seconds: MINT_AND_CONSUME_EXPIRY_SECS,
                single_use: true,
            })
            .await
            .map_err(|e| StorageError::Connection(format!("issue_app_auth_token failed: {}", e)))?;

        // Connect to app interface with signer (socket_addr, token, signer, origin)
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);
        let app_ws = AppWebsocket::connect(&app_addr, token.token, signer_arc.clone(), None)
            .await
            .map_err(|e| StorageError::Connection(format!("App connect failed: {}", e)))?;

        info!("Connected to app interface with signing");

        Ok(Self {
            config,
            conn: ConnectionSlot::new(HcConnection {
                admin_ws,
                app_ws,
                signer: signer_arc,
                generation: next_connection_generation(),
            }),
            cell_id,
            mishpat_cell_id,
            imagodei_cell_id,
        })
    }

    /// Make a signed zome call against the IMAGODEI cell (identity/qahal DNA).
    ///
    /// The default [`Self::call_zome`] targets the configured role's cell (lamad
    /// in production), whose DNA has no `imagodei` zome — routing identity /
    /// qahal calls there fails `ZomeNotFound` on every live conductor (and would
    /// fail signing first, the signer being per-cell). Mirrors
    /// [`Self::call_zome_mishpat`] exactly.
    pub async fn call_zome_imagodei(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        let cell_id = self.imagodei_cell_id.clone().ok_or_else(|| {
            StorageError::NotFound(
                "imagodei cell not provisioned in installed happ — identity/qahal zome calls \
                 unavailable on this node"
                    .to_string(),
            )
        })?;
        debug!(
            zome = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            "Making signed zome call (imagodei cell)"
        );
        refuse_write_on_closed_chain(&cell_id, zome_name, fn_name)?;
        // The target is MOVED into `ZomeCallTarget` below; keep a copy so a
        // rejected grant can be blamed on the right cell (Task 32 fix round 1).
        let cell_for_heal = cell_id.clone();
        // Same gate, same pool: the imagodei cell is a different DNA but the same
        // conductor, so its calls compete for the same read permits.
        // CHAIN LOCK FIRST, capacity INSIDE it — `chain_write_gate`'s
        // "Capacity is acquired INSIDE the lock" section has the why: a writer
        // parked on the mutex must not be sitting on a conductor read permit.
        let chain_key = crate::chain_write_gate::chain_key_of(&cell_id);
        let (result, _rtt) =
            crate::chain_write_gate::dispatch(&chain_key, zome_name, fn_name, || {
                let payload = payload.clone();
                let target = cell_id.clone();
                let heal_cell = cell_for_heal.clone();
                async move {
                    // Same gate, same pool: a different DNA on the same
                    // conductor competes for the same read permits. Held across
                    // the call only, then dropped.
                    let _permit = admit(AdmissionClass::Interactive, zome_name, fn_name).await?;
                    let conn = self.connection();
                    observe_conductor_attempt(
                        zome_name,
                        fn_name,
                        AdmissionClass::Interactive.label(),
                        conn.app_ws.call_zome(
                            ZomeCallTarget::CellId(target),
                            zome_name.into(),
                            fn_name.into(),
                            ExternIO::from(payload),
                        ),
                    )
                    .await
                    .inspect(|_| {
                        crate::hc_client_registry::note_transport_ok(
                            self.role_key(),
                            conn.generation,
                        )
                    })
                    .map_err(|e| self.zome_call_failed_on(&heal_cell, conn.generation, e))
                }
            })
            .await?;
        // THE TARGET's role, not this client's. A call that landed on the
        // imagodei cell proves the imagodei cell is serving and proves NOTHING
        // about the cell this client is configured for — filing it under
        // `role_key()` let an imagodei success clear a lamad episode, and
        // vice versa.
        record_role_success(IMAGODEI_ROLE);
        Ok(result.into_vec())
    }

    /// Make a signed zome call against the MISHPAT cell (governance DNA).
    /// The default `call_zome` targets the configured role's cell (lamad in
    /// production), whose DNA has no mishpat zome — routing governance calls
    /// there fails ZomeNotFound on every live conductor.
    pub async fn call_zome_mishpat(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        let cell_id = self.mishpat_cell_id.clone().ok_or_else(|| {
            StorageError::NotFound(
                "mishpat cell not provisioned in installed happ — governance zome calls \
                 unavailable on this node"
                    .to_string(),
            )
        })?;
        debug!(
            zome = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            "Making signed zome call (mishpat cell)"
        );
        refuse_write_on_closed_chain(&cell_id, zome_name, fn_name)?;
        // The target is MOVED into `ZomeCallTarget` below; keep a copy so a
        // rejected grant can be blamed on the right cell (Task 32 fix round 1).
        let cell_for_heal = cell_id.clone();
        // Same gate, same pool — see `call_zome_imagodei`.
        // CHAIN LOCK FIRST, capacity INSIDE it — `chain_write_gate`'s
        // "Capacity is acquired INSIDE the lock" section has the why: a writer
        // parked on the mutex must not be sitting on a conductor read permit.
        let chain_key = crate::chain_write_gate::chain_key_of(&cell_id);
        let (result, _rtt) =
            crate::chain_write_gate::dispatch(&chain_key, zome_name, fn_name, || {
                let payload = payload.clone();
                let target = cell_id.clone();
                let heal_cell = cell_for_heal.clone();
                async move {
                    // Same gate, same pool: a different DNA on the same
                    // conductor competes for the same read permits. Held across
                    // the call only, then dropped.
                    let _permit = admit(AdmissionClass::Interactive, zome_name, fn_name).await?;
                    let conn = self.connection();
                    observe_conductor_attempt(
                        zome_name,
                        fn_name,
                        AdmissionClass::Interactive.label(),
                        conn.app_ws.call_zome(
                            ZomeCallTarget::CellId(target),
                            zome_name.into(),
                            fn_name.into(),
                            ExternIO::from(payload),
                        ),
                    )
                    .await
                    .inspect(|_| {
                        crate::hc_client_registry::note_transport_ok(
                            self.role_key(),
                            conn.generation,
                        )
                    })
                    .map_err(|e| self.zome_call_failed_on(&heal_cell, conn.generation, e))
                }
            })
            .await?;
        // THE TARGET's role — see `call_zome_imagodei`. A mishpat call that
        // landed says the mishpat cell is serving, and `mishpat` is now an
        // OBSERVED role in its own right
        // (`hc_client_registry::OBSERVED_ROLES`), so its episodes begin and end
        // on its own evidence instead of being charged to lamad.
        record_role_success(MISHPAT_ROLE);
        Ok(result.into_vec())
    }

    /// Make a signed zome call.
    ///
    /// Admitted through the process-wide capacity gate
    /// ([`crate::conductor_admission`]) as an INTERACTIVE caller. See
    /// [`Self::call_zome_timed`] when the caller needs the gate's timing back.
    pub async fn call_zome(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        self.call_zome_timed(zome_name, fn_name, payload, AdmissionClass::Interactive)
            .await
            .map(|(bytes, _timing)| bytes)
    }

    /// [`Self::call_zome`] with the caller's admission class and the timing the
    /// gate observed.
    ///
    /// The timing is the honest pacing signal for a controller: `admission_wait`
    /// rises monotonically with offered load, whereas `RTT − in-wasm elapsed_ms`
    /// subtracts exactly the interval an extern spends blocked on a conductor
    /// read permit — so a stalling conductor reads as *zero* queue-wait there and
    /// invites a controller to ask for more.
    pub async fn call_zome_timed(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
        class: AdmissionClass,
    ) -> Result<(Vec<u8>, ZomeCallTiming), StorageError> {
        let is_head_record = is_correlated_head_record_call(zome_name, fn_name);
        debug!(
            zome = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            class = class.label(),
            "Making signed zome call"
        );

        // A close is a sealing act: refuse anything that would author on a
        // sealed chain BEFORE the admission gate, so a refused write costs the
        // conductor nothing and can never reach the websocket.
        refuse_write_on_closed_chain(&self.cell_id, zome_name, fn_name)?;

        // CHAIN LOCK BEFORE CAPACITY. The gate takes this cell's write lock (for
        // a write; a read passes straight through), and the closure below takes
        // the admission permit INSIDE it. That order is the whole point — see
        // `chain_write_gate`'s "Capacity is acquired INSIDE the lock": a writer
        // queued on the mutex must not be sitting on one of the pool's ~5
        // permits, or eight concurrent sweep declares starve every interactive
        // read into a 503 shed.
        //
        // Admission is still a bound on acquiring LOCAL capacity, never a
        // timeout: once admitted the call runs unbounded on our side exactly as
        // before, bounded on the far side by the extern's own in-wasm deadline.
        if is_head_record {
            info!(phase = "admission_waiting", "head-record conductor phase");
        }

        let dispatched_at = Instant::now();
        // The holochain_client handles signing automatically
        // Use ExternIO::from() for raw bytes - payload is already MessagePack encoded
        //
        // `rtt` is the gate's per-ATTEMPT measurement, not the serialized
        // wall-clock, so the controller signal documented above keeps meaning
        // "what the conductor took".
        let chain_key = crate::chain_write_gate::chain_key_of(&self.cell_id);
        let dispatched = crate::chain_write_gate::dispatch(&chain_key, zome_name, fn_name, || {
            let payload = payload.clone();
            let target = self.cell_id.clone();
            async move {
                let permit = match admit(class, zome_name, fn_name).await {
                    Ok(permit) => permit,
                    Err(error) => {
                        if is_head_record {
                            info!(phase = "admission_error", error = %error, "head-record conductor phase");
                        }
                        return Err(error);
                    }
                };
                let admission_wait = permit.wait();
                if is_head_record {
                    info!(
                        phase = "admission_acquired",
                        admission_wait_ms = admission_wait.as_millis(),
                        "head-record conductor phase"
                    );
                }
                let conn = self.connection();
                let result = observe_conductor_attempt(
                    zome_name,
                    fn_name,
                    class.label(),
                    conn.app_ws.call_zome(
                        ZomeCallTarget::CellId(target),
                        zome_name.into(),
                        fn_name.into(),
                        ExternIO::from(payload),
                    ),
                )
                .await;
                if result.is_ok() {
                    crate::hc_client_registry::note_transport_ok(self.role_key(), conn.generation);
                }
                let result = result.map_err(|e| self.zome_call_failed(conn.generation, e));
                // Held across the whole call on purpose: the permit models
                // capacity the conductor is still spending, and releasing it
                // early would understate occupancy by exactly the interval that
                // matters most. It is released HERE — before any backoff sleep
                // and before this writer re-queues on the chain lock.
                drop(permit);
                result.map(|bytes| (bytes, admission_wait))
            }
        })
        .await;
        let (result, rtt, admission_wait) = match dispatched {
            Ok(((result, admission_wait), rtt)) => (Ok(result), rtt, admission_wait),
            Err(error) => (Err(error), dispatched_at.elapsed(), Duration::ZERO),
        };
        let result = match result {
            Ok(result) => {
                if is_head_record {
                    info!(
                        phase = "zome_call_complete",
                        rtt_ms = rtt.as_millis(),
                        "head-record conductor phase"
                    );
                }
                result
            }
            Err(error) => {
                // Already mapped (and observed, and credential-healed) inside
                // the gated closure — one observation per ATTEMPT, which is the
                // honest count: each attempt really did fail at the conductor.
                if is_head_record {
                    info!(
                        phase = "zome_call_error",
                        rtt_ms = rtt.as_millis(),
                        error = %error,
                        "head-record conductor phase"
                    );
                }
                return Err(error);
            }
        };
        // THE TARGET's role, which on this path IS the configured role: this
        // method dispatches against `self.cell_id`, and `connect` resolves that
        // from `app_info.cell_info[role]`. The cross-cell methods above are the
        // two paths where target and configuration differ.
        record_role_success(self.role_key());

        let timing = ZomeCallTiming {
            admission_wait,
            rtt,
        };

        // Return raw bytes - caller will deserialize as needed
        Ok((result.into_vec(), timing))
    }

    /// Get the cell ID
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    /// The `CellId` this client can reach for `role` — its own configured role,
    /// or one of the two cross-cell roles it resolved from `app_info` at connect
    /// time.
    ///
    /// `None` is a real answer: a minimal local-dev bundle provisions no
    /// mishpat/imagodei role, and a caller asking about a role this client does
    /// not carry must get "I cannot ask" rather than the configured cell. The
    /// membership join uses this, and answering with the WRONG cell would report
    /// one role's running state under another role's name — the same
    /// misattribution `target_role_for_cell` exists to prevent on the error side.
    pub fn cell_id_for_role(&self, role: &str) -> Option<&CellId> {
        if role == self.role_key() {
            return Some(&self.cell_id);
        }
        match role {
            MISHPAT_ROLE => self.mishpat_cell_id.as_ref(),
            IMAGODEI_ROLE => self.imagodei_cell_id.as_ref(),
            _ => None,
        }
    }

    /// A handle to this client's conductor ADMIN websocket.
    ///
    /// `AdminWebsocket` is Clone (its internal state is refcounted), so this
    /// is a cheap handle to the SAME live connection — not a new dial. Lets
    /// admin-plane reads (`GET /db/p2p/conductor-diagnostics`: `agent_info`,
    /// `dump_network_stats`, `dump_network_metrics`) work on an EXTERNAL
    /// conductor (`--admin-url` mode), where no embedded-conductor admin
    /// handle exists but every registry role already holds one.
    pub fn admin_websocket(&self) -> AdminWebsocket {
        self.connection().admin_ws
    }

    /// Get DNA hash bytes
    pub fn dna_hash(&self) -> Vec<u8> {
        self.cell_id.dna_hash().get_raw_39().to_vec()
    }

    /// Get agent pubkey bytes
    pub fn agent_pub_key(&self) -> Vec<u8> {
        self.cell_id.agent_pubkey().get_raw_39().to_vec()
    }

    /// Get the cell's owner agent key as the canonical `uhCAk…` string.
    ///
    /// This is the `agent_cid` namespace used as the JOIN KEY across
    /// `humans.agent_pub_key`, `shard_locations.peer_id`, and
    /// `rea_commitments.provider` (see `CLAUDE.md` Identity table). It is NOT
    /// the same as `agent_pub_key()` (raw bytes) or `agent_key_hex()` (hex) —
    /// those are different encodings and silently empty any join against an
    /// `agent_cid` column. Used by the genesis self-heal-identity bootstrap to
    /// fill the pod's own `humans.agent_pub_key` from its own cell key.
    pub fn agent_key_uhcak(&self) -> String {
        self.cell_id.agent_pubkey().to_string()
    }

    /// Get conductor health metrics for backpressure decisions
    ///
    /// Fetches storage info, network stats, and network metrics from the conductor.
    /// Returns raw JSON responses for evaluation - we can parse specific fields later
    /// once we understand the response structure.
    pub async fn get_health(&self) -> ConductorHealth {
        let mut health = ConductorHealth {
            storage: None,
            network: None,
            raw_storage: None,
            raw_network_stats: None,
            raw_network_metrics: None,
        };

        // Fetch storage info
        match self.admin_websocket().storage_info().await {
            Ok(storage_info) => {
                // Convert to JSON for inspection
                let json = serde_json::to_string_pretty(&storage_info)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_storage = Some(json.clone());

                // Try to extract key metrics
                // StorageInfo has blobs.used_by_entries field
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let bytes_used = parsed["blobs"]["used_by_entries"].as_u64().unwrap_or(0);
                    let entry_count = parsed["blobs"]["used_by_entries_count"]
                        .as_u64()
                        .or_else(|| parsed["entries_count"].as_u64())
                        .unwrap_or(0);

                    health.storage = Some(StorageHealth {
                        bytes_used,
                        entry_count,
                    });
                }

                info!(
                    bytes_used = health.storage.as_ref().map(|s| s.bytes_used).unwrap_or(0),
                    "📊 STORAGE_INFO: Fetched conductor storage metrics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch storage_info");
                health.raw_storage = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network stats
        match self.admin_websocket().dump_network_stats().await {
            Ok(network_stats) => {
                let json = serde_json::to_string_pretty(&network_stats)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_stats = Some(json.clone());

                // Extract what we can
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let peer_count = parsed["peer_count"]
                        .as_u64()
                        .or_else(|| parsed["peers"].as_array().map(|a| a.len() as u64));

                    health.network = Some(NetworkHealth {
                        peer_count,
                        details: format!(
                            "Keys: {:?}",
                            parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
                        ),
                    });
                }

                info!(
                    peer_count = health.network.as_ref().and_then(|n| n.peer_count),
                    "📊 NETWORK_STATS: Fetched conductor network statistics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_stats");
                health.raw_network_stats = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network metrics (more detailed DHT info)
        // dump_network_metrics takes optional DNA hash filter and DHT summary flag
        match self
            .admin_websocket()
            .dump_network_metrics(None, true)
            .await
        {
            Ok(network_metrics) => {
                let json = serde_json::to_string_pretty(&network_metrics)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_metrics = Some(json);

                info!("📊 NETWORK_METRICS: Fetched conductor DHT metrics");
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_metrics");
                health.raw_network_metrics = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        health
    }

    /// Subscribe to InfrastructureSignals emitted by the conductor's app interface.
    ///
    /// Wraps `AppWebsocket::on_signal` — the official client filters signals to
    /// this app's cells. For each signal, we msgpack-decode the inner `AppSignal`
    /// payload into a `serde_json::Value`, then try to parse it as an
    /// `InfrastructureSignal`. Non-infrastructure signals (e.g. lamad DNA signals
    /// that share the same conductor) are logged at debug and ignored.
    ///
    /// Returns a subscription handle (string id) from the underlying client.
    /// Dropping the returned value does NOT unsubscribe — `AppWebsocket::on_signal`
    /// registers the handler for the lifetime of the websocket connection. When the
    /// HcClient is dropped, the subscription ends with it.
    pub async fn subscribe_infrastructure_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::InfrastructureSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.connection()
            .app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing signal
                    // (serde_json::Value cannot represent msgpack byte arrays)
                    // and dropped it at debug level: the 2026-06-12
                    // peer-statuses dark-surface root cause. Real misses are
                    // now LOUD; foreign signals on the shared app interface
                    // stay quiet.
                    match crate::signals::decode_infrastructure_signal(&bytes) {
                        Ok(infra) => handler(infra),
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::infrastructure_decode_miss_count(),
                                "InfrastructureSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Mishpat / REA / ElohimContent shape-mismatches cannot
                        // arise from the infra decoder (different mirror-variant
                        // sets), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag, ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag, ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-infra shape classification from infra decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to ReaProjectionSignals emitted by the content_store coordinator.
    /// Routes ReaCommitmentCommitted (incl. updates), AgreementCommitted, and
    /// ReaEconomicEventCommitted to the local SQL projection via
    /// `rea_projection::handle_rea_signal`.
    ///
    /// Wired by main.rs alongside the infrastructure + elohim-content
    /// subscribers. Required for the substrate-correct write path landed
    /// per 2026-05-26-substrate-rea-replication-fix.md — without this,
    /// HTTP POST /api/v1/commitments (project-epr) would create the DHT
    /// entry but the SQL projection would never land, causing the service
    /// layer's bounded poll to time out at 1s.
    ///
    /// Non-REA signals (Content, Attestation, etc.) are logged at debug and
    /// dropped — they have their own dedicated subscribers.
    pub async fn subscribe_rea_projection_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::rea_projection::ReaProjectionSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.connection()
            .app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing ReaProjectionSignal
                    // (action_hash/entry_hash/author are ActionHash/EntryHash/
                    // AgentPubKey on the DNA side → raw 39-byte arrays on the
                    // MessagePack wire, which serde_json::Value cannot represent)
                    // and dropped it at debug level: the same dark-drop class as
                    // d33b0e1f5. Without this, the substrate-correct write path
                    // (POST /commitments → project-epr → SQL projection) silently
                    // never lands and the service-layer bounded poll times out.
                    // Real misses are now LOUD; foreign signals on the shared app
                    // interface stay quiet.
                    match crate::rea_projection::decode_rea_projection_signal(&bytes) {
                        Ok(rea) => handler(rea),
                        Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::rea_projection::rea_decode_miss_count(),
                                "ReaProjectionSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Mishpat/ElohimContent shape mismatches cannot
                        // arise from the REA decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-REA shape classification from REA decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to ElohimContentSignals emitted by the elohim DNA's content_store
    /// coordinator. Each signal carries an `attestation:*` or `governance-action:*`
    /// Content entry's projection-relevant fields.
    ///
    /// Non-content signals (lamad/imagodei/mishpat) are logged at debug and ignored.
    pub async fn subscribe_elohim_content_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::ElohimContentSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.connection()
            .app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode of the REAL `ProjectionSignal::
                    // ContentCommitted` wire envelope, translated to the flat
                    // `ElohimContentSignal` the dispatcher consumes. The old
                    // `rmp → serde_json::Value → from_value::<ElohimContentSignal>`
                    // path failed on every real signal TWICE: serde_json::Value
                    // cannot hold the holo_hash byte arrays, AND the flat field
                    // names never matched the `{type, payload}` envelope — the
                    // attestation + recovery projections were dark since wiring.
                    // `Ok(None)` is a non-attestation/governance ContentCommitted
                    // (owned by the REA subscriber) — a quiet, expected skip.
                    match crate::signals::decode_elohim_content_signal(&bytes) {
                        Ok(Some(content)) => handler(content),
                        Ok(None) => {
                            debug!(
                                "ContentCommitted for a non-attestation/governance content_type — handled by REA subscriber, skipping"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::elohim_content_decode_miss_count(),
                                "ElohimContentSignal DECODE MISS — signal dropped, attestation/recovery projection will go dark"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Mishpat/Rea shape mismatches cannot arise from
                        // the elohim-content decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-content shape classification from content decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to MishpatSignals emitted by the mishpat DNA's coordinator
    /// (`mishpat` zome `post_commit` hook). Routes `CommitmentCommitted` (and the
    /// gate-decision / challenge variants) to `signals::handle_mishpat_signal`,
    /// which projects the commitment into `mishpat_commitments` with
    /// `dht_anchor_hash = action_hash`.
    ///
    /// `AppWebsocket::on_signal` is registered per app-websocket connection and
    /// receives EVERY `Signal::App` emitted by ANY cell in the app — it does NOT
    /// filter by cell or role. So although the mishpat zome lives in the mishpat
    /// role cell (a different DNA than this client's connected role), its
    /// post-commit signal still arrives here, as long as the mishpat cell is part
    /// of the same installed app. Non-mishpat signals (Infrastructure, REA,
    /// content) fail to decode as `MishpatSignal` and are logged at debug and
    /// dropped — each has its own dedicated subscriber.
    ///
    /// Without this subscriber the `mishpat_commitments` projection is never
    /// populated by live authoring: the provide reconciler's
    /// `live_commons_provides_for_provider` dedup query stays empty (re-authoring
    /// every tick → unbounded commitment proliferation) and rea graduation never
    /// fires. Slice-2b code-review fix.
    pub async fn subscribe_mishpat_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::MishpatSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.connection()
            .app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing MishpatSignal
                    // (serde_json::Value cannot represent msgpack byte arrays)
                    // and dropped it at debug level: the 2026-06-12
                    // dark-`CommitmentCommitted` root cause that left the Epic B
                    // provide projection (4626f820b) dead in production. Real
                    // misses are now LOUD; foreign signals on the shared app
                    // interface stay quiet.
                    match crate::signals::decode_mishpat_signal(&bytes) {
                        Ok(mishpat) => handler(mishpat),
                        Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::mishpat_decode_miss_count(),
                                "MishpatSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Rea/ElohimContent shape mismatches cannot arise
                        // from the mishpat decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-mishpat shape classification from mishpat decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Probe the authenticated APP interface used by every zome call.
    ///
    /// The admin websocket is deliberately not the probe. A conductor can keep
    /// answering admin `list_apps` after the app websocket has closed (or its
    /// one-shot auth token has become unusable), which is precisely the state
    /// the bridge supervisor must cure by constructing a fresh `HcClient`.
    /// `app_info` is side-effect-free, crosses the same authenticated websocket
    /// as `call_zome`, and inherits the client's bounded request timeout.
    /// A probe that got an answer: the websocket is alive, so the only
    /// remaining question is whether the APP behind it is.
    pub async fn ping(&self) -> Result<BridgeProbe, StorageError> {
        // Folded into the zome-path observer because on a node with NO zome
        // traffic this is the only evidence there is — and a node with no
        // traffic is exactly the reported shape: the bridge died at a conductor
        // restart and nothing asked it a question until a person did, 90s later.
        // Not admission-gated (it must answer even when the pool is full), but
        // it IS a conductor round-trip — counted so the cost is visible.
        crate::metrics::inc_conductor_call("-", "app_info", "ungated");
        // Observe the typed transport response, not application health: both
        // `Some(disabled)` and `None` are successful websocket completions.
        // The match below retains the existing health classification.
        let conn = self.connection();
        let answered =
            observe_conductor_attempt("-", "app_info", "ungated", conn.app_ws.app_info()).await;
        if answered.is_ok() {
            // Bytes crossed this mint's app websocket: it is not closed.
            crate::hc_client_registry::note_transport_ok(self.role_key(), conn.generation);
            // AND THE VERDICT HEARS IT. A failed ping files a transport failure;
            // an answered one must be able to cancel it, or a role whose only
            // failure predates a conductor restart stays `Dead` — latching
            // `/health/serving` 503 — until incidental traffic happens to reach
            // it (2026-09-25: a quiet imagodei held every household node dead).
            // Transport evidence only: it lifts `Dead`, never a not-running
            // episode; the status half below keeps its own asymmetry.
            for role in roles_a_probe_vouches_for(self.role_key()) {
                crate::conductor_bridge_health::record_role_responsive(role);
            }
        }
        match answered {
            // THE STATUS IS THE ANSWER. `app_info()` succeeds on a DISABLED app
            // — that is precisely how the 2026-09-18 incident hid for 38 hours
            // behind a probe that only asked `is_ok()` and threw the payload
            // away. Reading it is the whole of the fix on this side.
            Ok(Some(info)) => {
                let observation = crate::conductor_bridge_health::classify_app_status(&info.status);
                observe_role_app_status(self.role_key(), &observation);
                Ok(match observation {
                    crate::conductor_bridge_health::AppRunObservation::Running => {
                        BridgeProbe::Running
                    }
                    crate::conductor_bridge_health::AppRunObservation::NotRunning {
                        reason,
                        ..
                    } => BridgeProbe::NotRunning { reason },
                })
            }
            // The socket answered but named no app. Not a transport failure and
            // not a status we can read — treat it as not-running so it can
            // never launder into evidence of health.
            Ok(None) => {
                let reason =
                    "the conductor answered app_info with no app for this connection".to_string();
                observe_role_app_status(
                    self.role_key(),
                    &crate::conductor_bridge_health::AppRunObservation::NotRunning {
                        reason: reason.clone(),
                        // No status was read, so nothing is established about
                        // whether enable_app could act. `Unknown` keeps the
                        // pre-existing cure rather than licensing or barring one.
                        enable: crate::conductor_bridge_health::AppEnableEvidence::Unknown,
                    },
                );
                Ok(BridgeProbe::NotRunning { reason })
            }
            Err(e) => {
                let msg = format!("Conductor ping failed: {}", e);
                if failure_describes_current_mint(
                    &msg,
                    conn.generation,
                    self.connection_generation(),
                ) {
                    observe_role_zome_error(self.role_key(), &msg);
                }
                if crate::conductor_bridge_health::is_transport_dead(&msg) {
                    crate::hc_client_registry::note_transport_closed(
                        self.role_key(),
                        conn.generation,
                    );
                }
                Err(StorageError::Connection(msg))
            }
        }
    }
}

/// What one [`HcClient::ping`] observed, for a caller that must choose a CURE.
///
/// `Err` means the websocket is gone and the cure is a bridge re-mint.
/// `Ok(NotRunning)` means the websocket is fine and a re-mint would be pure
/// churn — the cure is `enable_app`. Collapsing the two into `is_ok()` is the
/// supervisor half of the 2026-09-18 defect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeProbe {
    /// The app answered and is enabled.
    Running,
    /// The app answered and is NOT running, for this reason.
    NotRunning { reason: String },
}

/// A `CellOwner` exposes the agent key of the cell a client is connected to.
///
/// The mode gate in `api/account.rs` uses this to assert that the caller's
/// resolved human key matches the connected cell's owner — a Tauri-direct
/// invariant. Trait-ifying it lets unit tests exercise the gate without
/// touching the holochain_client websocket layer.
pub trait CellOwner: Send + Sync {
    /// Returns the connected cell's owner agent key as a hex string,
    /// matching the encoding used by `api/identity::extract_agent_key`
    /// (X-Agent-Id header / local session row).
    fn agent_key_hex(&self) -> String;
}

impl CellOwner for HcClient {
    fn agent_key_hex(&self) -> String {
        hex::encode(self.cell_id.agent_pubkey().get_raw_39())
    }
}

#[cfg(test)]
mod head_record_trace_scope_tests {
    use super::*;

    #[test]
    fn head_record_info_phases_require_the_http_correlation_span() {
        let subscriber = tracing_subscriber::registry();
        tracing::subscriber::with_default(subscriber, || {
            assert!(!is_correlated_head_record_call(
                "content_store",
                "get_record_for_action"
            ));
            assert!(!is_correlated_head_record_call(
                "content_store",
                "resolve_canonical_election"
            ));

            let span = tracing::info_span!("head_record_http", head_record_request_id = 7_u64);
            span.in_scope(|| {
                assert!(is_correlated_head_record_call(
                    "content_store",
                    "get_record_for_action"
                ));
                assert!(!is_correlated_head_record_call(
                    "content_store",
                    "resolve_content_head_local"
                ));
                assert!(!is_correlated_head_record_call(
                    "mishpat",
                    "get_record_for_action"
                ));
            });

            let span =
                tracing::info_span!("candidate_head_http", candidate_head_request_id = 11_u64);
            span.in_scope(|| {
                assert!(is_correlated_head_record_call(
                    "content_store",
                    "resolve_canonical_election"
                ));
                // The two later conductor calls on the same candidate-head path —
                // fetching the elected action's record, then verifying it — are
                // part of the same phase-correlated request and must be included.
                assert!(is_correlated_head_record_call(
                    "content_store",
                    "get_record_for_action"
                ));
                assert!(is_correlated_head_record_call(
                    "content_store",
                    "validate_carried_head_record"
                ));
                // The batch/background variant stays OUT of scope: it is not part
                // of the single-content candidate-head HTTP read.
                assert!(!is_correlated_head_record_call(
                    "content_store",
                    "resolve_canonical_elections"
                ));
                assert!(!is_correlated_head_record_call(
                    "mishpat",
                    "resolve_canonical_election"
                ));
            });
        });
    }
}

#[cfg(test)]
mod cell_owner_tests {
    use super::*;

    /// A stub CellOwner used by `account.rs` mode-gate tests. Verifies the
    /// trait dispatch lands on the stub's `agent_key_hex()` return.
    struct StubOwner(String);
    impl CellOwner for StubOwner {
        fn agent_key_hex(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn stub_cell_owner_returns_configured_hex() {
        let stub = StubOwner("uhCAkSTUB".to_string());
        let dyn_owner: &dyn CellOwner = &stub;
        assert_eq!(dyn_owner.agent_key_hex(), "uhCAkSTUB");
    }
}

#[cfg(test)]
mod attribution_tests {
    use holochain_client::ConductorApiError;

    use super::{is_websocket_timeout, observe_conductor_attempt};

    /// Every zome call must be attributable to a function. `admit` is the one
    /// place that both takes a permit and counts the call; a second direct
    /// permit acquisition in this file is a call path the per-function series cannot
    /// see — the blind spot that left a resting node's largest burst unnamed.
    #[test]
    fn every_zome_call_is_admitted_through_the_counting_door() {
        let source = include_str!("hc_client.rs");
        let needle = [".acq", "uire("].concat();
        let direct = source.matches(needle.as_str()).count();
        assert_eq!(
            direct, 1,
            "hc_client.rs takes an admission permit in {direct} places; only `admit` may, \
             so every dispatched call is counted by zome, fn and class"
        );
    }

    #[test]
    fn a_dispatched_call_is_counted_by_zome_function_and_class() {
        let series = crate::metrics::CONDUCTOR_CALLS.with_label_values(&[
            "attribution_test_zome",
            "attribution_test_fn",
            "background",
        ]);
        let before = series.get();
        crate::metrics::inc_conductor_call(
            "attribution_test_zome",
            "attribution_test_fn",
            "background",
        );
        assert_eq!(series.get(), before + 1);
    }

    #[test]
    fn every_dispatch_observes_only_after_successful_admission() {
        let source = include_str!("hc_client.rs");
        for (name, end, admission) in [
            (
                "call_zome_imagodei",
                "/// Make a signed zome call against the MISHPAT",
                "let _permit = admit(",
            ),
            (
                "call_zome_mishpat",
                "/// Make a signed zome call.",
                "let _permit = admit(",
            ),
            (
                "call_zome_timed",
                "/// Get the cell ID",
                "let permit = match admit(",
            ),
        ] {
            let function = source
                .split(&format!("pub async fn {name}("))
                .nth(1)
                .unwrap_or_else(|| panic!("{name} exists"))
                .split(end)
                .next()
                .expect("function boundary");
            let admitted = function.find(admission).expect("admission call");
            let observed = function
                .find("observe_conductor_attempt(")
                .expect("attempt observer");
            assert!(
                observed > admitted,
                "{name} must start attempt observation only after admission"
            );
            assert_eq!(
                function.matches(".call_zome(").count(),
                1,
                "{name} must have exactly one observed websocket dispatch"
            );
        }
    }

    #[test]
    fn app_info_probe_uses_the_shared_typed_transport_observer() {
        let source = include_str!("hc_client.rs");
        let ping = source
            .split("pub async fn ping(&self)")
            .nth(1)
            .expect("ping exists")
            .split("/// What one [`HcClient::ping`] observed")
            .next()
            .expect("ping boundary");
        assert_eq!(ping.matches("observe_conductor_attempt(").count(), 1);
        assert_eq!(ping.matches("conn.app_ws.app_info()").count(), 1);
        assert!(ping.contains("match answered {"));
        assert!(ping.contains("\"app_info\""));
        assert!(ping.contains("\"ungated\""));
        assert!(ping.contains("Ok(Some(info))"));
        assert!(ping.contains("Ok(None)"));
    }

    #[tokio::test]
    async fn attempt_observer_records_success_error_and_typed_timeout() {
        let success = crate::metrics::CONDUCTOR_CALL_DURATION_MS.with_label_values(&[
            "observer_zome",
            "observer_fn",
            "interactive",
            "success",
        ]);
        let error = crate::metrics::CONDUCTOR_CALL_DURATION_MS.with_label_values(&[
            "observer_zome",
            "observer_fn",
            "interactive",
            "error",
        ]);
        let timeouts = crate::metrics::CONDUCTOR_CALL_TIMEOUTS.with_label_values(&[
            "observer_zome",
            "observer_fn",
            "interactive",
            "websocket",
        ]);
        let success_before = success.get_sample_count();
        let error_before = error.get_sample_count();
        let timeout_before = timeouts.get();

        let observed =
            observe_conductor_attempt("observer_zome", "observer_fn", "interactive", async {
                Ok::<_, ConductorApiError>(7_u8)
            })
            .await
            .expect("success is preserved");
        assert_eq!(observed, 7);
        assert!(observe_conductor_attempt::<(), _>(
            "observer_zome",
            "observer_fn",
            "interactive",
            async { Err(ConductorApiError::AppNotFound) },
        )
        .await
        .is_err());
        let elapsed = tokio::time::timeout(std::time::Duration::ZERO, async {
            std::future::pending::<()>().await
        })
        .await
        .expect_err("pending future exceeds a zero budget");
        assert!(observe_conductor_attempt::<(), _>(
            "observer_zome",
            "observer_fn",
            "interactive",
            async {
                Err(ConductorApiError::WebsocketError(
                    holochain_websocket::WebsocketError::Timeout(elapsed),
                ))
            },
        )
        .await
        .is_err());

        assert_eq!(success.get_sample_count(), success_before + 1);
        assert_eq!(error.get_sample_count(), error_before + 2);
        assert_eq!(timeouts.get(), timeout_before + 1);
    }

    #[tokio::test]
    async fn dropping_pending_attempt_is_censored_not_timed_out() {
        let dropped = crate::metrics::CONDUCTOR_CALL_DROPPED.with_label_values(&[
            "pending_zome",
            "pending_fn",
            "background",
        ]);
        let timeouts = crate::metrics::CONDUCTOR_CALL_TIMEOUTS.with_label_values(&[
            "pending_zome",
            "pending_fn",
            "background",
            "websocket",
        ]);
        let errors = crate::metrics::CONDUCTOR_CALL_DURATION_MS.with_label_values(&[
            "pending_zome",
            "pending_fn",
            "background",
            "error",
        ]);
        let dropped_before = dropped.get();
        let timeout_before = timeouts.get();
        let errors_before = errors.get_sample_count();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(observe_conductor_attempt(
            "pending_zome",
            "pending_fn",
            "background",
            async move {
                let _ = started_tx.send(());
                std::future::pending::<Result<(), ConductorApiError>>().await
            },
        ));
        started_rx.await.expect("attempt future started");
        task.abort();
        let _ = task.await;

        assert_eq!(dropped.get(), dropped_before + 1);
        assert_eq!(timeouts.get(), timeout_before);
        assert_eq!(errors.get_sample_count(), errors_before);
    }

    #[test]
    fn non_websocket_timeout_is_not_classified_as_transport_timeout() {
        assert!(!is_websocket_timeout(&ConductorApiError::AppNotFound));
        assert!(!is_websocket_timeout(&ConductorApiError::WebsocketError(
            holochain_websocket::WebsocketError::Other(
                "domain response mentioned timeout".to_string(),
            )
        )));
        assert!(!is_websocket_timeout(
            &ConductorApiError::SignZomeCallError("in-wasm timeout".to_string())
        ));
    }

    #[tokio::test]
    async fn websocket_timeout_is_classified_from_typed_error() {
        let elapsed = tokio::time::timeout(std::time::Duration::ZERO, async {
            std::future::pending::<()>().await
        })
        .await
        .expect_err("pending future exceeds a zero budget");
        let error = ConductorApiError::WebsocketError(
            holochain_websocket::WebsocketError::Timeout(elapsed),
        );

        assert!(is_websocket_timeout(&error));
    }
}

/// The re-mint must heal every `Arc<HcClient>` already handed out. These pin
/// the two pieces that make that true without a live conductor: the slot the
/// sockets live in is shared by every clone, and adoption refuses a mint whose
/// cells differ (a swap of sockets under an unchanged `cell_id` would sign for
/// cells the fresh signer holds no credentials for).
#[cfg(test)]
mod connection_adoption_tests {
    use super::*;
    use holochain_types::prelude::{AgentPubKey, DnaHash};

    fn cell(seed: u8) -> CellId {
        CellId::new(
            DnaHash::from_raw_32(vec![seed; 32]),
            AgentPubKey::from_raw_32(vec![seed.wrapping_add(3); 32]),
        )
    }

    /// Stands in for `HcClient`: immutable identity plus a swappable slot.
    struct Holder {
        conn: ConnectionSlot<(u64, &'static str)>,
    }

    #[test]
    fn a_boot_captured_clone_sees_the_replacement() {
        let registry_handle = Arc::new(Holder {
            conn: ConnectionSlot::new((1, "socket-before-restart")),
        });
        // A long-lived consumer clones the handle ONCE at boot and keeps it.
        let boot_captured = Arc::clone(&registry_handle);
        registry_handle.conn.replace((2, "socket-after-remint"));
        assert_eq!(boot_captured.conn.current(), (2, "socket-after-remint"));
    }

    #[test]
    fn generations_are_unique_and_increasing() {
        let a = next_connection_generation();
        let b = next_connection_generation();
        assert!(b > a);
    }

    #[test]
    fn the_same_cells_may_adopt() {
        let (c, m, i) = (cell(1), cell(2), cell(3));
        assert_eq!(
            adoption_refusal(
                ("elohim", &c, Some(&m), Some(&i)),
                ("elohim", &c, Some(&m), Some(&i))
            ),
            None
        );
        assert_eq!(
            adoption_refusal(("elohim", &c, None, None), ("elohim", &c, None, None)),
            None
        );
    }

    #[test]
    fn a_changed_identity_refuses_adoption() {
        let (c, m, i) = (cell(1), cell(2), cell(3));
        let old = ("elohim", &c, Some(&m), Some(&i));
        let reinstalled = cell(9);
        assert!(adoption_refusal(old, ("elohim", &reinstalled, Some(&m), Some(&i))).is_some());
        assert!(adoption_refusal(old, ("elohim@abc", &c, Some(&m), Some(&i))).is_some());
        assert!(adoption_refusal(old, ("elohim", &c, None, Some(&i))).is_some());
        assert!(adoption_refusal(old, ("elohim", &c, Some(&m), Some(&reinstalled))).is_some());
    }

    /// Every zome-call arm and the ping report transport evidence against the
    /// generation of the socket they actually used.
    #[test]
    fn every_dispatch_reports_transport_evidence_by_generation() {
        let source = include_str!("hc_client.rs");
        for name in ["call_zome_imagodei", "call_zome_mishpat", "call_zome_timed"] {
            let body = source
                .split(&format!("pub async fn {name}("))
                .nth(1)
                .unwrap_or_else(|| panic!("{name} exists"))
                .split("pub ")
                .next()
                .expect("boundary");
            assert!(body.contains("let conn = self.connection();"), "{name}");
            assert!(body.contains("conn.app_ws.call_zome("), "{name}");
            assert!(body.contains("note_transport_ok("), "{name}");
            assert!(body.contains("conn.generation, e)"), "{name}");
        }
        let failed = source
            .split("fn zome_call_failed_on(")
            .nth(1)
            .expect("mapper")
            .split("fn connection(&self)")
            .next()
            .expect("boundary");
        assert!(failed.contains("is_transport_dead(&msg)"));
        assert!(failed.contains("note_transport_closed(self.role_key(), generation)"));
    }
}

/// Station "restarting every conductor opens a window … closes on its own"
/// (app-delivery-refuses-fast, household 2026-09-25 14:00Z). After the
/// re-mint every role answered its probe on the fresh mint, yet a QUIET role
/// — imagodei on every peer — kept the transport failure it collected before
/// the restart, so the node read `dead` until incidental traffic reached it.
/// These pin the cure at the seam the ping and the zome-call mapper share,
/// against an isolated observer (never the process-wide one).
#[cfg(test)]
mod probe_liveness_tests {
    use super::*;
    use crate::conductor_bridge_health::{RoleBridgeHealth, ZomePathStatus};

    const CLOSED: &str = "Zome call failed: Websocket closed: No connection";
    const MINT_N: u64 = 7;
    const MINT_N1: u64 = 8;

    /// What `zome_call_failed_on` / the ping's `Err` arm do with one failure.
    fn fail(roles: &RoleBridgeHealth, role: &str, call_mint: u64, current_mint: u64) {
        if failure_describes_current_mint(CLOSED, call_mint, current_mint) {
            roles.for_role(role).observe_zome_error(CLOSED);
        }
    }

    /// What the ping's answered arm does.
    fn probe_answers(roles: &RoleBridgeHealth, probing_role: &str) {
        for role in roles_a_probe_vouches_for(probing_role) {
            roles.for_role(role).record_responsive();
        }
    }

    fn status(roles: &RoleBridgeHealth, role: &str) -> ZomePathStatus {
        roles.for_role(role).snapshot().status
    }

    #[test]
    fn failed_on_mint_n_and_answered_on_mint_n_plus_1_reads_serving() {
        let roles = RoleBridgeHealth::new();
        fail(&roles, "imagodei", MINT_N, MINT_N);
        assert_eq!(status(&roles, "imagodei"), ZomePathStatus::Dead);
        assert!(!roles.for_role("imagodei").snapshot().serving_ok());

        // The re-mint adopts N+1; its probe answers. No zome call reaches the
        // role — it is quiet — and it must still read serving.
        probe_answers(&roles, "imagodei");
        assert_eq!(status(&roles, "imagodei"), ZomePathStatus::Live);
        assert!(roles.for_role("imagodei").snapshot().serving_ok());
    }

    #[test]
    fn a_probe_that_fails_on_the_new_mint_keeps_the_role_dead() {
        let roles = RoleBridgeHealth::new();
        fail(&roles, "node_registry", MINT_N, MINT_N);
        // The fresh mint's own probe fails: that IS the current socket.
        fail(&roles, "node_registry", MINT_N1, MINT_N1);
        assert_eq!(status(&roles, "node_registry"), ZomePathStatus::Dead);
        assert!(!roles.for_role("node_registry").snapshot().serving_ok());
    }

    #[test]
    fn a_late_failure_from_the_replaced_mint_does_not_reopen_dead() {
        let roles = RoleBridgeHealth::new();
        fail(&roles, "lamad", MINT_N, MINT_N);
        probe_answers(&roles, "lamad");
        // A call that rode the corpse lands its failure after the adoption.
        fail(&roles, "lamad", MINT_N, MINT_N1);
        assert_eq!(status(&roles, "lamad"), ZomePathStatus::Live);
    }

    #[test]
    fn only_transport_failures_are_scoped_to_their_mint() {
        assert!(!failure_describes_current_mint(CLOSED, MINT_N, MINT_N1));
        assert!(failure_describes_current_mint(CLOSED, MINT_N1, MINT_N1));
        // A cell refusal or a domain error is about the conductor, not the
        // socket, and folds whichever mint carried it.
        assert!(failure_describes_current_mint(
            "Zome call failed: CellDisabled",
            MINT_N,
            MINT_N1
        ));
        assert!(failure_describes_current_mint(
            "Zome call failed: ZomeNotFound",
            MINT_N,
            MINT_N1
        ));
    }

    #[test]
    fn a_probe_answer_never_ends_a_not_running_episode() {
        let roles = RoleBridgeHealth::new();
        roles
            .for_role("imagodei")
            .record_app_disabled("CellDisabled: this cell is not running");
        probe_answers(&roles, "imagodei");
        assert_eq!(status(&roles, "imagodei"), ZomePathStatus::AppDisabled);
        assert!(!roles.for_role("imagodei").snapshot().serving_ok());
    }

    #[test]
    fn the_node_verdict_flips_only_when_every_role_is_proven() {
        let observed = crate::hc_client_registry::OBSERVED_ROLES;
        let roles = RoleBridgeHealth::new();
        for role in observed {
            fail(&roles, role, MINT_N, MINT_N);
        }
        assert_eq!(
            roles.derive_supervised_status(&observed),
            ZomePathStatus::Dead
        );
        // Every supervised role but the quiet one answers. `mishpat` has no
        // probe of its own; the driver's answer vouches for it.
        for role in crate::hc_client_registry::SUPERVISED_ROLES {
            if role != "imagodei" {
                probe_answers(&roles, role);
            }
        }
        assert_eq!(status(&roles, MISHPAT_ROLE), ZomePathStatus::Live);
        assert_eq!(
            roles.derive_supervised_status(&observed),
            ZomePathStatus::Dead,
            "one unproven role keeps the node dead"
        );
        probe_answers(&roles, "imagodei");
        assert_eq!(
            roles.derive_supervised_status(&observed),
            ZomePathStatus::Live
        );
    }

    #[test]
    fn only_the_driver_vouches_for_cross_cell_roles() {
        let driver = roles_a_probe_vouches_for(crate::hc_client_registry::CROSS_CELL_DRIVER_ROLE);
        for cross in crate::hc_client_registry::CROSS_CELL_ROLES {
            assert!(driver.contains(&cross));
        }
        // A role with its own supervisor is vouched for by its own probe only,
        // so two sockets never alternate one verdict.
        for role in crate::hc_client_registry::SUPERVISED_ROLES {
            let vouched = roles_a_probe_vouches_for(role);
            assert_eq!(vouched[0], role);
            for other in crate::hc_client_registry::SUPERVISED_ROLES {
                if other != role {
                    assert!(!vouched.contains(&other), "{role} vouches for {other}");
                }
            }
        }
    }

    /// The wiring: the pure decisions above are what the ping and the mapper
    /// actually call.
    #[test]
    fn the_ping_and_the_mapper_use_the_mint_scoped_decisions() {
        let source = include_str!("hc_client.rs");
        let ping = source
            .split("pub async fn ping(&self)")
            .nth(1)
            .expect("ping exists")
            .split("/// What one [`HcClient::ping`] observed")
            .next()
            .expect("ping boundary");
        let answered = ping
            .split("if answered.is_ok() {")
            .nth(1)
            .expect("answered arm")
            .split("match answered {")
            .next()
            .expect("answered arm boundary");
        assert!(answered.contains("roles_a_probe_vouches_for(self.role_key())"));
        assert!(answered.contains("record_role_responsive(role)"));
        let squashed: String = ping.split_whitespace().collect();
        assert!(squashed.contains(
            "failure_describes_current_mint(&msg,conn.generation,self.connection_generation()"
        ));
        let failed = source
            .split("fn zome_call_failed_on(")
            .nth(1)
            .expect("mapper")
            .split("fn connection(&self)")
            .next()
            .expect("boundary");
        assert!(failed.contains(
            "failure_describes_current_mint(&msg, generation, self.connection_generation())"
        ));
    }
}
