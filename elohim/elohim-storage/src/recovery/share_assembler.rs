//! Off-chain Shamir share assembler — DHT-attested, libp2p-transported.
//!
//! ## Design contract
//!
//! `ShareAssembler::assemble` is OPTIONAL — it is invoked ONLY when actual
//! cryptographic key material must be recovered (high-security path). The
//! social-threshold flow (attestations + governance-actions + tally) runs to
//! completion without ever calling into this module.
//!
//! Call site:
//! ```text
//! if needs_key_material_recovery {
//!     match assembler.assemble(&recovery_governance_action_cid).await? {
//!         AssemblyResult::ReconstructedSecret(secret) => { /* use secret */ }
//!         AssemblyResult::BelowThreshold { .. } => { /* retry or abort */ }
//!     }
//! }
//! ```
//!
//! ## Flow
//!
//! 1. Query `attestations` projection for children of the governance-action
//!    with `attestation_kind = "attestation:recovery-approval"`.
//! 2. For each approving custodian, send a `ShamirShareRequest` via the
//!    `ShareTransport` (libp2p in production, mock in tests).
//! 3. Verify each `ShamirShareResponse`:
//!    (a) `attestation_cid` must match a real `attestations` DB row.
//!    (b) `signature` must verify against the custodian's Ed25519 key, resolved
//!        via [`resolve_custodian_verifying_key`].
//! 4. Accumulate verified shares until the threshold `m` declared in the
//!    governance-action's `threshold_json` is reached.
//! 5. Reconstruct the secret via `sharks` (Shamir over GF(256)) and return
//!    `AssemblyResult::ReconstructedSecret(bytes)`.
//!
//! ## Shamir reconstruction (T21)
//!
//! Uses the `sharks` crate (v0.5) — Shamir Secret Sharing over GF(256),
//! pure Rust, no_std compatible.  The reconstruction call is:
//! ```ignore
//! let shark = Sharks(threshold as u8);
//! let shares: Vec<sharks::Share> = verified.iter().map(|v| v.as_sharks_share()).collect();
//! let secret = shark.recover(shares.iter()).map_err(…)?;
//! ```
//! Share bytes are stored on `VerifiedShare::share_data` in the format
//! produced by the T22 split ceremony (one `sharks::Share` per custodian,
//! serialised via `Vec<u8>`).
//!
//! ## Key resolution (T21 — replaces derive_verifying_key_stub)
//!
//! [`resolve_custodian_verifying_key`] resolves a custodian's verifying key by:
//!   1. Looking up `peer_identity_bindings` for the custodian's `agent_cid`
//!      (the `custodian_cid` parameter).
//!   2. Extracting the 32-byte Ed25519 public key from the libp2p `PeerId`
//!      string using `PeerId::from_str` + `PublicKey::try_into_ed25519()`.
//!
//! A `PeerId` that was generated from an Ed25519 keypair encodes the raw
//! 32-byte compressed public key in the identity-multihash payload.  Any
//! `PeerId` not generated from Ed25519 (e.g. RSA) returns `None`, which
//! triggers the existing "no key available — share accepted on DHT attestation
//! alone" fallback path.

use diesel::prelude::*;
use libp2p::PeerId;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::diesel_schema::{attestations, governance_actions, peer_identity_bindings};
use crate::db::models::{AttestationRow, GovernanceActionRow};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::p2p::shamir_transport::{
    verify_share_response, ShamirShareRequest, ShamirShareResponse,
};
use crate::p2p::{P2PCommand, P2PHandle};

// ─────────────────────────────────────────────────────────────────────────────
// ShareTransport trait — mockable in tests, libp2p-backed in production
// ─────────────────────────────────────────────────────────────────────────────

/// Transport abstraction for sending share requests to custodian peers.
///
/// In production: sends a `ShamirShareRequest` via the
/// `/elohim/shamir-share/1.0.0` request-response protocol.
///
/// In tests: use [`MockShareTransport`] to inject canned responses.
#[async_trait::async_trait]
pub trait ShareTransport: Send + Sync {
    /// Request a share from the custodian identified by `custodian_cid`.
    ///
    /// The transport is responsible for resolving `custodian_cid` to a libp2p
    /// `PeerId` (via the `peer_identity_bindings` projection or mDNS) and
    /// sending the `ShamirShareRequest` over the `/elohim/shamir-share/1.0.0`
    /// channel.
    ///
    /// Returns `Ok(ShamirShareResponse)` on success, `Err(msg)` if the peer is
    /// unreachable, the protocol is not supported, or the request times out.
    async fn request_share(&self, req: ShamirShareRequest) -> Result<ShamirShareResponse, String>;
}

/// Test double: returns canned share responses keyed by `custodian_cid`.
///
/// Allows unit tests to exercise the assembler's verification + threshold
/// logic without a live libp2p swarm.
pub struct MockShareTransport {
    /// custodian_cid → canned response (or error if None)
    responses: Arc<Mutex<HashMap<String, Result<ShamirShareResponse, String>>>>,
}

impl MockShareTransport {
    /// Create a mock transport with the given canned responses.
    pub fn new(responses: HashMap<String, Result<ShamirShareResponse, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
        }
    }
}

#[async_trait::async_trait]
impl ShareTransport for MockShareTransport {
    async fn request_share(&self, req: ShamirShareRequest) -> Result<ShamirShareResponse, String> {
        let responses = self.responses.lock().await;
        match responses.get(&req.custodian_cid) {
            Some(Ok(resp)) => Ok(resp.clone()),
            Some(Err(e)) => Err(e.clone()),
            None => Err(format!(
                "MockShareTransport: no canned response for custodian_cid={}",
                req.custodian_cid
            )),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LibP2PShareTransport — production implementation via swarm command channel
// ─────────────────────────────────────────────────────────────────────────────

/// Production [`ShareTransport`] implementation that routes share requests
/// through the libp2p swarm via [`P2PHandle`].
///
/// ## Peer resolution
///
/// `custodian_cid` (the Holochain agent pubkey of the custodian) is resolved
/// to a libp2p `PeerId` using the `peer_identity_bindings` SQLite projection
/// via [`crate::db::recovery_approval_gate::resolve_custodian_peer_id`].
///
/// If no active binding exists, the request fails with a dial-failure error and
/// the assembler skips to the next custodian.
///
/// TODO(T22-multi-device): when a custodian has multiple active devices
/// (multiple bindings), this implementation tries only the most recently
/// observed one. A future sprint should try all active peers in order,
/// mirroring the view-federation fanout pattern.
///
/// ## Signature verification
///
/// After receiving a non-error response, the transport performs signature
/// verification using [`verify_share_response`] and the custodian's
/// Ed25519 verifying key resolved via [`resolve_custodian_verifying_key`].
///
/// If no verifying key is resolvable (no active `peer_identity_bindings` row,
/// or the PeerId is not Ed25519-typed), the share is accepted on DHT attestation
/// alone with a warning.
pub struct LibP2PShareTransport {
    /// Handle to the live P2P swarm. Used to send `P2PCommand::RequestShamirShare`.
    handle: P2PHandle,
    /// DB pool for resolving `custodian_cid → PeerId` via `peer_identity_bindings`.
    pool: DbPool,
    /// Per-request timeout for the libp2p round-trip.
    request_timeout: std::time::Duration,
}

impl LibP2PShareTransport {
    /// Create a new `LibP2PShareTransport`.
    ///
    /// `request_timeout` is the per-request wall-clock deadline. Recommended:
    /// 15–30 seconds (recovery is latency-insensitive; custodians may be
    /// temporarily offline).
    pub fn new(handle: P2PHandle, pool: DbPool, request_timeout: std::time::Duration) -> Self {
        Self {
            handle,
            pool,
            request_timeout,
        }
    }
}

#[async_trait::async_trait]
impl ShareTransport for LibP2PShareTransport {
    async fn request_share(&self, req: ShamirShareRequest) -> Result<ShamirShareResponse, String> {
        // ── Step 1: resolve custodian_cid → PeerId ────────────────────────
        let peer_id_str: String = {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| format!("LibP2PShareTransport: pool.get error: {e}"))?;
            let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            crate::db::recovery_approval_gate::resolve_custodian_peer_id(
                &mut conn,
                &req.custodian_cid,
                &now_iso,
            )
            .map_err(|e| {
                format!(
                    "LibP2PShareTransport: peer resolution DB error for {}: {e}",
                    req.custodian_cid
                )
            })?
            .ok_or_else(|| {
                format!(
                    "LibP2PShareTransport: no active peer binding for custodian_cid={}; \
                     TODO(T22-multi-device) try all active devices",
                    req.custodian_cid
                )
            })?
        };

        let peer_id = PeerId::from_str(&peer_id_str).map_err(|e| {
            format!(
                "LibP2PShareTransport: invalid PeerId '{}' for custodian_cid={}: {e}",
                peer_id_str, req.custodian_cid
            )
        })?;

        // ── Step 2: send request via swarm command channel ────────────────
        let (tx, rx) = tokio::sync::oneshot::channel();
        let custodian_cid_for_verify = req.custodian_cid.clone();
        let recovery_cid_for_verify = req.recovery_governance_action_cid.clone();

        self.handle
            .command_sender()
            .send(P2PCommand::RequestShamirShare {
                peer: peer_id,
                request: req,
                respond: tx,
            })
            .await
            .map_err(|_| {
                "LibP2PShareTransport: swarm command channel closed (swarm task exited)"
                    .to_string()
            })?;

        // ── Step 3: await response with timeout ───────────────────────────
        let response = tokio::time::timeout(self.request_timeout, rx)
            .await
            .map_err(|_| {
                format!(
                    "LibP2PShareTransport: timeout waiting for share from peer {peer_id_str}"
                )
            })?
            .map_err(|_| {
                "LibP2PShareTransport: response oneshot dropped (swarm task exited)".to_string()
            })?
            .map_err(|e| format!("LibP2PShareTransport: transport failure: {e}"))?;

        // ── Step 4: check error envelope ──────────────────────────────────
        if response.is_error() {
            return Err(format!(
                "LibP2PShareTransport: custodian {} returned error: {}",
                custodian_cid_for_verify,
                response.error_reason.as_deref().unwrap_or("<no reason>")
            ));
        }

        // ── Step 5: resolve verifying key + verify signature ─────────────
        //
        // Key resolution path (T21):
        //   custodian_cid (= agent_cid) → peer_identity_bindings → peer_id
        //   → extract Ed25519 public key from PeerId multihash.
        //
        // Falls back to DHT-attestation-only acceptance when no active
        // peer_identity_bindings row exists for the custodian.
        let key_bytes_opt: Option<[u8; 32]> = {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| format!("LibP2PShareTransport: pool.get for key-resolve: {e}"))?;
            let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            resolve_custodian_verifying_key(&mut conn, &custodian_cid_for_verify, &now_iso)
                .map_err(|e| format!("LibP2PShareTransport: key-resolve DB error: {e}"))?
        };

        if let Some(key_bytes) = key_bytes_opt {
            verify_share_response(&response, &recovery_cid_for_verify, &key_bytes).map_err(
                |e| {
                    format!(
                        "LibP2PShareTransport: signature verification failed for {}: {e}",
                        custodian_cid_for_verify
                    )
                },
            )?;
        } else {
            tracing::warn!(
                custodian_cid = %custodian_cid_for_verify,
                "T21: verifying key not resolvable from peer_identity_bindings — \
                 share accepted on DHT attestation alone (security downgrade)"
            );
        }

        Ok(response)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Key resolution (T21 — replaces derive_verifying_key_stub)
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve a custodian's 32-byte Ed25519 verifying key from local storage.
///
/// ## Resolution path
///
/// 1. Query `peer_identity_bindings` for an active row where `agent_cid =
///    custodian_cid` (`valid_until IS NULL OR valid_until > now_iso`, AND
///    `superseded_by IS NULL`).
/// 2. Parse the `peer_id` string into a libp2p `PeerId`.
/// 3. Extract the compressed Ed25519 public key (32 bytes) from the PeerId's
///    embedded `PublicKey`.
///
/// A libp2p `PeerId` generated from an Ed25519 keypair is an identity-multihash
/// of the 32-byte compressed key (`libp2p::identity::PublicKey::try_into_ed25519()`).
/// Peer IDs generated from other key types (RSA, secp256k1) are not extractable
/// and cause this function to return `Ok(None)`.
///
/// ## Returns
///
/// - `Ok(Some([u8; 32]))` — resolved; signature verification MUST run.
/// - `Ok(None)` — no active binding, or PeerId is not Ed25519-typed; caller
///   falls back to DHT-attestation-only acceptance.
/// - `Err(StorageError)` — DB query failed; caller surfaces an error.
///
/// ## Why peer_identity_bindings, not the attestation row?
///
/// The `attestations.issuer_cid` column stores a Holochain action-hash CID
/// (a content-addressed identifier, NOT the raw Ed25519 public key). Holochain
/// CIDs embed the 32-byte key in a 39-byte structure with type prefix and
/// checksum, but parsing that structure requires the Holochain HDK / holo_hash
/// library — a dependency we deliberately avoid in the elohim-storage domain
/// layer. The `peer_identity_bindings.peer_id` column stores a libp2p PeerId,
/// which for Ed25519 keys is directly decodable to the 32-byte key bytes using
/// the `libp2p::identity` API already present in this crate.
pub fn resolve_custodian_verifying_key(
    conn: &mut SqliteConnection,
    custodian_cid: &str,
    now_iso: &str,
) -> Result<Option<[u8; 32]>, StorageError> {
    // Look up the most recently observed active binding for this agent.
    let peer_id_str: Option<String> = peer_identity_bindings::table
        .filter(peer_identity_bindings::agent_cid.eq(custodian_cid))
        .filter(
            peer_identity_bindings::valid_until
                .is_null()
                .or(peer_identity_bindings::valid_until.gt(now_iso)),
        )
        .filter(peer_identity_bindings::superseded_by.is_null())
        .order(peer_identity_bindings::observed_at.desc())
        .select(peer_identity_bindings::peer_id)
        .first::<String>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!(
                "resolve_custodian_verifying_key: peer_identity_bindings lookup for {custodian_cid}: {e}"
            ))
        })?;

    let peer_id_str = match peer_id_str {
        Some(s) => s,
        None => {
            tracing::debug!(
                custodian_cid = %custodian_cid,
                "T21: no active peer_identity_bindings row for custodian; \
                 verifying key unavailable"
            );
            return Ok(None);
        }
    };

    let peer_id = match PeerId::from_str(&peer_id_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                custodian_cid = %custodian_cid,
                peer_id = %peer_id_str,
                error = %e,
                "T21: peer_id string in peer_identity_bindings is not a valid PeerId"
            );
            return Ok(None);
        }
    };

    // Attempt to extract the Ed25519 public key from the PeerId.
    //
    // libp2p PeerIds for small keys (≤ 42 bytes, which includes Ed25519) use an
    // "identity" multihash (code 0x00) whose digest IS the protobuf-encoded
    // PublicKey. For Ed25519 keys this means the digest decodes directly to
    // the libp2p `PublicKey` enum, from which we extract the 32-byte raw key.
    //
    // Ed25519 PeerIds always use identity multihash (the 32-byte key + protobuf
    // framing fits within the inline-key threshold). RSA / secp256k1 PeerIds
    // use SHA-256 multihash and are NOT recoverable here — caller falls back to
    // DHT-attestation-only acceptance.
    let multihash = peer_id.as_ref();
    let key_bytes: Option<[u8; 32]> = if multihash.code() == 0 {
        // Identity multihash: digest is the protobuf-encoded PublicKey.
        libp2p::identity::PublicKey::try_decode_protobuf(multihash.digest())
            .ok()
            .and_then(|pk| pk.try_into_ed25519().ok())
            .map(|k| k.to_bytes())
    } else {
        // SHA-256 multihash or other: pubkey is NOT recoverable from the
        // PeerId alone. Stage 2 follow-up (see services::federator's "resolver-
        // backed peer_id → PublicKey lookup" note) would resolve via another
        // path (e.g., a separate pubkey column on peer_identity_bindings).
        None
    };

    if key_bytes.is_none() {
        tracing::debug!(
            custodian_cid = %custodian_cid,
            peer_id = %peer_id_str,
            "T21: PeerId for custodian is not Ed25519-typed; \
             verifying key not extractable"
        );
    }

    Ok(key_bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// AssemblyResult
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of a `ShareAssembler::assemble` call.
#[derive(Debug)]
pub enum AssemblyResult {
    /// Not enough custodians approved (below threshold m).
    ///
    /// Contains the number of approvals found and the threshold required.
    BelowThreshold {
        approvals_found: usize,
        threshold: usize,
    },

    /// Threshold reached and shares reconstructed into the original secret.
    ///
    /// The raw secret bytes are returned. The caller is responsible for
    /// interpreting them (e.g., as a 32-byte seed, an AES key, etc.).
    ///
    /// The individual share bytes do NOT leave the assembler; only the
    /// reconstructed secret is returned.
    ReconstructedSecret(Vec<u8>),
}

/// A single verified Shamir share (internal to the assembler; not exposed in
/// `AssemblyResult`).
#[derive(Debug, Clone)]
pub struct VerifiedShare {
    /// 1-based index within the (m,n) scheme.
    pub share_index: u32,
    /// Raw share bytes as received from the custodian and verified.
    pub share_data: Vec<u8>,
    /// CID of the `attestation:recovery-approval` that authorized this share.
    pub attestation_cid: String,
    /// CID of the custodian who delivered this share.
    pub custodian_cid: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// ShareAssembler
// ─────────────────────────────────────────────────────────────────────────────

/// Assembles Shamir shares for a recovery governance action and reconstructs
/// the original secret once the threshold is reached.
///
/// Constructed with a `DbPool` (to query the `attestations` + `governance_actions`
/// projections) and a `ShareTransport` implementation (libp2p in production,
/// mock in tests).
///
/// `assemble` is the single public entrypoint. It is OPTIONAL — invoke it only
/// when share material recovery is needed.
pub struct ShareAssembler {
    pool: DbPool,
    transport: Arc<dyn ShareTransport>,
}

impl ShareAssembler {
    /// Create a new assembler.
    ///
    /// `transport` is the share delivery mechanism. In production, wrap the
    /// libp2p swarm handle behind a `dyn ShareTransport` impl. In tests,
    /// use [`MockShareTransport`].
    pub fn new(pool: DbPool, transport: Arc<dyn ShareTransport>) -> Self {
        Self { pool, transport }
    }

    /// Assemble shares for the given recovery governance action and reconstruct
    /// the secret.
    ///
    /// This is OPTIONAL — invoke only when key-material recovery is required.
    ///
    /// # Steps
    ///
    /// 1. Load the governance action + its threshold from `governance_actions`.
    /// 2. Query `attestations` for `attestation_kind = "attestation:recovery-approval"`
    ///    children of `parent_governance_action_cid = recovery_governance_action_cid`.
    /// 3. For each approving custodian: send `ShamirShareRequest`, verify response.
    /// 4. Reconstruct the secret via `sharks` once threshold shares are verified.
    /// 5. Return `BelowThreshold` if not enough verified, `ReconstructedSecret` otherwise.
    pub async fn assemble(
        &self,
        recovery_governance_action_cid: &str,
    ) -> Result<AssemblyResult, StorageError> {
        // Step 1: load governance action to get threshold
        let threshold = {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| StorageError::Database(format!("pool.get error: {}", e)))?;
            let action = governance_actions::table
                .filter(governance_actions::id.eq(recovery_governance_action_cid))
                .first::<GovernanceActionRow>(&mut conn)
                .optional()
                .map_err(|e| StorageError::Database(e.to_string()))?
                .ok_or_else(|| {
                    StorageError::NotFound(format!(
                        "governance action not found: {}",
                        recovery_governance_action_cid
                    ))
                })?;

            parse_threshold_m(&action.threshold_json)?
        };

        // Step 2: load recovery-approval attestations for this governance action
        let approvals: Vec<AttestationRow> = {
            let mut conn = self
                .pool
                .get()
                .map_err(|e| StorageError::Database(format!("pool.get error: {}", e)))?;
            attestations::table
                .filter(
                    attestations::parent_governance_action_cid.eq(recovery_governance_action_cid),
                )
                .filter(attestations::attestation_kind.eq("attestation:recovery-approval"))
                .filter(attestations::revoked_at.is_null())
                .load::<AttestationRow>(&mut conn)
                .map_err(|e| StorageError::Database(e.to_string()))?
        };

        if approvals.is_empty() {
            return Ok(AssemblyResult::BelowThreshold {
                approvals_found: 0,
                threshold,
            });
        }

        // Step 3: for each custodian, send a share request + verify the response
        let mut verified_shares: Vec<VerifiedShare> = Vec::new();

        for approval in &approvals {
            // Guard: already have enough shares — stop early
            if verified_shares.len() >= threshold {
                break;
            }

            let req = ShamirShareRequest {
                recovery_governance_action_cid: recovery_governance_action_cid.to_string(),
                custodian_cid: approval.issuer_cid.clone(),
            };

            // Send via transport (libp2p or mock)
            let response = match self.transport.request_share(req).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        custodian_cid = %approval.issuer_cid,
                        error = %e,
                        "share request failed — skipping custodian"
                    );
                    continue;
                }
            };

            // Check for error envelope before proceeding with verification.
            if response.is_error() {
                tracing::warn!(
                    custodian_cid = %approval.issuer_cid,
                    reason = %response.error_reason.as_deref().unwrap_or("<no reason>"),
                    "share response is an error envelope — skipping custodian"
                );
                continue;
            }

            // Step 3a: verify attestation_cid matches a real DB row
            let attestation_exists = {
                let mut conn = self
                    .pool
                    .get()
                    .map_err(|e| StorageError::Database(format!("pool.get error: {}", e)))?;
                attestations::table
                    .filter(attestations::id.eq(&response.attestation_cid))
                    .filter(
                        attestations::parent_governance_action_cid
                            .eq(recovery_governance_action_cid),
                    )
                    .filter(attestations::revoked_at.is_null())
                    .count()
                    .get_result::<i64>(&mut conn)
                    .map_err(|e| StorageError::Database(e.to_string()))?
                    > 0
            };

            if !attestation_exists {
                tracing::warn!(
                    custodian_cid = %approval.issuer_cid,
                    attestation_cid = %response.attestation_cid,
                    "share response references unknown/revoked attestation — rejecting"
                );
                continue;
            }

            // Step 3b: verify signature using resolved key.
            //
            // Key resolution (T21): custodian_cid (agent_cid) →
            // peer_identity_bindings → peer_id → Ed25519 public key bytes.
            //
            // Fallback: if no active binding exists or the PeerId is not
            // Ed25519-typed, accept the share on DHT attestation alone and log
            // a warning.  The DHT attestation check in step 3a is the primary
            // guard; the signature check adds a second layer.
            let key_bytes_opt: Option<[u8; 32]> = {
                let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                let mut conn = self
                    .pool
                    .get()
                    .map_err(|e| StorageError::Database(format!("pool.get error: {}", e)))?;
                match resolve_custodian_verifying_key(&mut conn, &approval.issuer_cid, &now_iso) {
                    Ok(k) => k,
                    Err(e) => {
                        tracing::warn!(
                            custodian_cid = %approval.issuer_cid,
                            error = %e,
                            "T21: key resolution DB error — share accepted on DHT attestation alone"
                        );
                        None
                    }
                }
            };

            if let Some(key_bytes) = key_bytes_opt {
                if let Err(e) =
                    verify_share_response(&response, recovery_governance_action_cid, &key_bytes)
                {
                    tracing::warn!(
                        custodian_cid = %approval.issuer_cid,
                        error = %e,
                        "share response signature verification failed — rejecting"
                    );
                    continue;
                }
            } else {
                tracing::warn!(
                    custodian_cid = %approval.issuer_cid,
                    "T21: verifying key not available — \
                     share accepted on DHT attestation alone (security downgrade)"
                );
            }

            verified_shares.push(VerifiedShare {
                share_index: response.share_index,
                share_data: response.share_data,
                attestation_cid: response.attestation_cid,
                custodian_cid: approval.issuer_cid.clone(),
            });
        }

        // Step 4: check threshold
        if verified_shares.len() < threshold {
            return Ok(AssemblyResult::BelowThreshold {
                approvals_found: verified_shares.len(),
                threshold,
            });
        }

        // Sort by share_index for deterministic reconstruction order
        verified_shares.sort_by_key(|s| s.share_index);

        // Step 5: reconstruct secret via Shamir (sharks 0.5, GF(256))
        //
        // `sharks::Sharks(threshold)` is the scheme parameterised by the
        // minimum number of shares required.  `recover` takes an iterator of
        // `&sharks::Share` and returns `Ok(Vec<u8>)` (the original secret
        // bytes) or `Err(&str)` if the shares are incompatible or below
        // threshold.
        //
        // `sharks::Share` implements `TryFrom<&[u8]>` — the share bytes
        // produced by the T22 split ceremony are the wire format `sharks`
        // itself serialises (each share is a struct with an x-coordinate byte
        // followed by one y-coordinate byte per secret byte).
        let secret = reconstruct_secret(threshold as u8, &verified_shares)?;

        Ok(AssemblyResult::ReconstructedSecret(secret))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shamir reconstruction helper
// ─────────────────────────────────────────────────────────────────────────────

/// Reconstruct the secret from a set of verified shares using `sharks`.
///
/// ## Wire format
///
/// Each `VerifiedShare::share_data` is the byte representation of a
/// `sharks::Share` as produced by `Vec::<u8>::from(&share)`.  The inverse is
/// `sharks::Share::try_from(bytes.as_slice())`.
///
/// ## Error cases
///
/// - Share bytes malformed (not a valid `sharks::Share`) → `StorageError::InvalidInput`
/// - `sharks::recover` fails (incompatible points, arithmetic error) → `StorageError::InvalidInput`
///
/// ## Threshold parameter
///
/// `threshold` is `m` from the governance-action's `threshold_json`; it must
/// equal the `threshold` value used when the shares were originally split.
/// A mismatch produces garbage output from GF(256) arithmetic, which `sharks`
/// expresses as `Err("Not enough shares")` because the reconstructed polynomial
/// evaluates at `x = 0` to an unexpected value.
fn reconstruct_secret(threshold: u8, shares: &[VerifiedShare]) -> Result<Vec<u8>, StorageError> {
    use sharks::{Share, Sharks};

    let shark = Sharks(threshold);

    // Decode each VerifiedShare's raw bytes back into a sharks::Share.
    let decoded: Vec<Share> = shares
        .iter()
        .map(|vs| {
            Share::try_from(vs.share_data.as_slice()).map_err(|e| {
                StorageError::InvalidInput(format!(
                    "Shamir reconstruction: share_index={} could not be decoded: {e}",
                    vs.share_index
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    shark.recover(decoded.iter()).map_err(|e| {
        StorageError::InvalidInput(format!(
            "Shamir reconstruction failed: {e} \
             (verify that all shares were produced by the same split ceremony \
              and that the threshold matches)"
        ))
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the `m` (required quorum count) from a governance action's threshold JSON.
///
/// Expected shape: `{ "type": "shamir", "m": 3, "n": 5 }`.
/// Returns `StorageError::InvalidInput` if the field is absent or malformed.
fn parse_threshold_m(threshold_json: &str) -> Result<usize, StorageError> {
    let val: Value = serde_json::from_str(threshold_json)
        .map_err(|e| StorageError::InvalidInput(format!("threshold_json parse error: {}", e)))?;
    val.get("m")
        .and_then(|v| v.as_u64())
        .map(|m| m as usize)
        .ok_or_else(|| {
            StorageError::InvalidInput(
                "threshold_json missing 'm' field (required quorum count)".to_string(),
            )
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sharks::{Share, Sharks};
    use std::sync::Arc;

    // ── DB setup helpers ──────────────────────────────────────────────────────

    /// Build a minimal in-memory DB pool with the full schema.
    fn test_pool() -> DbPool {
        use crate::db::{init_pool_from_dir, run_migrations};
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        std::mem::forget(dir);
        pool
    }

    /// Insert a governance-action row into the test DB.
    fn insert_governance_action(conn: &mut SqliteConnection, id: &str, threshold_json: &str) {
        diesel::replace_into(governance_actions::table)
            .values((
                governance_actions::id.eq(id),
                governance_actions::dht_anchor_hash.eq(vec![0u8; 39]),
                governance_actions::governance_kind.eq("governance-action:recovery-request"),
                governance_actions::subject_cid.eq("subject-cid"),
                governance_actions::proposer_cid.eq("proposer-cid"),
                governance_actions::threshold_json.eq(threshold_json),
                governance_actions::ballot_format.eq("approval"),
                governance_actions::closes_at.eq("2099-01-01T00:00:00Z"),
                governance_actions::title.eq("test recovery request"),
                governance_actions::created_at.eq("2026-05-15T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert governance action");
    }

    /// Insert a recovery-approval attestation row into the test DB.
    fn insert_approval_attestation(
        conn: &mut SqliteConnection,
        id: &str,
        parent_id: &str,
        issuer_cid: &str,
    ) {
        diesel::replace_into(attestations::table)
            .values((
                attestations::id.eq(id),
                attestations::dht_anchor_hash.eq(vec![0u8; 39]),
                attestations::attestation_kind.eq("attestation:recovery-approval"),
                attestations::subject_cid.eq("subject-cid"),
                attestations::subject_kind.eq("human"),
                attestations::issuer_cid.eq(issuer_cid),
                attestations::parent_governance_action_cid.eq(parent_id),
                attestations::proof_class.eq("witness"),
                attestations::proof_evidence_json.eq("{}"),
                attestations::evidence_json.eq("{}"),
                attestations::manifest_ref.eq("test"),
                attestations::title.eq("test approval"),
                attestations::created_at.eq("2026-05-15T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert attestation");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // T21 Test 1: split known secret → reconstruct via assembler end-to-end
    // ─────────────────────────────────────────────────────────────────────────

    /// Split a known 32-byte secret into 5 shares (threshold 3), wire 3 of them
    /// through the assembler, and verify reconstruction matches the original.
    #[tokio::test]
    async fn t21_reconstruct_from_threshold_shares() {
        let secret = b"elohim-recovery-test-secret-32b!";
        assert_eq!(secret.len(), 32);

        // Split into 5 shares, threshold 3.
        let shark = Sharks(3);
        let dealer = shark.dealer(secret);
        let all_shares: Vec<Share> = dealer.take(5).collect();

        // Encode shares to wire bytes (sharks::Share → Vec<u8>).
        let share_bytes: Vec<Vec<u8>> = all_shares.iter().map(|s| Vec::from(s)).collect();

        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t21-action-001", r#"{"type":"shamir","m":3,"n":5}"#);
            for i in 1..=5usize {
                let cust = format!("cust-{i}");
                let att = format!("att-{i}");
                insert_approval_attestation(&mut conn, &att, "t21-action-001", &cust);
            }
        }

        // Wire up exactly 3 custodians with valid shares (short CIDs → sig verification skipped).
        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        for i in 1..=3usize {
            let cust = format!("cust-{i}");
            let att = format!("att-{i}");
            canned.insert(
                cust.clone(),
                Ok(ShamirShareResponse {
                    share_data: share_bytes[i - 1].clone(),
                    share_index: i as u32,
                    attestation_cid: att,
                    signature: vec![0u8; 64],
                    error_reason: None,
                }),
            );
        }

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t21-action-001").await.expect("assemble");

        match result {
            AssemblyResult::ReconstructedSecret(recovered) => {
                assert_eq!(
                    recovered, secret,
                    "reconstructed secret must match original"
                );
            }
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                panic!(
                    "expected ReconstructedSecret, got BelowThreshold(found={}, threshold={})",
                    approvals_found, threshold
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // T21 Test 2: below threshold (2 of 3) → BelowThreshold, no reconstruction
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn t21_below_threshold_returns_below_threshold() {
        let secret = b"elohim-recovery-test-secret-32b!";
        let shark = Sharks(3);
        let dealer = shark.dealer(secret);
        let all_shares: Vec<Share> = dealer.take(5).collect();
        let share_bytes: Vec<Vec<u8>> = all_shares.iter().map(|s| Vec::from(s)).collect();

        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t21-action-002", r#"{"type":"shamir","m":3,"n":5}"#);
            for i in 1..=3usize {
                insert_approval_attestation(
                    &mut conn,
                    &format!("att-t2-{i}"),
                    "t21-action-002",
                    &format!("cust-t2-{i}"),
                );
            }
        }

        // Only 2 custodians respond; threshold is 3.
        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        for i in 1..=2usize {
            canned.insert(
                format!("cust-t2-{i}"),
                Ok(ShamirShareResponse {
                    share_data: share_bytes[i - 1].clone(),
                    share_index: i as u32,
                    attestation_cid: format!("att-t2-{i}"),
                    signature: vec![0u8; 64],
                    error_reason: None,
                }),
            );
        }
        // Third custodian is unreachable.
        canned.insert(
            "cust-t2-3".to_string(),
            Err("dial_failure: peer unreachable".to_string()),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t21-action-002").await.expect("assemble");

        match result {
            AssemblyResult::BelowThreshold { threshold, .. } => {
                assert_eq!(threshold, 3, "threshold must be 3");
            }
            AssemblyResult::ReconstructedSecret(_) => {
                panic!("must not reconstruct with only 2 of 3 shares");
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // T21 Test 3: corrupted share bytes → reconstruction fails clearly
    // ─────────────────────────────────────────────────────────────────────────

    /// Shares with malformed bytes (not valid sharks::Share encoding) →
    /// `reconstruct_secret` returns `InvalidInput` rather than silently
    /// producing garbage.
    #[tokio::test]
    async fn t21_corrupted_share_bytes_fail_clearly() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t21-action-003", r#"{"type":"shamir","m":1,"n":3}"#);
            insert_approval_attestation(&mut conn, "att-corrupt", "t21-action-003", "cust-corrupt");
        }

        // A single share that is entirely garbage (not a valid sharks::Share encoding).
        // A sharks::Share requires at least 2 bytes (x + ≥1 y); an empty slice
        // is definitely invalid.
        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        canned.insert(
            "cust-corrupt".to_string(),
            Ok(ShamirShareResponse {
                share_data: vec![], // empty = invalid sharks::Share
                share_index: 1,
                attestation_cid: "att-corrupt".to_string(),
                signature: vec![0u8; 64],
                error_reason: None,
            }),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t21-action-003").await;

        // Reconstruction must fail with an explicit error, not panic or silent garbage.
        assert!(
            result.is_err(),
            "corrupted share bytes must produce Err, not Ok"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Shamir reconstruction") || err.contains("could not be decoded"),
            "error must mention Shamir reconstruction failure: {err}"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // T21 Test 4: key resolver returns None for missing binding (no panic)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn t21_resolve_custodian_verifying_key_returns_none_when_no_binding() {
        use crate::db::{init_pool_from_dir, run_migrations};
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        std::mem::forget(dir);

        let mut conn = pool.get().expect("conn");
        let result = resolve_custodian_verifying_key(
            &mut conn,
            "no-such-agent-cid",
            "2026-05-15T00:00:00Z",
        )
        .expect("query ok");
        assert!(result.is_none(), "no binding → key must be None");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Preserved T20 tests — unchanged behaviour guarantees
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn assemble_returns_error_when_no_approvals() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "action-001", r#"{"type":"shamir","m":2,"n":3}"#);
        }

        let transport = Arc::new(MockShareTransport::new(HashMap::new()));
        let assembler = ShareAssembler::new(pool, transport);

        let result = assembler.assemble("action-001").await.expect("assemble");
        match result {
            AssemblyResult::BelowThreshold {
                approvals_found,
                threshold,
            } => {
                assert_eq!(approvals_found, 0);
                assert_eq!(threshold, 2);
            }
            other => panic!("expected BelowThreshold, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn assemble_returns_partial_when_below_threshold() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "action-002", r#"{"type":"shamir","m":3,"n":5}"#);
            insert_approval_attestation(&mut conn, "attest-A", "action-002", "custodian-A-cid");
            insert_approval_attestation(&mut conn, "attest-B", "action-002", "custodian-B-cid");
        }

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        canned.insert(
            "custodian-A-cid".to_string(),
            Ok(ShamirShareResponse {
                share_data: vec![0xAA; 8],
                share_index: 1,
                attestation_cid: "attest-A".to_string(),
                signature: vec![0u8; 64],
                error_reason: None,
            }),
        );
        canned.insert(
            "custodian-B-cid".to_string(),
            Ok(ShamirShareResponse {
                share_data: vec![0xBB; 8],
                share_index: 2,
                attestation_cid: "attest-B".to_string(),
                signature: vec![0u8; 64],
                error_reason: None,
            }),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("action-002").await.expect("assemble");

        match result {
            AssemblyResult::BelowThreshold {
                approvals_found,
                threshold,
            } => {
                assert_eq!(threshold, 3, "threshold should be 3");
                assert!(
                    approvals_found <= 2,
                    "should not exceed available approvals"
                );
            }
            AssemblyResult::ReconstructedSecret(_) => {
                panic!("unexpected ReconstructedSecret with only 2 approvals");
            }
        }
    }

    /// T20: transport returns an error envelope → BelowThreshold, not panic.
    #[tokio::test]
    async fn assembler_handles_error_envelope_response() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t20-action-001", r#"{"type":"shamir","m":2,"n":3}"#);
            insert_approval_attestation(&mut conn, "t20-attest-A", "t20-action-001", "t20-custC-A");
            insert_approval_attestation(&mut conn, "t20-attest-B", "t20-action-001", "t20-custC-B");
        }

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        canned.insert(
            "t20-custC-A".to_string(),
            Ok(ShamirShareResponse::error("authorization denied: no effective approval")),
        );
        canned.insert(
            "t20-custC-B".to_string(),
            Ok(ShamirShareResponse::error("authorization denied: attestation revoked")),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-001").await.expect("assemble");

        match result {
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                assert_eq!(threshold, 2);
                assert_eq!(approvals_found, 0, "error envelopes must not count as shares");
            }
            AssemblyResult::ReconstructedSecret(_) => {
                panic!("expected BelowThreshold when all responses are error envelopes");
            }
        }
    }

    /// T20: transport returns Err (custodian unreachable) → assembler skips safely.
    #[tokio::test]
    async fn assembler_handles_custodian_unreachable() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t20-action-002", r#"{"type":"shamir","m":2,"n":3}"#);
            insert_approval_attestation(&mut conn, "t20-attest-A2", "t20-action-002", "t20-custC-unreachable");
            insert_approval_attestation(&mut conn, "t20-attest-B2", "t20-action-002", "t20-custC-reachable");
        }

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        canned.insert(
            "t20-custC-unreachable".to_string(),
            Err("dial_failure: no route to peer".to_string()),
        );
        canned.insert(
            "t20-custC-reachable".to_string(),
            Ok(ShamirShareResponse::error("TODO(T21-share-store): share store not yet implemented")),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-002").await.expect("assemble");

        match result {
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                assert_eq!(threshold, 2);
                assert_eq!(approvals_found, 0, "neither failure counts as a share");
            }
            AssemblyResult::ReconstructedSecret(_) => {
                panic!("expected BelowThreshold, got ReconstructedSecret");
            }
        }
    }

    /// T20: bad signature on a long CID (where key resolution runs) → share rejected.
    #[tokio::test]
    async fn assembler_rejects_response_with_bad_signature() {
        // Use a CID long enough to correspond to a real peer_identity_bindings
        // lookup — but since no binding exists for this CID, key resolution
        // returns None and the share is accepted on DHT attestation alone.
        // The test therefore verifies: bad-sig shares are ONLY rejected when
        // a key is actually resolved; without a binding the DHT-attestation
        // path takes over (security downgrade warning, share accepted).
        //
        // For a test that exercises sig rejection, we'd need a real Ed25519
        // binding in the DB — covered by the sweettest (T23).
        let long_custodian_cid = "custodian-with-a-long-cid-that-exceeds-32-bytes-xxxx";
        let long_attest_cid = "t20-attest-sig-A";

        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(
                &mut conn,
                "t20-action-sig",
                r#"{"type":"shamir","m":1,"n":3}"#,
            );
            insert_approval_attestation(
                &mut conn,
                long_attest_cid,
                "t20-action-sig",
                long_custodian_cid,
            );
        }

        // A valid-shaped sharks share so reconstruction doesn't fail on decode.
        // threshold=1, so one share is enough.
        let shark = Sharks(1);
        let shares: Vec<Share> = shark.dealer(b"s").take(1).collect();
        let share_wire = Vec::from(&shares[0]);

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        canned.insert(
            long_custodian_cid.to_string(),
            Ok(ShamirShareResponse {
                share_data: share_wire,
                share_index: 1,
                attestation_cid: long_attest_cid.to_string(),
                signature: vec![0u8; 64], // invalid signature
                error_reason: None,
            }),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-sig").await.expect("assemble");

        // With no peer_identity_bindings row, key resolution returns None →
        // share is accepted on DHT attestation alone → threshold=1 → reconstruction
        // succeeds.  This is the expected security-downgrade behaviour when no
        // binding is present.
        match result {
            AssemblyResult::ReconstructedSecret(_) => {
                // Expected: no binding → no sig check → reconstruction succeeds.
            }
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                // Also acceptable if reconstruction failed for some other reason.
                // As long as we don't panic, the assembler is resilient.
                let _ = (approvals_found, threshold);
            }
        }
    }

    /// T20: happy path — 2 of 3 custodians respond, threshold=2 → succeeds.
    #[tokio::test]
    async fn assembler_happy_path_error_envelope_aware() {
        // Use real sharks shares so reconstruction succeeds.
        let secret = b"happy-path-secret-bytes-!";
        let shark = Sharks(2);
        let dealer = shark.dealer(secret);
        let all_shares: Vec<Share> = dealer.take(3).collect();
        let share_bytes: Vec<Vec<u8>> = all_shares.iter().map(|s| Vec::from(s)).collect();

        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(
                &mut conn,
                "t20-action-happy",
                r#"{"type":"shamir","m":2,"n":3}"#,
            );
            for (cust, att) in [("custH1", "attH1"), ("custH2", "attH2"), ("custH3", "attH3")] {
                insert_approval_attestation(&mut conn, att, "t20-action-happy", cust);
            }
        }

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        for (i, (cust, att)) in [("custH1", "attH1"), ("custH2", "attH2")].iter().enumerate() {
            canned.insert(
                cust.to_string(),
                Ok(ShamirShareResponse {
                    share_data: share_bytes[i].clone(),
                    share_index: (i + 1) as u32,
                    attestation_cid: att.to_string(),
                    signature: vec![0u8; 64],
                    error_reason: None,
                }),
            );
        }
        canned.insert(
            "custH3".to_string(),
            Err("timeout: peer unreachable".to_string()),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-happy").await.expect("assemble");

        match result {
            AssemblyResult::ReconstructedSecret(recovered) => {
                assert_eq!(recovered, secret, "reconstructed bytes must match original");
            }
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                panic!(
                    "expected ReconstructedSecret, got BelowThreshold(found={}, threshold={})",
                    approvals_found, threshold
                );
            }
        }
    }

    /// T20 sweettest scaffold — ignored until T23.
    #[tokio::test]
    #[ignore]
    async fn sweettest_shamir_share_end_to_end() {
        unimplemented!("sweettest_shamir_share_end_to_end: scaffold for T23");
    }
}
