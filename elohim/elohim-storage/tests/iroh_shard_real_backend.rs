//! Phase 11 acceptance — shard ALPN over iroh QUIC dispatched into a
//! real [`ShardService`] backend (not the stub used by `iroh_shard_parity`).
//!
//! Proves the wire → backend → blob-store chain end-to-end on iroh:
//! provider mounts [`ShardServiceBackend`] over a real [`ShardService`]
//! holding a real [`BlobStore`]; an iroh client pushes a shard, fetches
//! it back, probes existence, and exercises the no-DB-pool error
//! paths. If the chain breaks anywhere — codec, ALPN dispatch,
//! adapter mapping, ShardService wiring, BlobStore I/O — the assertion
//! blows up.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the shard plane is dual-stack permanent. This test exercises the
//! legacy SHA-256 fetch plane the iroh side maintains for libp2p-
//! fallback peers; the iroh-canonical BLAKE3-streamed plane is
//! exercised separately by `iroh_blob_roundtrip`.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::p2p::shard_protocol::{ShardRequest, ShardResponse};
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohShardClient, IrohShardProtocol,
    ShardBackend, ShardServiceBackend, SHARD_ALPN,
};
use elohim_storage::shard_service::ShardService;
use tempfile::tempdir;

async fn build_real_backend_with_blob_store() -> (Arc<dyn ShardBackend>, Arc<BlobStore>) {
    // Caller manages the BlobStore so the test can pre-seed bytes.
    let dir = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
    // Box::leak the tempdir so its lifetime extends through the test —
    // we don't need to clean up here; the OS reclaims when the process
    // exits. (Alternative: thread the TempDir handle out alongside the
    // backend; chosen this way to keep the helper signature small.)
    Box::leak(Box::new(dir));
    let service = Arc::new(ShardService::new(blob_store.clone(), None));
    let backend: Arc<dyn ShardBackend> = Arc::new(ShardServiceBackend::new(service));
    (backend, blob_store)
}

async fn fixture_with_real_provider_backend(
    provider_dir: &std::path::Path,
    fetcher_dir: &std::path::Path,
) -> Result<(TwoNodeFixture, Arc<BlobStore>)> {
    let (backend, blob_store) = build_real_backend_with_blob_store().await;
    let handler = IrohShardProtocol::new(backend);
    let provider_extras: Vec<AlpnRegistration> = vec![(SHARD_ALPN.to_vec(), Box::new(handler))];
    let fixture =
        TwoNodeFixture::new_asymmetric(provider_dir, provider_extras, fetcher_dir, Vec::new())
            .await?;
    Ok((fixture, blob_store))
}

/// Get against a missing shard returns NotFound — the BlobStore lookup
/// failure must surface unchanged.
#[tokio::test]
async fn get_unknown_shard_via_iroh_returns_not_found() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _blob_store) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::Get {
                hash: "missing-hash".into(),
            },
        )
        .await?;
    match res {
        ShardResponse::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Pre-seed bytes into the provider's BlobStore, then fetch them via
/// iroh — proves the BlobStore handle in ShardService is the same one
/// that received the bytes (i.e. the service didn't get a different
/// store than expected).
#[tokio::test]
async fn pre_seeded_shard_serves_via_iroh_get() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, blob_store) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    // Seed bytes directly into the provider's BlobStore — the
    // ShardService is wrapping THIS instance (not a different one).
    let payload = b"hello shard via iroh".to_vec();
    let stored = blob_store.store(&payload).await.unwrap();
    let hash = stored.hash.clone();

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::Get { hash: hash.clone() },
        )
        .await?;
    match res {
        ShardResponse::Data(bytes) => {
            assert_eq!(
                bytes, payload,
                "bytes from iroh fetch must match what was seeded"
            );
        }
        other => panic!("expected Data, got {other:?}"),
    }

    // Have should also report true.
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::Have { hash },
        )
        .await?;
    match res {
        ShardResponse::Have(true) => {}
        other => panic!("expected Have(true), got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// Push a shard via iroh; provider's BlobStore must contain the bytes
/// after the call returns. Exercises the write side of the chain.
#[tokio::test]
async fn push_via_iroh_writes_to_provider_blob_store() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, blob_store) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    // Compute the hash by storing on a separate temp store first, just
    // so we know the canonical hash for the bytes. (We store on the
    // provider's pool below to verify the push got there.)
    let payload = b"push via iroh".to_vec();
    let probe_dir = tempdir().unwrap();
    let probe_store = BlobStore::new(probe_dir.path().to_path_buf()).await.unwrap();
    let canonical = probe_store.store(&payload).await.unwrap();

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::Push {
                hash: canonical.hash.clone(),
                data: payload.clone(),
            },
        )
        .await?;
    match res {
        ShardResponse::PushAck => {}
        other => panic!("expected PushAck, got {other:?}"),
    }

    // Provider's BlobStore now contains the bytes — verify directly.
    assert!(
        blob_store.exists(&canonical.hash).await,
        "provider BlobStore should contain pushed shard"
    );

    fixture.shutdown().await?;
    Ok(())
}

/// ListContent without a DB pool must return Error("No database pool")
/// — pinning the libp2p-side semantics on the iroh transport.
#[tokio::test]
async fn list_content_without_db_pool_via_iroh_errors() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _blob_store) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::ListContent {
                reach_filter: None,
                offset: 0,
                limit: 10,
            },
        )
        .await?;
    match res {
        ShardResponse::Error(msg) => {
            assert!(
                msg.contains("No database pool"),
                "expected 'No database pool' error; got: {msg}"
            );
        }
        other => panic!("expected Error, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}

/// ListContent with an unknown reach filter must return Error
/// (validates against schema-generated CORE_REACH_LEVELS).
#[tokio::test]
async fn list_content_with_unknown_reach_via_iroh_errors() -> Result<()> {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let (fixture, _blob_store) =
        fixture_with_real_provider_backend(provider_dir.path(), fetcher_dir.path()).await?;

    let client = IrohShardClient::new(fixture.fetcher.endpoint());
    let res = client
        .request(
            fixture.provider_addr.clone(),
            &ShardRequest::ListContent {
                reach_filter: Some("hyper-secret-mega-tier".into()),
                offset: 0,
                limit: 10,
            },
        )
        .await?;
    match res {
        ShardResponse::Error(msg) => {
            assert!(
                msg.contains("Unknown reach level"),
                "expected 'Unknown reach level' error; got: {msg}"
            );
        }
        other => panic!("expected Error, got {other:?}"),
    }

    fixture.shutdown().await?;
    Ok(())
}
