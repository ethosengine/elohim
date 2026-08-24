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
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // bounded-work: one finite peer-book snapshot per interval tick;
        // missed ticks collapse instead of accumulating catch-up rounds.
        loop {
            ticker.tick().await;
            run_iroh_sync_round(&endpoint, &peer_book, &sync_manager, db_pool.as_ref()).await;
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
) {
    crate::metrics::inc_iroh_sync_round();
    let peers = peer_book.snapshot(Some(&endpoint.node_id()));
    debug!(peer_count = peers.len(), "iroh sync round started");

    let client = IrohSyncClient::new(endpoint);
    for peer in peers {
        sync_peer(&client, peer.addr, sync_manager, db_pool).await;
    }
}

async fn sync_peer(
    client: &IrohSyncClient<'_>,
    peer: NodeAddr,
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
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
                    sync_document(client, peer.clone(), sync_manager, db_pool, document).await
                }
                // A duplicate doc_id in one page — the second copy has nothing
                // left to sync; free the slot rather than stalling the window.
                None => SettleOutcome::Other,
            };
            window.settle(&doc_id, outcome);
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
        match sync_manager
            .apply_changes(PROJECTION_NAMESPACE, &remote.doc_id, changes)
            .await
        {
            Ok(applied_heads) => {
                crate::metrics::add_iroh_sync_changes_applied(change_count);
                local_heads = applied_heads;
                reverse_project(sync_manager, db_pool, &remote.doc_id).await;
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
pub(crate) async fn reverse_project(
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    doc_id: &str,
) {
    if !doc_id.starts_with("node:") {
        return;
    }
    let Some(pool) = db_pool else {
        return;
    };
    match crate::sync::projector::reverse_project_content_doc(sync_manager, pool, doc_id).await {
        Ok(true) => debug!(doc_id = %doc_id, "iroh sync reverse-projected content pointer"),
        Ok(false) => {}
        Err(error) => {
            warn!(doc_id = %doc_id, error = %error, "iroh sync reverse projection failed")
        }
    }
}
