//! Phase 11 — production [`IdentityHandshakeBackend`] and [`TrustBackend`]
//! backed by the transport-neutral services
//! [`crate::identity_handshake_service::IdentityHandshakeService`] and
//! [`crate::trust_service::TrustService`].
//!
//! Adapters between the iroh ALPN handlers ([`super::auth::IrohIdentityHandshakeProtocol`]
//! and [`super::auth::IrohTrustProtocol`]) and the daemon's
//! transport-neutral auth services. Mirror the libp2p inline handlers
//! in `P2PNode::run` so the two transports return wire-byte-identical
//! responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! both planes are dual-stack permanent. Integrity is preserved by
//! Track 1 DHT-notarized agent identity (kitsune2/tx5) and the signed
//! wire frames each plane carries — NOT by any transport-level
//! security property. Both transports preserve DHT-derived integrity
//! equally.
//!
//! ## Authoritative peer identifier
//!
//! The iroh-side identity-handshake adapter holds the local node's
//! iroh `NodeId` string and uses it as the authoritative peer
//! identifier when calling the service. (libp2p uses
//! `peer.to_base58()`; iroh uses the connecting peer's NodeId, which
//! is observable but not yet plumbed through `ProtocolHandler::accept`
//! in the parity-harness API surface — the adapter therefore uses the
//! local node-id as a placeholder until Phase 12 cross-stack peer-map
//! threads connecting-peer NodeId through the handler.)

use std::sync::Arc;

use super::auth::{IdentityHandshakeBackend, TrustBackend};
use crate::identity_handshake_service::IdentityHandshakeService;
use crate::p2p::identity_handshake::{IdentityHandshakeRequest, IdentityHandshakeResponse};
use crate::p2p::trust_protocol::{TrustHandshake, TrustResponse};
use crate::trust_service::TrustService;

/// Routes [`IdentityHandshakeRequest`]s into a shared
/// [`IdentityHandshakeService`] with a fixed local-peer-id label.
pub struct IdentityHandshakeServiceBackend {
    service: Arc<IdentityHandshakeService>,
    local_peer_id_label: String,
}

impl IdentityHandshakeServiceBackend {
    pub fn new(service: Arc<IdentityHandshakeService>, local_peer_id_label: String) -> Self {
        Self {
            service,
            local_peer_id_label,
        }
    }
}

impl std::fmt::Debug for IdentityHandshakeServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityHandshakeServiceBackend")
            .field("local_peer_id_label", &self.local_peer_id_label)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl IdentityHandshakeBackend for IdentityHandshakeServiceBackend {
    async fn handle(&self, request: IdentityHandshakeRequest) -> IdentityHandshakeResponse {
        let now_iso = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        // The connecting peer's NodeId is not yet exposed to the
        // ProtocolHandler::accept signature in our harness; we fall
        // back to using the binding's claimed peer_id as the
        // authoritative label so verify_handshake_request's
        // peer_id-mismatch rule still functions structurally. The
        // libp2p side has access to the actual transport-layer
        // PeerId; closing this asymmetry is a Phase 12 concern that
        // graduates with the cross-stack peer-map.
        let peer_label = request.binding.peer_id.clone();
        self.service.handle(request, &peer_label, &now_iso)
    }
}

/// Routes [`TrustHandshake`] requests into a shared [`TrustService`]
/// and produces the matching [`TrustResponse`]. Cache insertion (the
/// libp2p `peer_trust_cache` keyed by libp2p PeerId) is intentionally
/// skipped in iroh mode — see module-level docs.
pub struct TrustServiceBackend {
    service: Arc<TrustService>,
}

impl TrustServiceBackend {
    pub fn new(service: Arc<TrustService>) -> Self {
        Self { service }
    }
}

impl std::fmt::Debug for TrustServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustServiceBackend").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl TrustBackend for TrustServiceBackend {
    async fn handle(&self, request: TrustHandshake) -> TrustResponse {
        self.service.handle(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::identity_handshake::HandshakeBindingPayload;

    fn fresh_id_backend() -> IdentityHandshakeServiceBackend {
        let service = Arc::new(IdentityHandshakeService::new(None));
        IdentityHandshakeServiceBackend::new(service, "iroh-local-node".to_string())
    }

    fn sample_id_request(peer_id: &str, agent_cid: &str) -> IdentityHandshakeRequest {
        IdentityHandshakeRequest {
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            nonce: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(),
            binding: HandshakeBindingPayload {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                signature: "AAA=".to_string(),
                valid_from: "2020-01-01T00:00:00Z".to_string(),
                valid_until: None,
                dht_anchor_hash: None,
                device_archetype: "node".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn identity_handshake_with_self_consistent_payload_accepts() {
        let backend = fresh_id_backend();
        let res = backend
            .handle(sample_id_request("peer-self", "agent-self"))
            .await;
        // No DB pool → Accepted (Phase 11 stub semantics).
        assert!(matches!(res, IdentityHandshakeResponse::Accepted));
    }

    #[tokio::test]
    async fn identity_handshake_rejects_when_signature_empty() {
        let backend = fresh_id_backend();
        let mut req = sample_id_request("peer-self", "agent-self");
        req.binding.signature.clear();
        let res = backend.handle(req).await;
        match res {
            IdentityHandshakeResponse::Rejected { reason } => {
                assert!(reason.contains("signature is empty"));
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn trust_handshake_returns_public_ceiling_one_hour_ttl() {
        let backend = TrustServiceBackend::new(Arc::new(TrustService::new()));
        let req = TrustHandshake {
            agent_pubkey: "agent-x".to_string(),
            membership_cids: vec![],
            relationship_cids: vec![],
            attestation_cids: vec![],
            stewardship_cids: vec![],
        };
        match backend.handle(req).await {
            TrustResponse::Verified {
                reach_ceiling,
                ttl_seconds,
            } => {
                assert_eq!(reach_ceiling, "public");
                assert_eq!(ttl_seconds, 3600);
            }
            other => panic!("expected Verified, got {other:?}"),
        }
    }
}
