//! Phase 11 acceptance — sync ALPN over iroh QUIC dispatched into a real
//! [`SyncManager`] backend (not the stub used by `iroh_sync_parity`).
//!
//! This proves the wire → backend → DocStore chain end-to-end on iroh:
//! provider mounts [`SyncManagerBackend`] over a real [`SyncManager`]; an
//! Automerge change is announced via the iroh client; the next GetHeads
//! request returns the heads the underlying DocStore actually wrote. If
//! the chain breaks anywhere — codec, ALPN dispatch, adapter mapping,
//! SyncManager wiring — the assertion blows up.
//!
//! Companion to `iroh_sync_parity` (which uses a fixed stub backend to
//! pin wire-byte parity). Together they cover wire framing AND production
//! dispatch.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::sync_protocol::{SyncRequest, SyncResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohSyncClient, IrohSyncProtocol,
    SyncBackend, SyncManagerBackend, SYNC_ALPN,
};
use elohim_storage::sync::{DocStore, StreamTracker, SyncManager};
use tempfile::tempdir;

async fn build_real_backend(dir: &std::path::Path) -> Arc<dyn SyncBackend> {
    let doc_store = Arc::new(
        DocStore::at_path(dir.join("sync.sled"))
            .await
            .expect("doc store"),
    );
    let stream_tracker = Arc::new(StreamTracker::new());
    let sync_manager = Arc::new(SyncManager::new(doc_store, stream_tracker));
    Arc::new(SyncManagerBackend::new(sync_manager))
}

/// Provider mounts a real SyncManagerBackend; fetcher mounts nothing
/// (it only initiates client requests).
async fn fixture_with_real_provider_backend(
    provider_dir: &std::path::Path,
    sync_state_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<TwoNodeFixture> {
    let backend = build_real_backend(sync_state_dir).await;
    let handler = IrohSyncProtocol::new(backend);
    let provider_extras: Vec<AlpnRegistration> = vec![(SYNC_ALPN.to_vec(), Box::new(handler))];
    TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new()).await
}

/// Build a real Automerge change blob — same primitives the libp2p side
/// emits, so the wire payload is genuinely runnable through Automerge.
fn make_change_blob(key: &str, value: &str) -> Vec<u8> {
    let mut doc = automerge::Automerge::new();
    let mut tx = doc.transaction();
    automerge::transaction::Transactable::put(&mut tx, automerge::ROOT, key, value).unwrap();
    tx.commit();
    doc.save()
}

/// Round-trip an AnnounceChange + GetHeads through the iroh ALPN against
/// a real SyncManager backend. Heads must reflect what the DocStore wrote.
#[tokio::test]
async fn announce_then_get_heads_via_iroh_dispatches_to_real_backend() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let backend_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture = fixture_with_real_provider_backend(
        provider_dir.path(),
        backend_dir.path(),
        fetcher_dir.path(),
    )
    .await?;

    let change_blob = make_change_blob("k1", "v1");
    let client = IrohSyncClient::new(fixture.fetcher.endpoint());

    // 1. Announce a change to the provider — must reach the real backend.
    let ack = client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-real".into(),
                change_hash: "ignored".into(),
                change_data: Some(change_blob),
            },
        )
        .await?;
    match ack {
        SyncResponse::ChangeAck {
            h_app_id,
            doc_id,
            was_new,
        } => {
            assert_eq!(h_app_id, "lamad");
            assert_eq!(doc_id, "doc-real");
            assert!(
                was_new,
                "real backend should report was_new=true on first apply"
            );
        }
        other => panic!("expected ChangeAck, got {other:?}"),
    }

    // 2. GetHeads against the same doc — must return non-empty heads
    // because the DocStore wrote them in step 1.
    let heads_resp = client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::GetHeads {
                h_app_id: "lamad".into(),
                doc_id: "doc-real".into(),
            },
        )
        .await?;
    match heads_resp {
        SyncResponse::Heads {
            h_app_id,
            doc_id,
            heads,
            change_count: _,
        } => {
            assert_eq!(h_app_id, "lamad");
            assert_eq!(doc_id, "doc-real");
            assert!(
                !heads.is_empty(),
                "real backend should return heads after AnnounceChange — got empty"
            );
            for h in &heads {
                assert_eq!(
                    h.len(),
                    64,
                    "Automerge heads are 32-byte hashes hex-encoded"
                );
            }
        }
        other => panic!("expected Heads, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// SyncChanges should return the bytes the DocStore stored, not an empty
/// or stub response. Catches "adapter dispatched but didn't read DocStore."
#[tokio::test]
async fn sync_changes_returns_real_bytes_via_iroh() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let backend_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture = fixture_with_real_provider_backend(
        provider_dir.path(),
        backend_dir.path(),
        fetcher_dir.path(),
    )
    .await?;

    let change_blob = make_change_blob("k2", "v2");
    let client = IrohSyncClient::new(fixture.fetcher.endpoint());

    client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-bytes".into(),
                change_hash: "ignored".into(),
                change_data: Some(change_blob),
            },
        )
        .await?;

    let resp = client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::SyncChanges {
                h_app_id: "lamad".into(),
                doc_id: "doc-bytes".into(),
                have_heads: vec![], // we have nothing — give us everything
                bloom_filter: None,
            },
        )
        .await?;

    match resp {
        SyncResponse::Changes {
            h_app_id,
            doc_id,
            changes,
            has_more,
            new_heads,
        } => {
            assert_eq!(h_app_id, "lamad");
            assert_eq!(doc_id, "doc-bytes");
            assert!(
                !changes.is_empty(),
                "expected real change bytes from DocStore"
            );
            assert!(!has_more, "single-blob fits in one response");
            assert!(!new_heads.is_empty(), "doc has heads after Announce");
        }
        other => panic!("expected Changes, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// ListDocuments after AnnounceChange must show the doc the backend wrote.
#[tokio::test]
async fn list_documents_via_iroh_sees_announced_doc() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let backend_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let fixture = fixture_with_real_provider_backend(
        provider_dir.path(),
        backend_dir.path(),
        fetcher_dir.path(),
    )
    .await?;

    let client = IrohSyncClient::new(fixture.fetcher.endpoint());

    client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::AnnounceChange {
                h_app_id: "lamad".into(),
                doc_id: "doc-listed".into(),
                change_hash: "ignored".into(),
                change_data: Some(make_change_blob("k3", "v3")),
            },
        )
        .await?;

    let resp = client
        .request(
            fixture.provider_addr.clone(),
            &SyncRequest::ListDocuments {
                h_app_id: "lamad".into(),
                prefix: None,
                offset: 0,
                limit: 50,
            },
        )
        .await?;

    match resp {
        SyncResponse::DocumentList {
            h_app_id,
            documents,
            total,
            has_more: _,
        } => {
            assert_eq!(h_app_id, "lamad");
            assert!(total >= 1, "expected at least one doc, got total={total}");
            assert!(
                documents.iter().any(|d| d.doc_id == "doc-listed"),
                "expected doc-listed in {:?}",
                documents.iter().map(|d| &d.doc_id).collect::<Vec<_>>()
            );
        }
        other => panic!("expected DocumentList, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
