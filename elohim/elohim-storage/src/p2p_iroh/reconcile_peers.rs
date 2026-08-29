//! [`ReconcilePeers`] over the iroh plane: peers come from the transport
//! manifest peer book, requests ride `IrohViewFederationClient` on the
//! process-wide fetch leg (`iroh_fetch_leg`). Labels match `pull_core`'s:
//! the announced libp2p PeerId when dual, else the agent CID, else the iroh
//! node id — so a peer keeps ONE name across the pull leg and the heal leg's
//! `discovered_via_peer` logs.

use std::time::Duration;

use async_trait::async_trait;
use iroh::NodeAddr;

use crate::p2p::reconcile_peers::{ReconcilePeer, ReconcilePeers};
use crate::p2p::FederationError;
use crate::p2p_iroh::peer_book::IrohPeerEntry;
use crate::p2p_iroh::{iroh_fetch_leg, IrohViewFederationClient};
use crate::views::{ViewFederationRequest, ViewFederationResponse};

pub struct IrohReconcilePeers {
    agent_pubkey: String,
}

impl IrohReconcilePeers {
    pub fn new(agent_pubkey: String) -> Self {
        Self { agent_pubkey }
    }

    fn book_peers() -> Vec<(String, NodeAddr)> {
        let Some(leg) = iroh_fetch_leg() else {
            return Vec::new();
        };
        let me = leg.endpoint().node_id();
        label_entries(leg.book().snapshot(Some(&me)))
    }
}

/// The one name a peer carries across the pull leg and the heal leg: the
/// announced libp2p PeerId when dual, else the agent CID, else the iroh node id.
pub(crate) fn label_entries(entries: Vec<IrohPeerEntry>) -> Vec<(String, NodeAddr)> {
    entries
        .into_iter()
        .map(|e| {
            let label = e
                .libp2p_peer_id
                .clone()
                .or(e.agent_cid.clone())
                .unwrap_or_else(|| e.addr.node_id.to_string());
            (label, e.addr)
        })
        .collect()
}

/// Resolve a reconcile peer id back to a dial target: by label first, then by
/// the raw iroh node id (an arm that learned a peer from another plane's log
/// line can still reach it).
pub(crate) fn find_addr(peers: Vec<(String, NodeAddr)>, peer_id: &str) -> Option<NodeAddr> {
    peers
        .into_iter()
        .find(|(label, addr)| label == peer_id || addr.node_id.to_string() == peer_id)
        .map(|(_, addr)| addr)
}

#[async_trait]
impl ReconcilePeers for IrohReconcilePeers {
    fn transport(&self) -> &'static str {
        "iroh"
    }

    fn agent_pubkey(&self) -> &str {
        &self.agent_pubkey
    }

    async fn list_peers(&self) -> Vec<ReconcilePeer> {
        Self::book_peers()
            .into_iter()
            .map(|(peer_id, _)| ReconcilePeer { peer_id })
            .collect()
    }

    async fn view_federate(
        &self,
        peer_id: &str,
        request: ViewFederationRequest,
        timeout: Duration,
    ) -> Result<ViewFederationResponse, FederationError> {
        let Some(leg) = iroh_fetch_leg() else {
            return Err(FederationError::SwarmGone);
        };
        let addr = find_addr(Self::book_peers(), peer_id).ok_or(FederationError::TransportError)?;
        let client = IrohViewFederationClient::new(leg.endpoint());
        match tokio::time::timeout(timeout, client.request(addr, &request)).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(error)) => {
                tracing::debug!(peer = %peer_id, error = %error, "iroh view-federation request failed");
                Err(FederationError::TransportError)
            }
            Err(_) => Err(FederationError::Timeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn entry(libp2p: Option<&str>, agent: Option<&str>) -> IrohPeerEntry {
        let mut rng = rand::rngs::OsRng;
        let key = SecretKey::generate(&mut rng);
        IrohPeerEntry {
            addr: NodeAddr::new(key.public()),
            agent_cid: agent.map(str::to_string),
            libp2p_peer_id: libp2p.map(str::to_string),
            announced_at_ms: 1,
        }
    }

    #[test]
    fn labels_prefer_libp2p_then_agent_then_node_id() {
        let entries = vec![
            entry(Some("12D3KooWdual"), Some("uhCAkagent")),
            entry(None, Some("uhCAkagent-only")),
            entry(None, None),
        ];
        let node_only = entries[2].addr.node_id.to_string();
        let labels: Vec<String> = label_entries(entries).into_iter().map(|(l, _)| l).collect();
        assert_eq!(
            labels,
            vec![
                "12D3KooWdual".to_string(),
                "uhCAkagent-only".to_string(),
                node_only
            ]
        );
    }

    #[test]
    fn find_addr_resolves_by_label_or_node_id_and_refuses_strangers() {
        let entries = vec![entry(Some("12D3KooWdual"), None), entry(None, None)];
        let by_node = entries[1].addr.node_id;
        let peers = label_entries(entries);
        assert!(find_addr(peers.clone(), "12D3KooWdual").is_some());
        assert_eq!(
            find_addr(peers.clone(), &by_node.to_string()).map(|a| a.node_id),
            Some(by_node)
        );
        assert!(find_addr(peers, "nobody").is_none());
    }
}
