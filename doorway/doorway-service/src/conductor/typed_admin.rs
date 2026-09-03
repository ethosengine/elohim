//! Typed Holochain Admin API client using the official `holochain_client` crate.
//!
//! Wraps `AdminWebsocket` in a thin adapter that converts between the typed
//! Holochain types and doorway's domain types (`Vec<u8>` agent keys, `CellIdPair` tuples).
//! All MessagePack encoding/decoding and format handling is delegated to the
//! official client — compile-time type safety replaces runtime format guessing.

use std::collections::BTreeSet;
use std::time::Duration;

use holo_hash::DnaHash;
use holochain_client::{
    AdminWebsocket, AgentPubKey, CellInfo, GrantedFunctions, InstallAppPayload,
    IssueAppAuthenticationTokenPayload,
};
use holochain_types::app::{AppBundleSource, AppStatus};
// 0.7 migration: capability access storage uses CapAccessType, while
// ZomeCallCapGrant's public payload remains CapAccess.
use holochain_zome_types::prelude::{
    CapAccess, CapSecret, GrantZomeCallCapabilityPayload, ZomeCallCapGrant,
};
use tracing::warn;

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
        // Resolve via tokio's ASYNC resolver first, then hand holochain_client an
        // already-resolved SocketAddr — its connect() runs a synchronous std::net
        // getaddrinfo (admin_websocket.rs:112) that otherwise parks a tokio worker
        // on a flapping/NXDOMAIN conductor (alpha doorway crashloop RCA 2026-06-15;
        // see super::resolve_host_port).
        let addr = super::resolve_host_port(&strip_ws_prefix(admin_url)).await?;
        let admin_ws = AdminWebsocket::connect(addr, None)
            .await
            .map_err(|e| format!("Admin WebSocket connect to {admin_url} failed: {e}"))?;
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
            .map_err(|e| format!("Admin error (generate_agent_pub_key): {e}"))?;
        Ok(agent_key.get_raw_39().to_vec())
    }

    /// Install an app on the conductor with the given agent key.
    pub async fn install_app(
        &self,
        installed_app_id: &str,
        agent_key: &[u8],
        bundle_path: &str,
    ) -> Result<(), String> {
        let agent: AgentPubKey = checked_hash("agent_key", agent_key)?;

        let payload = InstallAppPayload {
            source: AppBundleSource::Path(std::path::PathBuf::from(bundle_path)),
            agent_key: Some(agent),
            installed_app_id: Some(installed_app_id.to_string()),
            roles_settings: None,
            network_seed: None,
            ignore_genesis_failure: false,
            restore_from_dht: false,
        };

        self.admin_ws
            .install_app(payload)
            .await
            .map_err(|e| format!("Admin error (install_app): {e}"))?;

        Ok(())
    }

    /// Enable an installed app on the conductor.
    pub async fn enable_app(&self, installed_app_id: &str) -> Result<(), String> {
        self.admin_ws
            .enable_app(installed_app_id.to_string())
            .await
            .map_err(|e| format!("Admin error (enable_app): {e}"))?;
        Ok(())
    }

    /// List all installed apps on the conductor.
    ///
    /// Returns the installed_app_id and raw 39-byte agent public key for each app.
    pub async fn list_apps(&self) -> Result<Vec<InstalledAppInfo>, String> {
        let apps = self
            .admin_ws
            .list_apps(None)
            .await
            .map_err(|e| format!("Admin error (list_apps): {e}"))?;

        Ok(apps
            .into_iter()
            .map(|app| InstalledAppInfo {
                installed_app_id: app.installed_app_id.clone(),
                agent_pub_key: app.agent_pub_key.get_raw_39().to_vec(),
            })
            .collect())
    }

    /// Get detailed app info including cell IDs for each role.
    ///
    /// Uses `list_apps` + filter, then extracts provisioned cells via typed
    /// `CellInfo::Provisioned` enum matching — eliminates all format guessing.
    pub async fn get_app_info(&self, installed_app_id: &str) -> Result<AppInfoDetailed, String> {
        let apps = self
            .admin_ws
            .list_apps(None)
            .await
            .map_err(|e| format!("Admin error (list_apps for get_app_info): {e}"))?;

        let app = apps
            .into_iter()
            .find(|a| a.installed_app_id == installed_app_id)
            .ok_or_else(|| format!("App '{installed_app_id}' not found on conductor"))?;

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

        Ok(AppInfoDetailed {
            installed_app_id: app.installed_app_id.clone(),
            agent_pub_key,
            cell_ids,
            status,
            unrecoverable_cell_id,
            unrecoverable_reason,
        })
    }

    /// Grant zome call capability for a cell.
    ///
    /// Allows the given `signing_key` to make zome calls on `cell_id`
    /// using the provided `cap_secret`.
    pub async fn grant_zome_call_capability(
        &self,
        cell_id: (&[u8], &[u8]),
        cap_secret: &[u8],
        signing_key: &[u8],
        tag: &str,
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
            functions: GrantedFunctions::All,
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
    pub async fn issue_app_authentication_token(
        &self,
        installed_app_id: &str,
        expiry_seconds: u64,
    ) -> Result<Vec<u8>, String> {
        let result = self
            .admin_ws
            .issue_app_auth_token(IssueAppAuthenticationTokenPayload {
                installed_app_id: installed_app_id.to_string(),
                expiry_seconds,
                single_use: false,
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
        self.admin_ws
            .uninstall_app(installed_app_id.to_string(), false)
            .await
            .map_err(|e| format!("Admin error (uninstall_app): {e}"))?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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
