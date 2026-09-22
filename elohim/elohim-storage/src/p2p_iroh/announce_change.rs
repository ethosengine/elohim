//! The doorbell's SEND side on the iroh plane.
//!
//! The libp2p plane got a real doorbell on 2026-08-23 (`p2p/sync_round.rs`,
//! `p2p/mod.rs`): on a locally-authored change the node pushes THE announced
//! change by hash to every connected peer, bounded by
//! [`MAX_ANNOUNCE_PAYLOAD_BYTES`], and the receiver applies it through the same
//! verified path a pulled change takes. The iroh plane had neither half — its
//! receive arm bare-acked, and no sender existed at all, so a fresh write waited
//! up to a full round (60s) to reach an iroh peer. This module is the sender;
//! [`super::sync_backend`] is the receiver.
//!
//! ## Where the announce is wired
//!
//! The content-projection producer takes an optional announce channel. In
//! `main.rs` that producer is spawned in exactly two arms, so each change is
//! projected once:
//!
//! - the **pure-iroh arm** (`p2p_node.is_none()`) passes this bridge's channel
//!   with [`AnnounceScope::AllBookPeers`];
//! - the **libp2p arm** (`p2p_node.is_some()`) covers `Dual` mode, where both
//!   stacks share ONE `Arc<SyncManager>` (`sync.sled` takes an exclusive lock,
//!   so it is opened once per process). Its forwarder rings libp2p and, in
//!   Dual, also hands the change to this bridge with
//!   [`AnnounceScope::IrohOnlyPeers`].
//!
//! The scope is the dedup: a book entry carrying a `libp2p_peer_id` runs dual
//! and is already rung on libp2p, so only peers without one get the iroh push.
//! Before this, a dual node left iroh-only peers to the 60s round — measured on
//! the household 2026-09-22 at 38.6s against ~0.9s on libp2p.
//!
//! ## Bounded and lossy, deliberately
//!
//! One request per book peer per change, sequentially, each under
//! [`ANNOUNCE_REQUEST_TIMEOUT`]; no retry, no queue. The producer's channel is
//! `try_send`, so a saturated bridge drops the doorbell and costs propagation
//! latency, never correctness — the sync round is the reconciliation backstop.

use std::sync::Arc;
use std::time::Duration;

use iroh::Endpoint;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, info, warn};

use super::peer_book::{IrohPeerBook, IrohPeerEntry};
use super::sync::IrohSyncClient;
use crate::p2p::sync_protocol::SyncResponse;
use crate::p2p::sync_round::{announce_request, bounded_announce_payload};
use crate::sync::projector::{LocalChange, PROJECTION_NAMESPACE};
use crate::sync::SyncManager;

/// Ceiling on one announce round-trip. An announce is a courtesy, not a
/// commitment: a peer that stalls must not hold the sender's turn while later
/// changes queue behind it.
const ANNOUNCE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Which book peers an announce rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnounceScope {
    /// Pure-iroh node: iroh is the only plane, ring everyone.
    AllBookPeers,
    /// Dual node: libp2p already rang every peer that runs it, so ring only
    /// peers whose manifest carries no `libp2p_peer_id`.
    IrohOnlyPeers,
}

impl AnnounceScope {
    fn admits(self, entry: &IrohPeerEntry) -> bool {
        match self {
            AnnounceScope::AllBookPeers => true,
            AnnounceScope::IrohOnlyPeers => entry.libp2p_peer_id.is_none(),
        }
    }
}

/// Everything the iroh doorbell needs to ring.
pub struct IrohAnnounceInputs {
    pub endpoint: Endpoint,
    /// Who to ring. The book is the verified working set — entries only ever
    /// come from a signed `elohim/transport/manifest` announcement.
    pub book: IrohPeerBook,
    /// Where the announced change's bytes are read from — the same manager the
    /// producer just wrote into.
    pub sync_manager: Arc<SyncManager>,
    /// Which book peers to ring.
    pub scope: AnnounceScope,
}

/// Spawn the announce bridge: translate each locally-authored change reported
/// by the content-projection producer into an eager push at every book peer.
pub fn spawn_iroh_announce_bridge(
    inputs: IrohAnnounceInputs,
    mut rx: Receiver<LocalChange>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // bounded-work: one bounded fan-out per received change; the channel is
        // the only queue and the producer drops rather than blocking on it.
        while let Some(change) = rx.recv().await {
            announce_local_change(&inputs, &change.doc_id, &change.change_hash).await;
        }
        debug!("iroh announce bridge: producer channel closed — bridge stopping");
    })
}

/// Ring the doorbell for one change. Returns how many peers accepted a push
/// (`was_new: true`) — the rest either already had it, or are pulling.
pub async fn announce_local_change(
    inputs: &IrohAnnounceInputs,
    doc_id: &str,
    change_hash: &str,
) -> usize {
    // Carry THE announced change, addressed by the hash the producer already
    // names — never `get_changes_since(.., &[])`, which is every change since
    // genesis and overflows the bound on any mature doc (the libp2p plane's own
    // first cut had that bug; see commit d4b54537a). `None` is always safe: the
    // receiver pulls, and the round backstops.
    let change_data = match inputs
        .sync_manager
        .get_change_by_hash(PROJECTION_NAMESPACE, doc_id, change_hash)
        .await
    {
        Ok(Some(bytes)) => bounded_announce_payload(vec![bytes]),
        Ok(None) => {
            debug!(doc_id = %doc_id, change_hash = %change_hash, "iroh announce: doc no longer holds the announced change, sending doorbell only");
            None
        }
        Err(e) => {
            debug!(doc_id = %doc_id, error = %e, "iroh announce: could not load change bytes, sending doorbell only");
            None
        }
    };
    let eager_bytes = change_data.as_ref().map(|d| d.len()).unwrap_or(0);

    let me = inputs.endpoint.node_id();
    let peers: Vec<IrohPeerEntry> = inputs
        .book
        .snapshot(Some(&me))
        .into_iter()
        .filter(|entry| inputs.scope.admits(entry))
        .collect();
    if peers.is_empty() {
        debug!(doc_id = %doc_id, scope = ?inputs.scope, "iroh announce: no book peers in scope, the round remains the propagation path");
        return 0;
    }

    let client = IrohSyncClient::new(&inputs.endpoint);
    let mut accepted = 0usize;
    for entry in &peers {
        let request = announce_request(
            PROJECTION_NAMESPACE,
            doc_id,
            change_hash,
            change_data.clone(),
        );
        let peer_id = entry.addr.node_id;
        let sent = tokio::time::timeout(
            ANNOUNCE_REQUEST_TIMEOUT,
            client.request(entry.addr.clone(), &request),
        )
        .await;
        match sent {
            Err(_) => {
                crate::metrics::inc_iroh_sync_request("announce_change", "request_failed");
                warn!(peer = %peer_id, doc_id = %doc_id, "iroh announce timed out — the round will carry the change");
            }
            Ok(Err(e)) => {
                crate::metrics::inc_iroh_sync_request("announce_change", "request_failed");
                warn!(peer = %peer_id, doc_id = %doc_id, error = %e, "iroh announce failed — the round will carry the change");
            }
            Ok(Ok(SyncResponse::ChangeAck { was_new, .. })) => {
                crate::metrics::inc_iroh_sync_request("announce_change", "ok");
                if was_new {
                    accepted += 1;
                }
                debug!(peer = %peer_id, doc_id = %doc_id, was_new = was_new, "iroh announce acknowledged");
            }
            Ok(Ok(SyncResponse::Error { message })) => {
                crate::metrics::inc_iroh_sync_request("announce_change", "error_response");
                warn!(peer = %peer_id, doc_id = %doc_id, error = %message, "iroh announce rejected by peer");
            }
            Ok(Ok(other)) => {
                // A typed miss, not a catch-all shrug: an announce answered by
                // anything but a ChangeAck means the two planes have drifted.
                crate::metrics::inc_iroh_sync_request("announce_change", "error_response");
                warn!(peer = %peer_id, doc_id = %doc_id, response = ?other, "iroh announce returned an unexpected response");
            }
        }
    }
    info!(
        doc_id = %doc_id, change_hash = %change_hash,
        peers = peers.len(), accepted = accepted, eager_bytes = eager_bytes,
        "iroh announced local change to book peers"
    );
    accepted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{DocStore, StreamTracker};

    async fn sync_manager(dir: &std::path::Path) -> Arc<SyncManager> {
        let doc_store = Arc::new(
            DocStore::at_path(dir.join("sync.sled"))
                .await
                .expect("doc store"),
        );
        Arc::new(SyncManager::new(doc_store, Arc::new(StreamTracker::new())))
    }

    /// An empty book must not be an error path — a node with no known iroh
    /// peers rings nobody and lets the round carry the change.
    #[tokio::test]
    async fn an_empty_book_rings_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = crate::p2p_iroh::IrohConfig::from_storage_dir(dir.path());
        let endpoint = crate::p2p_iroh::build_endpoint(&cfg).await.unwrap();
        let inputs = IrohAnnounceInputs {
            endpoint: endpoint.clone(),
            book: IrohPeerBook::new(),
            sync_manager: sync_manager(dir.path()).await,
            scope: AnnounceScope::AllBookPeers,
        };
        assert_eq!(
            announce_local_change(&inputs, "node:nobody", "deadbeef").await,
            0
        );
        endpoint.close().await;
    }

    /// Dual scope rings only peers with no libp2p leg; pure-iroh rings all.
    #[test]
    fn dual_scope_admits_only_iroh_only_peers() {
        let node_id = iroh::SecretKey::from_bytes(&[7u8; 32]).public();
        let entry = |libp2p: Option<&str>| IrohPeerEntry {
            addr: iroh::NodeAddr::new(node_id),
            agent_cid: None,
            libp2p_peer_id: libp2p.map(str::to_string),
            user_agent: None,
            announced_at_ms: 0,
        };
        let dual = entry(Some("12D3KooWdual"));
        let iroh_only = entry(None);
        assert!(!AnnounceScope::IrohOnlyPeers.admits(&dual));
        assert!(AnnounceScope::IrohOnlyPeers.admits(&iroh_only));
        assert!(AnnounceScope::AllBookPeers.admits(&dual));
        assert!(AnnounceScope::AllBookPeers.admits(&iroh_only));
    }
}
