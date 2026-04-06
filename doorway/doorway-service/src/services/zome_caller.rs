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
    ZomeCallTarget,
};
use holochain_types::prelude::ExternIO;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

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

        // Step 1: Connect to admin interface
        let admin_ws =
            AdminWebsocket::connect(&self.admin_addr, Some(String::from("doorway-zome-caller")))
                .await
                .map_err(|e| format!("Admin connect failed: {e}"))?;

        info!("ZomeCaller connected to admin at {}", self.admin_addr);

        // Step 2: Find cell_id from app info
        let apps = admin_ws
            .list_apps(None)
            .await
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
                    let credentials = admin_ws
                        .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                            cell_id: p.cell_id.clone(),
                            functions: None,
                        })
                        .await
                        .map_err(|e| {
                            format!(
                                "authorize_signing_credentials failed for role '{role_name}': {e}"
                            )
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
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: self.installed_app_id.clone(),
                expiry_seconds: 3600,
                single_use: false,
            })
            .await
            .map_err(|e| format!("issue_app_auth_token failed: {e}"))?;

        // Step 5: Connect AppWebsocket with signer
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);
        let app_ws = AppWebsocket::connect(&self.app_addr, token.token, signer_arc, None)
            .await
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

        Ok(())
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

        // Hold read lock only for the call, then release
        let result = {
            let ws = self.app_ws.read().await;
            let app_ws = ws.as_ref().ok_or("Not connected")?;
            app_ws
                .call_zome(
                    ZomeCallTarget::RoleName(role_name.into()),
                    zome_name.into(),
                    fn_name.into(),
                    ExternIO::from(payload),
                )
                .await
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
                warn!("Zome call failed, clearing connection: {e}");
                let mut ws = self.app_ws.write().await;
                *ws = None;
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
}
