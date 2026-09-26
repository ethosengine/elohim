//! Periodic Automerge sync rounds over the iroh transport.
//!
//! The driver consumes the verified [`IrohPeerBook`] working set and reuses
//! the existing sync wire protocol. It owns no document truth: landed changes
//! go through [`SyncManager`], then the existing amber-tier content reverse
//! projector updates the local SQL serving projection when applicable.

use std::sync::Arc;
use std::time::Duration;

use iroh::{Endpoint, NodeAddr};
use tracing::{debug, info, warn};

use super::{IrohPeerBook, IrohSyncClient};
use crate::db::DbPool;
use crate::p2p::sync_protocol::{next_doc_list_offset, DocumentInfo, SyncRequest, SyncResponse};
use crate::p2p::sync_round::{FetchWindow, SettleOutcome, SYNC_LIST_PAGE_LIMIT};
use crate::services::head_adoption_trigger::TriggerGate;
use crate::sync::{projector::PROJECTION_NAMESPACE, SyncManager};

/// Hard ceiling for a future paginated `SyncChanges` backend. The current
/// backend always returns one page, but the consumer is bounded before that
/// protocol capability lands so a lying peer cannot create an infinite round.
const MAX_CHANGE_PAGES_PER_DOCUMENT: usize = 1_024;

/// Spawn periodic iroh sync rounds. The first interval tick fires immediately;
/// missed ticks are collapsed so a slow round never accumulates catch-up work.
pub fn spawn_iroh_sync_driver(
    endpoint: Endpoint,
    peer_book: IrohPeerBook,
    sync_manager: Arc<SyncManager>,
    db_pool: Option<DbPool>,
    head_adoption: Option<Arc<TriggerGate>>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // bounded-work: one finite peer-book snapshot per interval tick;
        // missed ticks collapse instead of accumulating catch-up rounds.
        loop {
            ticker.tick().await;
            run_iroh_sync_round(
                &endpoint,
                &peer_book,
                &sync_manager,
                db_pool.as_ref(),
                head_adoption.as_deref(),
            )
            .await;
        }
    })
}

/// Run one sync round against a stable snapshot of every known iroh peer.
///
/// Peer and document failures are isolated: one bad dial, response, document,
/// or reverse projection is logged and the rest of the finite snapshot keeps
/// moving. This mirrors the libp2p round's heal semantics without sharing its
/// swarm event loop.
pub async fn run_iroh_sync_round(
    endpoint: &Endpoint,
    peer_book: &IrohPeerBook,
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    head_adoption: Option<&TriggerGate>,
) {
    crate::metrics::inc_iroh_sync_round();
    let peers = peer_book.snapshot(Some(&endpoint.node_id()));
    debug!(peer_count = peers.len(), "iroh sync round started");

    let client = IrohSyncClient::new(endpoint);
    for peer in peers {
        sync_peer(&client, peer.addr, sync_manager, db_pool, head_adoption).await;
    }
}

async fn sync_peer(
    client: &IrohSyncClient<'_>,
    peer: NodeAddr,
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    head_adoption: Option<&TriggerGate>,
) {
    let peer_id = peer.node_id;
    let mut offset = 0;

    loop {
        let sync_request = SyncRequest::ListDocuments {
            h_app_id: PROJECTION_NAMESPACE.to_string(),
            prefix: None,
            offset,
            limit: SYNC_LIST_PAGE_LIMIT,
        };
        let response = match request(client, peer.clone(), "list_documents", &sync_request).await {
            Some(response) => response,
            None => return,
        };

        let (documents, has_more) = match response {
            SyncResponse::DocumentList {
                h_app_id,
                documents,
                has_more,
                ..
            } if h_app_id == PROJECTION_NAMESPACE => (documents, has_more),
            SyncResponse::Error { message } => {
                crate::metrics::inc_iroh_sync_request("list_documents", "error_response");
                warn!(peer = %peer_id, error = %message, "iroh sync list rejected by peer");
                return;
            }
            other => {
                crate::metrics::inc_iroh_sync_request("list_documents", "error_response");
                warn!(peer = %peer_id, response = ?other, "iroh sync list returned an unexpected response");
                return;
            }
        };
        crate::metrics::inc_iroh_sync_request("list_documents", "ok");

        let page_len = documents.len();
        // Same scheduler as the libp2p initiator, at window 1. This plane was
        // ALREADY strictly sequential — that is why a wiped DocStore refilled in
        // one round here (63s) while libp2p's unbounded fan-out took three
        // (181s, `Io(max sub-streams reached)`) — so adopting the type is about
        // having ONE mechanism, not about changing this plane's pacing. Every
        // outcome settles as `Ok`/`Other`: `IrohSyncClient` erases
        // connect-vs-stream errors into anyhow, so no failure here can be
        // truthfully called a transport refusal, and none is re-queued. The
        // round remains the backstop, exactly as before.
        let mut by_doc: std::collections::HashMap<String, DocumentInfo> =
            std::collections::HashMap::with_capacity(page_len);
        let mut window = FetchWindow::new(1);
        for document in documents {
            window.enqueue(document.doc_id.clone());
            by_doc.insert(document.doc_id.clone(), document);
        }
        while let Some(doc_id) = window.next_doc() {
            let outcome = match by_doc.remove(&doc_id) {
                Some(document) => {
                    sync_document(
                        client,
                        peer.clone(),
                        sync_manager,
                        db_pool,
                        head_adoption,
                        document,
                    )
                    .await
                }
                // A duplicate doc_id in one page — the second copy has nothing
                // left to sync; free the slot rather than stalling the window.
                None => SettleOutcome::Other,
            };
            // The re-queue effect is deliberately ignored here: this plane
            // never settles `Io` (see above), so `settle` can only return false.
            let _ = window.settle(&doc_id, outcome);
        }
        debug_assert!(
            window.drained(),
            "the iroh page window must drain before the next page is requested"
        );

        match next_doc_list_offset(offset, page_len, has_more) {
            Some(next) => offset = next,
            None => return,
        }
    }
}

async fn sync_document(
    client: &IrohSyncClient<'_>,
    peer: NodeAddr,
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    head_adoption: Option<&TriggerGate>,
    remote: DocumentInfo,
) -> SettleOutcome {
    let mut local_heads = match sync_manager
        .get_heads(PROJECTION_NAMESPACE, &remote.doc_id)
        .await
    {
        Ok(heads) => heads,
        Err(error) => {
            warn!(doc_id = %remote.doc_id, error = %error, "iroh sync could not read local heads; requesting full document");
            Vec::new()
        }
    };
    if local_heads == remote.heads {
        return SettleOutcome::Ok;
    }

    for page in 0..MAX_CHANGE_PAGES_PER_DOCUMENT {
        let sync_request = SyncRequest::SyncChanges {
            h_app_id: PROJECTION_NAMESPACE.to_string(),
            doc_id: remote.doc_id.clone(),
            have_heads: local_heads,
            bloom_filter: None,
        };
        let response = match request(client, peer.clone(), "sync_changes", &sync_request).await {
            Some(response) => response,
            None => return SettleOutcome::Other,
        };

        let (changes, has_more, new_heads) = match response {
            SyncResponse::Changes {
                h_app_id,
                doc_id,
                changes,
                has_more,
                new_heads,
            } if h_app_id == PROJECTION_NAMESPACE && doc_id == remote.doc_id => {
                (changes, has_more, new_heads)
            }
            SyncResponse::Error { message } => {
                crate::metrics::inc_iroh_sync_request("sync_changes", "error_response");
                warn!(peer = %peer.node_id, doc_id = %remote.doc_id, error = %message, "iroh sync changes rejected by peer");
                return SettleOutcome::Other;
            }
            other => {
                crate::metrics::inc_iroh_sync_request("sync_changes", "error_response");
                warn!(peer = %peer.node_id, doc_id = %remote.doc_id, response = ?other, "iroh sync changes returned an unexpected response");
                return SettleOutcome::Other;
            }
        };
        crate::metrics::inc_iroh_sync_request("sync_changes", "ok");

        if changes.is_empty() {
            if has_more {
                warn!(peer = %peer.node_id, doc_id = %remote.doc_id, "iroh sync peer claimed more changes after an empty page");
            }
            return SettleOutcome::Ok;
        }

        let change_count = changes.len() as u64;
        let origin_timestamps = crate::sync::new_change_origin_timestamps(
            sync_manager,
            PROJECTION_NAMESPACE,
            &remote.doc_id,
            &changes,
        )
        .await;
        match sync_manager
            .apply_changes(PROJECTION_NAMESPACE, &remote.doc_id, changes)
            .await
        {
            Ok(applied_heads) => {
                crate::metrics::add_iroh_sync_changes_applied(change_count);
                local_heads = applied_heads;
                let adoption = head_adoption.map(|gate| (gate, peer.node_id));
                if reverse_project(sync_manager, db_pool, &remote.doc_id, adoption).await {
                    crate::metrics::observe_sync_projected_apply_staleness(
                        "iroh",
                        origin_timestamps,
                    );
                }
                info!(peer = %peer.node_id, doc_id = %remote.doc_id, changes = change_count, "iroh sync changes applied");
            }
            Err(error) => {
                warn!(peer = %peer.node_id, doc_id = %remote.doc_id, error = %error, "iroh sync failed to apply changes");
                return SettleOutcome::Other;
            }
        }

        if !has_more || local_heads == new_heads {
            return SettleOutcome::Ok;
        }
        if page + 1 == MAX_CHANGE_PAGES_PER_DOCUMENT {
            warn!(peer = %peer.node_id, doc_id = %remote.doc_id, pages = MAX_CHANGE_PAGES_PER_DOCUMENT, "iroh sync change-page ceiling reached");
        }
    }
    // Page ceiling reached without converging — the slot frees, the doc is not
    // re-queued (a lying peer must not create an unbounded round).
    SettleOutcome::Other
}

async fn request(
    client: &IrohSyncClient<'_>,
    peer: NodeAddr,
    kind: &'static str,
    request: &SyncRequest,
) -> Option<SyncResponse> {
    match client.request(peer.clone(), request).await {
        Ok(response) => Some(response),
        Err(error) => {
            // `IrohSyncClient` currently erases connect-vs-stream errors into
            // anyhow, so the bounded truthful label here is request_failed.
            // `dial_failed` remains reserved for a future typed client error.
            crate::metrics::inc_iroh_sync_request(kind, "request_failed");
            warn!(peer = %peer.node_id, request_kind = kind, error = %error, "iroh sync request failed");
            None
        }
    }
}

/// CRDT-heal leg: re-derive a converged content doc into the local SQL serving
/// row (amber tier). `pub(crate)` because the announce receive arm
/// (`super::sync_backend`) owes the same heal a pulled change gets — a doorbell
/// that converges the DocStore but leaves the serving row stale is half a cure.
///
/// `head_adoption` is the gate and the peer whose apply this was. When the
/// projection completes, the id is offered to the head-adoption trigger exactly
/// as the libp2p heal leg offers it (`P2PNode::heal_content_row`): after the
/// pool guard, only for a doc this node has a projection home for, and never
/// for one it could not read. Without it an iroh-only peer's head stayed
/// sweep-bound (47–59 s or never) while a libp2p peer adopted in about a
/// second. The peer is the iroh node id — the courier route the iroh
/// reconcile leg resolves.
pub(crate) async fn reverse_project(
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    doc_id: &str,
    head_adoption: Option<(&TriggerGate, iroh::NodeId)>,
) -> bool {
    if !doc_id.starts_with("node:") {
        return false;
    }
    let Some(pool) = db_pool else {
        return false;
    };
    let projected =
        match crate::sync::projector::reverse_project_content_doc(sync_manager, pool, doc_id).await
        {
            Ok(true) => {
                debug!(doc_id = %doc_id, "iroh sync reverse-projected content pointer");
                true
            }
            Ok(false) => true,
            Err(error) => {
                warn!(doc_id = %doc_id, error = %error, "iroh sync reverse projection failed");
                false
            }
        };
    if projected {
        if let Some((gate, peer)) = head_adoption {
            let decision = gate.offer(PROJECTION_NAMESPACE, doc_id, &peer.to_string());
            crate::metrics::inc_head_adoption_trigger(decision.label());
        }
    }
    projected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::head_adoption_trigger::{TriggerGate, DEFAULT_TRIGGER_COOLDOWN};
    use crate::sync::{DocStore, DocStoreConfig, StreamTracker};

    async fn empty_sync_manager() -> (SyncManager, tempfile::TempDir) {
        let temp = tempfile::TempDir::new().unwrap();
        let store = DocStore::new(DocStoreConfig {
            db_path: temp.path().join("driver.sled"),
            ..Default::default()
        })
        .await
        .unwrap();
        (
            SyncManager::new(Arc::new(store), Arc::new(StreamTracker::new())),
            temp,
        )
    }

    fn a_peer() -> iroh::NodeId {
        iroh::SecretKey::generate(&mut rand::rngs::OsRng).public()
    }

    /// An iroh apply offers the content id to the head-adoption trigger exactly
    /// where the libp2p heal leg does: after the pool guard, for a content doc
    /// the projector could read. Before this, an iroh-only peer never raised
    /// the trigger and its head stayed sweep-bound.
    #[tokio::test]
    async fn an_iroh_apply_offers_the_content_id_to_the_adoption_trigger() {
        let (sync, _temp) = empty_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let (gate, mut rx) = TriggerGate::new(DEFAULT_TRIGGER_COOLDOWN);
        let peer = a_peer();

        assert!(reverse_project(&sync, Some(&pool), "node:iroh-head", Some((&gate, peer))).await);
        let trigger = rx.try_recv().expect("the apply raised the trigger");
        assert_eq!(trigger.content_id, "iroh-head");
        assert_eq!(
            trigger.peer,
            peer.to_string(),
            "the courier is the iroh peer"
        );

        // Nothing is offered for a doc with no projection home, or with no pool.
        assert!(!reverse_project(&sync, Some(&pool), "manifest:x", Some((&gate, peer))).await);
        assert!(!reverse_project(&sync, None, "node:no-pool", Some((&gate, peer))).await);
        assert!(rx.try_recv().is_err());
        assert_eq!(gate.claim_count(), 1);
    }
}
