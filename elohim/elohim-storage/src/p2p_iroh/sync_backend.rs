//! Phase 11 — production [`SyncBackend`] backed by [`crate::sync::SyncManager`].
//!
//! Adapter between the iroh sync ALPN handler ([`super::sync::IrohSyncProtocol`])
//! and the daemon's existing CRDT sync engine. Mirrors the libp2p side's
//! dispatch (`P2PNode::handle_sync_request` in `src/p2p/mod.rs`) exactly so the
//! two transports return wire-byte-identical responses for the same request.
//!
//! Construction:
//! ```ignore
//! let doc_store  = Arc::new(DocStore::at_path(storage_dir.join("sync.sled")).await?);
//! let stream_trk = Arc::new(StreamTracker::new());
//! let sync_mgr   = Arc::new(SyncManager::new(doc_store, stream_trk));
//! let backend: Arc<dyn SyncBackend> = Arc::new(SyncManagerBackend::new(sync_mgr));
//! let extras = vec![(SYNC_ALPN.to_vec(),
//!                    Box::new(IrohSyncProtocol::new(backend)) as Box<dyn DynProtocolHandler>)];
//! IrohNode::start_with_protocols(iroh_cfg, extras).await?;
//! ```

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use iroh::{Endpoint, NodeAddr, NodeId};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::peer_book::IrohPeerBook;
use super::sync::{IrohSyncClient, SyncBackend};
use crate::db::DbPool;
use crate::p2p::sync_protocol::{DocumentInfo, SyncRequest, SyncResponse};
use crate::p2p::sync_round::MAX_ANNOUNCE_PAYLOAD_BYTES;
use crate::sync::SyncManager;

/// Ceiling on the receive-side fallback pull. The pull is opened from inside a
/// spawned task, so a peer that accepts the connection and then stalls would
/// otherwise hold a task forever; the 60s round is the backstop either way.
const ANNOUNCE_PULL_TIMEOUT: Duration = Duration::from_secs(30);

/// What the announce receive arm needs to ring BACK at whoever rang us.
///
/// Late-bound (see [`SyncManagerBackend::set_pull_back`]) because the backend
/// is constructed BEFORE the iroh endpoint exists — the endpoint is built by
/// `IrohNode::start_with_protocols`, which takes the handler that holds this
/// backend. Absent, the announce arm degrades to exactly its pre-doorbell
/// behaviour: apply what fits, ack, let the round carry the rest.
struct AnnouncePullBack {
    endpoint: Endpoint,
    book: IrohPeerBook,
    /// For the amber-tier serving heal after a converged content doc.
    db_pool: Option<DbPool>,
}

/// Routes [`SyncRequest`] variants into a shared [`SyncManager`] and produces
/// the matching [`SyncResponse`]. Keep behavior in lockstep with the libp2p
/// handler at `src/p2p/mod.rs::P2PNode::handle_sync_request` — wire bytes are
/// the contract, transport is the variable.
pub struct SyncManagerBackend {
    sync_manager: Arc<SyncManager>,
    pull_back: OnceLock<AnnouncePullBack>,
}

impl SyncManagerBackend {
    pub fn new(sync_manager: Arc<SyncManager>) -> Self {
        Self {
            sync_manager,
            pull_back: OnceLock::new(),
        }
    }

    /// Give the announce arm a way to dial back at an announcer.
    ///
    /// Called once, after the iroh node and peer book exist. Returns `false` if
    /// already set (the second caller's endpoint is dropped, never swapped —
    /// a live handler must not have its dial target changed underneath it).
    pub fn set_pull_back(
        &self,
        endpoint: Endpoint,
        book: IrohPeerBook,
        db_pool: Option<DbPool>,
    ) -> bool {
        self.pull_back
            .set(AnnouncePullBack {
                endpoint,
                book,
                db_pool,
            })
            .is_ok()
    }

    /// The announce arm — the iroh plane's half of the doorbell.
    ///
    /// Mirrors `P2PNode::handle_sync_request`'s `AnnounceChange` arm decision
    /// for decision, because a dual-stack fleet where one plane converges
    /// eagerly and the other silently queues is worse than neither doing it:
    ///
    /// 1. an OVERSIZED payload is refused WITHOUT applying (the sender bounds
    ///    its own fan-out; a peer ignoring the bound must not make us allocate
    ///    past it) and degrades to the pull;
    /// 2. bytes that fit go through the SAME [`SyncManager::apply_changes`] a
    ///    pulled change takes — an eager push is never validated less;
    /// 3. DID IT LAND? Automerge QUEUES a change whose dependencies we lack and
    ///    `apply_changes` still returns `Ok` with the doc untouched. Acking
    ///    `was_new: true` there is silent data loss. Ask the doc whether it now
    ///    holds the announced change; if not, pull, which carries the deps;
    /// 4. a doorbell with NO bytes opens the same pull.
    async fn handle_announce(
        &self,
        peer: Option<NodeId>,
        h_app_id: String,
        doc_id: String,
        change_hash: String,
        change_data: Option<Vec<u8>>,
    ) -> SyncResponse {
        debug!(h_app_id = %h_app_id, doc_id = %doc_id, peer = ?peer, "iroh AnnounceChange");
        let data = change_data.filter(|d| {
            let ok = d.len() <= MAX_ANNOUNCE_PAYLOAD_BYTES;
            if !ok {
                warn!(
                    peer = ?peer, h_app_id = %h_app_id, doc_id = %doc_id, bytes = d.len(),
                    bound = MAX_ANNOUNCE_PAYLOAD_BYTES,
                    "iroh announced change exceeds the payload bound — refusing the push, pulling instead"
                );
            }
            ok
        });

        let Some(data) = data else {
            self.spawn_announce_pull(peer, &h_app_id, &doc_id);
            return SyncResponse::ChangeAck {
                h_app_id,
                doc_id,
                was_new: false,
            };
        };

        match self
            .sync_manager
            .apply_changes(&h_app_id, &doc_id, vec![data])
            .await
        {
            Ok(_) => {
                let landed = self
                    .sync_manager
                    .get_change_by_hash(&h_app_id, &doc_id, &change_hash)
                    .await
                    .unwrap_or(None)
                    .is_some();
                if !landed {
                    debug!(
                        peer = ?peer, h_app_id = %h_app_id, doc_id = %doc_id,
                        "iroh announced change is missing its dependencies here — pulling the doc instead"
                    );
                    self.spawn_announce_pull(peer, &h_app_id, &doc_id);
                    return SyncResponse::ChangeAck {
                        h_app_id,
                        doc_id,
                        was_new: false,
                    };
                }
                crate::metrics::add_iroh_sync_changes_applied(1);
                info!(h_app_id = %h_app_id, doc_id = %doc_id, "iroh applied announced change");
                // Amber-tier heal, same leg the pull path gets — a converged
                // DocStore with a stale serving row is half a cure.
                if let Some(pb) = self.pull_back.get() {
                    super::sync_driver::reverse_project(
                        &self.sync_manager,
                        pb.db_pool.as_ref(),
                        &doc_id,
                    )
                    .await;
                }
                SyncResponse::ChangeAck {
                    h_app_id,
                    doc_id,
                    was_new: true,
                }
            }
            Err(e) => {
                warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "iroh apply change failed");
                SyncResponse::Error {
                    message: format!("Failed to apply change: {}", e),
                }
            }
        }
    }

    /// Open the round's own per-doc pull, at this ONE doc, back at the
    /// announcer — detached, so the ack returns immediately.
    ///
    /// Detached deliberately: this runs inside the ALPN handler's per-connection
    /// stream loop, and awaiting a dial there would stall every later stream on
    /// that connection behind an unreachable peer. libp2p's twin is equally
    /// fire-and-forget (`send_request` returns at once and the answer lands in
    /// the event loop). Bounded and lossy: one request, one timeout, no retry,
    /// no queue — the 60s round is still the backstop.
    fn spawn_announce_pull(&self, peer: Option<NodeId>, h_app_id: &str, doc_id: &str) {
        let Some(peer) = peer else {
            debug!(
                h_app_id = %h_app_id, doc_id = %doc_id,
                "iroh announce: connection carried no peer identity — cannot pull back, the round will carry it"
            );
            return;
        };
        let Some(pb) = self.pull_back.get() else {
            debug!(
                h_app_id = %h_app_id, doc_id = %doc_id,
                "iroh announce: no pull-back wiring — the round will carry it"
            );
            return;
        };
        // The book is the verified dial target. Falling back to a bare NodeId
        // is only useful when a discovery service can resolve it; it is never
        // wrong, just possibly unreachable.
        let addr: NodeAddr = pb
            .book
            .get(&peer)
            .map(|e| e.addr)
            .unwrap_or_else(|| NodeAddr::new(peer));
        let endpoint = pb.endpoint.clone();
        let db_pool = pb.db_pool.clone();
        let sync_manager = self.sync_manager.clone();
        let h_app_id = h_app_id.to_string();
        let doc_id = doc_id.to_string();
        tokio::spawn(async move {
            match tokio::time::timeout(
                ANNOUNCE_PULL_TIMEOUT,
                pull_announced_doc(
                    &endpoint,
                    addr,
                    &sync_manager,
                    db_pool.as_ref(),
                    &h_app_id,
                    &doc_id,
                ),
            )
            .await
            {
                Ok(()) => {}
                Err(_) => {
                    crate::metrics::inc_iroh_sync_request("announce_pull", "request_failed");
                    warn!(
                        peer = %peer, doc_id = %doc_id,
                        "iroh announce pull timed out — the round remains the backstop"
                    );
                }
            }
        });
    }
}

/// One `SyncChanges` round-trip at one doc, applied through the same path the
/// sync-round driver uses. Free function so the spawned task owns no `&self`.
async fn pull_announced_doc(
    endpoint: &Endpoint,
    addr: NodeAddr,
    sync_manager: &SyncManager,
    db_pool: Option<&DbPool>,
    h_app_id: &str,
    doc_id: &str,
) {
    let peer_id = addr.node_id;
    let have_heads = sync_manager
        .get_heads(h_app_id, doc_id)
        .await
        .unwrap_or_default();
    let request = SyncRequest::SyncChanges {
        h_app_id: h_app_id.to_string(),
        doc_id: doc_id.to_string(),
        have_heads,
        bloom_filter: None,
    };
    let client = IrohSyncClient::new(endpoint);
    let response = match client.request(addr, &request).await {
        Ok(r) => r,
        Err(e) => {
            crate::metrics::inc_iroh_sync_request("announce_pull", "request_failed");
            warn!(peer = %peer_id, doc_id = %doc_id, error = %e, "iroh announce pull failed");
            return;
        }
    };
    let changes = match response {
        SyncResponse::Changes {
            h_app_id: ns,
            doc_id: id,
            changes,
            ..
        } if ns == h_app_id && id == doc_id => changes,
        SyncResponse::Error { message } => {
            crate::metrics::inc_iroh_sync_request("announce_pull", "error_response");
            warn!(peer = %peer_id, doc_id = %doc_id, error = %message, "iroh announce pull rejected by peer");
            return;
        }
        other => {
            crate::metrics::inc_iroh_sync_request("announce_pull", "error_response");
            warn!(peer = %peer_id, doc_id = %doc_id, response = ?other, "iroh announce pull returned an unexpected response");
            return;
        }
    };
    crate::metrics::inc_iroh_sync_request("announce_pull", "ok");
    if changes.is_empty() {
        return;
    }
    let count = changes.len() as u64;
    match sync_manager.apply_changes(h_app_id, doc_id, changes).await {
        Ok(_) => {
            crate::metrics::add_iroh_sync_changes_applied(count);
            info!(peer = %peer_id, doc_id = %doc_id, changes = count, "iroh announce pull applied changes");
            super::sync_driver::reverse_project(sync_manager, db_pool, doc_id).await;
        }
        Err(e) => {
            warn!(peer = %peer_id, doc_id = %doc_id, error = %e, "iroh announce pull failed to apply changes")
        }
    }
}

impl std::fmt::Debug for SyncManagerBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncManagerBackend").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl SyncBackend for SyncManagerBackend {
    /// Peer-less entry point — every variant but `AnnounceChange` is
    /// peer-independent, and an announce with no known sender degrades to the
    /// old bare ack. Real traffic arrives through [`SyncBackend::handle_from`].
    async fn handle(&self, request: SyncRequest) -> SyncResponse {
        self.dispatch(None, request).await
    }

    async fn handle_from(&self, peer: Option<NodeId>, request: SyncRequest) -> SyncResponse {
        self.dispatch(peer, request).await
    }
}

impl SyncManagerBackend {
    async fn dispatch(&self, peer: Option<NodeId>, request: SyncRequest) -> SyncResponse {
        match request {
            SyncRequest::GetHeads { h_app_id, doc_id } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, "iroh GetHeads");
                match self.sync_manager.get_heads(&h_app_id, &doc_id).await {
                    Ok(heads) => {
                        let change_count = match self
                            .sync_manager
                            .list_documents(&h_app_id, Some(&doc_id), 0, 1)
                            .await
                        {
                            Ok((docs, _)) => docs.first().map(|d| d.change_count).unwrap_or(0),
                            Err(_) => 0,
                        };
                        SyncResponse::Heads {
                            h_app_id,
                            doc_id,
                            heads,
                            change_count,
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "iroh GetHeads failed");
                        SyncResponse::Error {
                            message: format!("Failed to get heads: {}", e),
                        }
                    }
                }
            }
            SyncRequest::SyncChanges {
                h_app_id,
                doc_id,
                have_heads,
                bloom_filter: _, // mirrored TODO from libp2p side: bloom not consumed
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, have_heads = ?have_heads, "iroh SyncChanges");
                match self
                    .sync_manager
                    .get_changes_since(&h_app_id, &doc_id, &have_heads)
                    .await
                {
                    Ok((changes, new_heads)) => {
                        info!(h_app_id = %h_app_id, doc_id = %doc_id, changes_count = changes.len(), "iroh sending changes");
                        SyncResponse::Changes {
                            h_app_id,
                            doc_id,
                            changes,
                            has_more: false, // mirrored TODO: pagination
                            new_heads,
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "iroh SyncChanges failed");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::GetChanges {
                h_app_id,
                doc_id,
                change_hashes,
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, change_hashes = ?change_hashes, "iroh GetChanges");
                // Mirrored TODO: selective fetch not implemented; full sync.
                match self
                    .sync_manager
                    .get_changes_since(&h_app_id, &doc_id, &[])
                    .await
                {
                    Ok((changes, _)) => {
                        let changes_with_hashes: Vec<(String, Vec<u8>)> = changes
                            .into_iter()
                            .map(|c| {
                                let mut hasher = Sha256::new();
                                hasher.update(&c);
                                let result = hasher.finalize();
                                let hash = hex::encode(&result[..8]);
                                (hash, c)
                            })
                            .collect();
                        SyncResponse::RequestedChanges {
                            h_app_id,
                            doc_id,
                            changes: changes_with_hashes,
                            not_found: vec![],
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "iroh GetChanges failed");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::AnnounceChange {
                h_app_id,
                doc_id,
                change_hash,
                change_data,
            } => {
                self.handle_announce(peer, h_app_id, doc_id, change_hash, change_data)
                    .await
            }
            SyncRequest::ListDocuments {
                h_app_id,
                prefix,
                offset,
                limit,
            } => {
                debug!(h_app_id = %h_app_id, prefix = ?prefix, offset = offset, limit = limit, "iroh ListDocuments");
                match self
                    .sync_manager
                    .list_documents(&h_app_id, prefix.as_deref(), offset, limit)
                    .await
                {
                    Ok((docs, total)) => {
                        let documents: Vec<DocumentInfo> = docs
                            .into_iter()
                            .map(|d| DocumentInfo {
                                doc_id: d.doc_id,
                                doc_type: d.doc_type,
                                change_count: d.change_count,
                                last_modified: d.last_modified,
                                heads: d.heads,
                            })
                            .collect();
                        let has_more = (offset as u64 + documents.len() as u64) < total;
                        SyncResponse::DocumentList {
                            h_app_id,
                            documents,
                            total,
                            has_more,
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, error = %e, "iroh ListDocuments failed");
                        SyncResponse::Error {
                            message: format!("Failed to list documents: {}", e),
                        }
                    }
                }
            }
            SyncRequest::ListDocumentsSince {
                h_app_id,
                prefix,
                corpus_digest,
                limit,
            } => {
                // Mirrors the libp2p arm exactly (see `p2p/mod.rs`): equal
                // digests ⇒ InSync and NO enumeration; different ⇒ the same full
                // list the ListDocuments arm above would have produced. Both
                // transports must agree or a dual-stack fleet would converge on
                // one plane and re-enumerate forever on the other.
                match self
                    .sync_manager
                    .list_documents(
                        &h_app_id,
                        prefix.as_deref(),
                        0,
                        crate::p2p::sync_round::SYNC_LIST_PAGE_LIMIT,
                    )
                    .await
                {
                    Ok((docs, total)) => {
                        let local = crate::p2p::sync_round::LocalCorpusState {
                            docs: docs
                                .iter()
                                .map(|d| crate::p2p::sync_round::DocHead {
                                    doc_id: d.doc_id.clone(),
                                    heads: d.heads.clone(),
                                })
                                .collect(),
                        };
                        let ours = crate::p2p::sync_round::corpus_digest(&local);
                        if ours == corpus_digest {
                            debug!(
                                h_app_id = %h_app_id,
                                digest = %ours,
                                docs = local.len(),
                                "iroh ListDocumentsSince: digests match — InSync"
                            );
                            SyncResponse::InSync {
                                h_app_id,
                                corpus_digest: ours,
                            }
                        } else {
                            debug!(
                                h_app_id = %h_app_id,
                                ours = %ours,
                                theirs = %corpus_digest,
                                "iroh ListDocumentsSince: digests differ — full enumeration"
                            );
                            let documents: Vec<DocumentInfo> = docs
                                .into_iter()
                                .take(limit as usize)
                                .map(|d| DocumentInfo {
                                    doc_id: d.doc_id,
                                    doc_type: d.doc_type,
                                    change_count: d.change_count,
                                    last_modified: d.last_modified,
                                    heads: d.heads,
                                })
                                .collect();
                            let has_more = (documents.len() as u64) < total;
                            SyncResponse::DocumentList {
                                h_app_id,
                                documents,
                                total,
                                has_more,
                            }
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, error = %e, "iroh ListDocumentsSince failed");
                        SyncResponse::Error {
                            message: format!("Failed to list documents: {}", e),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{DocStore, StreamTracker};
    use tempfile::tempdir;

    async fn fresh_backend(dir: &std::path::Path) -> SyncManagerBackend {
        let doc_store = Arc::new(
            DocStore::at_path(dir.join("sync.sled"))
                .await
                .expect("doc store"),
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        let sync_manager = Arc::new(SyncManager::new(doc_store, stream_tracker));
        SyncManagerBackend::new(sync_manager)
    }

    #[tokio::test]
    async fn get_heads_for_unknown_doc_returns_empty() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        let res = backend
            .handle(SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: "missing".into(),
            })
            .await;

        match res {
            SyncResponse::Heads {
                h_app_id,
                doc_id,
                heads,
                change_count,
            } => {
                assert_eq!(h_app_id, "lamad");
                assert_eq!(doc_id, "missing");
                assert!(heads.is_empty());
                assert_eq!(change_count, 0);
            }
            other => panic!("expected Heads, got {other:?}"),
        }
    }

    /// Build a real Automerge change blob via the same primitives the libp2p
    /// side uses, so the wire payload is genuinely runnable. Returns
    /// (bytes, change_hash) — the hash is what the announce ADDRESSES, and the
    /// receive arm now verifies against it.
    fn one_change(key: &str, value: &str) -> (Vec<u8>, String) {
        let mut doc = automerge::Automerge::new();
        let mut tx = doc.transaction();
        automerge::transaction::Transactable::put(&mut tx, automerge::ROOT, key, value).unwrap();
        tx.commit();
        let head = hex::encode(doc.get_heads()[0].0);
        (doc.save(), head)
    }

    #[tokio::test]
    async fn announce_then_get_heads_round_trips() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        let (change_blob, change_hash) = one_change("k", "v");

        let ack = backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-roundtrip".into(),
                change_hash,
                change_data: Some(change_blob),
            })
            .await;
        match ack {
            SyncResponse::ChangeAck { was_new, .. } => assert!(
                was_new,
                "a dependency-free change addressed by its REAL hash must land"
            ),
            other => panic!("expected ChangeAck, got {other:?}"),
        }

        let heads = backend
            .handle(SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: "doc-roundtrip".into(),
            })
            .await;
        match heads {
            SyncResponse::Heads { heads, .. } => {
                assert!(!heads.is_empty(), "expected heads after announce");
            }
            other => panic!("expected Heads, got {other:?}"),
        }
    }

    /// An announce whose `change_hash` we cannot parse is applied but NOT
    /// claimed as landed.
    ///
    /// The landing check asks the doc "do you now hold change X?"; an
    /// unparseable X makes that question unanswerable, and `get_change_by_hash`
    /// returns `Ok(None)` rather than erroring. The honest answer is
    /// `was_new: false` plus a fallback pull — never `true` on faith, because
    /// `was_new: true` is what tells the announcer propagation completed. The
    /// bytes are still applied, so this degrades to redundant work, never to
    /// loss. (Every real announcer sends the hash the producer named; this pins
    /// the behaviour for one that does not.)
    #[tokio::test]
    async fn an_unverifiable_change_hash_applies_but_never_claims_a_landing() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        let (change_blob, _real_hash) = one_change("k", "v");
        let ack = backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-unverifiable".into(),
                change_hash: "not-a-hash".into(),
                change_data: Some(change_blob),
            })
            .await;
        match ack {
            SyncResponse::ChangeAck { was_new, .. } => assert!(
                !was_new,
                "an unverifiable landing must not be claimed as one"
            ),
            other => panic!("expected ChangeAck, got {other:?}"),
        }

        // ...and the bytes DID apply — the degradation is redundant work, not
        // a dropped change.
        match backend
            .handle(SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: "doc-unverifiable".into(),
            })
            .await
        {
            SyncResponse::Heads { heads, .. } => {
                assert!(!heads.is_empty(), "the change must still have been applied")
            }
            other => panic!("expected Heads, got {other:?}"),
        }
    }

    /// An oversized payload is REFUSED without applying: the sender bounds its
    /// own fan-out, and a peer that ignores the bound must not be able to make
    /// us allocate past it. Propagation is preserved by the pull; amplification
    /// is not.
    #[tokio::test]
    async fn an_oversized_announce_is_refused_without_applying() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        let ack = backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-oversized".into(),
                change_hash: "deadbeef".into(),
                change_data: Some(vec![0u8; MAX_ANNOUNCE_PAYLOAD_BYTES + 1]),
            })
            .await;
        match ack {
            SyncResponse::ChangeAck { was_new, .. } => assert!(!was_new),
            other => panic!("expected ChangeAck, got {other:?}"),
        }
        match backend
            .handle(SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: "doc-oversized".into(),
            })
            .await
        {
            SyncResponse::Heads { heads, .. } => assert!(
                heads.is_empty(),
                "an over-bound payload must never be applied"
            ),
            other => panic!("expected Heads, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_documents_for_empty_namespace() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        let res = backend
            .handle(SyncRequest::ListDocuments {
                h_app_id: "lamad".into(),
                prefix: None,
                offset: 0,
                limit: 10,
            })
            .await;
        match res {
            SyncResponse::DocumentList {
                documents,
                total,
                has_more,
                ..
            } => {
                assert!(documents.is_empty());
                assert_eq!(total, 0);
                assert!(!has_more);
            }
            other => panic!("expected DocumentList, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_changes_returns_sha256_truncated_hashes() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        // Seed a document so get_changes_since has bytes to hash. The announce
        // names the change's real hash: the receive arm verifies the landing
        // against it, so a placeholder here would exercise the fallback path
        // instead of the seed this test wants.
        let (change_blob, change_hash) = one_change("seed", "42");

        backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-hash".into(),
                change_hash,
                change_data: Some(change_blob),
            })
            .await;

        let res = backend
            .handle(SyncRequest::GetChanges {
                h_app_id: "lamad".into(),
                doc_id: "doc-hash".into(),
                change_hashes: vec![],
            })
            .await;

        match res {
            SyncResponse::RequestedChanges { changes, .. } => {
                assert!(!changes.is_empty(), "expected at least one change");
                for (hash, _bytes) in &changes {
                    assert_eq!(hash.len(), 16, "expected first-8-bytes hex hash");
                }
            }
            other => panic!("expected RequestedChanges, got {other:?}"),
        }
    }
}
