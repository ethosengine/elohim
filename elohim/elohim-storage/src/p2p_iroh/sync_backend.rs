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

use std::sync::Arc;

use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::sync::SyncBackend;
use crate::p2p::sync_protocol::{DocumentInfo, SyncRequest, SyncResponse};
use crate::sync::SyncManager;

/// Routes [`SyncRequest`] variants into a shared [`SyncManager`] and produces
/// the matching [`SyncResponse`]. Keep behavior in lockstep with the libp2p
/// handler at `src/p2p/mod.rs::P2PNode::handle_sync_request` — wire bytes are
/// the contract, transport is the variable.
pub struct SyncManagerBackend {
    sync_manager: Arc<SyncManager>,
}

impl SyncManagerBackend {
    pub fn new(sync_manager: Arc<SyncManager>) -> Self {
        Self { sync_manager }
    }
}

impl std::fmt::Debug for SyncManagerBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncManagerBackend").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl SyncBackend for SyncManagerBackend {
    async fn handle(&self, request: SyncRequest) -> SyncResponse {
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
                change_hash: _,
                change_data,
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, "iroh AnnounceChange");
                if let Some(data) = change_data {
                    match self
                        .sync_manager
                        .apply_changes(&h_app_id, &doc_id, vec![data])
                        .await
                    {
                        Ok(_) => {
                            info!(h_app_id = %h_app_id, doc_id = %doc_id, "iroh applied announced change");
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
                } else {
                    SyncResponse::ChangeAck {
                        h_app_id,
                        doc_id,
                        was_new: false,
                    }
                }
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

    #[tokio::test]
    async fn announce_then_get_heads_round_trips() {
        let dir = tempdir().unwrap();
        let backend = fresh_backend(dir.path()).await;

        // Build a real Automerge change blob via the same primitives the
        // libp2p side uses, so the wire payload is genuinely runnable.
        let mut doc = automerge::Automerge::new();
        let mut tx = doc.transaction();
        automerge::transaction::Transactable::put(&mut tx, automerge::ROOT, "k", "v").unwrap();
        tx.commit();
        let change_blob = doc.save();

        let ack = backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-roundtrip".into(),
                change_hash: "ignored".into(),
                change_data: Some(change_blob),
            })
            .await;
        match ack {
            SyncResponse::ChangeAck { was_new, .. } => assert!(was_new),
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

        // Seed a document so get_changes_since has bytes to hash.
        let mut doc = automerge::Automerge::new();
        let mut tx = doc.transaction();
        automerge::transaction::Transactable::put(&mut tx, automerge::ROOT, "seed", 42i64).unwrap();
        tx.commit();
        let change_blob = doc.save();

        backend
            .handle(SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-hash".into(),
                change_hash: "ignored".into(),
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
