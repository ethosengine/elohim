//! Transport-neutral peer source for the projection-reconcile arms.
//!
//! The reconcile stream (`projection_reconcile` / `participations_reconcile`
//! and the adopt-before-author head-record fetch) only ever asks a peer three
//! things: who are you (`agent_pubkey`), who is here (`list_peers`), and one
//! view-federation request (`view_federate`). Before this seam those were
//! `P2PHandle` (libp2p) methods, so a pure-iroh node had NO discovery arm at
//! all: a row whose anchor drifted from the DHT never healed, because the
//! heal leg is fed by peer-advertised inventory — measured 2026-08-29 as the
//! homo-iroh warm recovery's P1 red (survivor `bafkrei…`, recovering peer
//! `sha256-…`, same bytes) that homo-libp2p healed in 58 s.
//!
//! Both planes implement this trait; the arms take `&dyn ReconcilePeers` and
//! address peers by their string id (libp2p base58, or the iroh peer-book
//! label), never by a transport-specific key.

use std::time::Duration;

use async_trait::async_trait;

use super::{FederationError, P2PHandle};
use crate::views::{ViewFederationRequest, ViewFederationResponse};

/// One peer the reconcile arms may ask. `peer_id` is the string the source
/// resolves in [`ReconcilePeers::view_federate`] — opaque to the arms.
#[derive(Debug, Clone)]
pub struct ReconcilePeer {
    pub peer_id: String,
}

#[async_trait]
pub trait ReconcilePeers: Send + Sync {
    /// Transport label for logs/metrics (`libp2p` | `iroh`).
    fn transport(&self) -> &'static str;
    /// This node's agent CID — `agent_cid` on outbound federation requests.
    fn agent_pubkey(&self) -> &str;
    /// Peers currently askable on this plane.
    async fn list_peers(&self) -> Vec<ReconcilePeer>;
    /// One view-federation round trip to `peer_id`, bounded by `timeout`.
    async fn view_federate(
        &self,
        peer_id: &str,
        request: ViewFederationRequest,
        timeout: Duration,
    ) -> Result<ViewFederationResponse, FederationError>;
}

#[async_trait]
impl ReconcilePeers for P2PHandle {
    fn transport(&self) -> &'static str {
        "libp2p"
    }

    fn agent_pubkey(&self) -> &str {
        P2PHandle::agent_pubkey(self)
    }

    async fn list_peers(&self) -> Vec<ReconcilePeer> {
        P2PHandle::list_peers(self)
            .await
            .into_iter()
            .map(|p| ReconcilePeer { peer_id: p.peer_id })
            .collect()
    }

    async fn view_federate(
        &self,
        peer_id: &str,
        request: ViewFederationRequest,
        timeout: Duration,
    ) -> Result<ViewFederationResponse, FederationError> {
        let peer = peer_id
            .parse::<libp2p::PeerId>()
            .map_err(|_| FederationError::TransportError)?;
        P2PHandle::view_federate(self, peer, request, timeout).await
    }
}
