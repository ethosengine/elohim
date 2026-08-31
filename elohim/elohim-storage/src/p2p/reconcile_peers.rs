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

/// BOTH planes as ONE peer source — what "dual transport" must mean for the
/// reconcile stream.
///
/// DRIFT CURED (2026-08-31, measured live): peer-source selection was
/// `libp2p if present, else iroh` — so a DUAL fleet polled ONLY its libp2p
/// peers, and an iroh-only peer (the workspace supplier W2) was invisible to
/// every inventory poll, hint, and head-record fetch BY CONSTRUCTION. No
/// board, book, or fleet reboot could cure it: the arms never asked that
/// plane. The household mesh missed it for the same reason a single-plane
/// fixture always will — every mesh peer lived on one plane.
///
/// Semantics:
/// - `list_peers` is the UNION of the legs' peers, deduplicated by string id
///   in leg order (both legs label fleet peers by the same libp2p id string —
///   the iroh book reuses it as its label — so a dual peer is polled once).
/// - `view_federate` tries the legs in order and falls through ONLY on
///   transport-shaped refusals (`TransportError` / `SwarmGone` — "this leg
///   cannot reach that peer"), never on an answered failure: a timeout or an
///   inbound error from a leg that DID carry the request is that request's
///   real outcome, and retrying it on the other plane would double-spend the
///   responder's budget and blur attribution.
///
/// **Concerns:** C4 — an unreachable-on-this-leg peer is not evidence about
/// the peer, so the other leg is consulted; a leg that answered (even with an
/// error) IS the answer. C6a — at most one extra attempt per call, no ladder.
pub struct CompositeReconcilePeers {
    legs: Vec<std::sync::Arc<dyn ReconcilePeers>>,
    agent_pubkey: String,
}

impl CompositeReconcilePeers {
    pub fn new(legs: Vec<std::sync::Arc<dyn ReconcilePeers>>, agent_pubkey: String) -> Self {
        Self { legs, agent_pubkey }
    }
}

#[async_trait]
impl ReconcilePeers for CompositeReconcilePeers {
    fn transport(&self) -> &'static str {
        "dual"
    }

    fn agent_pubkey(&self) -> &str {
        &self.agent_pubkey
    }

    async fn list_peers(&self) -> Vec<ReconcilePeer> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for leg in &self.legs {
            for p in leg.list_peers().await {
                if seen.insert(p.peer_id.clone()) {
                    out.push(p);
                }
            }
        }
        out
    }

    async fn view_federate(
        &self,
        peer_id: &str,
        request: ViewFederationRequest,
        timeout: Duration,
    ) -> Result<ViewFederationResponse, FederationError> {
        let mut last = FederationError::SwarmGone;
        for leg in &self.legs {
            match leg.view_federate(peer_id, request.clone(), timeout).await {
                Ok(resp) => return Ok(resp),
                // Transport-shaped refusal: this leg cannot reach that peer at
                // all — the next leg may. Everything else is an ANSWERED
                // outcome and is returned as-is (C4).
                Err(e @ (FederationError::TransportError | FederationError::SwarmGone)) => {
                    last = e;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod composite_tests {
    use super::*;
    use std::sync::Arc;

    struct FakeLeg {
        peers: Vec<&'static str>,
        answer: Result<(), FederationError>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ReconcilePeers for FakeLeg {
        fn transport(&self) -> &'static str {
            "fake"
        }
        fn agent_pubkey(&self) -> &str {
            "uhCAk-fake"
        }
        async fn list_peers(&self) -> Vec<ReconcilePeer> {
            self.peers
                .iter()
                .map(|p| ReconcilePeer {
                    peer_id: p.to_string(),
                })
                .collect()
        }
        async fn view_federate(
            &self,
            peer_id: &str,
            _request: ViewFederationRequest,
            _timeout: Duration,
        ) -> Result<ViewFederationResponse, FederationError> {
            self.asked.lock().unwrap().push(peer_id.to_string());
            match &self.answer {
                Ok(()) => Err(FederationError::Timeout), // "answered" stand-in
                Err(FederationError::TransportError) => Err(FederationError::TransportError),
                Err(FederationError::SwarmGone) => Err(FederationError::SwarmGone),
                Err(_) => Err(FederationError::Timeout),
            }
        }
    }

    fn leg(peers: Vec<&'static str>, answer: Result<(), FederationError>) -> Arc<FakeLeg> {
        Arc::new(FakeLeg {
            peers,
            answer,
            asked: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn req() -> ViewFederationRequest {
        ViewFederationRequest {
            view_kind: crate::views::ViewKind::Cluster,
            agent_cid: "uhCAk-fake".into(),
            request_id: "r".into(),
            inventory_offset: None,
            head_corpus_digest: None,
        }
    }

    /// A dual peer (both legs) is listed ONCE; an iroh-only peer is listed —
    /// the 2026-08-31 invisibility class this type exists to kill.
    #[tokio::test]
    async fn union_dedupes_dual_peers_and_includes_single_plane_peers() {
        let a = leg(vec!["fleet-1", "fleet-2"], Ok(()));
        let b = leg(vec!["fleet-1", "w2-iroh-only"], Ok(()));
        let c = CompositeReconcilePeers::new(vec![a, b], "uhCAk-me".into());
        let ids: Vec<String> = c
            .list_peers()
            .await
            .into_iter()
            .map(|p| p.peer_id)
            .collect();
        assert_eq!(ids, vec!["fleet-1", "fleet-2", "w2-iroh-only"]);
    }

    /// Transport-shaped refusal on leg 1 falls through to leg 2; an ANSWERED
    /// outcome (here: Timeout) from leg 1 is returned without consulting leg 2
    /// (no double-spend of a responder that already carried the request).
    #[tokio::test]
    async fn fallthrough_is_transport_shaped_only() {
        let unreachable = leg(vec![], Err(FederationError::TransportError));
        let reached = leg(vec!["p"], Ok(()));
        let c = CompositeReconcilePeers::new(
            vec![unreachable.clone(), reached.clone()],
            "uhCAk-me".into(),
        );
        let out = c.view_federate("p", req(), Duration::from_millis(10)).await;
        assert!(matches!(out, Err(FederationError::Timeout)));
        assert_eq!(
            reached.asked.lock().unwrap().len(),
            1,
            "second leg consulted"
        );

        let answered = leg(vec!["p"], Ok(()));
        let never = leg(vec!["p"], Ok(()));
        let c2 =
            CompositeReconcilePeers::new(vec![answered.clone(), never.clone()], "uhCAk-me".into());
        let out2 = c2
            .view_federate("p", req(), Duration::from_millis(10))
            .await;
        assert!(matches!(out2, Err(FederationError::Timeout)));
        assert_eq!(
            never.asked.lock().unwrap().len(),
            0,
            "an answered outcome must NOT be retried on the other plane"
        );
    }
}
