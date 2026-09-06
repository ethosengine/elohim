//! Phase 11 — transport-neutral EPR-atom service.
//!
//! Extracted from `p2p::P2PNode::handle_epr_atom_request` so both the
//! libp2p request-response handler and the iroh-side `EprAtomBackend`
//! can route fetch / announce / fetch-batch / integrity-notify through
//! the same code path.
//!
//! Per [`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR-atom plane is dual-stack permanent. The receive logic
//! (decode CBOR envelope, dedup on CID, ingest via
//! `services::epr_service::ingest`, reach-gate served atoms by caller
//! identity) is identical on both transports — only the way the
//! `CallerIdentity` is resolved differs (libp2p `PeerId` →
//! `PeerIdentityMap` lookup; iroh `NodeId` will go through the cross-
//! stack peer-map once Phase 12 graduates `peer_transport_manifest`).
//! Until then, iroh-mode callers default to
//! [`CallerIdentity::Anonymous`] — the slow-path reach gate still
//! correctly serves Commons/Public atoms; tighter reach tiers fall
//! through to NotFound (leak-free, matches libp2p semantics for
//! unauthenticated callers).

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::p2p::dedup::DedupLru;
use crate::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse, MAX_BATCH_CIDS};
use crate::p2p::identity_map::CallerIdentity;
use crate::p2p::reach_gate_allows;

/// Holds the dependencies needed to answer an EPR-atom request without
/// any transport-specific state. Cheap to clone.
#[derive(Clone)]
pub struct EprAtomService {
    db_pool: Option<DbPool>,
    dedup: Arc<DedupLru>,
    /// This node's OWN content-cell DNA hash, when it has one.
    ///
    /// Accountable-correction contract §1/§4: a `feedback-signal` notification
    /// naming a DIFFERENT origin DNA is rejected — carrying and fetching from
    /// another space is explicitly not slice 1. `None` means the node cannot
    /// make that judgement, so it refuses the notification rather than guessing.
    origin_dna_hash: Option<String>,
}

impl std::fmt::Debug for EprAtomService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EprAtomService")
            .field("has_db_pool", &self.db_pool.is_some())
            .finish_non_exhaustive()
    }
}

impl EprAtomService {
    pub fn new(db_pool: Option<DbPool>, dedup: Arc<DedupLru>) -> Self {
        Self {
            db_pool,
            dedup,
            origin_dna_hash: None,
        }
    }

    /// Bind this node's own content-cell DNA hash (contract §1).
    ///
    /// Additive by design: every existing `new(..)` call site keeps compiling
    /// and keeps the honest `None`, which REFUSES a `feedback-signal`
    /// notification rather than accepting one it cannot scope.
    pub fn with_origin_dna_hash(mut self, dna_hash: impl Into<String>) -> Self {
        self.origin_dna_hash = Some(dna_hash.into());
        self
    }

    /// Dispatch an [`EprAtomRequest`].
    ///
    /// `peer_label` is purely for tracing — pass `peer.to_string()` from
    /// libp2p, the iroh `NodeId` from iroh; the service does not derive
    /// caller identity from it (that already happened upstream and is
    /// passed in via `caller`).
    pub fn handle(
        &self,
        peer_label: &str,
        caller: CallerIdentity,
        request: EprAtomRequest,
    ) -> EprAtomResponse {
        match request {
            EprAtomRequest::Fetch { cid } => self.handle_fetch(&cid, &caller),
            EprAtomRequest::Announce { envelope_bytes } => {
                self.handle_announce(peer_label, envelope_bytes)
            }
            EprAtomRequest::FetchBatch { cids } => self.handle_fetch_batch(cids, &caller),
            EprAtomRequest::IntegrityNotify {
                kind,
                payload_bytes,
            } => self.handle_integrity_notify(peer_label, kind, payload_bytes),
        }
    }

    fn handle_fetch(&self, cid: &str, caller: &CallerIdentity) -> EprAtomResponse {
        let Some(pool) = self.db_pool.as_ref() else {
            warn!(cid = %cid, "EPR atom fetch: db pool unavailable");
            return EprAtomResponse::Error {
                message: "storage unavailable".to_string(),
            };
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(cid = %cid, error = %e, "EPR atom fetch: db pool exhausted");
                return EprAtomResponse::Error {
                    message: "storage busy".to_string(),
                };
            }
        };

        match crate::services::epr_service::fetch_wire_bytes_by_cid(&mut conn, cid) {
            Ok(Some(fetched)) => {
                if reach_gate_allows(&fetched.reach, caller, Some(&fetched.signer_cid)) {
                    debug!(
                        cid = %cid,
                        reach = %fetched.reach,
                        bytes = fetched.wire_bytes.len(),
                        "EPR atom fetch served"
                    );
                    EprAtomResponse::Atom {
                        envelope_bytes: fetched.wire_bytes,
                    }
                } else {
                    // Leak-free: caller cannot distinguish missing from unauthorized.
                    debug!(
                        cid = %cid,
                        reach = %fetched.reach,
                        "EPR atom fetch denied by reach gate"
                    );
                    EprAtomResponse::NotFound
                }
            }
            Ok(None) => EprAtomResponse::NotFound,
            Err(e) => {
                warn!(cid = %cid, error = ?e, "EPR atom fetch error");
                EprAtomResponse::Error {
                    message: "internal error".to_string(),
                }
            }
        }
    }

    fn handle_announce(&self, peer_label: &str, envelope_bytes: Vec<u8>) -> EprAtomResponse {
        // D.6: decode the envelope first (CBOR only — no DB I/O) to
        // extract the CID string cheaply, then dedup-check before
        // committing the pool connection or running ingest.
        let epr: elohim_epr::Epr = match ciborium::de::from_reader(envelope_bytes.as_slice()) {
            Ok(e) => e,
            Err(err) => {
                debug!(
                    bytes = envelope_bytes.len(),
                    reason = %err,
                    "EPR atom announce: cbor decode failed"
                );
                return EprAtomResponse::Announced {
                    accepted: false,
                    reason: Some(format!("cbor decode: {err}")),
                };
            }
        };
        let cid_str = epr.envelope.cid.to_string();

        // D.6 wire point A: dedup on CID before DB connection + ingest.
        if !self.dedup.insert(&cid_str) {
            debug!(
                target: "elohim_storage::dedup",
                from = %peer_label,
                cid = %cid_str,
                "duplicate Announce — dropped (no-op)"
            );
            return EprAtomResponse::Announced {
                accepted: false,
                reason: Some("duplicate (already seen)".to_string()),
            };
        }

        let Some(pool) = self.db_pool.as_ref() else {
            warn!(
                bytes = envelope_bytes.len(),
                "EPR atom announce: db pool unavailable"
            );
            return EprAtomResponse::Announced {
                accepted: false,
                reason: Some("storage unavailable".to_string()),
            };
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "EPR atom announce: db pool exhausted");
                return EprAtomResponse::Announced {
                    accepted: false,
                    reason: Some("storage busy".to_string()),
                };
            }
        };

        match crate::services::epr_service::ingest(&mut conn, epr) {
            Ok(ingested) => {
                debug!(
                    cid = %ingested.cid,
                    bytes = envelope_bytes.len(),
                    "EPR atom announce accepted"
                );
                // record_predecessor wiring lives in p2p/mod.rs::handle_epr_atom_request
                // (W2A landed; see services::back_prop::record_predecessor for the recorder).
                // EprAtomService stays transport-neutral and does not call record_predecessor —
                // the libp2p sender PeerId is only available at the protocol boundary.
                EprAtomResponse::Announced {
                    accepted: true,
                    reason: None,
                }
            }
            Err(crate::error::StorageError::InvalidInput(msg)) => {
                debug!(
                    bytes = envelope_bytes.len(),
                    reason = %msg,
                    "EPR atom announce rejected (invalid)"
                );
                EprAtomResponse::Announced {
                    accepted: false,
                    reason: Some(format!("verification failed: {msg}")),
                }
            }
            Err(e) => {
                warn!(
                    bytes = envelope_bytes.len(),
                    error = ?e,
                    "EPR atom announce: persistence error"
                );
                EprAtomResponse::Announced {
                    accepted: false,
                    reason: Some("persistence error".to_string()),
                }
            }
        }
    }

    fn handle_fetch_batch(&self, cids: Vec<String>, caller: &CallerIdentity) -> EprAtomResponse {
        if cids.len() > MAX_BATCH_CIDS {
            debug!(
                count = cids.len(),
                max = MAX_BATCH_CIDS,
                "EPR atom fetch batch rejected — oversized"
            );
            return EprAtomResponse::Error {
                message: format!(
                    "batch too large: {} cids (max {})",
                    cids.len(),
                    MAX_BATCH_CIDS
                ),
            };
        }

        let Some(pool) = self.db_pool.as_ref() else {
            warn!(
                count = cids.len(),
                "EPR atom fetch batch: db pool unavailable"
            );
            return EprAtomResponse::Error {
                message: "storage unavailable".to_string(),
            };
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(count = cids.len(), error = %e, "EPR atom fetch batch: db pool exhausted");
                return EprAtomResponse::Error {
                    message: "storage busy".to_string(),
                };
            }
        };

        let mut atoms: Vec<Option<Vec<u8>>> = Vec::with_capacity(cids.len());
        let mut served = 0usize;
        for cid in &cids {
            let slot = match crate::services::epr_service::fetch_wire_bytes_by_cid(&mut conn, cid) {
                Ok(Some(fetched))
                    if reach_gate_allows(&fetched.reach, caller, Some(&fetched.signer_cid)) =>
                {
                    served += 1;
                    Some(fetched.wire_bytes)
                }
                Ok(_) => None,
                Err(e) => {
                    warn!(cid = %cid, error = ?e, "EPR atom fetch batch: row error");
                    None
                }
            };
            atoms.push(slot);
        }
        debug!(
            total = cids.len(),
            served = served,
            "EPR atom fetch batch completed"
        );
        EprAtomResponse::AtomBatch { atoms }
    }

    fn handle_integrity_notify(
        &self,
        peer_label: &str,
        kind: String,
        payload_bytes: Vec<u8>,
    ) -> EprAtomResponse {
        match kind.as_str() {
            "KeyRevocation" => {
                match crate::p2p::recovery_revocation::RecoveryRevocationMessage::from_bytes(
                    &payload_bytes,
                ) {
                    Ok(msg) => {
                        // D.6 wire point C: dedup on synthetic
                        // KeyRevocation:<id> key. Same revocation arriving
                        // via direct-notify + gossipsub will not double-
                        // process after the first delivery.
                        let dedup_key = format!("KeyRevocation:{}", msg.revocation_id);
                        if !self.dedup.insert(&dedup_key) {
                            debug!(
                                target: "elohim_storage::dedup",
                                from = %peer_label,
                                revocation_id = %msg.revocation_id,
                                "duplicate KeyRevocation direct-notify — dropped"
                            );
                            return EprAtomResponse::IntegrityAck {
                                received: true,
                                reason: Some("duplicate".to_string()),
                            };
                        }
                        info!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            revocation_id = %msg.revocation_id,
                            human_id = %msg.human_id,
                            status = %msg.status,
                            "D.5: Received KeyRevocation via direct-notify"
                        );
                        EprAtomResponse::IntegrityAck {
                            received: true,
                            reason: None,
                        }
                    }
                    Err(e) => {
                        warn!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            error = %e,
                            "D.5: Failed to decode RecoveryRevocationMessage from direct-notify"
                        );
                        EprAtomResponse::IntegrityAck {
                            received: false,
                            reason: Some(format!("decode failed: {e}")),
                        }
                    }
                }
            }
            "KeyRotation" => {
                match crate::p2p::recovery_rotation::RecoveryRotationMessage::from_bytes(
                    &payload_bytes,
                ) {
                    Ok(msg) => {
                        // W2B: dedup on synthetic KeyRotation:<id> key. Same rotation
                        // arriving via direct-notify + signal stream will not double-
                        // process after the first delivery.
                        let dedup_key = format!("KeyRotation:{}", msg.rotation_id);
                        if !self.dedup.insert(&dedup_key) {
                            debug!(
                                target: "elohim_storage::dedup",
                                from = %peer_label,
                                rotation_id = %msg.rotation_id,
                                "duplicate KeyRotation direct-notify — dropped"
                            );
                            return EprAtomResponse::IntegrityAck {
                                received: true,
                                reason: Some("duplicate".to_string()),
                            };
                        }
                        info!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            rotation_id = %msg.rotation_id,
                            human_agent_pubkey = %msg.human_agent_pubkey,
                            rotated_at = %msg.rotated_at,
                            "W2B: Received KeyRotation via direct-notify"
                        );
                        // Note: the canonical write to key_rotations table happens via
                        // the local conductor's RecoveryV2Signal::KeyRotationCommitted
                        // handler (signals.rs:1013). Direct-notify is delivery-
                        // optimistic — it does not write to the projection here, to
                        // avoid divergence with the signal-stream-driven canonical path.
                        EprAtomResponse::IntegrityAck {
                            received: true,
                            reason: None,
                        }
                    }
                    Err(e) => {
                        warn!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            error = %e,
                            "W2B: Failed to decode RecoveryRotationMessage from direct-notify"
                        );
                        EprAtomResponse::IntegrityAck {
                            received: false,
                            reason: Some(format!("decode failed: {e}")),
                        }
                    }
                }
            }
            "RevocationAttestation" => {
                match crate::p2p::revocation_attestation_message::RevocationAttestationMessage::from_bytes(
                    &payload_bytes,
                ) {
                    Ok(msg) => {
                        // Dedup on synthetic key. Same attestation arriving via direct-
                        // notify + signal stream will not double-process after the first
                        // delivery. The dedup key includes the action_hash so two
                        // attestations on the same revocation (a 'request' and a 'vote',
                        // or two different stewards' votes) do not collide.
                        let dedup_key = format!(
                            "RevocationAttestation:{}:{}",
                            msg.revocation_id, msg.action_hash
                        );
                        if !self.dedup.insert(&dedup_key) {
                            debug!(
                                target: "elohim_storage::dedup",
                                from = %peer_label,
                                revocation_id = %msg.revocation_id,
                                action_hash = %msg.action_hash,
                                "duplicate RevocationAttestation direct-notify — dropped"
                            );
                            return EprAtomResponse::IntegrityAck {
                                received: true,
                                reason: Some("duplicate".to_string()),
                            };
                        }
                        info!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            revocation_id = %msg.revocation_id,
                            action_hash = %msg.action_hash,
                            attestation_kind = %msg.attestation_kind,
                            steward_id = %msg.steward_id,
                            threshold_reached = %msg.threshold_reached,
                            "W2: Received RevocationAttestation via direct-notify"
                        );
                        // Per D3 duality: the canonical write to the projection happens
                        // via the AttestationProjector (for attestation:revocation-vote
                        // children landing in `attestations`) and via the
                        // RecoveryFlowProjector (for governance-action:key-revocation
                        // openers / key-revocation:effective signals landing in
                        // `key_revocations`). Direct-notify is delivery-optimistic — it
                        // does not write to projections here, to avoid divergence with
                        // the canonical signal-stream-driven path.
                        EprAtomResponse::IntegrityAck {
                            received: true,
                            reason: None,
                        }
                    }
                    Err(e) => {
                        warn!(
                            target: "elohim_storage::recovery",
                            from = %peer_label,
                            error = %e,
                            "W2: Failed to decode RevocationAttestationMessage from direct-notify"
                        );
                        EprAtomResponse::IntegrityAck {
                            received: false,
                            reason: Some(format!("decode failed: {e}")),
                        }
                    }
                }
            }
            // Accountable correction (contract §4). A notification is a
            // POINTER, never evidence: this branch decodes it, refuses a
            // foreign origin DNA, and durably enqueues the referenced act for
            // the projector to FETCH AND VERIFY on its next tick.
            //
            // The fetch deliberately does NOT happen here. Per-notification
            // conductor work must be bounded before the uncancellable
            // `call_zome`, and this handler is a synchronous request-response
            // arm with no conductor client — doing the verification inline
            // would put an unbounded, uncancellable DHT read on the notify
            // path. Enqueue-then-project keeps the notification cheap and the
            // single application path single.
            "feedback-signal" => self.handle_feedback_signal_notify(peer_label, &payload_bytes),
            other_kind => {
                warn!(
                    target: "elohim_storage::integrity",
                    from = %peer_label,
                    kind = %other_kind,
                    "D.5: Received IntegrityNotify with unhandled kind — Stage 2/3 will add handlers"
                );
                EprAtomResponse::IntegrityAck {
                    received: false,
                    reason: Some(format!("unhandled integrity kind: {other_kind}")),
                }
            }
        }
    }

    /// Receive a `feedback-signal` direct notification (contract §4).
    ///
    /// Three refusals, all of them honest acks rather than silent drops:
    ///   - the payload does not decode as the semantic `FeedbackSignal`;
    ///   - it carries no act reference (a pre-slice-1 peer): the semantic claim
    ///     alone is not something this node will project, and discovery will
    ///     find the act anyway;
    ///   - the reference names a DIFFERENT origin DNA hash than this node's own
    ///     content cell.
    ///
    /// On acceptance the referenced CORRECTION ACTION is added to the durable
    /// subscription set, so the projector enumerates it, fetches the signed
    /// record, and verifies §1 for itself.
    fn handle_feedback_signal_notify(
        &self,
        peer_label: &str,
        payload_bytes: &[u8],
    ) -> EprAtomResponse {
        use crate::db::feedback_subscriptions as sub_db;

        let signal: crate::p2p::feedback_signal::FeedbackSignal =
            match rmp_serde::from_slice(payload_bytes) {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        target: "elohim_storage::feedback",
                        from = %peer_label,
                        error = %e,
                        "feedback-signal notify: payload did not decode"
                    );
                    return EprAtomResponse::IntegrityAck {
                        received: false,
                        reason: Some(format!("decode failed: {e}")),
                    };
                }
            };

        let Some(act_ref) = signal.act_ref.as_ref() else {
            debug!(
                target: "elohim_storage::feedback",
                from = %peer_label,
                "feedback-signal notify carries no act reference (pre-slice-1 peer) — the \
                 semantic claim alone is not projected; discovery still finds the act"
            );
            return EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some("no act reference — semantic claim alone is not evidence".to_string()),
            };
        };

        let Some(local_dna) = self.origin_dna_hash.as_deref() else {
            warn!(
                target: "elohim_storage::feedback",
                from = %peer_label,
                "feedback-signal notify refused: this node has no bound content-cell DNA hash, \
                 so it cannot scope the reference"
            );
            return EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some("no local content DNA hash bound".to_string()),
            };
        };
        if act_ref.origin_dna_hash != local_dna {
            warn!(
                target: "elohim_storage::feedback",
                from = %peer_label,
                foreign = %act_ref.origin_dna_hash,
                local = %local_dna,
                "feedback-signal notify refused: foreign origin DNA hash"
            );
            return EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some("foreign origin DNA hash".to_string()),
            };
        }

        // Dedup on the SIGNED ACT, not on the semantic payload: two authors'
        // identical corrections share an entry hash and are two acts.
        let dedup_key = format!(
            "feedback-signal:{}:{}",
            act_ref.origin_dna_hash, act_ref.action_hash
        );
        if !self.dedup.insert(&dedup_key) {
            debug!(
                target: "elohim_storage::dedup",
                from = %peer_label,
                action = %act_ref.action_hash,
                "duplicate feedback-signal notify — dropped"
            );
            return EprAtomResponse::IntegrityAck {
                received: true,
                reason: Some("duplicate".to_string()),
            };
        }

        let Some(pool) = self.db_pool.as_ref() else {
            return EprAtomResponse::Error {
                message: "storage unavailable".to_string(),
            };
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "feedback-signal notify: db pool exhausted");
                return EprAtomResponse::Error {
                    message: "storage busy".to_string(),
                };
            }
        };
        let now = chrono::Utc::now().to_rfc3339();
        // Subscribe to the ACT ITSELF and to its routing key. The act reference
        // is what the projector fetches; the routing key is the content target
        // whose OTHER acts (including this one's acceptance) hang off it.
        for (kind, key) in [
            (sub_db::KIND_CORRECTION_ACTION, act_ref.action_hash.as_str()),
            (sub_db::KIND_CONTENT_TARGET, act_ref.routing_key.as_str()),
        ] {
            if let Err(e) = sub_db::add_member(
                &mut conn,
                kind,
                key,
                &act_ref.origin_dna_hash,
                sub_db::SOURCE_NOTIFIED,
                &now,
            ) {
                warn!(
                    target: "elohim_storage::feedback",
                    error = %e,
                    key = %key,
                    "feedback-signal notify: could not persist the subscription"
                );
                return EprAtomResponse::Error {
                    message: "could not persist subscription".to_string(),
                };
            }
        }

        info!(
            target: "elohim_storage::feedback",
            from = %peer_label,
            action = %act_ref.action_hash,
            kind = ?signal.signal_kind,
            "feedback-signal notify accepted — the act is enqueued for fetch-and-verify"
        );
        EprAtomResponse::IntegrityAck {
            received: true,
            reason: None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_service() -> EprAtomService {
        EprAtomService::new(None, Arc::new(DedupLru::new()))
    }

    #[test]
    fn fetch_with_no_db_pool_returns_error() {
        let svc = fresh_service();
        match svc.handle(
            "peer-x",
            CallerIdentity::Anonymous,
            EprAtomRequest::Fetch {
                cid: "bafy-missing".into(),
            },
        ) {
            EprAtomResponse::Error { message } => {
                assert!(message.contains("storage unavailable"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn fetch_batch_oversized_is_rejected() {
        let svc = fresh_service();
        let cids: Vec<String> = (0..MAX_BATCH_CIDS + 1)
            .map(|i| format!("bafy-{i}"))
            .collect();
        match svc.handle(
            "peer-x",
            CallerIdentity::Anonymous,
            EprAtomRequest::FetchBatch { cids },
        ) {
            EprAtomResponse::Error { message } => {
                assert!(message.contains("batch too large"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn integrity_notify_keyrotation_acks_received_true() {
        let svc = fresh_service();
        let msg = crate::p2p::recovery_rotation::RecoveryRotationMessage {
            rotation_id: "uhCAkRecoveryRequest-test1".into(),
            human_agent_pubkey: "uhCAkHumanAgentPubkey".into(),
            new_agent_pubkey: "uhCAkNewKey".into(),
            superseded_agent_pubkey: "uhCAkOldKey".into(),
            recovery_request_hash: "uhCAkRecoveryRequest-test1".into(),
            rotated_at: "2026-05-11T12:00:00Z".into(),
            sender_peer_id: "12D3KooWtest".into(),
            sent_at: "2026-05-11T12:00:01Z".into(),
        };
        let bytes = msg.to_bytes().expect("encode");

        let response = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "KeyRotation".to_string(),
                payload_bytes: bytes,
            },
        );

        match response {
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: None,
            } => {}
            other => panic!(
                "expected IntegrityAck {{ received: true }}, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn integrity_notify_keyrotation_dedup_returns_duplicate_reason() {
        let svc = fresh_service();
        let msg = crate::p2p::recovery_rotation::RecoveryRotationMessage {
            rotation_id: "uhCAkRecoveryRequest-dedup".into(),
            human_agent_pubkey: "uhCAkHumanPubkey".into(),
            new_agent_pubkey: "uhCAkNewKey2".into(),
            superseded_agent_pubkey: "uhCAkOldKey2".into(),
            recovery_request_hash: "uhCAkRecoveryRequest-dedup".into(),
            rotated_at: "2026-05-11T12:00:00Z".into(),
            sender_peer_id: "12D3KooWdedup".into(),
            sent_at: "2026-05-11T12:00:01Z".into(),
        };
        let bytes = msg.to_bytes().expect("encode");

        // First delivery: received: true, reason: None
        let _ = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "KeyRotation".into(),
                payload_bytes: bytes.clone(),
            },
        );

        // Second delivery: dedup'd, received: true, reason: Some("duplicate")
        let response = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "KeyRotation".into(),
                payload_bytes: bytes,
            },
        );

        match response {
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: Some(r),
            } if r == "duplicate" => {}
            other => panic!("expected dedup'd IntegrityAck, got {:?}", other),
        }
    }

    #[test]
    fn integrity_notify_revocation_attestation_acks_received_true() {
        let svc = fresh_service();
        let msg = crate::p2p::revocation_attestation_message::RevocationAttestationMessage {
            signal_type: "revocationAttestation".into(),
            action_hash: "uhCkk1".into(),
            revocation_id: "u-rev-int-1".into(),
            steward_id: "agent-cid".into(),
            approved: true,
            attestation_kind: "vote".into(),
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: "2026-05-15T11:00:00Z".into(),
            emitted_at: "2026-05-15T11:00:01Z".into(),
        };
        let bytes = msg.to_bytes().expect("encode");

        let response = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "RevocationAttestation".to_string(),
                payload_bytes: bytes,
            },
        );

        match response {
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: None,
            } => {}
            other => panic!(
                "expected IntegrityAck {{ received: true }}, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn integrity_notify_revocation_attestation_dedup_returns_duplicate_reason() {
        let svc = fresh_service();
        let msg = crate::p2p::revocation_attestation_message::RevocationAttestationMessage {
            signal_type: "revocationAttestation".into(),
            action_hash: "uhCkk2".into(),
            revocation_id: "u-rev-int-dedup".into(),
            steward_id: "agent-cid".into(),
            approved: true,
            attestation_kind: "vote".into(),
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: "2026-05-15T11:00:00Z".into(),
            emitted_at: "2026-05-15T11:00:01Z".into(),
        };
        let bytes = msg.to_bytes().expect("encode");

        // First delivery: received: true, reason: None
        let _ = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "RevocationAttestation".into(),
                payload_bytes: bytes.clone(),
            },
        );

        // Second delivery: dedup'd, received: true, reason: Some("duplicate")
        let response = svc.handle(
            "test-peer",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "RevocationAttestation".into(),
                payload_bytes: bytes,
            },
        );

        match response {
            EprAtomResponse::IntegrityAck {
                received: true,
                reason: Some(r),
            } if r == "duplicate" => {}
            other => panic!("expected dedup'd IntegrityAck, got {:?}", other),
        }
    }

    #[test]
    fn integrity_notify_unhandled_kind_acks_with_reason() {
        let svc = fresh_service();
        match svc.handle(
            "peer-x",
            CallerIdentity::Anonymous,
            EprAtomRequest::IntegrityNotify {
                kind: "MysteryKind".into(),
                payload_bytes: b"x".to_vec(),
            },
        ) {
            EprAtomResponse::IntegrityAck { received, reason } => {
                assert!(!received);
                assert!(reason.unwrap_or_default().contains("MysteryKind"));
            }
            other => panic!("expected IntegrityAck, got {other:?}"),
        }
    }

    #[test]
    fn announce_decode_failure_reports_cbor_error() {
        let svc = fresh_service();
        match svc.handle(
            "peer-x",
            CallerIdentity::Anonymous,
            EprAtomRequest::Announce {
                envelope_bytes: b"not-cbor".to_vec(),
            },
        ) {
            EprAtomResponse::Announced { accepted, reason } => {
                assert!(!accepted);
                assert!(reason.unwrap_or_default().contains("cbor decode"));
            }
            other => panic!("expected Announced, got {other:?}"),
        }
    }
}
