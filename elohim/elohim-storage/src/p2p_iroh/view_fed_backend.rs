//! Phase 11 — production [`ViewFederationBackend`] backed by [`crate::view_fed_service::ViewFedService`].
//!
//! Adapter between the iroh view-federation ALPN handler
//! ([`super::view_fed::IrohViewFederationProtocol`]) and the daemon's
//! transport-neutral view-federation service. Mirrors the libp2p side
//! (the inline handler in `P2PNode::run`'s ViewFederation Request arm,
//! which calls `build_response_slice` directly) so the two transports
//! return wire-byte-identical responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the view-federation plane is dual-stack permanent. 256 KiB cap on
//! responses applies on both transports.
//!
//! ## Connected peers
//!
//! Iroh-mode `connected_peers` is currently empty — the cross-stack
//! peer-map's `peer_transport_manifest` projection (Phase 12) will
//! provide a unified connection list across both transports. Until
//! then, iroh-mode PeerTopology slices return a local-only view; the
//! libp2p side continues to project its full connected-peers
//! snapshot.
//!
//! ## Signing failure
//!
//! If `build_response_slice` fails to sign (e.g. the keypair is a
//! public-key-only handle), the adapter mirrors the libp2p side's
//! behavior: it logs a warning and constructs a "best-effort"
//! response with an Offline freshness slice and an empty signature.
//! The libp2p path drops the channel (libp2p surfaces an
//! InboundFailure::ResponseOmission); since iroh's accept loop
//! requires a response or an error, the adapter chooses the
//! lower-loss path of returning a degraded-but-shaped response. A
//! follow-up may surface this differently as the spec evolves.

use std::sync::Arc;

use tracing::warn;

use super::view_fed::ViewFederationBackend;
use crate::view_fed_service::ViewFedService;
use crate::views::{
    Freshness, FreshnessState, JsonVal, ViewFederationRequest, ViewFederationResponse, ViewSlice,
};

/// Routes [`ViewFederationRequest`]s into a shared
/// [`ViewFedService`] and produces the matching
/// [`ViewFederationResponse`].
pub struct ViewFedServiceBackend {
    service: Arc<ViewFedService>,
    /// Reported in the (rare) signing-failure shape so receivers can
    /// see which peer emitted the degraded response. The libp2p side
    /// embeds this in slice.peer_id; here we track it explicitly so
    /// the failure path can do the same.
    local_peer_id_fallback: String,
}

impl ViewFedServiceBackend {
    pub fn new(service: Arc<ViewFedService>, local_peer_id_fallback: String) -> Self {
        Self {
            service,
            local_peer_id_fallback,
        }
    }
}

impl std::fmt::Debug for ViewFedServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewFedServiceBackend")
            .field("local_peer_id_fallback", &self.local_peer_id_fallback)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl ViewFederationBackend for ViewFedServiceBackend {
    async fn handle(&self, request: ViewFederationRequest) -> ViewFederationResponse {
        // Iroh-mode connected_peers is currently an empty libp2p
        // PeerId slice — see module-level docs.
        match self.service.handle(request.clone(), &[]).await {
            Ok(res) => res,
            Err(e) => {
                warn!(
                    target: "elohim_storage::view_federation",
                    error = %e,
                    agent_cid = %request.agent_cid,
                    request_id = %request.request_id,
                    "iroh-mode view-federation: signing failed; returning degraded shape"
                );
                ViewFederationResponse {
                    view_kind: request.view_kind.clone(),
                    agent_cid: request.agent_cid,
                    request_id: request.request_id,
                    slice: ViewSlice {
                        peer_id: self.local_peer_id_fallback.clone(),
                        view_kind: request.view_kind,
                        freshness: Freshness {
                            state: FreshnessState::Offline,
                            stale_since_ms: None,
                        },
                        payload: JsonVal(serde_json::Value::Null),
                        signature: String::new(),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::ViewKind;
    use libp2p::identity::Keypair;
    use libp2p::PeerId;

    fn fresh_backend() -> ViewFedServiceBackend {
        let kp = Keypair::generate_ed25519();
        let peer_id = PeerId::from(kp.public()).to_string();
        let service = Arc::new(ViewFedService::new(
            "agent-cid-self".into(),
            peer_id.clone(),
            kp,
            None,
        ));
        ViewFedServiceBackend::new(service, peer_id)
    }

    #[tokio::test]
    async fn handle_owning_agent_returns_signed_live_slice() {
        let backend = fresh_backend();
        let res = backend
            .handle(ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: "agent-cid-self".into(),
                request_id: "r1".into(),
            })
            .await;
        assert_eq!(res.agent_cid, "agent-cid-self");
        assert!(!res.slice.signature.is_empty());
        assert_eq!(res.slice.freshness.state, FreshnessState::Live);
    }

    #[tokio::test]
    async fn handle_non_owning_agent_returns_offline_slice() {
        let backend = fresh_backend();
        let res = backend
            .handle(ViewFederationRequest {
                view_kind: ViewKind::Cluster,
                agent_cid: "some-other-agent".into(),
                request_id: "r2".into(),
            })
            .await;
        assert_eq!(res.slice.freshness.state, FreshnessState::Offline);
    }
}
