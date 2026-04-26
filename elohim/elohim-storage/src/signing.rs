//! Conductor Signing Client — EPR Phase 2B, Task C.1
//!
//! Wraps the imagodei `sign_for_agent` coordinator function so elohim-storage
//! can request the conductor to sign canonical bytes under an agent's
//! lair-managed key. Used by `/api/v1/signal/emit` (Task C.2) when the caller
//! cannot sign in-browser — agent keys live in the conductor, not the browser.
//!
//! ## Wire schema
//! `elohim/sdk/schemas/v1/conductor-signing.schema.json`
//!
//! ## Architecture position
//!
//! Storage holds neither key material nor signing authority. The conductor's
//! lair holds the agent's private key; this client requests a signature and
//! gets back `(signature, signer_pubkey)`. The caller composes an EPR Envelope
//! using the signature, then calls `EprService::ingest`. The ingested
//! Envelope is Category A (DHT-notarized); this RPC is Category C (operational).
//!
//! ## Why a separate client and not `HcClient`
//!
//! `HcClient` in `hc_client.rs` is configured for ONE app/role at startup
//! (typically `lamad`). `sign_for_agent` lives in the `imagodei` zome, which
//! requires a connection to the imagodei app interface and authorization for
//! the imagodei cell. Rather than reconfigure `HcClient` to support multiple
//! roles, this client maintains its own connection scoped to imagodei. The
//! connect path mirrors `HolochainAppSignalStream::connect` (same cell, same
//! authorization pattern) — Batch A established that pattern and Batch C
//! reuses it.

use std::sync::Arc;

use holochain_client::{
    AdminWebsocket, AllowedOrigins, AppWebsocket, AuthorizeSigningCredentialsPayload, CellId,
    ClientAgentSigner, ExternIO, ZomeCallTarget,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("conductor connection failed: {0}")]
    Connect(String),

    #[error("signing credentials authorization failed: {0}")]
    Credentials(String),

    #[error("imagodei app/role not found: {0}")]
    NotFound(String),

    #[error("zome call failed: {0}")]
    ZomeCall(String),

    #[error("encoding failed: {0}")]
    Encode(String),

    #[error("decoding failed: {0}")]
    Decode(String),
}

// =============================================================================
// Wire types — must match imagodei's sign_for_agent.rs I/O
// =============================================================================

/// Input for `sign_for_agent`. Field names match the wire schema at
/// `elohim/sdk/schemas/v1/conductor-signing.schema.json` (request).
///
/// Holochain coordinator functions decode struct inputs as MessagePack maps
/// with string keys (named-field serialization). `rmp_serde::to_vec_named`
/// (or `ExternIO::encode`, which uses HSB / named-field encoding internally)
/// is required — positional encoding will fail to decode on the zome side.
#[derive(Debug, Clone, Serialize)]
pub struct SignForAgentInput {
    pub agent_cid: String,
    pub canonical_bytes: Vec<u8>,
}

/// Output from `sign_for_agent`. Field names match the wire schema at
/// `elohim/sdk/schemas/v1/conductor-signing.schema.json` (response).
#[derive(Debug, Clone, Deserialize)]
pub struct SignForAgentOutput {
    /// Raw 64-byte Ed25519 signature over `canonical_bytes`.
    pub signature: Vec<u8>,
    /// Raw 32-byte Ed25519 public key of the signing agent.
    pub signer_pubkey: Vec<u8>,
}

// =============================================================================
// ConductorSigningClient
// =============================================================================

/// Connection to the imagodei app interface for signing requests.
///
/// Holds a live `AppWebsocket`; dropping the client closes the connection.
/// Thread-safe for concurrent `sign()` calls — `app_ws.call_zome` takes
/// `&self`, and the underlying holochain_client handles request multiplexing.
#[allow(dead_code)] // Used by /api/v1/signal/emit (Task C.2)
pub struct ConductorSigningClient {
    app_ws: AppWebsocket,
    cell_id: CellId,
}

impl ConductorSigningClient {
    /// Connect to the imagodei conductor app interface.
    ///
    /// `admin_url` — admin WebSocket (e.g. `ws://localhost:4444`)
    /// `app_url`   — app WebSocket (e.g. `ws://localhost:4445`)
    /// `app_id`    — installed app id (typically `"imagodei"`)
    /// `role`      — cell role name (typically `"imagodei"`)
    ///
    /// Authorizes signing credentials for the imagodei cell so subsequent
    /// `sign()` calls can issue signed zome calls. Returns `Err` if the
    /// conductor is unreachable, the app is not installed, or signing
    /// credentials cannot be authorized.
    pub async fn connect(
        admin_url: &str,
        app_url: &str,
        app_id: &str,
        role: &str,
    ) -> Result<Self, SigningError> {
        let admin_addr = strip_ws_scheme(admin_url);
        let app_addr = strip_ws_scheme(app_url);

        info!(
            admin_addr = %admin_addr,
            app_addr = %app_addr,
            app_id = %app_id,
            role = %role,
            "ConductorSigningClient: connecting to imagodei conductor"
        );

        let admin_ws = AdminWebsocket::connect(&admin_addr, None)
            .await
            .map_err(|e| SigningError::Connect(format!("admin ws: {e}")))?;

        // Ensure app interface exists — non-fatal if already attached.
        let app_port: u16 = app_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4445);
        if let Err(e) = admin_ws
            .attach_app_interface(app_port, None, AllowedOrigins::Any, None)
            .await
        {
            debug!(port = app_port, error = %e, "attach_app_interface (may already exist)");
        }

        // Find the imagodei cell.
        let apps = admin_ws
            .list_apps(None)
            .await
            .map_err(|e| SigningError::Connect(format!("list_apps: {e}")))?;

        let app_info = apps
            .iter()
            .find(|a| a.installed_app_id == app_id)
            .ok_or_else(|| SigningError::NotFound(format!("app '{app_id}' not found")))?;

        let cell_id = app_info
            .cell_info
            .get(role)
            .and_then(|cells| cells.first())
            .and_then(|cell| match cell {
                holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                SigningError::NotFound(format!("role '{role}' not found in app '{app_id}'"))
            })?;

        // Authorize signing credentials for this cell.
        let signer = ClientAgentSigner::default();
        let credentials = admin_ws
            .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                cell_id: cell_id.clone(),
                functions: None,
            })
            .await
            .map_err(|e| SigningError::Credentials(e.to_string()))?;
        signer.add_credentials(cell_id.clone(), credentials);
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);

        // Issue an app auth token for the AppWebsocket connection.
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: app_id.to_string(),
                expiry_seconds: 3600,
                single_use: false,
            })
            .await
            .map_err(|e| SigningError::Credentials(format!("issue_app_auth_token: {e}")))?;

        let app_ws = AppWebsocket::connect(&app_addr, token.token, signer_arc, None)
            .await
            .map_err(|e| SigningError::Connect(format!("app ws: {e}")))?;

        info!(
            app_id = %app_id,
            role = %role,
            "ConductorSigningClient: connected"
        );

        Ok(Self { app_ws, cell_id })
    }

    /// Request a signature over `canonical_bytes` under the agent identified
    /// by `agent_cid`.
    ///
    /// The conductor's lair signs only with private keys it holds. The zome's
    /// pre-commit gate further verifies that the calling agent (this storage
    /// process's authorized signer) matches the Agent EPR identified by
    /// `agent_cid`. Both checks must pass.
    ///
    /// Returns the raw 64-byte signature and the 32-byte signer public key.
    pub async fn sign(
        &self,
        agent_cid: &str,
        canonical_bytes: Vec<u8>,
    ) -> Result<SignForAgentOutput, SigningError> {
        let input = SignForAgentInput {
            agent_cid: agent_cid.to_string(),
            canonical_bytes,
        };

        let payload = ExternIO::encode(&input)
            .map_err(|e| SigningError::Encode(format!("encode SignForAgentInput: {e}")))?;

        debug!(agent_cid = %agent_cid, "ConductorSigningClient: calling sign_for_agent");

        let result = self
            .app_ws
            .call_zome(
                ZomeCallTarget::CellId(self.cell_id.clone()),
                "imagodei".into(),
                "sign_for_agent".into(),
                payload,
            )
            .await
            .map_err(|e| SigningError::ZomeCall(format!("sign_for_agent: {e}")))?;

        let output: SignForAgentOutput = result
            .decode()
            .map_err(|e| SigningError::Decode(format!("decode SignForAgentOutput: {e}")))?;

        Ok(output)
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Strip `ws://` / `wss://` scheme so `holochain_client` receives a raw
/// `host:port` socket address. Mirrors `HcClient::to_socket_addr` and
/// `HolochainAppSignalStream::strip_ws_scheme`.
fn strip_ws_scheme(url: &str) -> String {
    let u = url.trim();
    if let Some(rest) = u.strip_prefix("wss://") {
        rest.to_string()
    } else if let Some(rest) = u.strip_prefix("ws://") {
        rest.to_string()
    } else {
        u.to_string()
    }
}

// =============================================================================
// Tests — see also tests/sign_for_agent_sweettest.rs for end-to-end
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ws_scheme_handles_all_forms() {
        assert_eq!(strip_ws_scheme("ws://localhost:4445"), "localhost:4445");
        assert_eq!(strip_ws_scheme("wss://example.com:443"), "example.com:443");
        assert_eq!(strip_ws_scheme("localhost:4445"), "localhost:4445");
        assert_eq!(strip_ws_scheme("  ws://host:1  "), "host:1");
    }

    #[test]
    fn input_serializes_with_named_fields() {
        // MessagePack named-field encoding (map with string keys) is required
        // for the holochain coordinator to decode. Positional encoding (array)
        // would fail. ExternIO::encode uses HSB which produces named fields.
        let input = SignForAgentInput {
            agent_cid: "bafy123".to_string(),
            canonical_bytes: vec![0x01, 0x02, 0x03],
        };
        let bytes = rmp_serde::to_vec_named(&input).expect("encode");
        // First byte 0x82 = fixmap with 2 entries (agent_cid, canonical_bytes)
        assert_eq!(
            bytes[0], 0x82,
            "expected fixmap-2 prefix; got {:#04x}",
            bytes[0]
        );
    }

    #[test]
    fn output_decodes_signature_and_pubkey() {
        // Wire shape: { signature: Vec<u8>, signer_pubkey: Vec<u8> }
        let signature = vec![0xAB; 64];
        let signer_pubkey = vec![0xCD; 32];
        let wire = serde_json::json!({
            "signature": signature.clone(),
            "signer_pubkey": signer_pubkey.clone(),
        });
        // Round-trip through MessagePack with named fields.
        let bytes = rmp_serde::to_vec_named(&wire).expect("encode");
        let decoded: SignForAgentOutput = rmp_serde::from_slice(&bytes).expect("decode");
        assert_eq!(decoded.signature.len(), 64);
        assert_eq!(decoded.signer_pubkey.len(), 32);
    }
}
