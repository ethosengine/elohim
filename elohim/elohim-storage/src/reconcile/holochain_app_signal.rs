//! Real DNA signal stream — wraps `holochain_client::AppWebsocket` and emits
//! controller-facing `DnaSignal` events.
//!
//! ## Translation pipeline
//!
//! ```text
//! AppWebsocket (imagodei app interface)
//!     │
//!     │  on_signal callback (holochain_client)
//!     ▼
//! Signal::App { signal: AppSignal }
//!     │
//!     │  rmp_serde::from_slice → serde_json::Value
//!     ▼
//! Try RecoveryV2Signal decode (tag = "type", no content wrapper)
//!   OR ImagodeiSignal decode  (tag = "type", content = "payload")
//!     │
//!     │  translate_recovery_v2 / translate_imagodei
//!     ▼
//! DnaSignal::* forwarded to mpsc channel
//!     │
//!     │  next_signal() drains
//!     ▼
//! ReconcileController::dispatch
//! ```
//!
//! ## Compromise_at derivation (T6 note)
//!
//! `RecoveryV2Signal::KeyRevocationEffective` was the legacy revocation signal.
//! T6 replaced it with the `DnaSignal::KeyRevocation(KeyRevocationEnvelope)` path
//! in `signals::handle_imagodei_dna_signal`, which carries `compromise_at` directly
//! in the envelope metadata. The legacy `KeyRevocationEffective` arm in
//! `translate_recovery_v2` is now a no-op (producer side still emits the variant;
//! M4 deletes producer emissions as a follow-up). `derive_compromise_at` has been
//! removed; the envelope path is the single source of revocation projection.
//!
//! ## Agent_cid derivation
//!
//! The DNA `KeyRotationCommitted` signal carries `rotation.human_agent_pubkey`
//! (the new key after rotation). The `DnaSignal::KeyRotation` `agent_cid` field
//! is populated with this pubkey directly — the pubkey IS the agent identifier
//! in the current protocol stage. Per `dna-signal-stream.schema.json`, `agentCid`
//! accepts any agent identifier string; the pubkey satisfies this contract.
//!
//! For `AgentPeerBindingCreated`, `binding.agent_cid` is already a CID and is
//! passed through directly.
//!
//! ## Stage classification
//!
//! This is a **Stage 1** implementation per `project_bootstrap_to_elohim_security_gradient`.
//! Signal authentication (verifying the conductor's identity) is not enforced here;
//! the conductor's TLS/WS transport is trusted. Stage 2 adds the additional
//! verification layer when the imagodei defender lands.
//!
//! ## TODO(A.12)
//!
//! Sweettest integration test: `real_signal_stream_delivers_rotation_from_coordinator`.
//! This requires a nix-based Holochain conductor and the imagodei DNA — deferred
//! to Task A.12 where the sweettest harness is available.
//!
//! Reconnect / resume semantics: the current implementation reconnects from
//! scratch (no cursor replay). A proper resume pass should query the conductor
//! for signals since the last delivered action_hash. Deferred to Task A.12.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use holochain_client::{
    AdminWebsocket, AllowedOrigins, AppWebsocket, AuthorizeSigningCredentialsPayload,
    ClientAgentSigner,
};
use holochain_types::signal::Signal;

use crate::db::DbPool;
use crate::reconcile::signal_stream::{
    AgentPeerBindingSignal, AttestationKind, DeviceArchetype, DnaSignal, DnaSignalStream,
    KeyRotationSignal, RevocationAttestationSignal, SignalCursor, SignalStreamError,
};
use crate::signals::{ImagodeiSignal, RecoveryV2Signal};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors specific to `HolochainAppSignalStream` construction.
#[derive(Debug, Error)]
pub enum AppSignalStreamError {
    /// WebSocket or admin connection failed.
    #[error("conductor connect failed: {0}")]
    Connect(String),

    /// The requested app or role was not found in the conductor.
    #[error("app/role not found: {0}")]
    NotFound(String),

    /// Signing credential authorization failed.
    #[error("signing credentials failed: {0}")]
    Credentials(String),
}

// ---------------------------------------------------------------------------
// Internal: translation helpers
// ---------------------------------------------------------------------------

/// Convert a Holochain Timestamp (microseconds i64) to `DateTime<Utc>`.
///
/// Timestamp values outside the representable range are clamped to `Utc::now()`.
fn micros_to_datetime(micros: i64) -> DateTime<Utc> {
    let secs = micros / 1_000_000;
    Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now)
}

/// Parse an ISO 8601 string to `DateTime<Utc>`.
///
/// Falls back to `Utc::now()` on parse failure — avoids panicking on
/// malformed conductor output while still delivering the signal downstream.
fn parse_iso(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|e| {
        warn!(raw = %s, error = %e, "Failed to parse ISO timestamp; falling back to now()");
        Utc::now()
    })
}

/// Translate a `RecoveryV2Signal` into a `DnaSignal`, if the variant is
/// one of the four the reconcile controller handles.
///
/// Returns `None` for variants not yet consumed by the controller
/// (`RecoveryRequestCreated`, `IntimateWitnessSubmitted`,
/// `KeyRevocationEffective` — see T6 note in module doc-comment).
fn translate_recovery_v2(signal: RecoveryV2Signal) -> Option<DnaSignal> {
    let emitted_at = Utc::now();

    match signal {
        RecoveryV2Signal::KeyRotationCommitted {
            action_hash,
            rotation,
        } => {
            let rotated_at = crate::signals::timestamp_to_iso(&rotation.rotated_at);
            let rotated_at_dt = parse_iso(&rotated_at);
            // agent_cid: use `human_agent_pubkey` (the agent's current key identifier).
            // See module docstring for the Stage 1 agent_cid derivation rationale.
            let agent_cid = rotation.human_agent_pubkey.clone();

            Some(DnaSignal::KeyRotation(KeyRotationSignal {
                action_hash,
                agent_cid,
                new_pubkey: rotation.new_agent_pubkey,
                old_pubkey: rotation.superseded_agent_pubkey,
                rotated_at: rotated_at_dt,
                emitted_at,
            }))
        }

        RecoveryV2Signal::KeyRevocationRequested {
            id,
            human_id: _,
            revoked_key: _,
            reason: _,
            trigger_type: _,
            initiated_by: steward_id,
            required_votes,
            current_votes,
            threshold_reached,
            effective_at: _,
            created_at,
        } => {
            let attested_at = parse_iso(&created_at);
            Some(DnaSignal::RevocationAttestation(
                RevocationAttestationSignal {
                    action_hash: id.clone(),
                    revocation_id: id,
                    steward_id,
                    approved: true, // A request is an affirmative act
                    attestation_kind: AttestationKind::Request,
                    current_votes,
                    required_votes,
                    threshold_reached,
                    attested_at,
                    emitted_at,
                },
            ))
        }

        RecoveryV2Signal::RevocationVoteSubmitted {
            id,
            revocation_id,
            steward_id,
            approved,
            attestation: _,
            voted_at,
            current_votes,
            required_votes,
            threshold_now_reached,
        } => {
            let attested_at = parse_iso(&voted_at);
            Some(DnaSignal::RevocationAttestation(
                RevocationAttestationSignal {
                    action_hash: id,
                    revocation_id,
                    steward_id,
                    approved,
                    attestation_kind: AttestationKind::Vote,
                    current_votes,
                    required_votes,
                    threshold_reached: threshold_now_reached,
                    attested_at,
                    emitted_at,
                },
            ))
        }

        // Not consumed by the reconcile controller.
        // KeyRevocationEffective: superseded by the T18 DnaSignal::KeyRevocation envelope path
        // (signals::handle_imagodei_dna_signal). Producer side still emits this variant;
        // M4 deletes producer emissions as a follow-up.
        RecoveryV2Signal::KeyRevocationEffective { .. }
        | RecoveryV2Signal::RecoveryRequestCreated { .. }
        | RecoveryV2Signal::IntimateWitnessSubmitted { .. } => {
            debug!("RecoveryV2Signal variant not consumed by reconcile controller — skipping");
            None
        }
    }
}

/// Translate an `ImagodeiSignal` into a `DnaSignal`, if the variant is
/// consumed by the reconcile controller.
///
/// Only `AgentPeerBindingCreated` produces a signal. All others return `None`.
fn translate_imagodei(signal: ImagodeiSignal) -> Option<DnaSignal> {
    let emitted_at = Utc::now();

    match signal {
        ImagodeiSignal::AgentPeerBindingCreated {
            action_hash,
            binding,
        } => {
            let valid_from = micros_to_datetime(binding.valid_from);
            let valid_until = binding.valid_until.map(micros_to_datetime);
            let device_archetype = parse_device_archetype(&binding.device_archetype);
            // The DNA emits superseded_by as a JSON string when present (the
            // ActionHash of the binding being superseded), or null/absent when
            // this is a fresh binding. Pull the string out of the Value if it
            // is a string; treat anything else (null, object, etc.) as absent.
            let superseded_by = binding
                .superseded_by
                .as_ref()
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            Some(DnaSignal::AgentPeerBinding(AgentPeerBindingSignal {
                action_hash: action_hash.clone(),
                peer_id: binding.peer_id,
                agent_cid: binding.agent_cid,
                valid_from,
                valid_until,
                device_archetype,
                binding_action_hash: action_hash,
                superseded_by,
                emitted_at,
            }))
        }

        // Not yet consumed by the reconcile controller.
        ImagodeiSignal::HumanCommitted { .. }
        | ImagodeiSignal::AgentCommitted { .. }
        | ImagodeiSignal::RelationshipCommitted { .. }
        | ImagodeiSignal::AttestationCommitted { .. } => {
            debug!("ImagodeiSignal variant not consumed by reconcile controller — skipping");
            None
        }
    }
}

/// Parse a device archetype string (snake_case) into `DeviceArchetype`.
///
/// Defaults to `DeviceArchetype::Node` on unknown values with a warning.
/// Unknown archetypes are forward-compatible: new DNA variants won't crash
/// the storage stream, but will be recorded as `Node` until the mirror is
/// updated. Operators will see a warning log.
fn parse_device_archetype(s: &str) -> DeviceArchetype {
    match s {
        "node" => DeviceArchetype::Node,
        "desktop" => DeviceArchetype::Desktop,
        "mobile" => DeviceArchetype::Mobile,
        "steward" => DeviceArchetype::Steward,
        other => {
            warn!(
                archetype = %other,
                "Unknown device_archetype in AgentPeerBindingCreated; defaulting to Node"
            );
            DeviceArchetype::Node
        }
    }
}

/// Attempt to decode an app-signal byte payload as `RecoveryV2Signal` or
/// `ImagodeiSignal`, returning the translated `DnaSignal` if recognized.
///
/// Wire format: MessagePack-encoded `serde_json::Value`, then re-decoded as
/// the target enum. Both signal enums use internally-tagged serde (`tag = "type"`).
///
/// Returns `None` for:
/// - MessagePack decode failures (malformed wire data)
/// - Neither enum matches (e.g. lamad DNA signals on the same conductor)
/// - Recognized variants not consumed by the controller
fn try_decode_and_translate(bytes: &[u8], _db_pool: Option<&Arc<DbPool>>) -> Option<DnaSignal> {
    // Step 1: decode MessagePack → JSON Value.
    let value = match rmp_serde::from_slice::<serde_json::Value>(bytes) {
        Ok(v) => v,
        Err(e) => {
            debug!(error = %e, "Failed to msgpack-decode app signal payload");
            return None;
        }
    };

    // Step 2a: try RecoveryV2Signal (tag = "type", no content wrapper).
    // RecoveryV2Signal variants have the `type` tag directly on the object.
    if let Ok(rv2) = serde_json::from_value::<RecoveryV2Signal>(value.clone()) {
        return translate_recovery_v2(rv2);
    }

    // Step 2b: try ImagodeiSignal (tag = "type", content = "payload").
    // ImagodeiSignal wraps variant payload under a "payload" key.
    if let Ok(imago) = serde_json::from_value::<ImagodeiSignal>(value.clone()) {
        return translate_imagodei(imago);
    }

    // Step 3: unrecognized signal — another DNA on the same conductor.
    debug!(
        value_type = %value.get("type").and_then(|t| t.as_str()).unwrap_or("<none>"),
        "App signal not recognized as RecoveryV2Signal or ImagodeiSignal — skipping"
    );
    None
}

// ---------------------------------------------------------------------------
// HolochainAppSignalStream
// ---------------------------------------------------------------------------

/// Real DNA signal stream — subscribes to an imagodei `AppWebsocket` and
/// translates conductor signals into controller-facing `DnaSignal` events.
///
/// ## Construction
///
/// Use `HolochainAppSignalStream::connect(admin_url, app_url, app_id, role)` to
/// connect and subscribe. The stream is immediately live after construction.
///
/// ## Lifecycle
///
/// The `AppWebsocket` subscription is registered on a background task that
/// forwards translated `DnaSignal` values into an internal `mpsc` channel.
/// `next_signal()` drains this channel. When the conductor disconnects, the
/// channel is closed and `next_signal()` returns `None`.
///
/// ## Reconnection
///
/// TODO(A.12): Add reconnect + replay-from-cursor semantics. The current
/// implementation returns `None` (stream closed) when the conductor drops.
/// The production startup wrapper in `main.rs` logs a warning and does not
/// automatically reconnect.
pub struct HolochainAppSignalStream {
    /// Receiver half of the internal translator channel.
    rx: mpsc::Receiver<DnaSignal>,
    /// Count of delivered signals (for cursor tracking).
    delivered: u64,
    /// Live AppWebsocket connection — MUST NOT drop.
    ///
    /// The `on_signal` callback is registered inside the `AppWebsocket` struct.
    /// Dropping `app_ws` closes the WebSocket and fires the callback's Drop,
    /// which closes `tx`. `rx.recv()` then immediately returns `None` and the
    /// controller loop exits. This field keeps `app_ws` alive for the full
    /// lifetime of the stream.
    ///
    /// Unit tests that construct `HolochainAppSignalStream` directly (without
    /// going through `connect`) use `for_testing(rx)` which sets this to `None`.
    /// The live path always sets it to `Some(app_ws)`.
    _app_ws: Option<AppWebsocket>,
}

impl HolochainAppSignalStream {
    /// Construct a stream for unit tests, bypassing `connect`.
    ///
    /// Tests that exercise the signal-delivery logic directly inject a channel
    /// receiver here without needing a real conductor. The `_app_ws` field is
    /// `None` because there is no live WebSocket — the test drives the channel
    /// directly via the paired sender.
    ///
    /// **Never use this in production code.** Use `connect()` instead, which
    /// stores the live `AppWebsocket` in `_app_ws` to prevent premature drop.
    #[cfg(test)]
    pub fn for_testing(rx: mpsc::Receiver<DnaSignal>) -> Self {
        Self {
            rx,
            delivered: 0,
            _app_ws: None,
        }
    }

    /// Connect to the imagodei app interface and subscribe to signals.
    ///
    /// `admin_url` — Holochain admin WebSocket URL (e.g. `ws://localhost:4444`).
    /// `app_url`   — app interface URL (e.g. `ws://localhost:4445`).
    /// `app_id`    — installed app ID (e.g. `"imagodei"`).
    /// `role`      — cell role name (e.g. `"imagodei"`).
    /// `db_pool`   — accepted for API compatibility; no longer used internally
    ///               (T6 deleted the `compromise_at` derivation path).
    ///
    /// Returns `Err` if the conductor is unreachable or the app/role is not
    /// installed. Returns `Ok(stream)` otherwise — the stream immediately starts
    /// delivering signals.
    pub async fn connect(
        admin_url: &str,
        app_url: &str,
        app_id: &str,
        role: &str,
        _db_pool: Option<Arc<DbPool>>,
    ) -> Result<Self, AppSignalStreamError> {
        let admin_addr = strip_ws_scheme(admin_url);
        let app_addr = strip_ws_scheme(app_url);

        info!(
            admin_addr = %admin_addr,
            app_addr = %app_addr,
            app_id = %app_id,
            role = %role,
            "HolochainAppSignalStream: connecting to imagodei conductor"
        );

        // Connect admin WebSocket.
        let admin_ws = AdminWebsocket::connect(&admin_addr, None)
            .await
            .map_err(|e| AppSignalStreamError::Connect(e.to_string()))?;

        // Ensure app interface exists on the target port.
        let app_port: u16 = app_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4445);
        if let Err(e) = admin_ws
            .attach_app_interface(app_port, None, AllowedOrigins::Any, None)
            .await
        {
            // May already exist — non-fatal.
            debug!(port = app_port, error = %e, "attach_app_interface (may already exist)");
        }

        // Find the target cell.
        let apps = admin_ws
            .list_apps(None)
            .await
            .map_err(|e| AppSignalStreamError::Connect(format!("list_apps: {e}")))?;

        let app_info = apps
            .iter()
            .find(|a| a.installed_app_id == app_id)
            .ok_or_else(|| AppSignalStreamError::NotFound(format!("app '{app_id}' not found")))?;

        let cell_id = app_info
            .cell_info
            .get(role)
            .and_then(|cells| cells.first())
            .and_then(|cell| match cell {
                holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                AppSignalStreamError::NotFound(format!("role '{role}' not found in app '{app_id}'"))
            })?;

        // Authorize signing credentials.
        let signer = ClientAgentSigner::default();
        let credentials = admin_ws
            .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                cell_id: cell_id.clone(),
                functions: None,
            })
            .await
            .map_err(|e| AppSignalStreamError::Credentials(e.to_string()))?;
        signer.add_credentials(cell_id.clone(), credentials);
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);

        // Issue app auth token.
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: app_id.to_string(),
                expiry_seconds: 3600,
                single_use: false,
            })
            .await
            .map_err(|e| AppSignalStreamError::Credentials(format!("issue_app_auth_token: {e}")))?;

        // Connect app WebSocket.
        let app_ws = AppWebsocket::connect(&app_addr, token.token, signer_arc.clone(), None)
            .await
            .map_err(|e| AppSignalStreamError::Connect(format!("app ws: {e}")))?;

        info!(
            app_id = %app_id,
            role = %role,
            "HolochainAppSignalStream: connected — subscribing to signals"
        );

        // Build the internal mpsc channel.
        // Buffer of 256: sufficient for burst reconciliation passes without
        // blocking the conductor signal callback. The controller drains
        // continuously via `run_loop()`, so steady-state depth is ~1.
        let (tx, rx) = mpsc::channel::<DnaSignal>(256);

        // Register the signal callback.
        // `on_signal` clones the closure into the WebSocket internals.
        // The `tx` clone keeps the channel open for the connection lifetime.
        let tx_clone = tx.clone();
        app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    if let Some(dns) = try_decode_and_translate(&bytes, None) {
                        // `try_send` is non-blocking. If the channel is full
                        // (controller is lagging), drop the signal and warn.
                        // The DHT is the source of truth; the controller can
                        // recover from a full rescan on reconnect.
                        if let Err(e) = tx_clone.try_send(dns) {
                            warn!(
                                error = %e,
                                "HolochainAppSignalStream: channel full or closed — \
                                 signal dropped (will be recovered from DHT on reconnect)"
                            );
                        }
                    }
                }
            })
            .await;

        // No warm-up zome call — successful on_signal registration is sufficient.
        // Reading from imagodei without a Human profile would error and add no value.
        // get_my_mastery takes a String (content_id) and requires a Human profile;
        // passing () would always fail. The on_signal registration above is the
        // validation that the connection is live.

        Ok(Self {
            rx,
            delivered: 0,
            // CRITICAL: keep app_ws alive — the on_signal callback lives inside
            // AppWebsocket. Dropping app_ws here would close the WS and make tx
            // unreachable, causing next_signal() to return None immediately.
            _app_ws: Some(app_ws),
        })
    }
}

#[async_trait]
impl DnaSignalStream for HolochainAppSignalStream {
    async fn next_signal(&mut self) -> Option<DnaSignal> {
        match self.rx.recv().await {
            Some(signal) => {
                self.delivered += 1;
                Some(signal)
            }
            None => None,
        }
    }

    async fn cursor(&self) -> Option<SignalCursor> {
        if self.delivered == 0 {
            None
        } else {
            Some(SignalCursor::new(self.delivered.to_string()))
        }
    }

    async fn resume_from(&mut self, cursor: SignalCursor) -> Result<(), SignalStreamError> {
        // TODO(A.12): implement cursor-based replay by querying the conductor for
        // signals since the given action_hash cursor. The current implementation
        // reconnects from scratch (no replay) — missed signals are recoverable
        // from a full DHT rescan on reconnect.
        Err(SignalStreamError::NotImplemented(format!(
            "HolochainAppSignalStream does not yet support resume_from (cursor = {cursor:?}); \
             Task A.12 will add replay-from-cursor semantics. \
             Call connect() to start a fresh subscription from the current head."
        )))
    }
}

// ---------------------------------------------------------------------------
// URL helper
// ---------------------------------------------------------------------------

/// Strip `ws://` or `wss://` scheme prefix for `holochain_client` socket APIs.
///
/// `holochain_client` expects `"host:port"` format, not `"ws://host:port"`.
fn strip_ws_scheme(url: &str) -> String {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("wss://") {
        rest.to_string()
    } else if let Some(rest) = url.strip_prefix("ws://") {
        rest.to_string()
    } else {
        url.to_string()
    }
}

// ---------------------------------------------------------------------------
// Unit tests (synthetic feeder — no real conductor required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::signal_stream::{AttestationKind, DnaSignal};

    // -----------------------------------------------------------------------
    // Fixtures: synthetic RecoveryV2Signal / ImagodeiSignal JSON values
    // that match the wire format emitted by the imagodei conductor.
    // -----------------------------------------------------------------------

    fn rotation_signal_value() -> serde_json::Value {
        serde_json::json!({
            "type": "KeyRotationCommitted",
            "action_hash": "uhCkk-rotation-action-hash",
            "rotation": {
                "human_agent_pubkey": "uhCAkNewPublicKeyBase64",
                "new_agent_pubkey": "uhCAkNewPublicKeyBase64",
                "superseded_agent_pubkey": "uhCAkOldPublicKeyBase64",
                "recovery_request_hash": "uhCkk-recovery-hash",
                "authority": { "type": "IntimateQuorum", "threshold": 3 },
                "rotated_at": 1_700_000_000_000_000i64
            }
        })
    }

    fn revocation_requested_value() -> serde_json::Value {
        serde_json::json!({
            "type": "KeyRevocationRequested",
            "id": "rev-test-001",
            "human_id": "human-matthew",
            "revoked_key": "uhCAkRevokedKeyBase64",
            "reason": "compromised",
            "trigger_type": "voluntary",
            "initiated_by": "steward-alice",
            "required_votes": 3u32,
            "current_votes": 1u32,
            "threshold_reached": false,
            "effective_at": null,
            "created_at": "2026-04-24T00:00:00Z"
        })
    }

    fn revocation_vote_value() -> serde_json::Value {
        serde_json::json!({
            "type": "RevocationVoteSubmitted",
            "id": "vote-001",
            "revocation_id": "rev-test-001",
            "steward_id": "steward-bob",
            "approved": true,
            "attestation": "signed-bytes",
            "voted_at": "2026-04-24T01:00:00Z",
            "current_votes": 2u32,
            "required_votes": 3u32,
            "threshold_now_reached": false
        })
    }

    fn revocation_effective_value() -> serde_json::Value {
        serde_json::json!({
            "type": "KeyRevocationEffective",
            "revocation_id": "rev-test-001",
            "revoked_key": "uhCAkRevokedKeyBase64",
            "human_id": "human-matthew",
            "effective_at": "2026-04-24T02:00:00Z",
            "triggering_vote_id": "vote-003"
        })
    }

    fn binding_created_value() -> serde_json::Value {
        // ImagodeiSignal uses `tag = "type", content = "payload"` — the outer
        // object has a "type" key and a "payload" key.
        serde_json::json!({
            "type": "AgentPeerBindingCreated",
            "payload": {
                "action_hash": "uhCkk-binding-action-hash",
                "binding": {
                    "peer_id": "12D3KooWTestPeerId",
                    "agent_cid": "bafybeicid-agent-1",
                    "valid_from": 1_700_000_000_000_000i64,
                    "valid_until": null,
                    "device_archetype": "node",
                    "signature": [1u8, 2, 3],
                    "superseded_by": null
                }
            }
        })
    }

    /// Encode a JSON value as MessagePack bytes, simulating the conductor wire format.
    fn to_msgpack(v: &serde_json::Value) -> Vec<u8> {
        rmp_serde::to_vec(v).expect("msgpack encode")
    }

    // -----------------------------------------------------------------------
    // Translate: KeyRotationCommitted → DnaSignal::KeyRotation
    // -----------------------------------------------------------------------

    #[test]
    fn translates_key_rotation_committed() {
        let bytes = to_msgpack(&rotation_signal_value());
        let dns = try_decode_and_translate(&bytes, None).expect("should translate");
        match dns {
            DnaSignal::KeyRotation(sig) => {
                assert_eq!(sig.action_hash, "uhCkk-rotation-action-hash");
                assert_eq!(sig.new_pubkey, "uhCAkNewPublicKeyBase64");
                assert_eq!(sig.old_pubkey, "uhCAkOldPublicKeyBase64");
                assert_eq!(sig.agent_cid, "uhCAkNewPublicKeyBase64");
                // rotated_at should be non-epoch (parsed from micros)
                assert!(
                    sig.rotated_at.timestamp() > 0,
                    "rotated_at must be positive"
                );
            }
            other => panic!("expected KeyRotation, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Translate: KeyRevocationEffective → None (T6 no-op)
    // The legacy signal is superseded by the T18 DnaSignal::KeyRevocation
    // envelope path in signals::handle_imagodei_dna_signal.
    // -----------------------------------------------------------------------

    #[test]
    fn key_revocation_effective_is_no_op_after_t6_deletion() {
        let bytes = to_msgpack(&revocation_effective_value());
        // T6: KeyRevocationEffective is in the "not consumed" catch-all — returns None.
        let dns = try_decode_and_translate(&bytes, None);
        assert!(
            dns.is_none(),
            "KeyRevocationEffective must return None after T6 deletion (T18 envelope path is canonical)"
        );
    }

    // -----------------------------------------------------------------------
    // Translate: KeyRevocationRequested → DnaSignal::RevocationAttestation (Request)
    // -----------------------------------------------------------------------

    #[test]
    fn translates_revocation_requested_to_attestation_request_kind() {
        let bytes = to_msgpack(&revocation_requested_value());
        let dns = try_decode_and_translate(&bytes, None).expect("should translate");
        match dns {
            DnaSignal::RevocationAttestation(sig) => {
                assert_eq!(sig.revocation_id, "rev-test-001");
                assert_eq!(sig.attestation_kind, AttestationKind::Request);
                assert!(sig.approved, "request kind is always approved");
                assert_eq!(sig.current_votes, 1);
                assert_eq!(sig.required_votes, 3);
                assert!(!sig.threshold_reached);
            }
            other => panic!("expected RevocationAttestation, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Translate: RevocationVoteSubmitted → DnaSignal::RevocationAttestation (Vote)
    // -----------------------------------------------------------------------

    #[test]
    fn translates_revocation_vote_submitted_to_attestation_vote_kind() {
        let bytes = to_msgpack(&revocation_vote_value());
        let dns = try_decode_and_translate(&bytes, None).expect("should translate");
        match dns {
            DnaSignal::RevocationAttestation(sig) => {
                assert_eq!(sig.revocation_id, "rev-test-001");
                assert_eq!(sig.action_hash, "vote-001");
                assert_eq!(sig.attestation_kind, AttestationKind::Vote);
                assert_eq!(sig.steward_id, "steward-bob");
                assert!(sig.approved);
                assert_eq!(sig.current_votes, 2);
                assert!(!sig.threshold_reached);
            }
            other => panic!("expected RevocationAttestation, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Translate: AgentPeerBindingCreated → DnaSignal::AgentPeerBinding
    // -----------------------------------------------------------------------

    #[test]
    fn translates_agent_peer_binding_created() {
        let bytes = to_msgpack(&binding_created_value());
        let dns = try_decode_and_translate(&bytes, None).expect("should translate");
        match dns {
            DnaSignal::AgentPeerBinding(sig) => {
                assert_eq!(sig.action_hash, "uhCkk-binding-action-hash");
                assert_eq!(sig.binding_action_hash, "uhCkk-binding-action-hash");
                assert_eq!(sig.peer_id, "12D3KooWTestPeerId");
                assert_eq!(sig.agent_cid, "bafybeicid-agent-1");
                assert_eq!(sig.device_archetype, DeviceArchetype::Node);
                assert!(sig.valid_until.is_none());
                assert!(sig.valid_from.timestamp() > 0);
                // superseded_by is null in the fixture — translator must yield None.
                assert!(sig.superseded_by.is_none());
            }
            other => panic!("expected AgentPeerBinding, got {other:?}"),
        }
    }

    /// T03b: the translator extracts `superseded_by` from the binding payload
    /// when the DNA emits a JSON-string ActionHash. This is the authoritative
    /// supersession reference — the controller writes it verbatim to the
    /// `peer_identity_bindings` projection.
    #[test]
    fn translates_agent_peer_binding_created_with_superseded_by() {
        let value = serde_json::json!({
            "type": "AgentPeerBindingCreated",
            "payload": {
                "action_hash": "uhCkk-binding-action-hash-new",
                "binding": {
                    "peer_id": "12D3KooWTestPeerId",
                    "agent_cid": "bafybeicid-agent-1",
                    "valid_from": 1_700_000_000_000_000i64,
                    "valid_until": null,
                    "device_archetype": "mobile",
                    "signature": [1u8, 2, 3],
                    "superseded_by": "uhCkk-prev-binding-action-hash"
                }
            }
        });
        let bytes = to_msgpack(&value);
        let dns = try_decode_and_translate(&bytes, None).expect("should translate");
        match dns {
            DnaSignal::AgentPeerBinding(sig) => {
                assert_eq!(sig.device_archetype, DeviceArchetype::Mobile);
                assert_eq!(
                    sig.superseded_by,
                    Some("uhCkk-prev-binding-action-hash".to_string()),
                    "translator must lift superseded_by out of the binding payload"
                );
            }
            other => panic!("expected AgentPeerBinding, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Unknown / unrelated signal → None
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_signal_returns_none() {
        let unknown = serde_json::json!({ "type": "LamadContentPublished", "payload": {} });
        let bytes = to_msgpack(&unknown);
        let result = try_decode_and_translate(&bytes, None);
        assert!(result.is_none(), "unrecognized signal must return None");
    }

    // -----------------------------------------------------------------------
    // Malformed bytes → None (no panic)
    // -----------------------------------------------------------------------

    #[test]
    fn malformed_bytes_return_none() {
        let bytes = b"not valid msgpack \xff\xfe";
        let result = try_decode_and_translate(bytes, None);
        assert!(result.is_none(), "malformed bytes must return None");
    }

    // -----------------------------------------------------------------------
    // Channel close → next_signal returns None
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn next_signal_returns_none_after_close() {
        let (tx, rx) = mpsc::channel::<DnaSignal>(8);
        let mut stream = HolochainAppSignalStream::for_testing(rx);

        // Push one signal then drop the sender.
        let rotation = DnaSignal::KeyRotation(KeyRotationSignal {
            action_hash: "ah".into(),
            agent_cid: "cid".into(),
            new_pubkey: "new".into(),
            old_pubkey: "old".into(),
            rotated_at: Utc::now(),
            emitted_at: Utc::now(),
        });
        tx.send(rotation).await.unwrap();
        drop(tx);

        // Should deliver the one signal then return None.
        let first = stream.next_signal().await;
        assert!(matches!(first, Some(DnaSignal::KeyRotation(_))));
        let done = stream.next_signal().await;
        assert!(done.is_none(), "must return None after channel closed");
    }

    // -----------------------------------------------------------------------
    // Cursor increments on delivery
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn cursor_increments_on_delivery() {
        let (tx, rx) = mpsc::channel::<DnaSignal>(8);
        let mut stream = HolochainAppSignalStream::for_testing(rx);

        assert_eq!(stream.cursor().await, None);

        let rotation = DnaSignal::KeyRotation(KeyRotationSignal {
            action_hash: "ah".into(),
            agent_cid: "cid".into(),
            new_pubkey: "new".into(),
            old_pubkey: "old".into(),
            rotated_at: Utc::now(),
            emitted_at: Utc::now(),
        });
        tx.send(rotation).await.unwrap();
        drop(tx);

        let _ = stream.next_signal().await;
        assert_eq!(stream.cursor().await, Some(SignalCursor::new("1")));
    }

    // -----------------------------------------------------------------------
    // resume_from always errors (Stage 1) — returns NotImplemented variant
    //
    // NOTE: This test constructs via for_testing() to avoid needing a real
    // conductor. The live connect() path is validated by sweettest in Task A.12.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn resume_from_returns_error() {
        let (_tx, rx) = mpsc::channel::<DnaSignal>(1);
        let mut stream = HolochainAppSignalStream::for_testing(rx);
        let result = stream.resume_from(SignalCursor::new("0")).await;
        assert!(
            matches!(result, Err(SignalStreamError::NotImplemented(_))),
            "HolochainAppSignalStream.resume_from must return NotImplemented at Stage 1"
        );
    }

    // -----------------------------------------------------------------------
    // Device archetype fallback
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_device_archetype_falls_back_to_node() {
        let archetype = parse_device_archetype("quantum-blade");
        assert_eq!(archetype, DeviceArchetype::Node);
    }

    #[test]
    fn all_known_device_archetypes_parse() {
        assert_eq!(parse_device_archetype("node"), DeviceArchetype::Node);
        assert_eq!(parse_device_archetype("desktop"), DeviceArchetype::Desktop);
        assert_eq!(parse_device_archetype("mobile"), DeviceArchetype::Mobile);
        assert_eq!(parse_device_archetype("steward"), DeviceArchetype::Steward);
    }
}
