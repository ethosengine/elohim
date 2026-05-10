//! Phase 11 — transport-neutral EPR-atom service.
//!
//! Extracted from `p2p::P2PNode::handle_epr_atom_request` so both the
//! libp2p request-response handler and the iroh-side `EprAtomBackend`
//! can route fetch / announce / fetch-batch / integrity-notify through
//! the same code path.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
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
        Self { db_pool, dedup }
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
                // record_predecessor TODO is intentionally left in
                // P2PNode-side wiring for now; see the original site in
                // p2p/mod.rs::handle_epr_atom_request.
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
