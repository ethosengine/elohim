//! Phase 11 — transport-neutral identity-handshake service.
//!
//! Wraps the verify + persist sequence already factored into
//! [`crate::p2p::identity_handshake`] (verify_handshake_request,
//! binding_row_from_handshake_request, upsert_preserving_supersession)
//! so both the libp2p inbound handler and the iroh-side
//! [`crate::p2p_iroh::IdentityHandshakeBackend`] can produce
//! wire-byte-identical responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the identity-handshake plane is dual-stack permanent. Integrity is
//! preserved by Track 1 DHT-notarized agent identity (kitsune2/tx5)
//! and the signed wire frames the handshake exchanges — NOT by any
//! transport-level security property. Both transports preserve
//! DHT-derived integrity equally; this service is the shared
//! enforcement point.
//!
//! ## Authoritative peer identifier
//!
//! `peer_id_str` is the transport-layer's authoritative peer
//! identifier (libp2p PeerId base58 / iroh NodeId string), NOT the
//! `binding.peer_id` field in the request payload (which the verify
//! step cross-checks against `peer_id_str` to defeat spoofing).

use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::p2p::identity_handshake::{
    binding_row_from_handshake_request, verify_handshake_request, IdentityHandshakeRequest,
    IdentityHandshakeResponse, VerifyOutcome,
};

/// Holds the dependencies needed to verify + persist identity bindings.
#[derive(Clone)]
pub struct IdentityHandshakeService {
    db_pool: Option<DbPool>,
}

impl std::fmt::Debug for IdentityHandshakeService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityHandshakeService")
            .field("has_db_pool", &self.db_pool.is_some())
            .finish_non_exhaustive()
    }
}

impl IdentityHandshakeService {
    pub fn new(db_pool: Option<DbPool>) -> Self {
        Self { db_pool }
    }

    /// Verify the handshake request against `peer_id_str`, then upsert
    /// the resulting binding row using the supersession-preserving
    /// writer. Returns the wire response identical to what the libp2p
    /// inline handler would produce for the same input.
    pub fn handle(
        &self,
        request: IdentityHandshakeRequest,
        peer_id_str: &str,
        now_iso: &str,
    ) -> IdentityHandshakeResponse {
        match verify_handshake_request(&request, peer_id_str, now_iso) {
            VerifyOutcome::Invalid(reason) => {
                info!(
                    peer = %peer_id_str,
                    reason = %reason,
                    "Identity handshake rejected"
                );
                IdentityHandshakeResponse::Rejected { reason }
            }
            VerifyOutcome::Valid => {
                let row = binding_row_from_handshake_request(&request, peer_id_str, now_iso);
                match self.db_pool.as_ref() {
                    Some(pool) => match pool.get() {
                        Ok(mut conn) => {
                            // Non-authoritative writer: must NOT clobber any
                            // `superseded_by` previously written by the
                            // DHT-arrival path. Use the preserving variant.
                            match crate::db::peer_identity_bindings::upsert_preserving_supersession(
                                &mut conn, &row,
                            ) {
                                Ok(()) => {
                                    debug!(
                                        peer = %peer_id_str,
                                        agent_cid = %row.agent_cid,
                                        "Identity handshake: binding recorded"
                                    );
                                    IdentityHandshakeResponse::Accepted
                                }
                                Err(e) => {
                                    warn!(
                                        peer = %peer_id_str,
                                        error = %e,
                                        "Identity handshake: db upsert failed"
                                    );
                                    IdentityHandshakeResponse::Error(format!("db error: {e}"))
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                peer = %peer_id_str,
                                error = %e,
                                "Identity handshake: pool exhausted"
                            );
                            IdentityHandshakeResponse::Error("pool exhausted".to_string())
                        }
                    },
                    None => {
                        // No pool configured — accept but do not persist.
                        debug!(
                            peer = %peer_id_str,
                            "Identity handshake: no db_pool configured, skipping persistence"
                        );
                        IdentityHandshakeResponse::Accepted
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::identity_handshake::HandshakeBindingPayload;

    fn fresh_service() -> IdentityHandshakeService {
        IdentityHandshakeService::new(None)
    }

    fn sample_request(peer_id: &str, agent_cid: &str) -> IdentityHandshakeRequest {
        IdentityHandshakeRequest {
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            nonce: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(), // 16 zero bytes b64
            binding: HandshakeBindingPayload {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                signature: "AAA=".to_string(), // non-empty placeholder
                valid_from: "2020-01-01T00:00:00Z".to_string(),
                valid_until: None,
                dht_anchor_hash: None,
                device_archetype: "node".to_string(),
            },
        }
    }

    #[test]
    fn verify_with_no_pool_accepts_without_persistence() {
        let svc = fresh_service();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let req = sample_request("peer-x", "agent-x");
        let res = svc.handle(req, "peer-x", &now);
        assert!(matches!(res, IdentityHandshakeResponse::Accepted));
    }

    #[test]
    fn verify_rejects_when_payload_peer_id_mismatches_actual() {
        let svc = fresh_service();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let req = sample_request("peer-payload", "agent-x");
        let res = svc.handle(req, "peer-actual", &now);
        match res {
            IdentityHandshakeResponse::Rejected { reason } => {
                assert!(reason.contains("does not match"));
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[test]
    fn verify_rejects_when_signature_empty() {
        let svc = fresh_service();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut req = sample_request("peer-x", "agent-x");
        req.binding.signature.clear();
        let res = svc.handle(req, "peer-x", &now);
        match res {
            IdentityHandshakeResponse::Rejected { reason } => {
                assert!(reason.contains("signature is empty"));
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }
}
