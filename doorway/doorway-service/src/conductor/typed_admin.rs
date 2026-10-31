//! Typed Holochain Admin API client using the official `holochain_client` crate.
//!
//! Wraps `AdminWebsocket` in a thin adapter that converts between the typed
//! Holochain types and doorway's domain types (`Vec<u8>` agent keys, `CellIdPair` tuples).
//! All MessagePack encoding/decoding and format handling is delegated to the
//! official client — compile-time type safety replaces runtime format guessing.

use std::collections::BTreeSet;
use std::time::Duration;

use async_trait::async_trait;
use holo_hash::DnaHash;
use holochain_client::{
    AdminWebsocket, AgentPubKey, CellInfo, ConductorApiError, GrantedFunctions, InstallAppPayload,
    IssueAppAuthenticationTokenPayload,
};
use holochain_types::app::{AppBundleSource, AppStatus};
use holochain_websocket::WebsocketError;
// 0.7 migration: capability access storage uses CapAccessType, while
// ZomeCallCapGrant's public payload remains CapAccess.
use holochain_zome_types::prelude::{
    CapAccess, CapSecret, GrantZomeCallCapabilityPayload, ZomeCallCapGrant,
};
use tracing::warn;

/// What a failed admin operation ESTABLISHES about the conductor — the only
/// granularity provisioning is allowed to decide on.
///
/// The three are not degrees of badness; they answer "could the operation have
/// taken effect?" differently, and an identity-bearing operation (`install_app`)
/// must be handled differently under each:
///
/// - after `NotSent` the work may be offered to another conductor;
/// - after `SentOutcomeUnknown` it must be resolved on the SAME conductor,
///   because it may have landed there;
/// - after `Refused` the conductor has answered, and the answer stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultClass {
    /// Nothing reached the conductor: the connect or its handshake failed, or
    /// the socket was already gone before the request was written. The
    /// operation cannot have taken effect.
    NotSent,
    /// The request was written and then the socket dropped, timed out, or
    /// answered something undecodable — or the conductor answered that a pipe
    /// of ITS OWN broke mid-operation. The operation may have landed; a
    /// conductor does not cancel work when its client stops listening.
    SentOutcomeUnknown,
    /// The conductor answered with an error of its own. A verdict.
    Refused,
}

/// A failed admin operation: its class, decided where the typed error still
/// exists, and the rendered message for humans and logs.
///
/// Everything downstream of this module decides on [`AdminFault::class`] and
/// never re-reads [`AdminFault::message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminFault {
    pub class: FaultClass,
    pub message: String,
}

impl AdminFault {
    pub fn not_sent(message: impl Into<String>) -> Self {
        Self {
            class: FaultClass::NotSent,
            message: message.into(),
        }
    }

    pub fn sent_outcome_unknown(message: impl Into<String>) -> Self {
        Self {
            class: FaultClass::SentOutcomeUnknown,
            message: message.into(),
        }
    }

    pub fn refused(message: impl Into<String>) -> Self {
        Self {
            class: FaultClass::Refused,
            message: message.into(),
        }
    }

    /// A request over an ESTABLISHED admin socket failed. `op` names the admin
    /// call for the message, which is rendered exactly as it was before the
    /// class existed.
    fn request(op: &str, error: &ConductorApiError) -> Self {
        Self {
            class: classify_request_error(error),
            message: format!("Admin error ({op}): {error}"),
        }
    }
}

impl std::fmt::Display for AdminFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AdminFault {}

impl From<AdminFault> for String {
    fn from(fault: AdminFault) -> Self {
        fault.message
    }
}

/// Classify a failed request on an established admin socket.
///
/// THE ONE PLACE A STRING IS INSPECTED. Everything the client hands us typed is
/// decided on its type. Two things arrive only as text, and both are read here
/// and nowhere else:
///
/// 1. `WebsocketError::Close("No connection")` — `holochain_websocket` returns
///    this constant when the socket was already torn down BEFORE the request
///    was written (`WsCoreSync::exec`). Every other `Close` (the observed
///    `Websocket closed: ConnectionClosed`) means the socket died while a
///    written request was waiting for its answer.
/// 2. `ExternalApiWireError` — the conductor's answer, whose payload is the
///    conductor's own error rendered to a string. Normally that is a verdict.
///    But the conductor also relays a broken pipe of ITS OWN this way (the
///    2026-09 `elohim-genesis/dev` #1576 refusals: `InternalError("Other:
///    {\"error\":\"BrokenPipe\"}")`), and that says the operation was
///    interrupted part-way, not that it was refused.
///
/// Matching is case-insensitive. A marker added here widens
/// `SentOutcomeUnknown` at the expense of `Refused`, which is the safe
/// direction: it costs a same-conductor lookup, never a second identity.
pub(crate) fn classify_request_error(error: &ConductorApiError) -> FaultClass {
    /// How a conductor's rendered answer spells "a transport of mine dropped".
    const DROPPED_TRANSPORT_MARKERS: [&str; 12] = [
        "brokenpipe",
        "broken pipe",
        "connectionaborted",
        "connection aborted",
        "connectionreset",
        "connection reset",
        "connectionrefused",
        "connection refused",
        "connectionclosed",
        "connection closed",
        "websocket closed",
        "handshake not finished",
    ];
    /// `holochain_websocket`'s constant for "there was no socket to write to".
    const NEVER_WRITTEN_CLOSE_REASON: &str = "no connection";

    match error {
        ConductorApiError::WebsocketError(WebsocketError::Close(reason))
            if reason
                .trim()
                .eq_ignore_ascii_case(NEVER_WRITTEN_CLOSE_REASON) =>
        {
            FaultClass::NotSent
        }
        // Closed mid-request, a send that failed part-way, the client's own
        // request timeout, an IO error, an answer that would not decode, a
        // dropped responder: the request was (or may have been) written.
        ConductorApiError::WebsocketError(_) | ConductorApiError::IoError(_) => {
            FaultClass::SentOutcomeUnknown
        }
        ConductorApiError::ExternalApiWireError(wire) => {
            let rendered = format!("{wire:?}").to_ascii_lowercase();
            if DROPPED_TRANSPORT_MARKERS
                .iter()
                .any(|marker| rendered.contains(marker))
            {
                FaultClass::SentOutcomeUnknown
            } else {
                FaultClass::Refused
            }
        }
        // Raised by the client before anything is written. Not produced by an
        // admin request today; classed by what they establish if one ever is.
        ConductorApiError::CellNotFound
        | ConductorApiError::AppNotFound
        | ConductorApiError::SignZomeCallError(_)
        | ConductorApiError::FreshNonceError(_) => FaultClass::NotSent,
    }
}

/// How long an admin request is waited for before the client gives up.
///
/// Read from the library that enforces it, not restated: every admin socket
/// here is opened with `AdminWebsocket::connect`, which uses
/// `WebsocketConfig::CLIENT_DEFAULT`, and `holochain_websocket` fails a request
/// with `WebsocketError::Timeout` once `default_request_timeout` has passed.
/// The conductor does NOT stop working on a request when this fires.
pub fn admin_request_timeout() -> Duration {
    holochain_websocket::WebsocketConfig::CLIENT_DEFAULT.default_request_timeout
}

/// The admin operations hosted provisioning performs on ONE conductor, with
/// their failures typed.
///
/// A trait so the provisioner's decisions — re-offer, resolve in place, or
/// stop — can be driven by a scripted conductor in tests. [`TypedAdminClient`]
/// is the only production implementation.
#[async_trait]
pub trait ProvisionAdmin: Send + Sync {
    /// Install the app under `installed_app_id` and let the conductor mint its
    /// agent key. Returns the raw 39-byte key the conductor minted.
    ///
    /// There is no "install with this key" here on purpose: a supplied key makes
    /// the conductor list every seed its keystore holds in one lair frame, which
    /// fails once a hosting conductor has churned enough agents
    /// ([`TypedAdminClient::install_app_minting_key`]). Hosted provisioning
    /// therefore never holds a key before the install lands; how it still proves
    /// an app is a person's is `AgentProvisioner::finish_forward`.
    async fn install_app(
        &self,
        installed_app_id: &str,
        bundle_path: &str,
    ) -> Result<Vec<u8>, AdminFault>;

    /// Enable an installed app.
    async fn enable_app(&self, installed_app_id: &str) -> Result<(), AdminFault>;

    /// Look the app up. `Ok(None)` is the conductor ANSWERING that it has no
    /// such app — a different fact from being unable to ask, which is `Err`.
    async fn find_app(&self, installed_app_id: &str)
        -> Result<Option<AppInfoDetailed>, AdminFault>;

    /// Uninstall an app (cleanup).
    async fn uninstall_app(&self, installed_app_id: &str) -> Result<(), AdminFault>;

    /// Every app installed on the conductor, with its agent key. Used only by
    /// the operator's recovery walk.
    async fn list_apps(&self) -> Result<Vec<InstalledAppInfo>, AdminFault>;
}

/// Opens an admin connection to a conductor. A failure here is `NotSent` by
/// construction: no admin request exists yet.
#[async_trait]
pub trait AdminConnector: Send + Sync {
    async fn connect(&self, admin_url: &str) -> Result<Box<dyn ProvisionAdmin>, AdminFault>;
}

/// The production connector: a real [`TypedAdminClient`] per connect.
#[derive(Debug, Default, Clone, Copy)]
pub struct LiveAdminConnector;

#[async_trait]
impl AdminConnector for LiveAdminConnector {
    async fn connect(&self, admin_url: &str) -> Result<Box<dyn ProvisionAdmin>, AdminFault> {
        let client = TypedAdminClient::connect_typed(admin_url).await?;
        Ok(Box::new(client))
    }
}

/// Info about an installed app discovered on a conductor.
#[derive(Debug, Clone)]
pub struct InstalledAppInfo {
    /// The installed app ID (e.g. "elohim", "elohim-conductor-0-abc123")
    pub installed_app_id: String,
    /// Raw 39-byte Holochain agent public key
    pub agent_pub_key: Vec<u8>,
}

/// A cell ID: (dna_hash, agent_pub_key) — both 39 bytes.
pub type CellIdPair = (Vec<u8>, Vec<u8>);

/// Detailed app info including cell IDs per role.
///
/// Used by the Chaperone endpoint to grant capabilities per cell.
#[derive(Debug, Clone)]
pub struct AppInfoDetailed {
    /// The installed app ID
    pub installed_app_id: String,
    /// Raw 39-byte Holochain agent public key
    pub agent_pub_key: Vec<u8>,
    /// Cell IDs keyed by role name
    pub cell_ids: Vec<(String, CellIdPair)>,
    /// App status from conductor ("running", "disabled", "paused", or "unrecoverable")
    pub status: Option<String>,
    /// Cell whose permanent restore failure made this app unrecoverable.
    pub unrecoverable_cell_id: Option<String>,
    /// Operator-facing payload for the permanent restore failure.
    pub unrecoverable_reason: Option<String>,
}

/// Typed Holochain admin API client backed by `holochain_client::AdminWebsocket`.
///
/// Maintains a persistent WebSocket connection to the conductor's admin interface.
/// Checked construction for the holo_hash types this client builds from raw bytes.
///
/// `HoloHash::from_raw_39` is documented to "panic if hash_type does not match
/// or hash is incorrect length" — it `unwrap()`s `try_from_raw_39`, which
/// rejects BOTH a wrong length (`BadSize`) and a wrong 3-byte type prefix
/// (`BadPrefix`). Every input here originates outside this process: a
/// browser-supplied signing key, or a cell id round-tripped through the
/// registry. An unchecked conversion therefore turns malformed input into a
/// panicked tokio worker mid-request rather than an error the caller can see.
///
/// Measured 2026-08-27 against the local stack: `POST /hc/connect` killed a
/// doorway worker twice over — first with a 32-byte key (`BadSize`), then with
/// 39 well-sized bytes carrying the wrong prefix (`BadPrefix`). A length-only
/// guard fixes the first and not the second, which is why this delegates to
/// the library's own checked constructor rather than pre-validating the size.
fn checked_hash<T: holo_hash::HashType>(
    label: &str,
    bytes: &[u8],
) -> Result<holo_hash::HoloHash<T>, String> {
    holo_hash::HoloHash::<T>::try_from_raw_39(bytes.to_vec())
        .map_err(|e| format!("Invalid {label} ({} bytes): {e:?}", bytes.len()))
}

/// Methods convert between Holochain typed structs and doorway's domain types.
pub struct TypedAdminClient {
    admin_ws: AdminWebsocket,
    url: String,
}

impl TypedAdminClient {
    /// Connect to the conductor's admin WebSocket.
    ///
    /// The URL should be in `ws://host:port` format. The `ws://` prefix is
    /// stripped for `AdminWebsocket::connect()` which expects `host:port`.
    pub async fn connect(admin_url: &str) -> Result<Self, String> {
        Self::connect_typed(admin_url).await.map_err(String::from)
    }

    /// [`Self::connect`], with the failure typed.
    ///
    /// Always [`FaultClass::NotSent`], and by construction rather than by
    /// inspection: whether the name did not resolve, the TCP connect was
    /// refused, or the websocket handshake did not finish (the observed
    /// `Handshake not finished`), no admin request has been written yet.
    pub async fn connect_typed(admin_url: &str) -> Result<Self, AdminFault> {
        // Resolve via tokio's ASYNC resolver first, then hand holochain_client an
        // already-resolved SocketAddr — its connect() runs a synchronous std::net
        // getaddrinfo (admin_websocket.rs:112) that otherwise parks a tokio worker
        // on a flapping/NXDOMAIN conductor (alpha doorway crashloop RCA 2026-06-15;
        // see super::resolve_host_port).
        let addr = super::resolve_host_port(&strip_ws_prefix(admin_url))
            .await
            .map_err(AdminFault::not_sent)?;
        let admin_ws = AdminWebsocket::connect(addr, None)
            .await
            .map_err(|e| connect_fault(admin_url, &e))?;
        Ok(Self {
            admin_ws,
            url: admin_url.to_string(),
        })
    }

    /// Get the admin WebSocket URL.
    pub fn admin_url(&self) -> &str {
        &self.url
    }

    /// Generate a new agent public key on the conductor.
    ///
    /// Returns the raw 39-byte agent key.
    pub async fn generate_agent_pub_key(&self) -> Result<Vec<u8>, String> {
        let agent_key = self
            .admin_ws
            .generate_agent_pub_key()
            .await
            .map_err(|e| String::from(AdminFault::request("generate_agent_pub_key", &e)))?;
        Ok(agent_key.get_raw_39().to_vec())
    }

    /// Install an app and let the conductor mint its agent key; returns the raw
    /// 39-byte key it minted.
    ///
    /// Why not `install_app` with a key from `generate_agent_pub_key`: a supplied key
    /// makes the conductor confirm it is in the keystore by listing EVERY seed lair
    /// holds (`conductor.rs` `list_public_keys`), in one lair frame capped at 8 KiB
    /// (`lair_keystore_api` `sodium_secretstream` `MAX_FRAME`). Uninstalling a hosted
    /// cell never deletes its seed, so a hosting conductor's seed count only grows,
    /// and past roughly a hundred seeds every keyed install fails with lair
    /// `ConnectionAborted` (household, 2026-10-20). A key the conductor mints itself
    /// skips that listing.
    pub async fn install_app_minting_key(
        &self,
        installed_app_id: &str,
        bundle_path: &str,
    ) -> Result<Vec<u8>, String> {
        ProvisionAdmin::install_app(self, installed_app_id, bundle_path)
            .await
            .map_err(String::from)
    }

    /// Enable an installed app on the conductor.
    pub async fn enable_app(&self, installed_app_id: &str) -> Result<(), String> {
        ProvisionAdmin::enable_app(self, installed_app_id)
            .await
            .map_err(String::from)
    }

    /// List all installed apps on the conductor.
    ///
    /// Returns the installed_app_id and raw 39-byte agent public key for each app.
    pub async fn list_apps(&self) -> Result<Vec<InstalledAppInfo>, String> {
        ProvisionAdmin::list_apps(self).await.map_err(String::from)
    }

    /// Get detailed app info including cell IDs for each role.
    ///
    /// Uses `list_apps` + filter, then extracts provisioned cells via typed
    /// `CellInfo::Provisioned` enum matching — eliminates all format guessing.
    ///
    /// An app the conductor does not have is an `Err` here, as it always was.
    /// A caller that must tell "not installed" from "could not ask" uses
    /// [`ProvisionAdmin::find_app`].
    pub async fn get_app_info(&self, installed_app_id: &str) -> Result<AppInfoDetailed, String> {
        match ProvisionAdmin::find_app(self, installed_app_id).await {
            Ok(Some(info)) => Ok(info),
            Ok(None) => Err(format!("App '{installed_app_id}' not found on conductor")),
            Err(fault) => Err(fault.message),
        }
    }

    /// Grant zome call capability for a cell.
    ///
    /// Allows the given `signing_key` to make zome calls on `cell_id`
    /// using the provided `cap_secret`. All functions — the human session's
    /// grant, unchanged. A stewarded runner's session uses
    /// [`Self::grant_zome_call_capability_scoped`] with a listed set.
    pub async fn grant_zome_call_capability(
        &self,
        cell_id: (&[u8], &[u8]),
        cap_secret: &[u8],
        signing_key: &[u8],
        tag: &str,
    ) -> Result<(), String> {
        self.grant_zome_call_capability_scoped(
            cell_id,
            cap_secret,
            signing_key,
            tag,
            GrantedFunctions::All,
        )
        .await
    }

    /// Grant zome call capability for a cell, limited to `functions`.
    pub async fn grant_zome_call_capability_scoped(
        &self,
        cell_id: (&[u8], &[u8]),
        cap_secret: &[u8],
        signing_key: &[u8],
        tag: &str,
        functions: GrantedFunctions,
    ) -> Result<(), String> {
        use holochain_client::CellId as HcCellId;

        let dna: DnaHash = checked_hash("cell dna hash", cell_id.0)?;
        let agent: AgentPubKey = checked_hash("cell agent key", cell_id.1)?;
        let hc_cell_id = HcCellId::new(dna, agent);

        let signer: AgentPubKey = checked_hash("signing_key", signing_key)?;

        // Build CapSecret from raw bytes (64 bytes expected)
        let secret: CapSecret = cap_secret.try_into().map_err(|_| {
            format!(
                "Invalid cap secret length: expected 64, got {}",
                cap_secret.len()
            )
        })?;

        let mut assignees = BTreeSet::new();
        assignees.insert(signer);

        let cap_grant = ZomeCallCapGrant {
            tag: tag.to_string(),
            access: CapAccess::Assigned { secret, assignees },
            functions,
        };

        self.admin_ws
            .grant_zome_call_capability(GrantZomeCallCapabilityPayload {
                cell_id: hc_cell_id,
                cap_grant,
            })
            .await
            .map_err(|e| format!("Admin error (grant_zome_call_capability): {e}"))?;

        Ok(())
    }

    /// Issue an app authentication token for AppWebsocket connections.
    ///
    /// Returns the raw token bytes that the client passes to `AppWebsocket.connect()`.
    ///
    /// `single_use` is caller-decided, not hardcoded: a token minted and
    /// consumed immediately by a connect in the same flow (never stored,
    /// cached, or reused on reconnect) should pass `true`; a token that is
    /// cached, handed to a worker pool, or returned across an API boundary to
    /// a caller that reconnects later must pass `false`.
    pub async fn issue_app_authentication_token(
        &self,
        installed_app_id: &str,
        expiry_seconds: u64,
        single_use: bool,
    ) -> Result<Vec<u8>, String> {
        let result = self
            .admin_ws
            .issue_app_auth_token(IssueAppAuthenticationTokenPayload {
                installed_app_id: installed_app_id.to_string(),
                expiry_seconds,
                single_use,
            })
            .await
            .map_err(|e| format!("Admin error (issue_app_authentication_token): {e}"))?;

        Ok(result.token)
    }

    /// List app interface ports on the conductor.
    pub async fn list_app_interfaces(&self) -> Result<Vec<u16>, String> {
        let interfaces = self
            .admin_ws
            .list_app_interfaces()
            .await
            .map_err(|e| format!("Admin error (list_app_interfaces): {e}"))?;

        Ok(interfaces.into_iter().map(|i| i.port).collect())
    }

    /// Uninstall an app from the conductor (cleanup).
    pub async fn uninstall_app(&self, installed_app_id: &str) -> Result<(), String> {
        ProvisionAdmin::uninstall_app(self, installed_app_id)
            .await
            .map_err(String::from)
    }

    // =========================================================================
    // Retry helpers
    // =========================================================================

    /// Retry an admin operation that may fail due to source chain contention.
    ///
    /// When multiple cap grants happen in sequence, the source chain head can
    /// move between read and commit. This helper retries with exponential
    /// backoff on known conflict errors.
    pub async fn with_source_chain_retry<F, Fut>(
        op: F,
        desc: &str,
        max_retries: u32,
    ) -> Result<(), String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        for attempt in 0..max_retries {
            match op().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let is_retryable =
                        e.contains("HeadMoved") || e.contains("source chain head has moved");

                    if is_retryable && attempt + 1 < max_retries {
                        let delay = 100 * (1 << attempt); // 100ms, 200ms, 400ms
                        warn!(
                            attempt = attempt + 1,
                            max_retries,
                            delay_ms = delay,
                            "{desc}: source chain conflict, retrying"
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }

                    return Err(e);
                }
            }
        }

        Err(format!("{desc}: max retries ({max_retries}) exceeded"))
    }
}

/// The typed truth of every provisioning operation. The inherent methods of
/// the same names are these, with the class dropped for callers that only
/// report the message.
#[async_trait]
impl ProvisionAdmin for TypedAdminClient {
    async fn install_app(
        &self,
        installed_app_id: &str,
        bundle_path: &str,
    ) -> Result<Vec<u8>, AdminFault> {
        let payload = InstallAppPayload {
            source: AppBundleSource::Path(std::path::PathBuf::from(bundle_path)),
            agent_key: None,
            installed_app_id: Some(installed_app_id.to_string()),
            roles_settings: None,
            network_seed: None,
            ignore_genesis_failure: false,
            restore_from_dht: false,
        };

        let info = self
            .admin_ws
            .install_app(payload)
            .await
            .map_err(|e| AdminFault::request("install_app", &e))?;

        Ok(info.agent_pub_key.get_raw_39().to_vec())
    }

    async fn enable_app(&self, installed_app_id: &str) -> Result<(), AdminFault> {
        self.admin_ws
            .enable_app(installed_app_id.to_string())
            .await
            .map_err(|e| AdminFault::request("enable_app", &e))?;
        Ok(())
    }

    async fn find_app(
        &self,
        installed_app_id: &str,
    ) -> Result<Option<AppInfoDetailed>, AdminFault> {
        let apps = self
            .admin_ws
            .list_apps(None)
            .await
            .map_err(|e| AdminFault::request("list_apps for get_app_info", &e))?;

        let Some(app) = apps
            .into_iter()
            .find(|a| a.installed_app_id == installed_app_id)
        else {
            return Ok(None);
        };

        let agent_pub_key = app.agent_pub_key.get_raw_39().to_vec();

        // Extract all provisioned cells — typed enum matching, no format guessing
        let mut cell_ids = Vec::new();
        for (role_name, cells) in &app.cell_info {
            for cell in cells {
                if let CellInfo::Provisioned(p) = cell {
                    let dna_hash = p.cell_id.dna_hash().get_raw_39().to_vec();
                    let agent_key = p.cell_id.agent_pubkey().get_raw_39().to_vec();
                    cell_ids.push((role_name.clone(), (dna_hash, agent_key)));
                    break; // First provisioned cell per role
                }
            }
        }

        // Convert typed status enum to string
        let status = extract_status_string(&app.status);
        let (unrecoverable_cell_id, unrecoverable_reason) = match &app.status {
            AppStatus::Unrecoverable(cell_id, reason) => {
                (Some(cell_id.to_string()), Some(format!("{reason:?}")))
            }
            _ => (None, None),
        };

        Ok(Some(AppInfoDetailed {
            installed_app_id: app.installed_app_id.clone(),
            agent_pub_key,
            cell_ids,
            status,
            unrecoverable_cell_id,
            unrecoverable_reason,
        }))
    }

    async fn uninstall_app(&self, installed_app_id: &str) -> Result<(), AdminFault> {
        self.admin_ws
            .uninstall_app(installed_app_id.to_string(), false)
            .await
            .map_err(|e| AdminFault::request("uninstall_app", &e))?;
        Ok(())
    }

    async fn list_apps(&self) -> Result<Vec<InstalledAppInfo>, AdminFault> {
        let apps = self
            .admin_ws
            .list_apps(None)
            .await
            .map_err(|e| AdminFault::request("list_apps", &e))?;

        Ok(apps
            .into_iter()
            .map(|app| InstalledAppInfo {
                installed_app_id: app.installed_app_id.clone(),
                agent_pub_key: app.agent_pub_key.get_raw_39().to_vec(),
            })
            .collect())
    }
}

/// A failed admin CONNECT, rendered exactly as before.
fn connect_fault(admin_url: &str, error: &ConductorApiError) -> AdminFault {
    AdminFault::not_sent(format!(
        "Admin WebSocket connect to {admin_url} failed: {error}"
    ))
}

/// Extract status string from the typed AppInfo status field.
///
/// `AppInfo.status` is `AppStatus` enum — we convert to the same string
/// format that callers expect from the old MessagePack-based client.
fn extract_status_string(status: &AppStatus) -> Option<String> {
    match status {
        AppStatus::Enabled => Some("running".to_string()),
        AppStatus::Disabled(_) => Some("disabled".to_string()),
        AppStatus::AwaitingMemproofs => Some("paused".to_string()),
        AppStatus::AwaitingRestore => Some("paused".to_string()),
        AppStatus::Unrecoverable(_, _) => Some("unrecoverable".to_string()),
    }
}

/// Strip `ws://` or `wss://` prefix from a URL to get a socket address.
///
/// `AdminWebsocket::connect()` expects `host:port` format, not full URLs.
fn strip_ws_prefix(url: &str) -> String {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("wss://") {
        rest.to_string()
    } else if let Some(rest) = url.strip_prefix("ws://") {
        rest.to_string()
    } else {
        url.to_string()
    }
}

/// A function-listed capability over exactly `pairs` (`(zome, function)`).
/// Pure. An empty slice yields an empty listed set — a grant that can call
/// nothing — which callers avoid authoring at all.
pub fn listed_functions(pairs: &[(&str, &str)]) -> GrantedFunctions {
    use holochain_zome_types::prelude::{FunctionName, ZomeName};
    GrantedFunctions::Listed(
        pairs
            .iter()
            .map(|(zome, function)| (ZomeName::from(*zome), FunctionName::from(*function)))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_functions_names_exactly_the_given_pairs() {
        use holochain_zome_types::prelude::{FunctionName, ZomeName};
        let GrantedFunctions::Listed(set) = listed_functions(&[("content_store", "compute_task")])
        else {
            panic!("a listed grant must never become All");
        };
        assert_eq!(set.len(), 1);
        assert!(set.contains(&(
            ZomeName::from("content_store"),
            FunctionName::from("compute_task")
        )));
        assert!(!set.contains(&(
            ZomeName::from("imagodei"),
            FunctionName::from("create_human")
        )));
    }

    #[test]
    fn test_strip_ws_prefix() {
        assert_eq!(strip_ws_prefix("ws://localhost:4444"), "localhost:4444");
        assert_eq!(strip_ws_prefix("wss://host:4444"), "host:4444");
        assert_eq!(strip_ws_prefix("localhost:4444"), "localhost:4444");
    }

    #[test]
    fn extract_status_string_covers_restore_states() {
        use holochain_types::app::{UnrecoverableCellReason, WarrantSummary};
        use holochain_zome_types::prelude::{CellId, Timestamp};

        assert_eq!(
            extract_status_string(&AppStatus::AwaitingRestore).as_deref(),
            Some("paused")
        );

        let agent = AgentPubKey::from_raw_36(vec![0; 36]);
        let unrecoverable = AppStatus::Unrecoverable(
            CellId::new(DnaHash::from_raw_36(vec![0; 36]), agent.clone()),
            UnrecoverableCellReason::ChainForkWarrant(Box::new(WarrantSummary {
                author: agent.clone(),
                warrantee: agent,
                timestamp: Timestamp::from_micros(0),
            })),
        );
        assert_eq!(
            extract_status_string(&unrecoverable).as_deref(),
            Some("unrecoverable")
        );
    }

    // ── typed provisioning faults ───────────────────────────────────────────
    //
    // One test per error actually observed on the fleet, built as the TYPED
    // value the client returns and asserted on the class — with the rendered
    // text pinned alongside, so the string each test stands for is on the page.

    fn wire_internal(text: &str) -> ConductorApiError {
        ConductorApiError::ExternalApiWireError(
            holochain_conductor_api::ExternalApiWireError::InternalError(text.to_string()),
        )
    }

    /// OBSERVED: `Websocket closed: ConnectionClosed`. The socket died while a
    /// written request waited for its answer — the old substring matcher knew
    /// `websocket closed` only in lower case and let this through as a verdict.
    #[test]
    fn websocket_closed_connection_closed_is_sent_outcome_unknown() {
        let error = ConductorApiError::WebsocketError(WebsocketError::Close(
            "ConnectionClosed".to_string(),
        ));
        assert_eq!(
            error.to_string(),
            "Websocket error: Websocket closed: ConnectionClosed"
        );

        let fault = AdminFault::request("install_app", &error);
        assert_eq!(fault.class, FaultClass::SentOutcomeUnknown);
        assert_eq!(
            fault.message,
            "Admin error (install_app): Websocket error: Websocket closed: ConnectionClosed"
        );
    }

    /// The same variant with the library's "there was no socket" reason: the
    /// request was never written, so the work is safe to offer elsewhere.
    #[test]
    fn a_request_on_a_socket_already_gone_is_not_sent() {
        for reason in ["No connection", "no connection", " No Connection "] {
            let error =
                ConductorApiError::WebsocketError(WebsocketError::Close(reason.to_string()));
            assert_eq!(
                classify_request_error(&error),
                FaultClass::NotSent,
                "{reason:?}"
            );
        }
    }

    /// OBSERVED: `Handshake not finished`. It is the websocket handshake of a
    /// CONNECT failing, so it is `NotSent` because of where it happens, not
    /// because of how it reads.
    #[test]
    fn handshake_not_finished_on_connect_is_not_sent() {
        let error = ConductorApiError::WebsocketError(WebsocketError::Other(
            "WebSocket protocol error: Handshake not finished".to_string(),
        ));
        let fault = connect_fault("ws://conductor-3:4444", &error);
        assert_eq!(fault.class, FaultClass::NotSent);
        assert!(
            fault.message.contains("Handshake not finished"),
            "{}",
            fault.message
        );
        assert!(fault
            .message
            .starts_with("Admin WebSocket connect to ws://conductor-3:4444 failed: "));
    }

    /// …and if a conductor ever RELAYS that text in an answer, the request did
    /// reach it: the outcome is unknown, never a clean verdict and never
    /// "not sent".
    #[test]
    fn handshake_not_finished_relayed_in_an_answer_is_sent_outcome_unknown() {
        assert_eq!(
            classify_request_error(&wire_internal(
                "lair connection: WebSocket protocol error: Handshake not finished"
            )),
            FaultClass::SentOutcomeUnknown
        );
    }

    /// OBSERVED (`elohim-genesis/dev` #1576, verbatim payloads): the conductor
    /// ANSWERED, but with a pipe of its own broken mid-operation.
    #[test]
    fn the_two_build_1576_refusals_are_sent_outcome_unknown() {
        let generate_key = wire_internal("Other: {\"error\":\"BrokenPipe\"}");
        assert_eq!(
            AdminFault::request("generate_agent_pub_key", &generate_key).message,
            "Admin error (generate_agent_pub_key): External API wire error: \
             InternalError(\"Other: {\\\"error\\\":\\\"BrokenPipe\\\"}\")"
        );
        assert_eq!(
            classify_request_error(&generate_key),
            FaultClass::SentOutcomeUnknown
        );

        let install = wire_internal(
            "Conductor returned an error while using a ConductorApi: \
             Other({\"error\":\"ConnectionAborted\"})",
        );
        assert_eq!(
            classify_request_error(&install),
            FaultClass::SentOutcomeUnknown
        );
    }

    /// Case must not decide the class: the markers are matched lower-cased.
    #[test]
    fn relayed_transport_markers_match_in_any_case() {
        for text in [
            "BROKENPIPE",
            "broken pipe",
            "Connection Reset by peer",
            "connectionRESET",
            "Connection closed",
            "WebSocket Closed",
        ] {
            assert_eq!(
                classify_request_error(&wire_internal(text)),
                FaultClass::SentOutcomeUnknown,
                "{text}"
            );
        }
    }

    /// The client's own request timeout and a raw IO error: the request was
    /// written and nothing came back.
    #[test]
    fn io_errors_on_a_request_are_sent_outcome_unknown() {
        use std::io::{Error as IoError, ErrorKind};
        assert_eq!(
            classify_request_error(&ConductorApiError::IoError(IoError::new(
                ErrorKind::BrokenPipe,
                "broken pipe"
            ))),
            FaultClass::SentOutcomeUnknown
        );
        assert_eq!(
            classify_request_error(&ConductorApiError::WebsocketError(WebsocketError::Io(
                IoError::new(ErrorKind::ConnectionReset, "connection reset by peer")
            ))),
            FaultClass::SentOutcomeUnknown
        );
        assert_eq!(
            classify_request_error(&ConductorApiError::WebsocketError(WebsocketError::Other(
                "ResponderDropped".to_string()
            ))),
            FaultClass::SentOutcomeUnknown
        );
    }

    /// A real answer is a verdict, whatever it says.
    #[test]
    fn a_conductor_answer_is_refused() {
        for text in [
            "AppNotFound",
            "App already installed: elohim-conductor-0-ab12cd",
            "bundle not found at /app/elohim.happ",
            "CellDisabled",
        ] {
            assert_eq!(
                classify_request_error(&wire_internal(text)),
                FaultClass::Refused,
                "{text}"
            );
        }
        assert_eq!(
            classify_request_error(&ConductorApiError::ExternalApiWireError(
                holochain_conductor_api::ExternalApiWireError::RibosomeError(
                    "validation failed".to_string()
                )
            )),
            FaultClass::Refused
        );
    }

    /// The class never leaks into the message: callers that only report the
    /// text see exactly what they saw before.
    #[test]
    fn a_fault_renders_as_its_message_alone() {
        let fault = AdminFault::sent_outcome_unknown("Admin error (install_app): x");
        assert_eq!(fault.to_string(), "Admin error (install_app): x");
        assert_eq!(String::from(fault), "Admin error (install_app): x");
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_succeeds_immediately() {
        let result =
            TypedAdminClient::with_source_chain_retry(|| async { Ok(()) }, "test op", 3).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_non_retryable_error() {
        let result = TypedAdminClient::with_source_chain_retry(
            || async { Err("some other error".to_string()) },
            "test op",
            3,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("some other error"));
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_exhausts_retries() {
        let result = TypedAdminClient::with_source_chain_retry(
            || async { Err("HeadMoved: source chain conflict".to_string()) },
            "test op",
            2,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HeadMoved"));
    }
}
