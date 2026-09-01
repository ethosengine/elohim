//! Production iroh sync-round driver: a hand-filled peer book drives
//! ListDocuments -> SyncChanges -> apply_changes between two endpoints.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use automerge::transaction::Transactable;
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, run_iroh_sync_round, AlpnRegistration, IrohPeerBook,
    IrohPeerEntry, IrohSyncProtocol, SyncBackend, SyncManagerBackend, SYNC_ALPN,
};
use elohim_storage::sync::{projector::PROJECTION_NAMESPACE, DocStore, StreamTracker, SyncManager};
use tempfile::tempdir;

async fn sync_manager(dir: &std::path::Path) -> Arc<SyncManager> {
    let doc_store = Arc::new(
        DocStore::at_path(dir.join("sync.sled"))
            .await
            .expect("doc store"),
    );
    Arc::new(SyncManager::new(doc_store, Arc::new(StreamTracker::new())))
}

fn content_doc(blob_hash: &str) -> Vec<u8> {
    let mut doc = automerge::Automerge::new();
    let mut tx = doc.transaction();
    tx.put(automerge::ROOT, "blobHash", blob_hash).unwrap();
    tx.commit();
    doc.save()
}

#[tokio::test]
async fn hand_filled_peer_book_converges_a_to_bs_document() -> Result<()> {
    let provider_dir = tempdir()?;
    let provider_sync_dir = tempdir()?;
    let fetcher_dir = tempdir()?;
    let fetcher_sync_dir = tempdir()?;

    let provider_sync = sync_manager(provider_sync_dir.path()).await;
    provider_sync
        .apply_changes(
            PROJECTION_NAMESPACE,
            "node:iroh-driver-proof",
            vec![content_doc("blake3-driver-proof")],
        )
        .await?;
    let backend: Arc<dyn SyncBackend> = Arc::new(SyncManagerBackend::new(provider_sync.clone()));
    let provider_protocols: Vec<AlpnRegistration> =
        vec![(SYNC_ALPN.to_vec(), Box::new(IrohSyncProtocol::new(backend)))];
    let fixture = TwoNodeFixture::new_asymmetric(
        provider_dir.path(),
        provider_protocols,
        fetcher_dir.path(),
        Vec::new(),
    )
    .await?;

    let peer_book = IrohPeerBook::new();
    assert!(peer_book.upsert(IrohPeerEntry {
        addr: fixture.provider_addr.clone(),
        agent_cid: Some("provider-b".into()),
        libp2p_peer_id: None,
        user_agent: None,
        announced_at_ms: 1,
    }));
    let fetcher_sync = sync_manager(fetcher_sync_dir.path()).await;

    let rounds_before = elohim_storage::metrics::IROH_SYNC_ROUNDS.get();
    let changes_before = elohim_storage::metrics::IROH_SYNC_CHANGES_APPLIED.get();
    run_iroh_sync_round(fixture.fetcher.endpoint(), &peer_book, &fetcher_sync, None).await;

    assert_eq!(
        fetcher_sync
            .get_doc_field(PROJECTION_NAMESPACE, "node:iroh-driver-proof", "blobHash")
            .await?,
        "blake3-driver-proof"
    );
    assert!(elohim_storage::metrics::IROH_SYNC_ROUNDS.get() > rounds_before);
    assert!(elohim_storage::metrics::IROH_SYNC_CHANGES_APPLIED.get() > changes_before);

    fixture.shutdown().await?;
    Ok(())
}
