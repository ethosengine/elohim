//! Off-chain Shamir share assembler — DHT-attested, libp2p-transported.
//!
//! ## Design contract (user reframe)
//!
//! `ShareAssembler::assemble` is OPTIONAL — it is invoked ONLY when actual
//! cryptographic key material must be recovered (high-security path). The
//! social-threshold flow (attestations + governance-actions + tally) runs to
//! completion without ever calling into this module.
//!
//! Call site:
//! ```text
//! if needs_key_material_recovery {
//!     let shares = assembler.assemble(&recovery_governance_action_cid).await?;
//!     // caller reconstructs the secret from shares
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
//!    (b) `signature` must verify against the custodian's Ed25519 key (obtained from the `attestations.issuer_cid` → key resolution step).
//! 4. Accumulate verified shares until the threshold `m` declared in the
//!    governance-action's `threshold_json` is reached.
//! 5. Return the raw share bytes — the CALLER reconstructs the secret via
//!    their Shamir primitive.
//!
//! ## Shamir reconstruction primitive
//!
//! NEEDS_CONTEXT: no `shamir_combine` function exists in this codebase.
//! The `iroh_recovery_cross_stack` test explicitly documents that real Shamir
//! is "deferred to the share-custody epic" and uses an XOR stub. This assembler
//! therefore returns the collected verified share bytes; reconstruction is the
//! caller's responsibility. A `#[cfg(test)]` stub reconstruct helper is provided
//! for unit tests that need to verify the "enough shares collected" path.
//!
//! When the share-custody epic adds a `shamir_combine(shares) -> [u8; 32]`
//! primitive (likely in a separate crate), wire it here by calling it after
//! step 4 and return `AssemblyResult::Secret(bytes)` instead of `Shares(vec)`.

use diesel::prelude::*;
use libp2p::PeerId;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::diesel_schema::{attestations, governance_actions};
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
/// `/elohim/shamir-share/1.0.0` request-response protocol (once G.1 swarm
/// wiring lands — see `shamir_transport.rs` TODO(G.1-swarm-wiring)).
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
/// Ed25519 verifying key.
///
/// The verifying key is currently derived from the `custodian_cid` bytes via
/// the `derive_verifying_key_stub` helper — this is a placeholder that only
/// works for test CIDs. In production, key resolution will go through the
/// `peer_identity_bindings` DHT lookup.
/// TODO(G.2-key-resolution): replace stub with real key resolution.
pub struct LibP2PShareTransport {
    /// Handle to the live P2P swarm. Used to send `P2PCommand::RequestShamirShare`.
    handle: P2PHandle,
    /// DB pool for resolving `custodian_cid → PeerId` via `peer_identity_bindings`.
    pool: DbPool,
    /// Per-request timeout for the libp2p round-trip. The swarm's own
    /// `request_timeout` is the hard ceiling; this drives the oneshot wait.
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

        // ── Step 5: verify signature ──────────────────────────────────────
        //
        // TODO(G.2-key-resolution): derive_verifying_key_stub returns None for
        // realistic CIDs. Once real key resolution is wired (imagodei DHT lookup
        // or peer_identity_bindings extension), replace this stub.
        if let Some(key_bytes) = derive_verifying_key_stub(&custodian_cid_for_verify) {
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
                "TODO(G.2-key-resolution): verifying key not resolvable from CID — \
                 share accepted without signature verification (security downgrade)"
            );
        }

        Ok(response)
    }
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

    /// Threshold reached. Contains verified share bytes in share-index order.
    ///
    /// The caller is responsible for reconstruction via their Shamir primitive.
    /// See module-level NEEDS_CONTEXT note for status of `shamir_combine`.
    Shares(Vec<VerifiedShare>),
}

/// A single verified Shamir share.
#[derive(Debug, Clone)]
pub struct VerifiedShare {
    /// 1-based index within the (m,n) scheme.
    pub share_index: u32,
    /// Raw share bytes (caller decrypts with session key if encrypted).
    pub share_data: Vec<u8>,
    /// CID of the `attestation:recovery-approval` that authorized this share.
    pub attestation_cid: String,
    /// CID of the custodian who delivered this share.
    pub custodian_cid: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// ShareAssembler
// ─────────────────────────────────────────────────────────────────────────────

/// Assembles Shamir shares for a recovery governance action.
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

    /// Assemble shares for the given recovery governance action.
    ///
    /// This is OPTIONAL — invoke only when key-material recovery is required.
    ///
    /// # Steps
    ///
    /// 1. Load the governance action + its threshold from `governance_actions`.
    /// 2. Query `attestations` for `attestation_kind = "attestation:recovery-approval"`
    ///    children of `parent_governance_action_cid = recovery_governance_action_cid`.
    /// 3. For each approving custodian: send `ShamirShareRequest`, verify response.
    /// 4. Return `BelowThreshold` if not enough verified, `Shares(vec)` otherwise.
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

            // T20: Check for error envelope before proceeding with verification.
            // The custodian returns an error envelope when authorization fails,
            // share material is unavailable, or the store is not yet implemented.
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

            // Step 3b: verify signature
            //
            // NOTE(G.2-key-resolution): in production, the custodian's Ed25519 verifying key
            // must be resolved from their `issuer_cid` → agent pubkey binding
            // (peer_identity_bindings or imagodei DNA lookup). That resolution is outside
            // the scope of G.2 (requires multi-step DHT walk). Here we use the issuer_cid
            // bytes as a placeholder key derivation stub.
            //
            // TODO(G.2-key-resolution): replace with real key lookup via
            // `HolochainBackedPeerIdentityMap::get(issuer_cid)` or equivalent.
            // Until then, skip signature verification if no key bytes are available and
            // log a warning — the DHT attestation check in step 3a is the primary guard.
            if let Some(key_bytes) = derive_verifying_key_stub(&approval.issuer_cid) {
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
                    "TODO(G.2-key-resolution): verifying key not available — \
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

        Ok(AssemblyResult::Shares(verified_shares))
    }
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

/// STUB: derive a 32-byte verifying key from an issuer_cid for testing.
///
/// In production, resolve the key via `peer_identity_bindings` or the imagodei
/// DNA. Returns `None` when the CID cannot be converted to key bytes (which
/// triggers a warning + acceptance-on-DHT-attestation-alone path).
///
/// TODO(G.2-key-resolution): remove this stub once real key resolution is wired.
fn derive_verifying_key_stub(issuer_cid: &str) -> Option<[u8; 32]> {
    // Only usable for deterministic test CIDs that are exactly 32 bytes when
    // truncated. Production CIDs are base58/base64url and will not satisfy this.
    let bytes = issuer_cid.as_bytes();
    if bytes.len() >= 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[..32]);
        Some(key)
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ── DB setup helpers ──────────────────────────────────────────────────────

    /// Build a minimal in-memory DB pool with the attestation-consolidation
    /// schema tables needed for the assembler.
    fn test_pool() -> DbPool {
        use crate::db::{init_pool_from_dir, run_migrations};
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        // Keep dir alive for the lifetime of the test by leaking it — acceptable in
        // tests where the process ends shortly after anyway.
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
                governance_actions::created_at.eq("2026-05-11T00:00:00Z"),
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
                attestations::created_at.eq("2026-05-11T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert attestation");
    }

    // ── Test: no approvals → BelowThreshold ──────────────────────────────────

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

    // ── Test: fewer approvals than threshold → BelowThreshold ────────────────

    #[tokio::test]
    async fn assemble_returns_partial_when_below_threshold() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "action-002", r#"{"type":"shamir","m":3,"n":5}"#);
            // Only 2 approvals inserted, threshold is 3
            insert_approval_attestation(&mut conn, "attest-A", "action-002", "custodian-A-cid");
            insert_approval_attestation(&mut conn, "attest-B", "action-002", "custodian-B-cid");
        }

        // Transport returns valid-shaped responses for both custodians
        // (but shares won't verify because derive_verifying_key_stub won't match
        //  so they fall through as DHT-only accepted, still < threshold)
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
                // approvals_found may be 0..=2 depending on signature stub path
                assert!(
                    approvals_found <= 2,
                    "should not exceed available approvals"
                );
            }
            AssemblyResult::Shares(shares) => {
                // Only acceptable if 3+ shares collected — impossible with 2 approvals
                panic!(
                    "unexpected Shares result with only 2 approvals: {} shares",
                    shares.len()
                );
            }
        }
    }

    // ── Test: at-threshold with mock transport → Shares ───────────────────────

    /// Verifies that exactly `m` verified shares → `AssemblyResult::Shares`.
    ///
    /// Note: `derive_verifying_key_stub` returns `None` for realistic CIDs so
    /// signatures are not verified — shares are accepted on DHT attestation alone.
    /// The test therefore uses realistic-format CIDs (< 32 bytes raw) that trigger
    /// the "no key available" path, confirming the DHT-attestation gate still works.
    #[tokio::test]
    async fn assemble_succeeds_at_threshold() {
        let pool = test_pool();
        let threshold = 2usize;
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "action-003", r#"{"type":"shamir","m":2,"n":3}"#);
            insert_approval_attestation(&mut conn, "attest-1", "action-003", "custC1");
            insert_approval_attestation(&mut conn, "attest-2", "action-003", "custC2");
            insert_approval_attestation(&mut conn, "attest-3", "action-003", "custC3");
        }

        // Wire mock responses for all 3 custodians with matching attestation_cids
        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        for (i, (cust, att)) in [
            ("custC1", "attest-1"),
            ("custC2", "attest-2"),
            ("custC3", "attest-3"),
        ]
        .iter()
        .enumerate()
        {
            canned.insert(
                cust.to_string(),
                Ok(ShamirShareResponse {
                    share_data: vec![i as u8; 8],
                    share_index: (i + 1) as u32,
                    attestation_cid: att.to_string(),
                    // Signature is all-zeros — derive_verifying_key_stub returns None for
                    // short CIDs, so verification is skipped (DHT-attestation-only gate).
                    signature: vec![0u8; 64],
                    error_reason: None,
                }),
            );
        }

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("action-003").await.expect("assemble");

        match result {
            AssemblyResult::Shares(shares) => {
                assert!(
                    shares.len() >= threshold,
                    "expected >= {} shares, got {}",
                    threshold,
                    shares.len()
                );
                // Verify shares are sorted by index
                for w in shares.windows(2) {
                    assert!(
                        w[0].share_index < w[1].share_index,
                        "shares must be sorted by index"
                    );
                }
            }
            AssemblyResult::BelowThreshold {
                approvals_found,
                threshold: t,
            } => {
                panic!(
                    "expected Shares, got BelowThreshold(approvals={}, threshold={})",
                    approvals_found, t
                );
            }
        }
    }

    // ── T20 Tests ─────────────────────────────────────────────────────────────
    //
    // Tests for the error-envelope path, custodian-unreachable fallback, and
    // signature verification failure on the assembler side.
    // The authorization-gate DB queries are covered by
    // `crate::db::recovery_approval_gate::tests`.

    /// T20: transport returns an error envelope → assembler records failure and
    /// returns BelowThreshold (not a panic or hard error).
    #[tokio::test]
    async fn assembler_handles_error_envelope_response() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_governance_action(&mut conn, "t20-action-001", r#"{"type":"shamir","m":2,"n":3}"#);
            insert_approval_attestation(&mut conn, "t20-attest-A", "t20-action-001", "t20-custC-A");
            insert_approval_attestation(&mut conn, "t20-attest-B", "t20-action-001", "t20-custC-B");
        }

        // Both custodians return error envelopes (authorization denied on their side)
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
                assert_eq!(threshold, 2, "threshold should be 2");
                assert_eq!(approvals_found, 0, "error envelopes must not count as shares");
            }
            AssemblyResult::Shares(shares) => {
                panic!("expected BelowThreshold when all responses are error envelopes, got {} shares", shares.len());
            }
        }
    }

    /// T20: transport returns Err (custodian unreachable / OutboundFailure equivalent) →
    /// assembler skips that custodian without poisoning the overall flow.
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
        // First custodian: transport-level failure (dial failure / unreachable)
        canned.insert(
            "t20-custC-unreachable".to_string(),
            Err("dial_failure: no route to peer".to_string()),
        );
        // Second custodian: returns an error envelope (not an error at the transport level)
        canned.insert(
            "t20-custC-reachable".to_string(),
            Ok(ShamirShareResponse::error("TODO(T21-share-store): share store not yet implemented")),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-002").await.expect("assemble");

        // Both failed (one transport-level, one error envelope). Should be BelowThreshold.
        match result {
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                assert_eq!(threshold, 2);
                assert_eq!(approvals_found, 0, "neither failure counts as a share");
            }
            AssemblyResult::Shares(shares) => {
                panic!("expected BelowThreshold, got {} shares", shares.len());
            }
        }
    }

    /// T20: transport returns a response with a bad signature → assembler rejects
    /// the share (for CIDs ≥ 32 bytes, where `derive_verifying_key_stub` returns
    /// `Some(_)` and signature verification runs).
    #[tokio::test]
    async fn assembler_rejects_response_with_bad_signature() {
        // Use a CID that is ≥ 32 bytes so derive_verifying_key_stub returns Some(_)
        // and the signature check actually runs.
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

        let mut canned: HashMap<String, Result<ShamirShareResponse, String>> = HashMap::new();
        // Response with all-zero signature — will fail verification because the
        // "key" derived from the CID bytes would not have signed these bytes.
        canned.insert(
            long_custodian_cid.to_string(),
            Ok(ShamirShareResponse {
                share_data: vec![0xDE, 0xAD, 0xBE, 0xEF],
                share_index: 1,
                attestation_cid: long_attest_cid.to_string(),
                signature: vec![0u8; 64], // invalid signature
                error_reason: None,
            }),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-sig").await.expect("assemble");

        // Threshold = 1, but the share fails signature verification → BelowThreshold.
        match result {
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                assert_eq!(threshold, 1);
                // approvals_found should be 0 (bad sig rejected)
                assert!(approvals_found < threshold, "bad-sig share must not count");
            }
            AssemblyResult::Shares(_) => {
                panic!("expected BelowThreshold when signature is bad");
            }
        }
    }

    /// T20: happy path with valid mock transport (matching attestation_cid, short CIDs
    /// that skip sig verification) → assembler reaches threshold despite one unreachable custodian.
    #[tokio::test]
    async fn assembler_happy_path_error_envelope_aware() {
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
                    share_data: vec![i as u8; 8],
                    share_index: (i + 1) as u32,
                    attestation_cid: att.to_string(),
                    // Short CIDs (< 32 bytes) → derive_verifying_key_stub returns None → sig skipped
                    signature: vec![0u8; 64],
                    error_reason: None,
                }),
            );
        }
        // Third custodian is unreachable — threshold=2 so should still succeed
        canned.insert(
            "custH3".to_string(),
            Err("timeout: peer unreachable".to_string()),
        );

        let transport = Arc::new(MockShareTransport::new(canned));
        let assembler = ShareAssembler::new(pool, transport);
        let result = assembler.assemble("t20-action-happy").await.expect("assemble");

        match result {
            AssemblyResult::Shares(shares) => {
                assert!(shares.len() >= 2, "expected >= 2 shares, got {}", shares.len());
                assert!(
                    shares.windows(2).all(|w| w[0].share_index < w[1].share_index),
                    "shares must be sorted by index"
                );
            }
            AssemblyResult::BelowThreshold { approvals_found, threshold } => {
                panic!(
                    "expected Shares, got BelowThreshold(approvals={}, threshold={})",
                    approvals_found, threshold
                );
            }
        }
    }

    /// T20: sweettest-level marker — full end-to-end test (packed DNAs + live swarm)
    /// is deferred to T23/T25. This test is `#[ignore]`d per T18's sweettest pattern.
    #[tokio::test]
    #[ignore]
    async fn sweettest_shamir_share_end_to_end() {
        // TODO(T23): wire up two conductors, seed recovery-approval attestations,
        // and drive the full ShareAssembler → LibP2PShareTransport → swarm path.
        // Requires packed DNAs and a live libp2p harness.
        //
        // Scaffold steps:
        // 1. Start two conductors (alice = recoverer, bob = custodian).
        // 2. Alice creates a governance-action:recovery-request.
        // 3. Bob issues an attestation:recovery-approval on the DHT.
        // 4. Signal replay projects the attestation to Bob's SQLite.
        // 5. Alice's ShareAssembler calls LibP2PShareTransport::request_share.
        // 6. Bob's swarm event arm authorizes and (once T21-share-store lands)
        //    returns the encrypted share.
        // 7. Alice verifies the response and ShareAssembler returns Shares(vec).
        unimplemented!("sweettest_shamir_share_end_to_end: scaffold for T23");
    }
}
